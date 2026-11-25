use super::*;
use contract::provider::{live_dispatch, policy::{ModelPolicy, OfflinePolicy}};
use crate::agent_runtime::provider::FixtureProducer;

#[path = "../../agent_text/provider/fixture_events.rs"]
mod fixture;

const MODEL: &str = "fixture-model";

fn app() -> Runtime {
    let mut r = ready();
    r.provider_budget = Some(ProviderBudget::new(OfflinePolicy {
        models: vec![ModelPolicy { id:MODEL.into(), max_input_units:128, max_output_tokens:64,
            input_unit_price:2, output_unit_price:3 }],
        total_cost_units:5000, max_requests_per_window:8, rate_window_ms:1000,
        max_sse_frame_bytes:8192, max_stream_bytes:65536,
    }).unwrap());
    r
}

fn request(r: &Runtime, request_id: u32) -> Start {
    Start { session:r.host.inspect().session, request_id, text:"fixture".into() }
}

fn admit(r: &mut Runtime, n: u32) -> Result<Identity, ErrorCode> {
    r.start_provider(request(r,n), MODEL, r.host.now_ms())
}

fn dispatch(r: &mut Runtime, id: &Identity) -> Result<FixtureProducer, ErrorCode> {
    r.dispatch_provider(id, r.host.now_ms())
}

fn streaming() -> (Runtime, Identity, FixtureProducer) {
    let mut r = app();
    let id = admit(&mut r,1).unwrap();
    let producer = dispatch(&mut r,&id).unwrap();
    (r,id,producer)
}

fn synthetic_start(r: &mut Runtime, n: u32, lease: &str) -> Result<(),ErrorCode> {
    let req = request(r,n);
    r.start(&InvokeBody::Json(serde_json::json!({"namespace":NAMESPACE,"host_instance_id":"host",
        "ui_context_epoch":req.session.ui_context_epoch,"request_id":n,"text":"synthetic"})),lease.into())
}

#[test]
fn alternating_sources_share_one_identity_sequence_and_revision() {
    let mut r = app(); start(&mut r);
    let first = r.host.inspect();
    cancel(&mut r);
    let id = admit(&mut r,2).unwrap();
    let p = dispatch(&mut r,&id).unwrap();
    assert!(r.view().2.is_none());
    assert_eq!(r.host.inspect().source_kind,"provider");
    assert!(r.host.inspect().snapshot_revision > first.snapshot_revision);
    p.send(fixture::all(&fixture::events("한글"))).unwrap();
    r.native_tick(10).unwrap();
    let accepted = r.host.inspect();
    let run = accepted.run.unwrap();
    assert_eq!((run.state,run.cleanup,run.terminal_count),(State::Running,Cleanup::Pending,0));
    assert_eq!(run.text,"한글");
    assert_eq!(p.send(vec![0]), Err(ErrorCode::InvalidIdentity));
    assert_eq!(synthetic_start(&mut r,3,"next"),Err(ErrorCode::Busy));
    drop(p);
    r.native_tick(11).unwrap();
    assert_eq!(r.host.current().unwrap().state,State::Succeeded);
    assert_eq!(r.host.current().unwrap().terminal_count,1);
    assert!(r.host.events().iter().all(|e|e.source_kind=="provider" && e.identity==id));
    synthetic_start(&mut r,3,"next").unwrap();
    let next = r.host.inspect();
    assert_eq!(next.source_kind,"synthetic");
    assert!(next.snapshot_revision > accepted.snapshot_revision);
    let last = next.run.unwrap().identity;
    let first = first.run.unwrap().identity;
    assert_eq!((first.run_id,id.run_id,last.run_id),(1,2,3));
    assert_eq!((first.attempt_id,id.attempt_id,last.attempt_id),(1,2,3));
    assert_eq!(first.host_instance_id,last.host_instance_id);
    assert_eq!(first.session_generation,last.session_generation);
}

#[test]
fn denied_admission_preserves_owner_lease_run_and_reservation() {
    let mut r=app(); start(&mut r); control(&mut r,"dispatch").unwrap();
    let before=r.view();
    assert_eq!(admit(&mut r,2),Err(ErrorCode::Busy));
    assert_eq!(r.view(),before);
    assert!(r.provider_budget.as_ref().unwrap().accounting().is_none());
    cancel(&mut r); control(&mut r,"release").unwrap();
    let id=admit(&mut r,2).unwrap();
    let p=dispatch(&mut r,&id).unwrap();
    let before=r.view();
    let reserved=r.provider_budget.as_ref().unwrap().accounting();
    assert_eq!(synthetic_start(&mut r,3,"replacement"),Err(ErrorCode::Busy));
    assert_eq!(admit(&mut r,3),Err(ErrorCode::Busy));
    assert_eq!(r.view(),before);
    assert_eq!(r.provider_budget.as_ref().unwrap().accounting(),reserved);
    p.send(fixture::all(&fixture::events("still running")[..5])).unwrap();
    r.native_tick(1).unwrap();
    assert_eq!(r.host.current().unwrap().text,"still running");
}

#[test]
fn provider_rejects_renderer_and_stale_native_identities_before_parsing() {
    let mut r=app(); start(&mut r); cancel(&mut r);
    let id=admit(&mut r,2).unwrap();
    let before=r.host.inspect();
    for field in ["attempt","generation"] {
        let mut other=id.clone();
        if field=="attempt" {other.attempt_id+=1;} else {other.session_generation+=1;}
        assert!(matches!(dispatch(&mut r,&other),Err(ErrorCode::InvalidIdentity)));
    }
    for lease in ["lease","forged"] {
        for action in ["dispatch","frame","release"] {
            let body=InvokeBody::Json(serde_json::json!({"lease":lease,"action":action,"data":{"malformed":true}}));
            assert_eq!(r.runtime_input(&body),Err(ErrorCode::InvalidIdentity));
        }
    }
    assert!(r.cancel_input(&json("{}")).is_err());
    for field in ["source","approved"] {
        let mut v=serde_json::json!({"namespace":NAMESPACE,"host_instance_id":"host","ui_context_epoch":1,"request_id":3,"text":"x"});
        v[field]=true.into();
        assert_eq!(r.start(&InvokeBody::Json(v),"fake".into()),Err(ErrorCode::InvalidInput));
    }
    assert!(r.view().2.is_none());
    assert_eq!(r.host.inspect(),before);
    assert_eq!(live_dispatch(),Err(ErrorCode::ProviderDisabled));
}

#[test]
fn queued_cancel_has_no_reader_or_context_and_needs_no_ack() {
    let mut r=app();
    let id=admit(&mut r,1).unwrap();
    cancel(&mut r);
    let run=r.host.current().unwrap();
    assert_eq!((run.state,run.cleanup,run.terminal_count),(State::Cancelled,Cleanup::NotRequired,1));
    assert!(dispatch(&mut r,&id).is_err());
    synthetic_start(&mut r,2,"synthetic").unwrap();
}

#[test]
fn cancel_and_completion_barriers_preserve_partial_and_one_terminal() {
    for complete_first in [false,true] {
        let (mut r,_,p)=streaming();
        let events=fixture::events("partial");
        p.send(fixture::all(&events[..5])).unwrap();
        r.native_tick(1).unwrap();
        p.send(fixture::all(&events[5..])).unwrap();
        if complete_first {r.native_tick(2).unwrap();}
        cancel(&mut r);
        r.native_tick(3).unwrap();
        assert_eq!(r.host.current().unwrap().state,State::CancelRequested);
        assert_eq!(p.send(vec![]),Err(ErrorCode::InvalidIdentity));
        drop(p); r.native_tick(4).unwrap();
        let run=r.host.current().unwrap();
        assert_eq!((run.state,run.text.as_str(),run.terminal_count),(State::Cancelled,"partial",1));
    }
}

#[test]
fn timer_expires_idle_or_run_before_reading_buffered_input() {
    for idle in [true,false] {
        let (mut r,_,p)=streaming();
        let events=fixture::events("x");
        p.send(fixture::all(&events[..5])).unwrap();
        r.native_tick(1).unwrap();
        if !idle {
            for n in 1..12 {
                let mut e=events[4].clone();
                e["sequence_number"]=(4+n).into();
                p.send(fixture::bytes(&e)).unwrap(); r.native_tick(n*1000).unwrap();
            }
        }
        p.send(fixture::all(&events[5..])).unwrap();
        let partial=r.host.current().unwrap().text.clone();
        let deadline=if idle {3001} else {12000};
        r.native_tick(deadline).unwrap();
        r.native_tick(deadline+2000).unwrap();
        let run=r.host.current().unwrap();
        let code=if idle {ErrorCode::IdleTimeout} else {ErrorCode::RunTimeout};
        assert_eq!(run.error.as_ref().unwrap().code,code);
        assert_eq!(run.cleanup,Cleanup::Unconfirmed);
        drop(p); r.native_tick(deadline+2001).unwrap();
        let run=r.host.current().unwrap();
        assert_eq!(run.error.as_ref().unwrap().code,code);
        assert_eq!((run.cleanup,run.terminal_count,&run.text),(Cleanup::Confirmed,1,&partial));
    }
}

#[test]
fn reload_discards_old_queue_and_waits_for_native_context_after_page_replacement() {
    let (mut r,id,p)=streaming();
    p.send(fixture::all(&fixture::events("late"))).unwrap();
    r.lost().unwrap(); r.page_replaced().unwrap();
    assert_eq!(admit(&mut r,2),Err(ErrorCode::StaleSession));
    r.attach("host",1,true).unwrap();
    assert!(dispatch(&mut r,&id).is_err());
    assert!(p.send(vec![]).is_err());
    assert_eq!(admit(&mut r,2),Err(ErrorCode::Busy));
    r.native_tick(2000).unwrap();
    assert_eq!(r.host.current().unwrap().cleanup,Cleanup::Unconfirmed);
    drop(p); r.native_tick(2001).unwrap();
    let run=r.host.current().unwrap();
    assert_eq!(run.error.as_ref().unwrap().code,ErrorCode::RuntimeLost);
    assert_eq!((run.text.as_str(),run.terminal_count,run.cleanup),("",1,Cleanup::Confirmed));
    assert_eq!(admit(&mut r,2).unwrap().session_generation,2);
}

#[test]
fn quit_retains_unconfirmed_cleanup_until_producer_retires() {
    let (mut r,_,p)=streaming();
    r.shutdown().unwrap();
    assert!(r.host.inspect().closing && !r.idle());
    assert_eq!(admit(&mut r,2),Err(ErrorCode::StaleSession));
    assert_eq!(synthetic_start(&mut r,2,"closed"),Err(ErrorCode::StaleSession));
    assert!(p.send(vec![]).is_err());
    r.native_tick(2000).unwrap();
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::CleanupUnconfirmed);
    drop(p); r.native_tick(2001).unwrap();
    assert!(r.idle()); assert_eq!(r.host.current().unwrap().terminal_count,1);
}

#[test]
fn malformed_and_duplicate_sse_fail_the_bound_run_without_repeating_text() {
    for duplicate in [false,true] {
        let (mut r,_,p)=streaming();
        let events=fixture::events("once");
        p.send(fixture::all(&events[..5])).unwrap();
        r.native_tick(0).unwrap();
        p.send(if duplicate {fixture::bytes(&events[4])} else {b"data: {\n\n".to_vec()}).unwrap();
        let rejection=if duplicate {ErrorCode::DuplicateEvent} else {ErrorCode::ProtocolError};
        assert_eq!(r.native_tick(1),Err(rejection));
        assert!(!r.idle());
        drop(p); r.native_tick(2).unwrap();
        let run=r.host.current().unwrap();
        assert_eq!((run.error.as_ref().unwrap().code,run.text.as_str(),run.terminal_count),
                   (ErrorCode::ProtocolError,"once",1));
    }
}
