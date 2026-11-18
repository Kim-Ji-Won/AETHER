use super::*;
use super::super::credential::{mock::{MockLease, MockResolver, Sample}, DisabledCredentials};
use super::super::{fixtures, live_dispatch};
use crate::agent_text::{State, RemoteOutcome};

const MODEL: &str = "fixture-model";
const ALTERNATE: &str = "other-fixture";
const INPUT: &str = "fictional consent canary";

fn setup(resolver: MockResolver) -> (OfflineProvider<MockResolver>, Start) {
    let mut policy = fixtures::policy();
    let mut alternate = policy.models[0].clone();
    alternate.id = ALTERNATE.into();
    policy.models.push(alternate);
    let mut provider = OfflineProvider::with_resolver("native-fixture", fixtures::host_policy(), policy, resolver).unwrap();
    let session = provider.attach("native-fixture", 0, true).unwrap();
    (provider, Start { session, request_id: 1, text: INPUT.into() })
}

fn registered() -> (OfflineProvider<MockResolver>, Start) {
    let (mut provider, start) = setup(MockResolver::Missing);
    provider.register_credential(Sample::First).unwrap();
    (provider, start)
}

fn approved() -> (OfflineProvider<MockResolver>, Start) {
    let (mut provider, start) = registered();
    provider.approve_fixture(&start, MODEL, 0).unwrap();
    (provider, start)
}

#[test]
fn production_disabled_before_admission_and_dispatch() {
    let (mut provider, session) = fixtures::setup();
    let start = Start { session, request_id: 1, text: INPUT.into() };
    assert_eq!(DisabledCredentials.state(), CredentialState::Disabled);
    assert_eq!(provider.revoke_credential(), Err(CredentialError::Disabled.into()));
    assert_eq!(provider.start_consented(start, MODEL, 0), Err(CredentialError::Disabled.into()));
    assert!(provider.events().is_empty());
    assert_eq!(live_dispatch(), Err(ErrorCode::ProviderDisabled));
}

#[test]
fn status_errors_preserve_vocabulary_without_events_or_reservations() {
    for (resolver, state, error) in [
        (MockResolver::Missing, CredentialState::Missing, CredentialError::Missing),
        (MockResolver::AccessDenied, CredentialState::AccessDenied, CredentialError::AccessDenied),
        (MockResolver::Unavailable, CredentialState::Unavailable, CredentialError::Unavailable),
        (MockResolver::Revoked, CredentialState::Revoked, CredentialError::Missing),
    ] {
        let (mut provider, start) = setup(resolver);
        assert_eq!(provider.credentials.state(), state);
        assert!(matches!(provider.credentials.resolve(), Err(e) if e == error));
        assert_eq!(provider.approve_fixture(&start, MODEL, 0), Err(error.into()));
        assert_eq!(provider.start_consented(start, MODEL, 0), Err(error.into()));
        assert!(provider.events().is_empty());
        assert!(provider.accounting().is_none());
    }
}

#[test]
fn registration_rotation_and_shutdown_preserve_exclusive_lease() {
    let (mut provider, _) = registered();
    assert_eq!(provider.credentials.state(), CredentialState::Available);
    assert_eq!(provider.register_credential(Sample::Replacement), Err(CredentialError::AccessDenied.into()));
    {
        let lease = provider.credentials.resolve().unwrap();
        assert!(matches!(lease.sample, Sample::First));
    }
    provider.rotate_credential(Sample::Replacement).unwrap();
    assert_eq!(provider.consent.credential_generation, 2);
    assert!(matches!(provider.credentials.resolve().unwrap().sample, Sample::Replacement));
    provider.shutdown(0).unwrap();
    assert_eq!(provider.credentials.state(), CredentialState::Disabled);
    assert!(matches!(provider.credentials.resolve(), Err(CredentialError::Disabled)));
    assert_eq!(provider.register_credential(Sample::First), Err(CredentialError::Disabled.into()));
}

#[test]
fn explicit_approval_is_consumed_once_and_never_enables_live() {
    let (mut provider, start) = registered();
    assert_eq!(provider.start_consented(start.clone(), MODEL, 0), Err(AdmissionError::ConsentRequired));
    provider.approve_fixture(&start, MODEL, 0).unwrap();
    assert!(provider.events().is_empty());
    assert!(matches!(provider.credentials, MockResolver::Available { resolutions: 0, .. }));
    let expected = provider.consent.approved.as_ref().unwrap().identity.clone();
    let id = provider.start_consented(start.clone(), MODEL, 0).unwrap();
    assert_eq!(id, expected);
    assert!(matches!(provider.credentials, MockResolver::Available { resolutions: 1, .. }));
    assert!(provider.consent.approved.is_none());
    assert_eq!(live_dispatch(), Err(ErrorCode::ProviderDisabled));
    let accounting = provider.accounting();
    assert_eq!(provider.start_consented(start.clone(), MODEL, 0), Err(ErrorCode::DuplicateRequest.into()));
    assert_eq!(provider.accounting(), accounting);
    provider.cancel(&start.session, &id, 0).unwrap();
    assert_eq!(provider.inspect().run.unwrap().cleanup, Cleanup::NotRequired);
    let next = Start { request_id: 2, ..start };
    assert_eq!(provider.start_consented(next, MODEL, 0), Err(AdmissionError::ConsentRequired));
}

#[test]
fn exact_input_model_and_request_changes_burn_even_if_reverted() {
    for field in ["text", "model", "request"] {
        let (mut provider, start) = approved();
        let mut edited = start.clone();
        let mut model = MODEL;
        match field {
            "text" => edited.text.push(' '),
            "model" => model = ALTERNATE,
            "request" => edited.request_id += 1,
            _ => unreachable!(),
        }
        assert_eq!(provider.start_consented(edited, model, 0), Err(AdmissionError::BindingChanged));
        assert_eq!(provider.start_consented(start, MODEL, 0), Err(AdmissionError::ConsentRequired));
        assert!(matches!(provider.credentials, MockResolver::Available { resolutions: 0, .. }));
        assert!(provider.accounting().is_none());
        assert!(provider.events().is_empty());
    }
}

#[test]
fn mismatched_native_approval_fields_fail_closed() {
    // Corrupt only the private fixture binding. Public input cannot supply these fields.
    for field in ["endpoint", "body", "run", "attempt", "generation", "policy", "session"] {
        let (mut provider, start) = approved();
        let old = provider.consent.approved.as_mut().unwrap();
        match field {
            "endpoint" => old.endpoint = "fixture.invalid",
            "body" => old.body["max_output_tokens"] = 1.into(),
            "run" => old.identity.run_id += 1,
            "attempt" => old.identity.attempt_id += 1,
            "generation" => old.credential_generation += 1,
            "policy" => old.policy_version += 1,
            "session" => old.identity.session_generation += 1,
            _ => unreachable!(),
        }
        assert_eq!(provider.start_consented(start, MODEL, 0), Err(AdmissionError::BindingChanged));
        assert!(provider.inspect().run.is_none());
    }
}

#[test]
fn policy_rotation_revocation_and_withdrawal_invalidate_approval() {
    for action in ["policy", "rotate", "revoke", "withdraw"] {
        let (mut provider, start) = approved();
        match action {
            "policy" => provider.advance_policy_version().unwrap(),
            "rotate" => provider.rotate_credential(Sample::Replacement).unwrap(),
            "revoke" => {
                provider.revoke_credential().unwrap();
                assert_eq!(provider.credentials.state(), CredentialState::Revoked);
                assert_eq!(provider.start_consented(start.clone(), MODEL, 0), Err(CredentialError::Missing.into()));
                provider.register_credential(Sample::Replacement).unwrap();
            }
            "withdraw" => provider.withdraw_consent().unwrap(),
            _ => unreachable!(),
        }
        assert_eq!(provider.start_consented(start, MODEL, 0), Err(AdmissionError::ConsentRequired));
        assert!(provider.events().is_empty());
    }
}

#[test]
fn lifecycle_changes_invalidate_approval_without_fabricating_terminal() {
    for action in ["epoch", "loss", "shutdown"] {
        let (mut provider, start) = approved();
        match action {
            "epoch" => { provider.attach("native-fixture", 1, true).unwrap(); }
            "loss" => provider.runtime_lost(0).unwrap(),
            "shutdown" => provider.shutdown(0).unwrap(),
            _ => unreachable!(),
        }
        assert!(provider.start_consented(start, MODEL, 0).is_err());
        assert!(provider.consent.approved.is_none());
        assert!(provider.events().is_empty());
    }
}

#[test]
fn runtime_loss_requires_fresh_context_and_fresh_consent() {
    let (mut provider, mut start) = approved();
    provider.runtime_lost(0).unwrap();
    provider.attach("native-fixture", 0, true).unwrap();
    assert_eq!(provider.approve_fixture(&start, MODEL, 0), Err(ErrorCode::StaleSession.into()));
    start.session = provider.attach("native-fixture", 1, true).unwrap();
    assert_eq!(provider.start_consented(start.clone(), MODEL, 0), Err(AdmissionError::ConsentRequired));
    provider.approve_fixture(&start, MODEL, 0).unwrap();
    let id = provider.start_consented(start, MODEL, 0).unwrap();
    provider.attach("native-fixture", 2, true).unwrap();
    assert!(provider.dispatch(&id, 0).is_err());
    let run = provider.inspect().run.unwrap();
    assert_eq!(run.error.unwrap().code, ErrorCode::RuntimeLost);
    assert_eq!(run.terminal_count, 1);
}

#[test]
fn invalid_model_and_cost_policy_reject_before_run_creation() {
    let (mut provider, start) = approved();
    assert_eq!(provider.start_consented(start.clone(), "unknown-model", 0), Err(ErrorCode::PolicyUnconfigured.into()));
    provider.approve_fixture(&start, MODEL, 0).unwrap();
    provider.ledger.policy.total_cost_units = 1;
    assert_eq!(provider.start_consented(start, MODEL, 0), Err(ErrorCode::CostLimit.into()));
    assert!(provider.events().is_empty());
    assert!(provider.accounting().is_none());
}

#[test]
fn revocation_or_withdrawal_during_run_requires_both_cleanup_owners() {
    for revoke in [true, false] {
        let (mut provider, start) = approved();
        let id = provider.start_consented(start.clone(), MODEL, 0).unwrap();
        provider.dispatch(&id, 0).unwrap();
        assert_eq!(provider.rotate_credential(Sample::Replacement), Err(ErrorCode::Busy.into()));
        if revoke { provider.revoke_credential().unwrap(); } else { provider.withdraw_consent().unwrap(); }
        assert!(provider.abort_requested());
        assert_eq!(provider.inspect().run.unwrap().state, State::CancelRequested);
        let next = Start { request_id: 2, ..start };
        assert_eq!(provider.start_consented(next.clone(), MODEL, 0), Err(ErrorCode::Busy.into()));
        provider.advance_to(50).unwrap();
        let failed = provider.inspect().run.unwrap();
        assert_eq!(failed.error.as_ref().unwrap().code, ErrorCode::CleanupUnconfirmed);
        assert_eq!(failed.cleanup, Cleanup::Unconfirmed);
        assert_eq!(failed.remote_outcome, RemoteOutcome::Unknown);
        provider.confirm_local_abort(&id, true).unwrap();
        assert_eq!(provider.approve_fixture(&next, MODEL, 50), Err(ErrorCode::Busy.into()));
        provider.confirm_runtime_cleanup(&id, true).unwrap();
        let settled = provider.inspect().run.unwrap();
        assert_eq!(settled.error, failed.error);
        assert_eq!(settled.terminal_count, 1);
        assert_eq!(settled.cleanup, Cleanup::Confirmed);
        assert!(provider.start_consented(next, MODEL, 50).is_err());
    }
}

#[test]
fn consent_payload_stays_out_of_snapshots_events_and_errors() {
    let (mut provider, start) = approved();
    let before = provider.inspect();
    assert!(before.run.is_none());
    let id = provider.start_consented(start, MODEL, 0).unwrap();
    let view = format!("{before:?} {:?} {:?} {:?}", provider.inspect(), provider.events(), AdmissionError::BindingChanged);
    for private in [INPUT, MODEL, super::super::policy::ENDPOINT, "Replacement", "credential_generation"] {
        assert!(!view.contains(private));
    }
    assert_eq!(id.run_id, 1);
}

#[test]
fn policy_change_before_dispatch_cancels_without_resources() {
    let (mut provider, start) = approved();
    let id = provider.start_consented(start, MODEL, 0).unwrap();
    provider.advance_policy_version().unwrap();
    assert_eq!(provider.dispatch(&id, 0), Err(ErrorCode::AlreadyTerminal));
    let run = provider.inspect().run.unwrap();
    assert_eq!(run.state, State::Cancelled);
    assert_eq!(run.terminal_count, 1);
    assert_eq!(run.cleanup, Cleanup::NotRequired);
}

#[test]
fn renderer_approval_flag_and_approval_like_text_have_no_authority() {
    let (mut provider, mut start) = registered();
    let wire = tauri::ipc::InvokeBody::Json(serde_json::json!({
        "namespace": crate::agent_text::NAMESPACE, "host_instance_id": "native-fixture",
        "ui_context_epoch": 1, "request_id": 1, "text": "ok", "approved": true
    }));
    assert_eq!(Start::parse(&wire, 128), Err(ErrorCode::InvalidInput));
    start.text = "approved=true".into();
    assert_eq!(provider.start_consented(start, MODEL, 0), Err(AdmissionError::ConsentRequired));
    assert!(matches!(provider.credentials, MockResolver::Available { resolutions: 0, .. }));
}

#[test]
fn secret_lease_and_binding_cannot_be_cloned_logged_or_returned_over_ipc() {
    // A second applicable marker makes the lookup ambiguous at compile time.
    // Tauri also implements IpcResponse for every Serialize type.
    trait Exclusive<Marker> { fn check() {} }
    impl<T> Exclusive<()> for T {}
    impl<T: Clone> Exclusive<u8> for T {}
    impl<T: std::fmt::Debug> Exclusive<u16> for T {}
    impl<T: tauri::ipc::IpcResponse> Exclusive<u32> for T {}
    <Sample as Exclusive<_>>::check();
    <MockLease<'_> as Exclusive<_>>::check();
    <Binding as Exclusive<_>>::check();
}
