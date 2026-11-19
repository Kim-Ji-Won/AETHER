use super::*;
use super::super::{credential::mock::{MockResolver, Sample}, fixtures};
use std::io::{BufRead, BufReader, Write};

type Driver = OfflineProvider<MockResolver>;

fn admitted() -> (Driver, UiSession, Identity) {
    let mut hp = fixtures::host_policy();
    hp.run_ms = 15_000;
    let mut p = OfflineProvider::with_resolver("fixture", hp, fixtures::policy(), MockResolver::Missing).unwrap();
    let session = p.attach("fixture", 0, true).unwrap();
    let start = Start {session:session.clone(), request_id:1, text:"fixture only".into()};
    p.register_credential(Sample::First).unwrap();
    p.approve_fixture(&start, "fixture-model", 0).unwrap();
    let id = p.start_consented(start, "fixture-model", 0).unwrap();
    (p, session, id)
}

fn head(status:u16, sse:bool, length:Option<usize>) -> Step { Step::Head(Head{status,sse,length}) }

fn memory(p:&mut Driver, id:&Identity, steps:Vec<Step>) {
    p.begin_transfer(id,0, |_,_| Ok(Reader::Memory(steps.into_iter().map(Ok).collect()))).unwrap();
}

#[test]
fn production_dispatch_and_nonconsented_reader_stay_closed() {
    for field in ["url", "headers", "authorization", "proxy"] {
        let mut body = serde_json::json!({"namespace":NAMESPACE,"host_instance_id":"fixture",
            "ui_context_epoch":1,"request_id":1,"text":"fixture"});
        body[field] = "renderer-controlled".into();
        assert_eq!(Start::parse(&tauri::ipc::InvokeBody::Json(body),128),Err(ErrorCode::InvalidInput));
    }
    let (mut p,_,id) = admitted();
    assert_eq!(p.dispatch_http(&id,0),Err(ErrorCode::ProviderDisabled));
    let (mut p,s) = fixtures::setup();
    let id = fixtures::start(&mut p,&s,1);
    assert_eq!(p.begin_transfer(&id,0,|_,_| panic!("unconsented open")),Err(ErrorCode::PermissionDenied));
    assert_eq!(p.preview().unwrap().endpoint(),"https://api.openai.com/v1/responses");
    assert!(matches!(http::Http::open(p.preview().unwrap(), Duration::from_secs(1),
        http::Destination::Loopback("127.0.0.2:443".parse().unwrap(),None)),Err(ProviderError::Permission)));
    let max = serde_json::Value::String("a".repeat(MAX_REQUEST-2));
    assert_eq!(encode_request(&max).unwrap().len(), MAX_REQUEST);
    assert_eq!(encode_request(&serde_json::Value::String("a".repeat(MAX_REQUEST-1))),Err(ErrorCode::InputLimit));
}

#[test]
fn slow_consumer_preserves_bytes_and_timeout_closes_reader() {
    let (mut p,_,id)=admitted();
    memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::all(&fixtures::events("partial")[..5]))]);
    p.poll_http(&id,0,false).unwrap();
    p.poll_http(&id,1,true).unwrap();
    p.poll_http(&id,2,false).unwrap();
    assert_eq!(p.inspect().run.unwrap().text,"");
    p.poll_http(&id,3,true).unwrap();
    assert_eq!(p.inspect().run.unwrap().text,"partial");
    p.poll_http(&id,103,false).unwrap();
    let r=p.inspect().run.unwrap();
    assert_eq!(r.error.unwrap().code,ErrorCode::IdleTimeout);
    assert_eq!(r.text,"partial");
    assert_eq!(r.terminal_count,1);
    assert!(p.local_transport_closed());
    p.confirm_runtime_cleanup(&id,true).unwrap();
}

#[test]
fn native_reader_is_exclusive_and_cancel_discards_buffered_completion() {
    let (mut p,s,id)=admitted();
    memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::all(&fixtures::events("late")))]);
    assert_eq!(p.feed(&id,b"unbound",0),Err(ErrorCode::InvalidIdentity));
    p.cancel(&s,&id,0).unwrap();
    assert!(p.confirm_local_abort(&id,true).is_err());
    assert!(p.confirm_runtime_cleanup(&id,true).is_err());
    p.poll_http(&id,0,false).unwrap();
    assert_eq!(p.inspect().run.unwrap().state,State::CancelRequested);
    p.confirm_runtime_cleanup(&id,true).unwrap();
    p.confirm_runtime_cleanup(&id,true).unwrap();
    let r=p.inspect().run.unwrap();
    assert_eq!((r.state,r.terminal_count,r.text.as_str()),(State::Cancelled,1,""));
    assert_eq!(p.poll_http(&id,0,true),Err(ErrorCode::AlreadyTerminal));
    assert!(!p.accounting().unwrap().billing_confirmed);
}

#[test]
fn revoked_reader_waits_for_both_owners_and_cleanup_deadline_is_immutable() {
    let (mut p,_,id)=admitted();
    memory(&mut p,&id,vec![]);
    p.revoke_credential().unwrap();
    assert_eq!(p.revocation(),consent::Revocation::AwaitingCleanup);
    p.poll_http(&id,51,false).unwrap();
    let r=p.inspect().run.unwrap();
    assert_eq!(r.error.unwrap().code,ErrorCode::CleanupUnconfirmed);
    assert_eq!(p.revocation(),consent::Revocation::AwaitingCleanup);
    p.confirm_runtime_cleanup(&id,true).unwrap();
    assert_ne!(p.revocation(),consent::Revocation::AwaitingCleanup);
    assert_eq!(p.inspect().run.unwrap().terminal_count,1);
}

#[test]
fn runtime_loss_and_shutdown_discard_late_input() {
    for lost in [true,false] {
        let (mut p,_,id)=admitted();
        memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::all(&fixtures::events("late")))]);
        if lost {
            p.runtime_lost(1).unwrap();
            assert!(p.poll_http(&id,1,true).is_err());
            assert!(p.confirm_runtime_cleanup(&id,true).is_err());
            p.confirm_lost_owned_cleanup().unwrap();
        } else {
            p.shutdown(1).unwrap();
            p.poll_http(&id,1,false).unwrap();
            p.confirm_runtime_cleanup(&id,true).unwrap();
        }
        let r=p.inspect().run.unwrap();
        assert_eq!(r.terminal_count,1);
        assert!(p.local_transport_closed() && r.text.is_empty());
    }
}

#[test]
fn errors_cross_only_the_bounded_native_classification() {
    for (first, body, expected) in [
        (head(200,false,None),vec![],ProviderError::UnsupportedContent),
        (head(200,true,Some(65537)),vec![],ProviderError::BufferLimit),
        (head(307,false,None),vec![],ProviderError::Unknown),
        (head(429,false,None),br#"{"error":{"code":"insufficient_quota","message":"private-fixture"}}"#.to_vec(),ProviderError::Quota),
        (head(429,false,None),vec![65;MAX_ERROR+1],ProviderError::Unknown429),
        (head(200,true,None),vec![0;CHUNK+1],ProviderError::BufferLimit),
        (head(200,true,None),vec![],ProviderError::Incomplete),
    ] {
        let (mut p,_,id)=admitted();
        memory(&mut p,&id,vec![first,Step::Bytes(body),Step::End]);
        for _ in 0..3 { if p.poll_http(&id,0,true).is_err() {break;} }
        assert_eq!(p.last_error(),Some(expected));
        assert!(!format!("{:?}",p.inspect()).contains("private-fixture"));
        assert!(p.local_transport_closed());
        assert_eq!(p.inspect().run.unwrap().terminal_count,1);
    }
}

#[test]
fn metadata_counts_toward_event_budget_before_mapping() {
    let (mut p,_,id)=admitted();
    memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::bytes(&fixtures::events("x")[0]))]);
    p.stream.as_mut().unwrap().events=MAX_EVENTS;
    p.poll_http(&id,0,true).unwrap();
    assert_eq!(p.poll_http(&id,0,true),Err(ErrorCode::BufferLimit));
    assert_eq!(p.inspect().run.unwrap().terminal_count,1);
}

#[test]
fn usage_subsets_are_validated_without_double_charging_reasoning() {
    for (cached,reasoning,valid) in [(2,1,true),(3,1,false),(2,2,false)] {
        let (mut p,_,id)=admitted();
        let mut events=fixtures::events("fixture");
        let usage=&mut events[8]["response"]["usage"];
        usage["input_tokens_details"]["cached_tokens"]=cached.into();
        usage["output_tokens_details"]["reasoning_tokens"]=reasoning.into();
        memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::all(&events))]);
        p.poll_http(&id,0,true).unwrap();
        assert_eq!(p.poll_http(&id,0,true).is_ok(),valid);
        p.confirm_runtime_cleanup(&id,true).unwrap();
        let accounting=p.accounting().unwrap();
        assert!(!accounting.billing_confirmed);
        if valid {
            assert_eq!(accounting.reported_cost_units,Some(7));
        } else {
            assert_eq!(p.last_error(),Some(ProviderError::UsageInvalid));
            assert!(accounting.reported_usage.is_none());
        }
    }
}

#[test]
fn duplicate_read_keeps_partial_output_and_first_failure_after_ack() {
    let (mut p,_,id)=admitted();
    let e=fixtures::events("once");
    memory(&mut p,&id,vec![head(200,true,None),Step::Bytes(fixtures::all(&e[..5])),Step::Bytes(fixtures::bytes(&e[4]))]);
    p.poll_http(&id,0,true).unwrap();
    p.poll_http(&id,0,true).unwrap();
    assert_eq!(p.poll_http(&id,0,true),Err(ErrorCode::DuplicateEvent));
    p.confirm_runtime_cleanup(&id,true).unwrap();
    let r=p.inspect().run.unwrap();
    assert_eq!(r.error.unwrap().code,ErrorCode::ProtocolError);
    assert_eq!((r.terminal_count,r.text.as_str()),(1,"once"));
}

fn tls_peer(mode:&str) {
    use std::process::{Command,Stdio};
    let (mut p,s,id)=admitted();
    let messages=fixtures::events("테스트");
    let payload=if mode=="cancel" {fixtures::all(&messages[..5])}else{fixtures::all(&messages)};
    let mut child=Command::new("/usr/bin/python3").args(["-I","-B","-c",include_str!("loopback.py"),mode])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut stdin=child.stdin.take().unwrap();
    for byte in payload {write!(stdin,"{byte:02x}").unwrap();}
    writeln!(stdin,"\n{}",p.preview().unwrap().body()).unwrap();
    drop(stdin);
    let mut output=BufReader::new(child.stdout.take().unwrap());
    let mut line=String::new();
    output.read_line(&mut line).unwrap();
    let (port,ca)=line.trim_end().split_once(' ').expect("local TLS peer readiness");
    let addr=format!("127.0.0.1:{port}").parse().unwrap();
    let ca=(mode!="untrusted").then(||std::path::PathBuf::from(ca));
    p.begin_transfer(&id,0,|req,timeout|http::Http::open(req,timeout,http::Destination::Loopback(addr,ca)).map(Reader::Http)).unwrap();
    let start=std::time::Instant::now();
    while !p.local_transport_closed() {
        assert!(start.elapsed()<Duration::from_secs(10),"loopback deadline");
        let _=p.poll_http(&id,0,true);
        if mode=="cancel" && p.inspect().run.unwrap().text=="테스트" && !p.abort_requested() {
            p.cancel(&s,&id,0).unwrap();
        }
        if let Some(Reader::Http(reader))=p.transport.as_ref().and_then(|t|t.reader.as_ref()) {
            reader.wait_for_io();
        }
    }
    line.clear();
    output.read_line(&mut line).unwrap();
    assert!(child.wait().unwrap().success());
    if matches!(mode,"hostname"|"untrusted") {
        assert_eq!(line.trim(),"NO_HTTP");
        assert_eq!(p.last_error(),Some(ProviderError::Offline));
    } else {
        assert_eq!(line.trim(),"ONE_REQUEST_PEER_CLOSED");
    }
    if mode=="success" {assert_eq!(p.inspect().run.unwrap().terminal_count,0);}
    p.confirm_runtime_cleanup(&id,true).unwrap();
    let r=p.inspect().run.unwrap();
    assert_eq!(r.terminal_count,1);
    match mode {
        "success"=>assert_eq!((r.state,r.text.as_str()),(State::Succeeded,"테스트")),
        "cancel"=>assert_eq!((r.state,r.text.as_str()),(State::Cancelled,"테스트")),
        _=>assert_eq!(r.state,State::Failed),
    }
}
#[test] fn loopback_verified_stream_and_peer_close(){tls_peer("success")}
#[test] fn loopback_cancel_closes_socket_before_runtime_ack(){tls_peer("cancel")}
#[test] fn loopback_redirect_is_not_followed(){tls_peer("redirect")}
#[test] fn loopback_untrusted_ca_rejected_before_http(){tls_peer("untrusted")}
#[test] fn loopback_wrong_hostname_rejected_before_http(){tls_peer("hostname")}
