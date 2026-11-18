use super::*;
use super::fixtures::*;
use serde_json::{json,Value};
use credential::CredentialResolver;

#[test]
fn normal_stream_is_provider_owned_and_waits_for_runtime_cleanup(){
    let(mut p,_,id)=running();feed(&mut p,&id,&events("안녕🙂")).unwrap();
    assert_eq!(p.inspect().run.unwrap().state,State::Running);
    assert!(p.preview().is_none());clean(&mut p,&id);assert_terminal(&p,State::Succeeded);
    assert_eq!(p.inspect().run.unwrap().text,"안녕🙂");
    assert_eq!(p.accounting().unwrap().reported_cost_units,Some(7));
    assert!(!p.accounting().unwrap().billing_confirmed);
}
#[test]
fn every_byte_split_and_all_chunk_sizes_produce_same_unicode(){
    let data=all(&events("한국어🙂"));
    for size in [1,2,3,4,7,31,128,data.len()] {
        let(mut p,_,id)=running();let mut transport=MockTransport::new(&data,size);
        while transport.step(&mut p,&id,1).unwrap(){}
        clean(&mut p,&id);assert_terminal(&p,State::Succeeded);
        assert_eq!(p.inspect().run.unwrap().text,"한국어🙂");
    }
}
#[test]
fn empty_completed_response_and_empty_delta_are_valid(){
    for with_item in [false,true]{
        let(mut p,_,id)=running();let mut e=events("");
        if !with_item {e=vec![e[0].clone(),e[8].clone()];e[1]["sequence_number"]=json!(1);e[1]["response"]["output"]=json!([]);}
        feed(&mut p,&id,&e).unwrap();clean(&mut p,&id);assert_terminal(&p,State::Succeeded);
        assert!(p.inspect().run.unwrap().text.is_empty());
    }
}
#[test]
fn sse_crlf_cr_bom_comments_multiline_and_no_event_field(){
    for newline in ["\n","\r\n","\r"] {
        let(mut p,_,id)=running();let mut data=String::from("\u{feff}: comment\n\n");
        for e in events("plain") {
            data.push_str("id: ignored\nretry: 0\nunknown: ignored\n");
            let pretty=serde_json::to_string_pretty(&e).unwrap();
            for line in pretty.lines(){data.push_str("data: ");data.push_str(line);data.push('\n');}
            data.push('\n');
        }
        let data=data.replace('\n',newline);let mut t=MockTransport::new(data.as_bytes(),1);
        while t.step(&mut p,&id,0).unwrap(){} clean(&mut p,&id);assert_terminal(&p,State::Succeeded);
    }
}
#[test]
fn malformed_json_utf8_and_event_type_mismatch_fail_closed(){
    for data in [b"data: {bad}\n\n".to_vec(),b"data: \xff\n\n".to_vec(),b"data: [DONE]\n\n".to_vec(),
        format!("event: response.completed\ndata: {}\n\n",events("a")[0]).into_bytes()] {
        let(mut p,_,id)=running();assert!(p.feed(&id,&data,0).is_err());
        assert_terminal(&p,State::Failed);assert_eq!(p.last_error(),Some(ProviderError::MalformedSse));
    }
}
#[test]
fn invalid_or_partial_utf8_at_eof_never_succeeds(){
    let(mut p,_,id)=running();p.feed(&id,b"data: \xf0\x9f",0).unwrap();
    assert_eq!(p.end_of_stream(&id,0),Err(ErrorCode::ProtocolError));assert_terminal(&p,State::Failed);
}
#[test]
fn eof_at_every_event_boundary_requires_explicit_completion(){
    let e=events("partial");
    for length in 0..e.len(){
        let(mut p,_,id)=running();feed(&mut p,&id,&e[..length]).unwrap();
        assert_eq!(p.end_of_stream(&id,0),Err(ErrorCode::ProviderError));
        assert_eq!(p.last_error(),Some(ProviderError::Incomplete));assert_terminal(&p,State::Failed);
    }
}
#[test]
fn unterminated_complete_event_is_not_completion(){
    let(mut p,_,id)=running();let e=events("a");feed(&mut p,&id,&e[..8]).unwrap();
    let mut last=bytes(&e[8]);last.pop();p.feed(&id,&last,0).unwrap();
    assert!(p.end_of_stream(&id,0).is_err());assert_terminal(&p,State::Failed);
}
#[test]
fn duplicate_and_conflicting_sequence_fail_without_repeating_text() {
    for repeated_text in ["once", "conflict"] {
        let (mut provider, session, id) = running();
        let stream = events("once");
        feed(&mut provider, &id, &stream[..5]).unwrap();
        let accepted = provider.events().to_vec();
        let mut duplicate = stream[4].clone();
        duplicate["delta"] = json!(repeated_text);
        assert_eq!(duplicate["sequence_number"], stream[4]["sequence_number"]);
        assert_eq!(feed(&mut provider, &id, &[duplicate]), Err(ErrorCode::DuplicateEvent));
        assert_eq!(provider.last_error(), Some(ProviderError::Duplicate));

        let mut failed = provider.inspect().run.unwrap();
        let error = failed.error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::ProtocolError);
        assert!(error.partial_output);
        assert_eq!(failed.text, "once");
        assert_eq!(failed.cleanup, Cleanup::Pending);
        assert_terminal(&provider, State::Failed); // also checks exactly one terminal
        let (history, terminal) = provider.events().split_at(accepted.len());
        assert_eq!(history, accepted);
        assert!(matches!(terminal, [event] if event.payload == Payload::RunFailed(error.clone())));
        let failed_history = provider.events().to_vec();

        assert!(provider.abort_requested());
        let next = Start { session, request_id: 2, text: "next".into() };
        assert_eq!(provider.start(next.clone(), "fixture-model", 0), Err(ErrorCode::Busy));
        provider.confirm_local_abort(&id, true).unwrap();
        assert_eq!(provider.start(next.clone(), "fixture-model", 0), Err(ErrorCode::Busy));
        provider.confirm_runtime_cleanup(&id, true).unwrap();
        for late in [&stream[4], &stream[8]] {
            assert_eq!(feed(&mut provider, &id, std::slice::from_ref(late)), Err(ErrorCode::AlreadyTerminal));
        }
        failed.cleanup = Cleanup::Confirmed;
        assert_eq!(provider.inspect().run.unwrap(), failed);
        assert_eq!(provider.events(), failed_history);
        assert!(provider.start(next, "fixture-model", 0).is_ok());
    }
}
#[test]
fn stale_skipped_negative_and_noninteger_sequences_rejected(){
    for seq in [json!(0),json!(6),json!(-1),json!(1.5),Value::Null,json!("4")] {
        let(mut p,_,id)=running();let mut e=events("a");feed(&mut p,&id,&e[..4]).unwrap();e[4]["sequence_number"]=seq;
        assert!(feed(&mut p,&id,&e[4..5]).is_err());assert_terminal(&p,State::Failed);
    }
}
#[test]
fn sequence_may_start_at_zero_or_one_and_counts_metadata(){
    for offset in [0,1]{let(mut p,_,id)=running();let mut e=events("a");
        for(i,v)in e.iter_mut().enumerate(){v["sequence_number"]=json!(i+offset);}
        feed(&mut p,&id,&e).unwrap();clean(&mut p,&id);assert_terminal(&p,State::Succeeded);
        assert_eq!(p.events().len(),4); // Created, Started, Delta, Succeeded
    }
}
#[test]
fn wrong_response_item_and_content_identity_rejected(){
    for (idx,key,value) in [(4,"item_id",json!("msg_other")),(4,"output_index",json!(1)),(4,"content_index",json!(1))]{
        let(mut p,_,id)=running();let mut e=events("a");e[idx][key]=value;
        assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
    }
    let(mut p,_,id)=running();let mut e=events("a");e[8]["response"]["id"]=json!("resp_other");
    assert_eq!(feed(&mut p,&id,&e),Err(ErrorCode::ProtocolError));assert_terminal(&p,State::Failed);
}
#[test]
fn success_requires_completed_status_null_error_and_matching_final_text(){
    for (key,value) in [("status",json!("in_progress")),("error",json!({"code":"server_error"})),("incomplete_details",json!({"reason":"max_output_tokens"}))]{
        let(mut p,_,id)=running();let mut e=events("a");e[8]["response"][key]=value;
        assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
    }
    let(mut p,_,id)=running();let mut e=events("a");e[8]["response"]["output"][0]["content"][0]["text"]=json!("different");
    assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
}
#[test]
fn missing_required_delta_fields_are_not_defaulted(){
    for key in ["item_id","output_index","content_index","delta","logprobs","sequence_number","type"]{
        let(mut p,_,id)=running();let mut e=events("a");feed(&mut p,&id,&e[..4]).unwrap();
        e[4].as_object_mut().unwrap().remove(key);
        let data=format!("data: {}\n\n",e[4]);assert!(p.feed(&id,data.as_bytes(),0).is_err());assert_terminal(&p,State::Failed);
    }
}
#[test]
fn tools_audio_refusal_reasoning_and_unknown_events_are_unsupported(){
    for kind in ["response.audio.delta","response.function_call_arguments.delta","response.refusal.delta","response.reasoning.delta","unknown"]{
        let(mut p,_,id)=running();let e=events("a");feed(&mut p,&id,&e[..4]).unwrap();
        let v=json!({"type":kind,"sequence_number":4});assert_eq!(feed(&mut p,&id,&[v]),Err(ErrorCode::UnsupportedContent));
        assert_terminal(&p,State::Failed);
    }
    for kind in ["function_call","computer_call","audio","reasoning"] {
        let(mut p,_,id)=running();let mut e=events("a");e[2]["item"]["type"]=json!(kind);
        assert_eq!(feed(&mut p,&id,&e),Err(ErrorCode::UnsupportedContent));
    }
}
#[test]
fn response_failed_incomplete_and_top_level_error_are_not_success(){
    for kind in ["response.failed","response.incomplete","error"]{
        let(mut p,_,id)=running();let e=events("partial");feed(&mut p,&id,&e[..5]).unwrap();
        let mut r=response(if kind=="response.failed"{"failed"}else{"incomplete"});
        r["error"]=json!({"code":"server_error","message":"PRIVATE_CANARY"});
        r["incomplete_details"]=json!({"reason":"max_output_tokens"});
        let v=if kind=="error" {json!({"type":kind,"code":"server_error","message":"PRIVATE_CANARY","param":null,"sequence_number":5})}
            else{json!({"type":kind,"response":r,"sequence_number":5})};
        assert!(feed(&mut p,&id,&[v]).is_err());clean(&mut p,&id);assert_terminal(&p,State::Failed);
        let view=format!("{:?} {:?} {:?}",p.inspect(),p.last_error(),p.events());assert!(!view.contains("PRIVATE_CANARY"));
        assert!(p.inspect().run.unwrap().error.unwrap().partial_output);
    }
}
#[test]
fn top_level_error_can_precede_created(){
    let(mut p,_,id)=running();assert_eq!(feed(&mut p,&id,&[json!({"type":"error","code":null,"message":"canary","param":null,"sequence_number":0})]),Err(ErrorCode::ProviderError));
    assert_terminal(&p,State::Failed);
}
#[test]
fn frame_and_stream_limits_include_comments_and_metadata(){
    let(mut p,_,id)=running();assert_eq!(p.feed(&id,&vec![b'x';8193],0),Err(ErrorCode::BufferLimit));
    assert_terminal(&p,State::Failed);
    let(mut p,_,id)=running();assert_eq!(p.feed(&id,&b":\n\n".repeat(22_000),0),Err(ErrorCode::BufferLimit));
    assert_terminal(&p,State::Failed);
}
#[test]
fn oversized_delta_and_total_output_never_enter_host(){
    let(mut p,_,id)=running();assert_eq!(feed(&mut p,&id,&events(&"x".repeat(65))),Err(ErrorCode::OutputLimit));
    assert!(p.inspect().run.unwrap().text.is_empty());
    let mut hp=host_policy();hp.max_output_bytes=3;
    let mut p=OfflineProvider::new("native-fixture",hp,policy()).unwrap();let s=p.attach("native-fixture",0,true).unwrap();
    let id=start(&mut p,&s,1);p.dispatch(&id,0).unwrap();assert_eq!(feed(&mut p,&id,&events("한🙂")),Err(ErrorCode::OutputLimit));
}
#[test]
fn queued_cancel_needs_no_fake_cleanup_ack(){
    let(mut p,s)=setup();let id=start(&mut p,&s,1);p.cancel(&s,&id,0).unwrap();
    assert_terminal(&p,State::Cancelled);assert_eq!(p.inspect().run.unwrap().cleanup,Cleanup::NotRequired);
    assert_eq!(p.inspect().run.unwrap().remote_outcome,RemoteOutcome::NotDispatched);
    assert!(!p.abort_requested());assert!(p.dispatch(&id,0).is_err());
}
#[test]
fn running_cancel_requires_both_cleanup_confirmations(){
    let(mut p,s,id)=running();p.cancel(&s,&id,0).unwrap();
    assert!(p.abort_requested());p.confirm_local_abort(&id,false).unwrap();
    assert_eq!(p.confirm_runtime_cleanup(&id,true),Err(ErrorCode::InvalidInput));
    assert_eq!(p.inspect().run.unwrap().state,State::CancelRequested);
    p.confirm_local_abort(&id,true).unwrap();p.confirm_runtime_cleanup(&id,false).unwrap();
    assert_eq!(p.inspect().run.unwrap().state,State::CancelRequested);
    p.confirm_runtime_cleanup(&id,true).unwrap();assert_terminal(&p,State::Cancelled);
    assert_eq!(p.inspect().run.unwrap().remote_outcome,RemoteOutcome::Unknown);
}
#[test]
fn cancel_completion_race_uses_explicit_barrier_order(){
    for cancel_first in [false,true]{
        let(mut p,s,id)=running();let e=events("a");feed(&mut p,&id,&e[..8]).unwrap();
        if cancel_first {p.cancel(&s,&id,1).unwrap();assert!(p.feed(&id,&bytes(&e[8]),1).is_err());clean(&mut p,&id);assert_terminal(&p,State::Cancelled);}
        else {p.feed(&id,&bytes(&e[8]),1).unwrap();clean(&mut p,&id);assert_eq!(p.cancel(&s,&id,1),Err(ErrorCode::AlreadyTerminal));assert_terminal(&p,State::Succeeded);}
    }
}
#[test]
fn completion_pending_cleanup_can_still_be_cancelled(){
    let(mut p,s,id)=running();feed(&mut p,&id,&events("a")).unwrap();p.cancel(&s,&id,0).unwrap();
    clean(&mut p,&id);assert_terminal(&p,State::Cancelled);
}
#[test]
fn late_events_do_not_mutate_terminal_snapshot_or_accounting(){
    let(mut p,s,id)=running();p.cancel(&s,&id,0).unwrap();clean(&mut p,&id);
    let before=p.inspect();let accounting=p.accounting();assert!(feed(&mut p,&id,&events("late")).is_err());
    assert_eq!(p.inspect(),before);assert_eq!(p.accounting(),accounting);assert_terminal(&p,State::Cancelled);
}
#[test]
fn cleanup_timeout_stays_failed_after_late_ack(){
    let(mut p,s,id)=running();p.cancel(&s,&id,0).unwrap();p.advance_to(50).unwrap();
    assert_terminal(&p,State::Failed);assert_eq!(p.inspect().run.unwrap().cleanup,Cleanup::Unconfirmed);
    let result=p.start(Start{session:s,request_id:2,text:"a".into()},"fixture-model",50);assert_eq!(result,Err(ErrorCode::Busy));
    clean(&mut p,&id);assert_terminal(&p,State::Failed);assert_eq!(p.inspect().run.unwrap().cleanup,Cleanup::Confirmed);
}
#[test]
fn deadline_is_checked_before_delta_and_completion(){
    let(mut p,_,id)=running();let e=events("a");feed(&mut p,&id,&e[..4]).unwrap();
    assert_eq!(p.feed(&id,&all(&e[4..]),100),Err(ErrorCode::AlreadyTerminal));
    assert_eq!(p.inspect().run.unwrap().error.unwrap().code,ErrorCode::IdleTimeout);assert!(p.abort_requested());
    let(mut p,s)=setup();let id=start(&mut p,&s,1);assert!(p.dispatch(&id,1000).is_err());
    assert_eq!(p.inspect().run.unwrap().error.unwrap().code,ErrorCode::RunTimeout);
}
#[test]
fn metadata_does_not_extend_idle_deadline(){
    let(mut p,_,id)=running();p.feed(&id,&bytes(&events("a")[0]),99).unwrap();p.advance_to(100).unwrap();
    assert_eq!(p.inspect().run.unwrap().error.unwrap().code,ErrorCode::IdleTimeout);
}
#[test]
fn clock_regression_is_rejected_without_mutation(){
    let(mut p,_,_)=running();p.advance_to(10).unwrap();let before=p.inspect();
    assert_eq!(p.advance_to(9),Err(ErrorCode::ClockRegression));assert_eq!(p.inspect(),before);
}
#[test]
fn wrong_native_binding_cannot_fail_or_advance_current_host(){
    let(mut p,_,id)=running();let before=p.inspect();
    let mut wrong=id.clone();wrong.attempt_id+=1;
    assert_eq!(p.feed(&wrong,b"invalid",1000),Err(ErrorCode::InvalidIdentity));assert_eq!(p.inspect(),before);
    assert_eq!(p.host.now_ms(),0);
}
#[test]
fn runtime_loss_rejects_old_ack_and_preserves_terminal(){
    let(mut p,_,id)=running();p.runtime_lost(0).unwrap();
    assert_eq!(p.confirm_runtime_cleanup(&id,true),Err(ErrorCode::InvalidIdentity));
    assert_eq!(p.feed(&id,b"anything",0),Err(ErrorCode::InvalidIdentity));
    p.confirm_lost_owned_cleanup().unwrap();assert_terminal(&p,State::Failed);
    assert_eq!(p.inspect().run.unwrap().error.unwrap().code,ErrorCode::RuntimeLost);
}
#[test]
fn shutdown_blocks_start_and_cancels_owned_stream(){
    let(mut p,s,id)=running();p.shutdown(0).unwrap();clean(&mut p,&id);assert_terminal(&p,State::Cancelled);
    assert_eq!(p.start(Start{session:s,request_id:2,text:"a".into()},"fixture-model",0),Err(ErrorCode::StaleSession));
}
#[test]
fn fixed_request_preview_uses_only_host_model_policy(){
    let(mut p,s)=setup();start(&mut p,&s,1);let request=p.preview().unwrap();
    assert_eq!(request.endpoint(),"https://api.openai.com/v1/responses");let b=request.body();
    for key in ["store","background","parallel_tool_calls"]{assert_eq!(b[key],false);}
    assert_eq!(b["stream"],true);assert_eq!(b["tools"],json!([]));assert_eq!(b["tool_choice"],"none");
    assert_eq!(b["input"][0]["role"],"user");assert_eq!(b["max_output_tokens"],64);
    assert!(b.get("authorization").is_none());assert_eq!(request.estimated_input_units,6);
    assert_eq!(live_dispatch(),Err(ErrorCode::ProviderDisabled));
}
#[test]
fn credential_resolver_is_unusable_and_never_reads_secrets(){
    let mut c=credential::DisabledCredentials;
    assert_eq!(c.resolve(),Err(credential::CredentialError::Disabled));
    assert_eq!(c.revoke(),Err(credential::CredentialError::Disabled));c.shutdown();
}
#[test]
fn missing_model_invalid_policy_and_request_limits_rejected(){
    let(mut p,s)=setup();for text in ["","  ",&"x".repeat(129)]{
        assert!(p.start(Start{session:s.clone(),request_id:1,text:text.into()},"fixture-model",0).is_err());
    }
    assert_eq!(p.start(Start{session:s,request_id:1,text:"a".into()},"renderer-model",0),Err(ErrorCode::PolicyUnconfigured));
    assert!(p.inspect().run.is_none());assert!(p.accounting().is_none());
    let mut cfg=policy();cfg.models.clear();assert!(Ledger::new(cfg).is_err());
    let mut cfg=policy();cfg.models[0].input_unit_price=u64::MAX;assert!(Ledger::new(cfg).is_err());
}
#[test]
fn input_utf8_budget_exact_boundary_and_multibyte(){
    for (text,ok) in [("a".repeat(128),true),("a".repeat(129),false),("🙂".repeat(32),true),("한".repeat(43),false)]{
        let(mut p,s)=setup();assert_eq!(p.start(Start{session:s,request_id:1,text},"fixture-model",0).is_ok(),ok);
    }
}
#[test]
fn single_active_and_duplicate_start_do_not_consume_extra_budget(){
    let(mut p,s,id)=running();let before=p.accounting();
    assert_eq!(p.start(Start{session:s.clone(),request_id:2,text:"a".into()},"fixture-model",0),Err(ErrorCode::Busy));
    assert_eq!(p.start(Start{session:s,request_id:1,text:"질문".into()},"fixture-model",0),Err(ErrorCode::DuplicateRequest));
    assert_eq!(p.accounting(),before);assert_eq!(p.identity,Some(id));
}
#[test]
fn cost_reservation_is_not_refunded_on_cancel_or_unknown_usage(){
    let mut cfg=policy();cfg.total_cost_units=204;
    let mut p=OfflineProvider::new("native-fixture",host_policy(),cfg).unwrap();let s=p.attach("native-fixture",0,true).unwrap();
    let id=start(&mut p,&s,1);p.cancel(&s,&id,0).unwrap();
    assert_eq!(p.start(Start{session:s,request_id:2,text:"a".into()},"fixture-model",0),Err(ErrorCode::CostLimit));
    assert_eq!(p.accounting().unwrap().reported_usage,None);
}
#[test]
fn rate_window_uses_virtual_clock_and_no_automatic_retry(){
    let mut cfg=policy();cfg.max_requests_per_window=1;
    let mut p=OfflineProvider::new("native-fixture",host_policy(),cfg).unwrap();let s=p.attach("native-fixture",0,true).unwrap();
    let id=start(&mut p,&s,1);p.cancel(&s,&id,0).unwrap();
    let req=Start{session:s,request_id:2,text:"a".into()};assert_eq!(p.start(req.clone(),"fixture-model",999),Err(ErrorCode::RateLimited));
    assert_eq!(p.inspect().request_high_water,1);assert!(p.start(req,"fixture-model",1000).is_ok());
}
#[test]
fn usage_totals_missing_usage_overrun_and_unpriced_cache_writes_rejected(){
    for (key,value) in [("total_tokens",json!(99)),("output_tokens",json!(65)),("input_tokens",json!(-1))]{
        let(mut p,_,id)=running();let mut e=events("a");e[8]["response"]["usage"][key]=value;
        assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
    }
    for u in [Value::Null,json!({"input_tokens":2,"output_tokens":1,"total_tokens":3,"input_tokens_details":{"cached_tokens":0,"cache_write_tokens":1},"output_tokens_details":{"reasoning_tokens":0}})]{
        let(mut p,_,id)=running();let mut e=events("a");e[8]["response"]["usage"]=u;
        assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
    }
}
#[test]
fn accounting_overrun_is_recorded_and_blocks_further_spending(){
    let(mut p,_,id)=running();let mut e=events("a");let u=&mut e[8]["response"]["usage"];
    u["output_tokens"]=json!(65);u["total_tokens"]=json!(67);
    assert!(feed(&mut p,&id,&e).is_err());assert!(p.accounting().unwrap().policy_exceeded);
    assert_eq!(p.accounting().unwrap().reported_usage.unwrap().output_tokens,65);
}
#[test]
fn http_errors_distinguish_429_causes_without_retaining_raw_data(){
    use ProviderError::*;
    for (status,code,kind,expected) in [(400,None,None,InvalidRequest),(401,None,None,Authentication),(403,None,None,Permission),
        (429,Some("rate_limit_exceeded"),None,RateLimit),(429,Some("slow_down"),None,RateLimit),
        (429,Some("credit_balance_exhausted"),None,Credit),(429,Some("project_spend_limit_exceeded"),None,SpendLimit),
        (429,Some("organization_spend_limit_exceeded"),None,SpendLimit),(429,Some("organization_usage_limit_exceeded"),None,Quota),
        (429,None,Some("insufficient_quota"),Quota),(429,None,Some("rate_limit_error"),RateLimit),
        (429,Some("PRIVATE_CANARY"),None,Unknown429),(500,None,None,Server),(503,None,None,Server)]{
        let error=ProviderError::http(status,code,kind);assert_eq!(error,expected);assert!(!format!("{error:?}").contains("PRIVATE_CANARY"));
    }
}
#[test]
fn offline_disconnect_and_late_transport_error_are_safe(){
    for err in [ProviderError::Offline,ProviderError::Disconnected]{
        let(mut p,_,id)=running();assert_eq!(p.transport_error(&id,err,0),Err(ErrorCode::Offline));clean(&mut p,&id);assert_terminal(&p,State::Failed);
        let before=p.inspect();assert!(p.transport_error(&id,ProviderError::Server,0).is_err());assert_eq!(p.inspect(),before);
    }
}
#[test]
fn renderer_provider_claim_cannot_pass_existing_closed_wire_schema(){
    let body=tauri::ipc::InvokeBody::Json(json!({"namespace":NAMESPACE,"host_instance_id":"native-fixture","ui_context_epoch":1,"request_id":1,"text":"a","source_kind":"provider"}));
    assert_eq!(Start::parse(&body,128),Err(ErrorCode::InvalidInput));
    let(mut p,_,id)=running();assert!(synthetic::SyntheticAdapter::dispatch(&mut p.host,&id).is_err());
}
#[test]
fn model_text_is_never_interpreted_as_html_code_or_approval(){
    let text="<script>approve();</script> rm -rf /";
    let(mut p,_,id)=running();feed(&mut p,&id,&events(text)).unwrap();clean(&mut p,&id);
    assert_terminal(&p,State::Succeeded);assert_eq!(p.inspect().run.unwrap().text,text);
}

#[test]
fn http_body_is_bounded_and_raw_error_details_are_discarded(){
    let body=br#"{"error":{"code":"credit_balance_exhausted","type":"insufficient_quota","message":"PRIVATE_CANARY","param":"https://example.invalid/?PRIVATE_CANARY","header":"Authorization PRIVATE_CANARY"}}"#;
    assert_eq!(ProviderError::http_body(429,body),ProviderError::Credit);
    assert_eq!(ProviderError::http_body(429,b"malformed PRIVATE_CANARY"),ProviderError::Unknown429);
    assert_eq!(ProviderError::stream(Some("PRIVATE_CANARY")),ProviderError::Unknown);
    assert_eq!(ProviderError::http_body(429,&vec![b'x';8193]),ProviderError::Unknown429);
    let(mut p,_,id)=running();let e=ProviderError::http_body(429,body);
    assert!(p.transport_error(&id,e,0).is_err());
    assert!(!format!("{:?}{:?}{:?}",p.inspect(),p.events(),p.last_error()).contains("PRIVATE_CANARY"));
}
#[test]
fn reported_usage_is_accounted_on_incomplete_without_claiming_success_or_billing(){
    let(mut p,_,id)=running();let e=events("partial");feed(&mut p,&id,&e[..5]).unwrap();
    let mut r=response("incomplete");r["incomplete_details"]=json!({"reason":"content_filter"});r["usage"]=usage();
    assert!(feed(&mut p,&id,&[json!({"type":"response.incomplete","sequence_number":5,"response":r})]).is_err());
    assert_eq!(p.accounting().unwrap().reported_cost_units,Some(7));assert!(!p.accounting().unwrap().billing_confirmed);
    assert_terminal(&p,State::Failed);
}
#[test]
fn returned_model_must_match_host_allowlist_selection(){
    for index in [0,1,8]{let(mut p,_,id)=running();let mut e=events("a");e[index]["response"]["model"]=json!("unapproved-model");
        assert_eq!(feed(&mut p,&id,&e),Err(ErrorCode::ProtocolError));assert_terminal(&p,State::Failed);
    }
}
#[test]
fn final_item_missing_done_or_mutated_text_never_succeeds(){
    for index in [5,6,7]{let(mut p,_,id)=running();let mut e=events("a");
        match index {5=>e[5]["text"]=json!("bad"),6=>e[6]["part"]["text"]=json!("bad"),_=>e[7]["item"]["content"][0]["text"]=json!("bad")}
        assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
    }
    let(mut p,_,id)=running();let mut e=events("a");e.remove(7);e[7]["sequence_number"]=json!(7);
    assert!(feed(&mut p,&id,&e).is_err());assert_terminal(&p,State::Failed);
}

#[test]
fn provider_ledger_is_separate_from_legacy_synthetic_budget(){
    let mut hp=host_policy();hp.synthetic_budget=1;
    let mut p=OfflineProvider::new("native-fixture",hp,policy()).unwrap();let s=p.attach("native-fixture",0,true).unwrap();
    for request in [1,2]{let id=start(&mut p,&s,request);p.cancel(&s,&id,0).unwrap();}
    assert_eq!(p.host.budget_left,1);assert_eq!(p.inspect().request_high_water,2);
}
