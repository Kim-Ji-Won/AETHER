use super::*;
use tauri::ipc::InvokeResponseBody;

fn json(text: &str) -> InvokeBody {
    InvokeBody::Json(InvokeResponseBody::Json(text.into()).deserialize().unwrap())
}
fn ready() -> Runtime {
    let mut r = Runtime::new("host");
    r.attach("host", 0, true).unwrap();
    r
}
fn start(r: &mut Runtime) {
    r.start(&json(r#"{"namespace":"agent.text.v1","host_instance_id":"host","ui_context_epoch":1,"request_id":1,"text":"synthetic"}"#), "lease".into()).unwrap();
}
#[test]
fn dock_utf8_limit_is_enforced_by_host_without_ui_validation() {
    for text in ["a".repeat(128), "한".repeat(42) + "ab", "😀".repeat(32)] {
        let mut r = ready();
        let body = format!(r#"{{"namespace":"agent.text.v1","host_instance_id":"host","ui_context_epoch":1,"request_id":1,"text":"{text}"}}"#);
        r.start(&json(&body), "lease".into()).unwrap();
        assert_eq!(r.host.current().unwrap().state, State::Queued);
    }
    for (text, error) in [("".into(), ErrorCode::InvalidInput), ("　 ".into(), ErrorCode::InvalidInput),
        ("a".repeat(129), ErrorCode::InputLimit), ("한".repeat(43), ErrorCode::InputLimit),
        ("😀".repeat(32) + "a", ErrorCode::InputLimit)] {
        let mut r = ready();
        let before = r.host.inspect();
        let body = format!(r#"{{"namespace":"agent.text.v1","host_instance_id":"host","ui_context_epoch":1,"request_id":1,"text":"{text}"}}"#);
        assert_eq!(r.start(&json(&body), "lease".into()), Err(error));
        assert_eq!(r.host.inspect(), before);
    }
}
fn control(r: &mut Runtime, action: &str) -> Result<(), ErrorCode> {
    r.runtime_input(&json(&format!(r#"{{"lease":"lease","action":"{action}","data":{{}}}}"#)))
}
fn frame(r: &mut Runtime, seq: u32, kind: &str, payload: &str) -> Result<(), ErrorCode> {
    r.runtime_input(&json(&format!(r#"{{"lease":"lease","action":"frame","data":{{"namespace":"agent.text.v1","host_instance_id":"host","run_id":1,"request_id":1,"attempt_id":1,"session_generation":1,"correlation_id":"run-1-attempt-1","source_kind":"synthetic","source_seq":{seq},"type":"{kind}","payload":{payload}}}}}"#)))
}
fn cancel(r: &mut Runtime) {
    let s = r.host.inspect();
    r.host.cancel(&s.session, &s.run.unwrap().identity).unwrap();
    r.tick(r.host.now_ms()).unwrap();
}

#[test]
fn native_sender_is_required_in_addition_to_json_identity() {
    assert!(sender_allowed("main", "tauri", Some("localhost"), None));
    for (label, scheme, host, port) in [("other", "tauri", Some("localhost"), None),
        ("main", "https", Some("localhost"), None), ("main", "tauri", Some("evil"), None),
        ("main", "tauri", Some("localhost"), Some(1234))] {
        assert!(!sender_allowed(label, scheme, host, port));
    }
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    let before = r.host.inspect();
    assert_eq!(r.runtime_input(&json(r#"{"lease":"forged","action":"frame","data":{}}"#)), Err(ErrorCode::InvalidIdentity));
    assert_eq!(r.host.inspect(), before);
}
#[test]
fn bootstrap_zero_maps_to_one_and_cannot_roll_back_or_change_instance() {
    let mut r = Runtime::new("host");
    assert_eq!(r.host.inspect().session.ui_context_epoch, 0);
    assert!(r.attach("forged", 0, true).is_err());
    assert!(r.attach("host", 0, false).is_err());
    r.attach("host", 0, true).unwrap();
    assert_eq!(r.view().0.2, 1);
    r.lost().unwrap(); r.page_replaced().unwrap(); r.attach("host", 1, true).unwrap();
    assert_eq!(r.view().0.2, 2);
    assert!(r.attach("host", 0, true).is_err());
}
#[test]
fn c01_queued_cancel_creates_no_adapter_and_requires_no_ack() {
    let mut r = ready(); start(&mut r); cancel(&mut r);
    let run = r.host.current().unwrap();
    assert_eq!((run.state, run.cleanup, run.terminal_count), (State::Cancelled, Cleanup::NotRequired, 1));
    assert!(r.adapter.is_none());
    assert_eq!(control(&mut r, "dispatch"), Err(ErrorCode::AlreadyTerminal));
    assert_eq!(control(&mut r, "release"), Err(ErrorCode::InvalidIdentity));
}
#[test]
fn normal_completion_requires_both_real_adapter_close_and_runtime_release() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    frame(&mut r, 1, "TextDelta", r#"{"text":"안녕"}"#).unwrap();
    frame(&mut r, 2, "Completed", "{}").unwrap();
    assert_eq!(r.host.current().unwrap().state, State::Running);
    control(&mut r, "release").unwrap();
    let final_state = r.host.inspect();
    assert_eq!(r.host.current().unwrap().state, State::Succeeded);
    assert_eq!(r.host.current().unwrap().cleanup, Cleanup::Confirmed);
    assert_eq!(frame(&mut r, 3, "TextDelta", r#"{"text":"late"}"#), Err(ErrorCode::AlreadyTerminal));
    assert_eq!(frame(&mut r, 4, "Completed", "{}"), Err(ErrorCode::AlreadyTerminal));
    r.tick(20_000).unwrap();
    assert_eq!(r.host.inspect(), final_state);
}
#[test]
fn cancellation_wins_completion_and_abort_is_idempotent() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    frame(&mut r, 1, "TextDelta", r#"{"text":"partial"}"#).unwrap();
    cancel(&mut r); cancel(&mut r);
    assert_eq!(frame(&mut r, 2, "Completed", "{}"), Err(ErrorCode::AlreadyTerminal));
    assert_eq!(r.adapter.as_ref().unwrap().abort_count(), 1);
    control(&mut r, "release").unwrap();
    cancel(&mut r);
    assert_eq!(r.host.current().unwrap().terminal_count, 1);
    assert_eq!(r.host.current().unwrap().state, State::Cancelled);
}
#[test]
fn idle_deadline_aborts_without_renderer_and_missing_ack_is_unconfirmed() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    r.tick(2_999).unwrap(); assert_eq!(r.host.current().unwrap().state, State::Running);
    r.tick(3_000).unwrap();
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::IdleTimeout);
    assert_eq!(r.adapter.as_ref().unwrap().abort_count(), 1);
    r.tick(5_000).unwrap(); assert_eq!(r.host.current().unwrap().cleanup, Cleanup::Unconfirmed);
    assert!(!r.idle());
    control(&mut r, "release").unwrap();
    assert!(r.idle()); assert_eq!(r.host.current().unwrap().state, State::Failed);
}
#[test]
fn run_deadline_precedes_same_time_completion_and_does_not_follow_idle_extensions() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    for n in 1..12 { r.tick(n * 1_000).unwrap(); frame(&mut r, n as u32, "TextDelta", r#"{"text":"x"}"#).unwrap(); }
    r.tick(12_000).unwrap();
    assert_eq!(frame(&mut r, 12, "Completed", "{}"), Err(ErrorCode::AlreadyTerminal));
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::RunTimeout);
}
#[test]
fn cleanup_deadline_wins_late_ack_and_never_changes_terminal() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap(); cancel(&mut r);
    r.tick(2_000).unwrap();
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::CleanupUnconfirmed);
    control(&mut r, "release").unwrap();
    assert_eq!(r.host.current().unwrap().state, State::Failed);
    assert_eq!(r.host.current().unwrap().terminal_count, 1);
}
#[test]
fn reload_revokes_sender_and_waits_for_native_context_replacement() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    r.lost().unwrap();
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::RuntimeLost);
    assert!(!r.idle());
    assert_eq!(control(&mut r, "release"), Err(ErrorCode::InvalidIdentity));
    r.page_replaced().unwrap(); r.attach("host", 1, true).unwrap();
    assert!(r.idle()); assert_eq!(r.host.inspect().session_generation, 2);
    assert_eq!(r.host.current().unwrap().state, State::Failed);
}
#[test]
fn reload_without_finished_is_not_reported_clean() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap(); r.lost().unwrap();
    r.tick(2_000).unwrap();
    assert_eq!(r.host.current().unwrap().cleanup, Cleanup::Unconfirmed);
    assert!(!r.idle());
}
#[test]
fn quit_stops_intake_but_accepts_bound_cleanup_without_fake_success() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    r.shutdown().unwrap();
    assert!(r.host.inspect().closing);
    assert_eq!(r.host.current().unwrap().cancel_reason, Some(CancelReason::AppShutdown));
    assert_eq!(control(&mut r, "dispatch"), Err(ErrorCode::StaleSession));
    assert_eq!(frame(&mut r, 1, "Completed", "{}"), Err(ErrorCode::StaleSession));
    assert!(!r.idle()); control(&mut r, "release").unwrap(); assert!(r.idle());
    assert_eq!(r.host.current().unwrap().state, State::Cancelled);
}
#[test]
fn restart_rejects_old_session_without_restoring_text() {
    let mut old = ready(); start(&mut old); control(&mut old, "dispatch").unwrap();
    let mut new = Runtime::new("new-host"); new.attach("new-host", 0, true).unwrap();
    assert!(new.host.snapshot(&old.host.inspect().session).is_err());
    assert_eq!(control(&mut new, "release"), Err(ErrorCode::InvalidIdentity));
    assert!(new.host.current().is_none());
}
#[test]
fn bound_malformed_frame_fails_only_its_owned_stream_and_clears_adapter() {
    let mut r = ready(); start(&mut r); control(&mut r, "dispatch").unwrap();
    assert_eq!(r.runtime_input(&json(r#"{"lease":"lease","action":"frame","data":{"CANARY_SECRET_DO_NOT_LOG":true}}"#)), Err(ErrorCode::ProtocolError));
    r.tick(0).unwrap();
    assert_eq!(r.adapter.as_ref().unwrap().abort_count(), 1);
    assert_eq!(r.host.current().unwrap().error.as_ref().unwrap().code, ErrorCode::ProtocolError);
    assert!(!format!("{:?}", r.view()).contains("CANARY_SECRET"));
    control(&mut r, "release").unwrap();
    assert!(r.idle());
}
