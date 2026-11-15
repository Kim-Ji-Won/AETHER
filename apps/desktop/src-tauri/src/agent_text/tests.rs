use super::*;
use super::synthetic::SyntheticAdapter;
use tauri::ipc::{InvokeBody, InvokeResponseBody};

fn policy() -> Policy {
    Policy { max_input_bytes: 32, max_delta_bytes: 8, max_output_bytes: 32,
        max_deltas: 8, max_queue_frames: 2, max_queue_bytes: 16,
        run_ms: 1000, idle_ms: 100, cleanup_ms: 50,
        rate_window_ms: 1000, max_starts_per_window: 100, synthetic_budget: 100 }
}
fn setup_with(p: Policy) -> (Host, UiSession) {
    let mut h = Host::new("host-fixture-1", p).unwrap();
    let s = h.attach("host-fixture-1", 0, true).unwrap(); (h, s)
}
fn setup() -> (Host, UiSession) { setup_with(policy()) }
fn start(h: &mut Host, s: &UiSession, request: u32) -> Identity {
    h.start(Start { session: s.clone(), request_id: request, text: "질문".into() }).unwrap()
}
fn running() -> (Host, UiSession, Identity) {
    let (mut h, s) = setup(); let id = start(&mut h, &s, 1); h.dispatch(&id).unwrap(); (h, s, id)
}
fn frame(id: &Identity, seq: u32, payload: AdapterPayload) -> Frame {
    Frame { identity: id.clone(), source_seq: seq, payload }
}
fn delta(h: &mut Host, id: &Identity, seq: u32, text: &str) {
    h.ingest(frame(id, seq, AdapterPayload::TextDelta(text.into()))).unwrap();
}
fn complete(h: &mut Host, id: &Identity, seq: u32) { h.ingest(frame(id, seq, AdapterPayload::Completed)).unwrap(); }
fn release(h: &mut Host, id: &Identity) {
    h.release(id, CleanupPart::Adapter, true).unwrap(); h.release(id, CleanupPart::Runtime, true).unwrap();
}
fn view(h: &Host) -> &RunSnapshot { h.current().unwrap() }
fn terminal(h: &Host, state: State) {
    assert_eq!(view(h).state, state); assert_eq!(view(h).terminal_count, 1);
    assert_eq!(h.events().iter().filter(|e| matches!(e.payload,
        Payload::RunSucceeded | Payload::RunFailed(_) | Payload::RunCancelled { .. })).count(), 1);
    assert!(h.events().windows(2).all(|w| w[1].seq == w[0].seq + 1));
    assert!(h.events().iter().all(|e| e.namespace == NAMESPACE && e.source_kind == "synthetic"));
}
fn error(h: &Host, code: ErrorCode) { terminal(h, State::Failed); assert_eq!(view(h).error.as_ref().unwrap().code, code); }
fn body(json: &str) -> InvokeBody { InvokeBody::Json(InvokeResponseBody::Json(json.into()).deserialize().unwrap()) }
fn start_json() -> String {
    r#"{"namespace":"agent.text.v1","host_instance_id":"host-fixture-1","ui_context_epoch":1,"request_id":1,"text":"질문"}"#.into()
}
fn frame_json() -> String {
    r#"{"namespace":"agent.text.v1","host_instance_id":"host-fixture-1","run_id":1,"request_id":1,"attempt_id":1,"session_generation":1,"correlation_id":"run-1-attempt-1","source_kind":"synthetic","source_seq":1,"type":"TextDelta","payload":{"text":"안녕"}}"#.into()
}

#[test]
fn bootstrap_legacy_zero_is_not_a_wire_epoch_or_a_security_bypass() {
    let mut h = Host::new("host-fixture-1", policy()).unwrap();
    let zero = UiSession { host_instance_id: "host-fixture-1".into(), ui_context_epoch: 0 };
    let guessed_one = UiSession { ui_context_epoch: 1, ..zero.clone() };
    for s in [&zero, &guessed_one] {
        assert_eq!(h.snapshot(s), Err(ErrorCode::StaleSession));
        assert_eq!(h.start(Start { session:s.clone(), request_id:1,text:"hi".into() }), Err(ErrorCode::StaleSession));
    }
    assert_eq!(h.attach("wrong-host", 0, true), Err(ErrorCode::StaleSession));
    assert_eq!(h.attach("host-fixture-1", 0, false), Err(ErrorCode::StaleSession));
    let s = h.attach("host-fixture-1", 0, true).unwrap(); assert_eq!(s.ui_context_epoch, 1);
    let snap = h.snapshot(&s).unwrap(); assert_eq!(snap.run, None);
    assert_eq!(h.attach("host-fixture-1", 0, true).unwrap(), s);
    assert_eq!(h.snapshot(&s).unwrap(), snap); // repeated bootstrap does not reset counters/revision
    assert!(Start::parse(&body(&start_json().replace("\"ui_context_epoch\":1", "\"ui_context_epoch\":0")), 32).is_err());
}
#[test]
fn bootstrap_reload_snapshot_preserves_run_but_rejects_stale_ui() {
    let (mut h, old, id) = running(); delta(&mut h, &id, 1, "안녕");
    let before = h.snapshot(&old).unwrap();
    let new = h.attach("host-fixture-1", 1, true).unwrap();
    assert_eq!(new.ui_context_epoch, 2); assert_eq!(h.snapshot(&old), Err(ErrorCode::StaleSession));
    assert_eq!(h.cancel(&old, &id), Err(ErrorCode::StaleSession));
    assert_eq!(h.attach("host-fixture-1", 0, true), Err(ErrorCode::StaleSession));
    let after = h.snapshot(&new).unwrap(); assert_eq!(before.run, after.run);
    assert!(after.snapshot_revision > before.snapshot_revision);
    assert_eq!(after.request_high_water, 1); assert_eq!(after.session_generation, 1);
    delta(&mut h, &id, 2, "!"); assert_eq!(view(&h).text, "안녕!");
    assert_eq!(h.attach("host-fixture-1", u32::MAX, true), Err(ErrorCode::CounterExhausted));
    assert!(h.snapshot(&new).is_ok());
}
#[test]
fn normal_synthetic_text_barriers_and_explicit_completion() {
    let (mut h,s) = setup(); let id=start(&mut h,&s,1);
    assert_eq!(view(&h).state,State::Queued);
    let mut a = SyntheticAdapter::dispatch(&mut h,&id).unwrap();
    assert!(SyntheticAdapter::dispatch(&mut h,&id).is_err());
    for text in ["안녕", "!"] { a.enqueue(&mut h,AdapterPayload::TextDelta(text.into())).unwrap(); }
    assert_eq!(view(&h).text, ""); // queue is the controllable barrier
    assert!(a.step(&mut h).unwrap()); assert_eq!(view(&h).text,"안녕");
    assert!(a.step(&mut h).unwrap()); assert_eq!(view(&h).text,"안녕!");
    assert!(!a.step(&mut h).unwrap()); assert_eq!(view(&h).state,State::Running);
    a.enqueue(&mut h,AdapterPayload::Completed).unwrap(); a.step(&mut h).unwrap();
    assert_eq!(view(&h).state,State::Running); // no fabricated cleanup
    a.close(&mut h).unwrap(); h.release(&id,CleanupPart::Runtime,true).unwrap();
    terminal(&h,State::Succeeded); assert_eq!(view(&h).last_seq,5);
    assert_eq!(a.pending_frames(),0); assert_eq!(a.pending_bytes(),0);
    assert_eq!(h.events().iter().map(|e|e.payload.event_type()).collect::<Vec<_>>(),
        ["RunCreated","RunStarted","ResponseDelta","ResponseDelta","RunSucceeded"]);
}
#[test]
fn empty_response_requires_completed_and_cleanup() {
    let (mut h,_,id)=running(); complete(&mut h,&id,1); release(&mut h,&id);
    terminal(&h,State::Succeeded); assert_eq!(view(&h).last_seq,3); assert!(view(&h).text.is_empty());
}

#[test]
fn c01_queued_cancel_requires_no_fake_cleanup_ack() {
    let (mut h,s)=setup(); let id=start(&mut h,&s,1);
    h.cancel(&s,&id).unwrap(); terminal(&h,State::Cancelled);
    assert_eq!(view(&h).cleanup,Cleanup::NotRequired); assert_eq!(view(&h).remote_outcome,RemoteOutcome::NotDispatched);
    assert_eq!(view(&h).last_seq,3); assert!(!h.run.as_ref().unwrap().dispatched);
    let frozen=h.snapshot(&s).unwrap();
    assert!(SyntheticAdapter::dispatch(&mut h,&id).is_err());
    h.cancel(&s,&id).unwrap(); h.advance_to(1000).unwrap(); release(&mut h,&id);
    assert_eq!(h.snapshot(&s).unwrap(),frozen);
    start(&mut h,&s,2); // no phantom resource blocks admission
}
#[test]
fn c02_delta_cancel_waits_for_two_independent_releases() {
    let (mut h,s,id)=running(); delta(&mut h,&id,1,"안녕"); h.cancel(&s,&id).unwrap();
    assert_eq!(view(&h).state,State::CancelRequested); assert_eq!(view(&h).last_seq,4);
    h.release(&id,CleanupPart::Adapter,true).unwrap(); assert_eq!(view(&h).terminal_count,0);
    h.release(&id,CleanupPart::Runtime,true).unwrap(); terminal(&h,State::Cancelled);
    assert_eq!(view(&h).last_seq,5); assert_eq!(view(&h).text,"안녕");
    assert_eq!(view(&h).remote_outcome,RemoteOutcome::Unknown);
}
#[test]
fn c03_cancel_wins_before_terminal_commit() {
    let (mut h,s,id)=running(); h.cancel(&s,&id).unwrap();
    assert_eq!(h.ingest(frame(&id,1,AdapterPayload::Completed)),Err(ErrorCode::AlreadyTerminal));
    release(&mut h,&id); terminal(&h,State::Cancelled);
}
#[test]
fn c04_completion_wins_before_cancel() {
    let (mut h,s,id)=running(); complete(&mut h,&id,1); release(&mut h,&id);
    let frozen=h.snapshot(&s).unwrap(); assert_eq!(h.cancel(&s,&id),Err(ErrorCode::AlreadyTerminal));
    assert_eq!(h.snapshot(&s).unwrap(),frozen); terminal(&h,State::Succeeded);
}
#[test]
fn c05_duplicate_cancel_and_abort_are_idempotent() {
    let (mut h,s)=setup(); let id=start(&mut h,&s,1); let mut a=SyntheticAdapter::dispatch(&mut h,&id).unwrap();
    a.enqueue(&mut h,AdapterPayload::TextDelta("late".into())).unwrap();
    for _ in 0..3 { h.cancel(&s,&id).unwrap(); a.abort(&mut h).unwrap(); }
    assert_eq!(a.abort_count(),1); assert_eq!(a.pending_frames(),0);
    h.release(&id,CleanupPart::Runtime,true).unwrap(); let frozen=h.snapshot(&s).unwrap();
    h.cancel(&s,&id).unwrap(); a.abort(&mut h).unwrap();
    assert_eq!(h.snapshot(&s).unwrap(),frozen); terminal(&h,State::Cancelled);
}
#[test]
fn c06_missing_runtime_ack_blocks_next_run() {
    let (mut h,s,id)=running(); h.cancel(&s,&id).unwrap(); h.release(&id,CleanupPart::Adapter,true).unwrap();
    h.advance_to(50).unwrap(); error(&h,ErrorCode::CleanupUnconfirmed);
    assert_eq!(view(&h).cleanup,Cleanup::Unconfirmed);
    assert_eq!(h.start(Start{session:s,request_id:2,text:"next".into()}),Err(ErrorCode::Busy));
}
#[test]
fn c07_wrong_ack_rejected_late_correct_ack_only_updates_cleanup() {
    let (mut h,s,id)=running(); h.cancel(&s,&id).unwrap();
    for wrong in [Identity{attempt_id:2,..id.clone()},Identity{session_generation:2,..id.clone()}] {
        assert_eq!(h.release(&wrong,CleanupPart::Runtime,true),Err(ErrorCode::InvalidIdentity));
    }
    h.advance_to(50).unwrap(); let before=h.snapshot(&s).unwrap(); release(&mut h,&id);
    let after=h.snapshot(&s).unwrap(); error(&h,ErrorCode::CleanupUnconfirmed);
    assert_eq!(after.run.as_ref().unwrap().last_seq,before.run.as_ref().unwrap().last_seq);
    assert_eq!(after.run.as_ref().unwrap().error,before.run.as_ref().unwrap().error);
    assert!(after.snapshot_revision>before.snapshot_revision); assert_eq!(view(&h).cleanup,Cleanup::Confirmed);
}
#[test]
fn c08_runtime_ack_does_not_confirm_provider_abort() {
    let (mut h,_,id)=running(); let s=h.session(); h.cancel(&s,&id).unwrap();
    h.release(&id,CleanupPart::Runtime,true).unwrap(); h.release(&id,CleanupPart::Adapter,false).unwrap();
    h.advance_to(50).unwrap(); error(&h,ErrorCode::CleanupUnconfirmed);
}
#[test]
fn c09_late_delta_and_provider_error_cannot_override_cancel() {
    let (mut h,s,id)=running(); h.cancel(&s,&id).unwrap();
    for payload in [AdapterPayload::TextDelta("late".into()),AdapterPayload::Failed(ErrorCode::Offline)] {
        assert_eq!(h.ingest(frame(&id,1,payload)),Err(ErrorCode::AlreadyTerminal));
    }
    release(&mut h,&id); terminal(&h,State::Cancelled); assert!(view(&h).text.is_empty());
}
#[test]
fn c10_cleanup_deadline_boundary_is_inclusive() {
    for time in [49,50] {
        let (mut h,s,id)=running(); h.cancel(&s,&id).unwrap(); h.advance_to(time).unwrap(); release(&mut h,&id);
        terminal(&h,if time==49{State::Cancelled}else{State::Failed});
    }
}
#[test]
fn c11_cancel_can_win_while_completed_waits_for_cleanup() {
    let (mut h,s,id)=running(); complete(&mut h,&id,1); h.cancel(&s,&id).unwrap(); release(&mut h,&id);
    terminal(&h,State::Cancelled);
}
#[test]
fn c12_completion_cleanup_deadline_is_not_extended_by_cancel() {
    let (mut h,s,id)=running(); complete(&mut h,&id,1); h.advance_to(49).unwrap(); h.cancel(&s,&id).unwrap();
    h.advance_to(50).unwrap(); error(&h,ErrorCode::CleanupUnconfirmed);
    let (mut h,_,id)=running(); complete(&mut h,&id,1);
    assert_eq!(h.ingest(frame(&id,2,AdapterPayload::TextDelta("late".into()))),Err(ErrorCode::ProtocolError));
    error(&h,ErrorCode::ProtocolError); assert!(view(&h).text.is_empty());
}

#[test]
fn i01_duplicate_request_does_not_dispatch_twice() {
    let (mut h,s)=setup(); let id=start(&mut h,&s,1);
    for dispatched in [false,true] {
        if dispatched { h.dispatch(&id).unwrap(); }
        let before=h.snapshot(&s).unwrap();
        assert_eq!(h.start(Start{session:s.clone(),request_id:1,text:"질문".into()}),Err(ErrorCode::DuplicateRequest));
        assert_eq!(h.snapshot(&s).unwrap(),before);
    }
}
#[test]
fn i02_request_conflict_preserves_original_run() {
    let (mut h,s,_)=running(); let before=h.snapshot(&s).unwrap();
    assert_eq!(h.start(Start{session:s.clone(),request_id:1,text:"different".into()}),Err(ErrorCode::RequestConflict));
    assert_eq!(h.snapshot(&s).unwrap(),before);
}
#[test]
fn i03_high_water_survives_next_run_and_reload() {
    let (mut h,s)=setup(); let id=start(&mut h,&s,2); h.cancel(&s,&id).unwrap(); start(&mut h,&s,3);
    let new=h.attach("host-fixture-1",1,true).unwrap(); let before=h.snapshot(&new).unwrap();
    assert_eq!(h.start(Start{session:new.clone(),request_id:2,text:"질문".into()}),Err(ErrorCode::StaleRequest));
    assert_eq!(h.snapshot(&new).unwrap(),before);
}
#[test]
fn i04_wrong_identity_never_fails_the_current_run() {
    let (mut h,s,id)=running(); let before=h.snapshot(&s).unwrap();
    let wrongs=[Identity{host_instance_id:"old-host".into(),..id.clone()},Identity{run_id:2,..id.clone()},
        Identity{request_id:2,..id.clone()},Identity{attempt_id:2,..id.clone()},
        Identity{session_generation:2,..id.clone()},Identity{correlation_id:"wrong".into(),..id.clone()}];
    for wrong in wrongs {
        assert_eq!(h.ingest(frame(&wrong,1,AdapterPayload::Completed)),Err(ErrorCode::InvalidIdentity));
        assert_eq!(h.cancel(&s,&wrong),Err(ErrorCode::InvalidIdentity));
        assert_eq!(h.snapshot(&s).unwrap(),before);
    }
}
#[test]
fn i05_duplicate_frame_is_inert_conflict_fails_owned_stream_once() {
    let (mut h,s,id)=running(); delta(&mut h,&id,1,"hi"); let before=h.snapshot(&s).unwrap();
    assert_eq!(h.ingest(frame(&id,1,AdapterPayload::TextDelta("hi".into()))),Err(ErrorCode::DuplicateEvent));
    assert_eq!(h.snapshot(&s).unwrap(),before);
    assert_eq!(h.ingest(frame(&id,1,AdapterPayload::TextDelta("bad".into()))),Err(ErrorCode::EventConflict));
    error(&h,ErrorCode::ProtocolError); assert_eq!(view(&h).text,"hi");
}
#[test]
fn i06_gap_and_old_sequence_have_distinct_effects() {
    let (mut h,s,id)=running(); delta(&mut h,&id,1,"a"); delta(&mut h,&id,2,"b"); let before=h.snapshot(&s).unwrap();
    assert_eq!(h.ingest(frame(&id,1,AdapterPayload::Completed)),Err(ErrorCode::StaleEvent));
    assert_eq!(h.snapshot(&s).unwrap(),before);
    assert_eq!(h.ingest(frame(&id,4,AdapterPayload::Completed)),Err(ErrorCode::OutOfOrder)); error(&h,ErrorCode::ProtocolError);
}
#[test]
fn i07_closed_schemas_and_owned_malformed_stream() {
    let raw=start_json();
    for invalid in ["null".into(),"[]".into(),raw.replace("agent.text.v1","agent.text.v2"),
        raw.replace("\"text\":\"질문\"","\"text\":{}"),raw.replace("\"text\":\"질문\"","\"text\":null"),
        raw.replace("\"text\":\"질문\"","\"text\":true"),raw.replace(",\"text\":\"질문\"",""),
        raw.replace("\"text\":\"질문\"","\"text\":\"질문\",\"approved\":true")] {
        assert!(Start::parse(&body(&invalid),32).is_err());
    }
    let raw=frame_json();
    for invalid in ["null".into(),"[]".into(),raw.replace("TextDelta","ToolCall"),
        raw.replace("\"payload\":{\"text\":\"안녕\"}","\"payload\":null"),
        raw.replace("\"text\":\"안녕\"","\"text\":\"안녕\",\"metadata\":{}"),
        raw.replace("\"namespace\":\"agent.text.v1\",",""),raw.replace("agent.text.v1","agent.text.v2")] {
        assert!(Frame::parse(&body(&invalid),8).is_err());
    }
    let (mut h,_,id)=running(); h.stream_error(&id,ErrorCode::ProtocolError).unwrap(); error(&h,ErrorCode::ProtocolError);
}
#[test]
fn i08_numeric_and_opaque_id_boundaries_no_coercion() {
    for value in ["0","-1","1.5","4294967296","\"1\"","null","true"] {
        assert!(Start::parse(&body(&start_json().replace("\"request_id\":1",&format!("\"request_id\":{value}"))),32).is_err());
        for key in ["run_id","request_id","attempt_id","session_generation","source_seq"] {
            assert!(Frame::parse(&body(&frame_json().replace(&format!("\"{key}\":1"),&format!("\"{key}\":{value}"))),8).is_err());
        }
    }
    for id in ["".into(),"x".repeat(65),"한글".into(),"../path".into()] { assert!(Host::new(&id,policy()).is_err()); }
    assert!(Host::new(&"x".repeat(64),policy()).is_ok());
}
#[test]
fn i09_terminal_events_are_immutable() {
    for finish in [State::Succeeded,State::Failed,State::Cancelled] {
        let (mut h,s,id)=running();
        match finish {
            State::Succeeded=>{complete(&mut h,&id,1);release(&mut h,&id);},
            State::Failed=>{h.stream_error(&id,ErrorCode::ProtocolError).unwrap();release(&mut h,&id);},
            _=>{h.cancel(&s,&id).unwrap();release(&mut h,&id);}
        }
        let before=h.snapshot(&s).unwrap();
        for payload in [AdapterPayload::TextDelta("late".into()),AdapterPayload::Completed,AdapterPayload::Failed(ErrorCode::Offline)] {
            assert_eq!(h.ingest(frame(&id,99,payload)),Err(ErrorCode::AlreadyTerminal));
        }
        h.advance_to(2000).unwrap(); assert_eq!(h.snapshot(&s).unwrap(),before); terminal(&h,finish);
    }
}
#[test]
fn i10_busy_and_unconfirmed_cleanup_do_not_consume_admission() {
    let (mut h,s,id)=running();
    for stage in 0..3 {
        if stage==1 {h.cancel(&s,&id).unwrap();} if stage==2 {h.advance_to(50).unwrap();}
        let before=h.snapshot(&s).unwrap();
        assert_eq!(h.start(Start{session:s.clone(),request_id:2,text:"new".into()}),Err(ErrorCode::Busy));
        assert_eq!(h.snapshot(&s).unwrap(),before);
    }
    release(&mut h,&id); let next=start(&mut h,&s,2); assert_eq!(next.run_id,2);
}
#[test]
fn i11_synthetic_source_binding_rejects_provider_claim() {
    assert_eq!(Frame::parse(&body(&frame_json().replace("synthetic","provider")),8),Err(ErrorCode::InvalidIdentity));
    assert!(Start::parse(&body(&start_json().replace("\"text\":\"질문\"","\"text\":\"질문\",\"source_kind\":\"provider\"")),32).is_err());
}
#[test]
fn i12_identity_and_revision_exhaustion_do_not_wrap() {
    for field in 0..3 {
        let (mut h,s)=setup(); match field {0=>h.next_run=u32::MAX,1=>h.next_attempt=u32::MAX,_=>h.revision=u64::MAX-7};
        assert_eq!(h.start(Start{session:s,request_id:1,text:"hi".into()}),Err(ErrorCode::CounterExhausted));
        assert!(h.current().is_none()); assert_eq!(h.high_water,0);
    }
    let (mut h,s)=setup(); let id=start(&mut h,&s,u32::MAX); h.cancel(&s,&id).unwrap();
    assert_eq!(h.start(Start{session:s,request_id:1,text:"new".into()}),Err(ErrorCode::StaleRequest));
}

#[test]
fn e01_queued_run_deadline() {
    let (mut h,s)=setup(); let id=start(&mut h,&s,1); h.advance_to(999).unwrap(); assert_eq!(view(&h).state,State::Queued);
    h.advance_to(1000).unwrap(); error(&h,ErrorCode::RunTimeout); assert!(h.dispatch(&id).is_err());
    assert_eq!(view(&h).remote_outcome,RemoteOutcome::NotDispatched);
}
#[test]
fn e02_idle_deadline() {
    let (mut h,_,_)=running(); h.advance_to(99).unwrap(); assert_eq!(view(&h).state,State::Running);
    h.advance_to(100).unwrap(); error(&h,ErrorCode::IdleTimeout);
}
#[test]
fn e03_duplicates_do_not_extend_idle_deadline() {
    let (mut h,_,id)=running(); h.advance_to(90).unwrap(); delta(&mut h,&id,1,"a"); h.advance_to(180).unwrap();
    assert_eq!(h.ingest(frame(&id,1,AdapterPayload::TextDelta("a".into()))),Err(ErrorCode::DuplicateEvent));
    h.advance_to(190).unwrap(); error(&h,ErrorCode::IdleTimeout);
}
#[test]
fn e04_progress_cannot_extend_total_deadline() {
    let mut p=policy();p.max_output_bytes=128;p.max_deltas=16;
    let (mut h,s)=setup_with(p);let id=start(&mut h,&s,1);h.dispatch(&id).unwrap();
    for seq in 1..=11 {h.advance_to(seq as u64*90).unwrap();delta(&mut h,&id,seq,"a");}
    h.advance_to(1000).unwrap();error(&h,ErrorCode::RunTimeout);
}
#[test]
fn e05_cancel_keeps_cleanup_deadline_not_original_run_deadline() {
    let mut p=policy();p.run_ms=20;let(mut h,s)=setup_with(p);let id=start(&mut h,&s,1);h.dispatch(&id).unwrap();
    h.advance_to(19).unwrap();h.cancel(&s,&id).unwrap();h.advance_to(20).unwrap();assert_eq!(view(&h).state,State::CancelRequested);
    h.advance_to(69).unwrap();error(&h,ErrorCode::CleanupUnconfirmed);
}
#[test]
fn e06_deadline_wins_at_equal_tick_and_clock_cannot_regress() {
    for payload in [AdapterPayload::Completed,AdapterPayload::TextDelta("late".into())] {
        let (mut h,_,id)=running();h.advance_to(100).unwrap();
        assert_eq!(h.ingest(frame(&id,1,payload)),Err(ErrorCode::AlreadyTerminal));error(&h,ErrorCode::IdleTimeout);
        assert_eq!(h.advance_to(99),Err(ErrorCode::ClockRegression));
        assert_eq!(h.advance_to(u64::MAX),Err(ErrorCode::ClockRegression));
    }
}
#[test]
fn e07_input_delta_and_total_utf8_byte_boundaries() {
    for len in [31,32,33] {
        let(mut h,s)=setup();let result=h.start(Start{session:s,request_id:1,text:format!("한{}","x".repeat(len-3))});
        assert_eq!(result.is_ok(),len<=32);
    }
    for text in ["한글!","한글!!","한글!!!","😀😀","😀😀!"] {
        let(mut h,_,id)=running();let result=h.ingest(frame(&id,1,AdapterPayload::TextDelta(text.into())));
        assert_eq!(result.is_ok(),text.len()<=8);
    }
    for total in [31,32,33] {
        let(mut h,_,id)=running();for seq in 1..=4 {delta(&mut h,&id,seq,"1234567");}
        let result=h.ingest(frame(&id,5,AdapterPayload::TextDelta("x".repeat(total-28))));
        assert_eq!(result.is_ok(),total<=32);if total==33{error(&h,ErrorCode::OutputLimit);}
    }
}
#[test]
fn e08_bounded_utf8_decoder_split_invalid_and_overflow() {
    let mut d=wire::Utf8Decoder::new(8).unwrap();let bytes="한😀".as_bytes();
    assert_eq!(d.feed(&bytes[..2]).unwrap(),None);assert_eq!(d.feed(&bytes[2..4]).unwrap(),Some("한".into()));
    assert_eq!(d.feed(&bytes[4..]).unwrap(),Some("😀".into()));assert_eq!(d.finish(),Ok(()));
    assert_eq!(d.feed(&[0xff]),Err(ErrorCode::ProtocolError));
    assert_eq!(d.feed(&[b'a';9]),Err(ErrorCode::BufferLimit));
    d.feed(&[0xe3]).unwrap();assert_eq!(d.finish(),Err(ErrorCode::ProtocolError));
}
#[test]
fn e09_delta_cap_reserves_failure_terminal_space() {
    let(mut h,_,id)=running();for seq in 1..=8{delta(&mut h,&id,seq,"a");}
    assert_eq!(h.ingest(frame(&id,9,AdapterPayload::TextDelta("b".into()))),Err(ErrorCode::OutputLimit));
    error(&h,ErrorCode::OutputLimit);assert_eq!(h.events().len(),11);assert_eq!(view(&h).text,"aaaaaaaa");
}
#[test]
fn e10_queue_frame_and_byte_caps_fail_without_unbounded_growth() {
    for byte_cap in [16,3] {
        let mut p=policy();p.max_queue_bytes=byte_cap;let(mut h,s)=setup_with(p);let id=start(&mut h,&s,1);
        let mut a=SyntheticAdapter::dispatch(&mut h,&id).unwrap();a.enqueue(&mut h,AdapterPayload::TextDelta("aa".into())).unwrap();
        if byte_cap==16{a.enqueue(&mut h,AdapterPayload::TextDelta("bb".into())).unwrap();}
        assert_eq!(a.enqueue(&mut h,AdapterPayload::TextDelta("cc".into())),Err(ErrorCode::BufferLimit));
        error(&h,ErrorCode::BufferLimit);assert!(a.pending_frames()<=2);assert!(a.pending_bytes()<=byte_cap);
        a.abort(&mut h).unwrap();assert_eq!(a.pending_bytes(),0);
    }
}
#[test]
fn e11_safe_error_classification() {
    for (status,expected) in [(401,ErrorCode::AuthRequired),(403,ErrorCode::PermissionDenied),
        (429,ErrorCode::ProviderRateLimit),(503,ErrorCode::ProviderUnavailable),(499,ErrorCode::ProviderError)] {
        assert_eq!(ErrorCode::from_status(status),expected);
        let(mut h,_,id)=running();h.ingest(frame(&id,1,AdapterPayload::Failed(ErrorCode::from_status(status)))).unwrap();error(&h,expected);
    }
    for code in [ErrorCode::Offline,ErrorCode::QuotaExceeded,ErrorCode::ProviderError] {
        let(mut h,_,id)=running();h.ingest(frame(&id,1,AdapterPayload::Failed(code))).unwrap();error(&h,code);
    }
    assert_eq!(ErrorCode::AuthRequired.retry(),RetryAction::Configure);
    assert_eq!(ErrorCode::ProviderRateLimit.retry(),RetryAction::NewRequest);
}
#[test]
fn e12_eof_is_not_success_and_unsupported_content_never_executes() {
    let(mut h,s)=setup();let id=start(&mut h,&s,1);let mut a=SyntheticAdapter::dispatch(&mut h,&id).unwrap();
    a.enqueue(&mut h,AdapterPayload::TextDelta("partial".into())).unwrap();a.step(&mut h).unwrap();
    a.end_of_stream(&mut h).unwrap();error(&h,ErrorCode::ProtocolError);
    assert_eq!(view(&h).text,"partial");assert_eq!(view(&h).cleanup,Cleanup::Pending);
    h.release(&id,CleanupPart::Runtime,true).unwrap();assert_eq!(view(&h).cleanup,Cleanup::Confirmed);
    assert_eq!(h.events().iter().filter(|e|matches!(e.payload,Payload::RunSucceeded)).count(),0);
    let(mut h,s)=setup();let id=start(&mut h,&s,1);let mut a=SyntheticAdapter::dispatch(&mut h,&id).unwrap();
    a.enqueue(&mut h,AdapterPayload::Completed).unwrap();a.step(&mut h).unwrap();a.end_of_stream(&mut h).unwrap();
    h.release(&id,CleanupPart::Runtime,true).unwrap();terminal(&h,State::Succeeded);
    let(mut h,_,id)=running();h.stream_error(&id,ErrorCode::UnsupportedContent).unwrap();error(&h,ErrorCode::UnsupportedContent);
}
#[test]
fn e13_policy_rate_cost_admission_limits() {
    for index in 0..7 {
        let mut p=policy();match index {0=>p.max_input_bytes=0,1=>p.max_delta_bytes=usize::MAX,
            2=>p.run_ms=0,3=>p.cleanup_ms=u64::MAX,4=>p.max_queue_frames=0,5=>p.max_deltas=u32::MAX,_=>p.synthetic_budget=0};
        assert!(matches!(Host::new("host-fixture-1",p),Err(ErrorCode::PolicyUnconfigured)));
    }
    for cost in [false,true] {
        let mut p=policy();if cost{p.synthetic_budget=2;}else{p.max_starts_per_window=2;}
        let(mut h,s)=setup_with(p);for req in 1..=2{let id=start(&mut h,&s,req);h.cancel(&s,&id).unwrap();}
        let before=h.snapshot(&s).unwrap();
        assert_eq!(h.start(Start{session:s.clone(),request_id:3,text:"new".into()}),Err(if cost{ErrorCode::CostLimit}else{ErrorCode::RateLimited}));
        assert_eq!(h.snapshot(&s).unwrap(),before);
        if !cost{h.advance_to(1000).unwrap();start(&mut h,&s,3);}
    }
}
#[test]
fn e14_manual_retry_is_a_new_run_without_old_partial_text() {
    let(mut h,s,id)=running();delta(&mut h,&id,1,"partial");
    h.ingest(frame(&id,2,AdapterPayload::Failed(ErrorCode::Offline))).unwrap();release(&mut h,&id);
    assert!(view(&h).error.as_ref().unwrap().partial_output);
    h.advance_to(500).unwrap();assert_eq!(view(&h).identity,id); // no automatic retry
    let next=start(&mut h,&s,2);assert_ne!(next.attempt_id,id.attempt_id);assert_eq!(view(&h).text,"");
    assert_eq!(h.events().len(),1);assert_eq!(h.run.as_ref().unwrap().last_frame,None);
}
#[test]
fn e15_late_timeout_and_cleanup_cannot_create_second_terminal() {
    let(mut h,s,id)=running();h.stream_error(&id,ErrorCode::ProtocolError).unwrap();let before=view(&h).clone();
    h.advance_to(50).unwrap();assert_eq!(view(&h).cleanup,Cleanup::Unconfirmed);
    release(&mut h,&id);error(&h,ErrorCode::ProtocolError);
    assert_eq!(view(&h).last_seq,before.last_seq);assert_eq!(view(&h).error,before.error);
    let frozen=h.snapshot(&s).unwrap();h.advance_to(3000).unwrap();release(&mut h,&id);assert_eq!(h.snapshot(&s).unwrap(),frozen);
}

#[test]
fn security_s01_s03_s04_host_subset_only() {
    let canary="CANARY_SECRET_DO_NOT_LOG";
    let failed=frame_json().replace("TextDelta","Failed").replace("\"text\":\"안녕\"",&format!("\"code\":\"{canary}\""));
    let safe=Frame::parse(&body(&failed),8).unwrap();let(mut h,_,id)=running();h.ingest(safe).unwrap();
    assert!(!format!("{:?}",view(&h).error).contains(canary));error(&h,ErrorCode::ProviderError);
    for key in ["url","model","credential_ref","metadata"] {
        let invalid=start_json().replace("\"text\":\"질문\"",&format!("\"text\":\"질문\",\"{key}\":\"{canary}\""));
        assert!(Start::parse(&body(&invalid),32).is_err());
    }
    release(&mut h,&id);let s=h.session();start(&mut h,&s,2);assert_eq!(h.run.as_ref().unwrap().input,"질문");
    assert!(view(&h).text.is_empty());assert!(h.run.as_ref().unwrap().last_frame.is_none());
}
#[test]
fn logical_shutdown_and_generation_loss_do_not_claim_real_lifecycle_coverage() {
    let(mut h,s,id)=running();h.shutdown().unwrap();assert_eq!(view(&h).state,State::CancelRequested);
    assert_eq!(h.snapshot(&s),Err(ErrorCode::StaleSession));release(&mut h,&id);terminal(&h,State::Cancelled);
    assert_eq!(view(&h).cancel_reason,Some(CancelReason::AppShutdown));
    let(mut h,s,id)=running();h.runtime_lost().unwrap();error(&h,ErrorCode::RuntimeLost);
    assert_eq!(h.release(&id,CleanupPart::Runtime,true),Err(ErrorCode::InvalidIdentity));
    h.confirm_owned_cleanup(id.run_id).unwrap();let next=start(&mut h,&s,2);assert_eq!(next.session_generation,2);
    let mut fresh=Host::new("host-fixture-2",policy()).unwrap();fresh.attach("host-fixture-2",0,true).unwrap();
    assert_eq!(fresh.snapshot(&s),Err(ErrorCode::StaleSession));assert!(fresh.current().is_none());
}
#[test]
fn cancel_and_snapshot_wire_are_exact_and_epoch_checked() {
    let(mut h,s,id)=running();
    let raw=r#"{"namespace":"agent.text.v1","host_instance_id":"host-fixture-1","ui_context_epoch":1,"run_id":1,"request_id":1,"attempt_id":1,"session_generation":1}"#;
    h.cancel_command(wire::Cancel::parse(&body(raw)).unwrap()).unwrap();release(&mut h,&id);terminal(&h,State::Cancelled);
    assert!(wire::Cancel::parse(&body(&raw.replace("\"run_id\":1","\"run_id\":\"1\""))).is_err());
    let snap=r#"{"namespace":"agent.text.v1","host_instance_id":"host-fixture-1","ui_context_epoch":1}"#;
    assert_eq!(wire::snapshot_session(&body(snap)).unwrap(),s);
    assert!(wire::snapshot_session(&body(&snap.replace("\"ui_context_epoch\":1","\"ui_context_epoch\":1,\"extra\":true"))).is_err());
}
