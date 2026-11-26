use super::super::{
    consent::AdmissionError,
    credential::{
        mock::{MockResolver, Sample},
        CredentialResolver, CredentialState, DisabledCredentials,
    },
    fixtures, live_dispatch, OfflineProvider,
};
use super::*;
use crate::agent_text::{
    Cleanup, CleanupPart, ErrorCode, Identity, Payload, Start, State as RunState,
};

const INSTANCE: u64 = 11;
const SLOT: u64 = 73;

// Pending writes and committed snapshots have separate owners. A crash drops the
// pending ticket; it never promotes submitted bytes to durable evidence.
#[derive(Default)]
struct MemoryJournal {
    pending: Option<WriteTicket>,
    committed: Option<Record>,
    submissions: usize,
}

impl MemoryJournal {
    fn submit(&mut self, protocol: &mut Protocol) {
        if let Some(ticket) = protocol.take_write() {
            assert!(self.pending.is_none());
            self.pending = Some(ticket);
            self.submissions += 1;
        }
    }

    fn reply(&mut self, protocol: &mut Protocol, outcome: WriteOutcome) {
        let ticket = self.pending.take().expect("submitted write");
        if outcome == WriteOutcome::Committed {
            self.committed = Some(ticket.record());
        }
        protocol.complete_write(ticket, outcome).unwrap();
    }
}

// Reuses b-1's actual approval consumption/withdrawal and its one real Host,
// which already calls the shared Provider adapter. Never invokes b-1 revoke:
// that memory-only API intentionally has no journal barrier.
struct RunFixture {
    protocol: Protocol,
    journal: MemoryJournal,
    provider: OfflineProvider<MockResolver>,
    start: Start,
    identity: Option<Identity>,
    deletions: Vec<u64>,
    order: Vec<&'static str>,
}

impl RunFixture {
    fn new(dispatch: Option<bool>) -> Self {
        let mut provider = OfflineProvider::with_resolver(
            "revocation-fixture",
            fixtures::host_policy(),
            fixtures::policy(),
            MockResolver::Missing,
        )
        .unwrap();
        provider.register_credential(Sample::First).unwrap();
        let session = provider.attach("revocation-fixture", 0, true).unwrap();
        let start = Start {
            session,
            request_id: 1,
            text: "fictional journal input".into(),
        };
        provider
            .approve_fixture(&start, "fixture-model", 0)
            .unwrap();
        let identity = dispatch.map(|running| {
            let id = provider
                .start_consented(start.clone(), "fixture-model", 0)
                .unwrap();
            if running {
                provider.dispatch(&id, 0).unwrap();
                feed(&mut provider, &id, &fixtures::events("kept")[..5]).unwrap();
            }
            id
        });
        Self {
            protocol: Protocol::active_fixture(INSTANCE, SLOT, 1, 40).unwrap(),
            journal: MemoryJournal::default(),
            provider,
            start,
            identity,
            deletions: Vec::new(),
            order: Vec::new(),
        }
    }

    fn revoke(&mut self) -> Result<(), Error> {
        let need = if self
            .provider
            .inspect()
            .run
            .is_some_and(|run| matches!(run.cleanup, Cleanup::Pending | Cleanup::Unconfirmed))
        {
            CleanupNeed::Required
        } else {
            CleanupNeed::NotRequired
        };
        let result = self.protocol.request_revoke(need);
        assert!(!self.protocol.admission_allowed());
        self.order.push("admission closed");
        // Even an operation counter/write setup error must not skip cancellation.
        self.provider.withdraw_consent().unwrap();
        self.order
            .push("consent withdrawn and cancel/abort requested");
        self.journal.submit(&mut self.protocol);
        self.order.push("journal submitted");
        result
    }

    fn admit(&mut self) -> Result<Identity, Error> {
        if !self.protocol.admission_allowed() {
            return Err(Error::Locked);
        }
        self.provider
            .start_consented(
                Start {
                    request_id: 2,
                    ..self.start.clone()
                },
                "fixture-model",
                self.provider.host.now_ms(),
            )
            .map_err(|_| Error::InvalidInput)
    }

    fn assert_locked(&mut self) {
        let before = self.provider.inspect();
        let accounting = self.provider.accounting();
        assert!(matches!(self.admit(), Err(Error::Locked)));
        assert_eq!(self.provider.inspect(), before);
        assert_eq!(self.provider.accounting(), accounting);
        assert!(self.protocol.registration_generation().is_err());
    }

    fn release(&mut self, owner: Owner) {
        let id = self.identity.as_ref().unwrap();
        match owner {
            Owner::Adapter => self.provider.confirm_local_abort(id, true).unwrap(),
            // The independent owner can retire before the reader. Use the real
            // Host ACK API; do not pretend that Adapter ACK implies Runtime ACK.
            Owner::Runtime => self
                .provider
                .host
                .release(id, CleanupPart::Runtime, true)
                .unwrap(),
        }
        self.protocol
            .cleanup_ack(self.protocol.binding(), owner)
            .unwrap();
    }

    fn delete(&mut self, outcome: DeleteOutcome) {
        let ticket = self
            .protocol
            .take_delete()
            .expect("committed deletion intent");
        assert_eq!(ticket.record().slot, SLOT);
        assert!(ticket.binding() == self.protocol.binding());
        self.deletions.push(ticket.record().slot);
        if outcome == DeleteOutcome::Deleted {
            self.provider.credentials.revoke().unwrap();
        }
        self.protocol.complete_delete(ticket, outcome).unwrap();
    }

    fn assert_terminal(&self, state: RunState, text: &str) {
        let run = self.provider.inspect().run.unwrap();
        assert_eq!(run.state, state);
        assert_eq!(run.terminal_count, 1);
        assert_eq!(run.text, text);
        assert_eq!(
            self.provider
                .events()
                .iter()
                .filter(|event| matches!(
                    event.payload,
                    Payload::RunSucceeded | Payload::RunFailed(_) | Payload::RunCancelled { .. }
                ))
                .count(),
            1
        );
        assert!(self
            .provider
            .events()
            .windows(2)
            .all(|pair| pair[0].seq + 1 == pair[1].seq));
    }
}

fn feed(
    provider: &mut OfflineProvider<MockResolver>,
    id: &Identity,
    events: &[serde_json::Value],
) -> Result<(), ErrorCode> {
    provider.feed(id, &fixtures::all(events), provider.host.now_ms())
}

fn evidence(protocol: &Protocol, cleanup: CleanupProof) -> AbsenceEvidence {
    let b = protocol.binding();
    AbsenceEvidence::fixture(b.instance, b.slot, b.generation, b.operation, cleanup)
}

fn history(record: Option<Record>) -> Continuity {
    record.map_or(Continuity::Missing, |r| Continuity::Verified {
        generation: r.generation,
        operation: r.operation,
    })
}

fn copy_write(ticket: &WriteTicket) -> WriteTicket {
    // Fault injection only. The normal API deliberately does not implement Clone.
    WriteTicket {
        key: ticket.key,
        record: ticket.record,
    }
}

fn at_write(stage: Stage) -> (Protocol, WriteTicket) {
    at_write_generation(stage, 7)
}

fn at_write_generation(stage: Stage, generation: u64) -> (Protocol, WriteTicket) {
    let mut p = Protocol::active_fixture(INSTANCE, SLOT, generation, 0).unwrap();
    p.request_revoke(CleanupNeed::NotRequired).unwrap();
    let intent = p.take_write().unwrap();
    if stage == Stage::RevokeIntent {
        return (p, intent);
    }
    p.complete_write(intent, WriteOutcome::Committed).unwrap();
    let delete_intent = p.take_write().unwrap();
    if stage == Stage::DeleteIntent {
        return (p, delete_intent);
    }
    p.complete_write(delete_intent, WriteOutcome::Committed)
        .unwrap();
    let delete = p.take_delete().unwrap();
    p.complete_delete(delete, DeleteOutcome::Deleted).unwrap();
    p.reconcile_absence(evidence(&p, CleanupProof::NotRequired))
        .unwrap();
    let confirm = p.take_write().unwrap();
    (p, confirm)
}

#[test]
fn delayed_first_write_does_not_delay_abort_cleanup_or_terminal() {
    let mut f = RunFixture::new(Some(true));
    f.revoke().unwrap();
    assert_eq!(
        f.order,
        [
            "admission closed",
            "consent withdrawn and cancel/abort requested",
            "journal submitted"
        ]
    );
    assert_eq!(f.protocol.state(), State::AwaitingIntent);
    assert!(!f.protocol.durable_acceptance());
    assert!(f.provider.abort_requested());
    assert_eq!(
        f.provider.inspect().run.unwrap().state,
        RunState::CancelRequested
    );
    let id = f.identity.clone().unwrap();
    assert_eq!(
        feed(&mut f.provider, &id, &fixtures::events("late")[..5]),
        Err(ErrorCode::AlreadyTerminal)
    );
    f.provider.advance_to(10).unwrap();
    assert_eq!(f.provider.host.now_ms(), 10);
    f.release(Owner::Adapter);
    assert!(!f.protocol.cleanup_complete());
    f.release(Owner::Runtime);
    f.assert_terminal(RunState::Cancelled, "kept");
    f.assert_locked();
    assert!(f.protocol.take_delete().is_none());
    f.revoke().unwrap();
    assert_eq!(f.protocol.operation(), 41);
    assert_eq!(f.journal.submissions, 1);
    f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
    assert!(f.protocol.durable_acceptance());
    f.journal.submit(&mut f.protocol);
    assert_eq!(
        f.journal.pending.as_ref().unwrap().record().stage,
        Stage::DeleteIntent
    );
    assert!(f.protocol.take_delete().is_none());
    f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
    f.delete(DeleteOutcome::Deleted);
    assert!(f.protocol.take_delete().is_none());
    assert_eq!(f.protocol.state(), State::AwaitingAbsence);
    f.protocol
        .reconcile_absence(evidence(&f.protocol, CleanupProof::BothReleased))
        .unwrap();
    f.journal.submit(&mut f.protocol);
    f.assert_locked();
    f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
    assert_eq!(f.protocol.registration_generation(), Ok(2));
    assert!(!f.protocol.admission_allowed());
    assert_eq!(f.deletions, [SLOT]);
    f.assert_terminal(RunState::Cancelled, "kept");
}

#[test]
fn failed_first_write_keeps_cancel_and_cleanup_live_without_deletion() {
    let mut f = RunFixture::new(Some(true));
    f.revoke().unwrap();
    f.journal.reply(&mut f.protocol, WriteOutcome::Failed);
    assert_eq!(f.protocol.state(), State::Failed);
    assert!(!f.protocol.durable_acceptance());
    assert!(f.provider.abort_requested());
    let id = f.identity.clone().unwrap();
    assert_eq!(
        feed(&mut f.provider, &id, &fixtures::events("late")[..5]),
        Err(ErrorCode::AlreadyTerminal)
    );
    f.provider.advance_to(10).unwrap();
    f.release(Owner::Adapter);
    f.release(Owner::Runtime);
    f.assert_terminal(RunState::Cancelled, "kept");
    f.assert_locked();
    assert_eq!(
        f.protocol.request_revoke(CleanupNeed::Required),
        Err(Error::Locked)
    );
    assert!(f.protocol.take_write().is_none());
    assert!(f.protocol.take_delete().is_none());
    assert!(f.journal.committed.is_none());
    assert_eq!(f.journal.submissions, 1);
    assert!(matches!(
        f.provider.credentials,
        MockResolver::Available { resolutions: 1, .. }
    ));
}

#[test]
fn uncertain_first_write_never_acknowledges_durable_acceptance() {
    let (mut p, ticket) = at_write(Stage::RevokeIntent);
    p.complete_write(ticket, WriteOutcome::Uncertain).unwrap();
    assert_eq!(p.state(), State::Uncertain);
    assert!(!p.durable_acceptance());
    assert!(p.record().is_none());
    assert_eq!(
        p.request_revoke(CleanupNeed::NotRequired),
        Err(Error::Locked)
    );
    assert!(p.take_write().is_none() && p.take_delete().is_none());
}

#[test]
fn idle_revocation_withdraws_real_pending_consent_before_journal_reply() {
    let mut f = RunFixture::new(None);
    f.revoke().unwrap();
    assert!(f.provider.events().is_empty());
    assert!(!f.provider.abort_requested());
    // Inspect b-1 approval separately: even bypassing this fixture's outer gate,
    // the exact pending approval has actually been removed by withdraw_consent.
    assert_eq!(
        f.provider
            .start_consented(f.start.clone(), "fixture-model", 0),
        Err(AdmissionError::ConsentRequired)
    );
    assert!(matches!(
        f.provider.credentials,
        MockResolver::Available { resolutions: 0, .. }
    ));
    assert_eq!(
        f.protocol.cleanup_ack(f.protocol.binding(), Owner::Adapter),
        Err(Error::InvalidInput)
    );
    f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
    f.journal.submit(&mut f.protocol);
    assert!(f.protocol.take_delete().is_none());
    f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
    f.delete(DeleteOutcome::Deleted);
    assert_eq!(f.deletions, [SLOT]);
}

#[test]
fn queued_run_uses_not_required_without_fabricated_cleanup_ack() {
    let mut f = RunFixture::new(Some(false));
    f.revoke().unwrap();
    f.assert_terminal(RunState::Cancelled, "");
    assert_eq!(
        f.provider.inspect().run.unwrap().cleanup,
        Cleanup::NotRequired
    );
    assert!(!f.provider.abort_requested());
    assert!(f.protocol.cleanup_complete());
    assert!(f.protocol.take_delete().is_none());
    f.assert_locked();
}

#[test]
fn already_completed_host_keeps_its_terminal_while_journal_gates_delete() {
    let mut f = RunFixture::new(Some(true));
    let id = f.identity.clone().unwrap();
    feed(&mut f.provider, &id, &fixtures::events("kept")[5..]).unwrap();
    f.provider.confirm_runtime_cleanup(&id, true).unwrap();
    f.assert_terminal(RunState::Succeeded, "kept");
    f.revoke().unwrap();
    assert!(f.protocol.cleanup_complete());
    assert!(f.protocol.take_delete().is_none());
    f.assert_terminal(RunState::Succeeded, "kept");
}

#[test]
fn either_single_owner_and_deadline_cannot_release_delete_barrier() {
    for first in [Owner::Adapter, Owner::Runtime] {
        let mut f = RunFixture::new(Some(true));
        f.revoke().unwrap();
        f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
        f.release(first);
        assert!(!f.protocol.cleanup_complete());
        assert!(f.protocol.take_write().is_none() && f.protocol.take_delete().is_none());
        f.provider.advance_to(50).unwrap();
        f.assert_terminal(RunState::Failed, "kept");
        assert_eq!(
            f.provider.inspect().run.unwrap().cleanup,
            Cleanup::Unconfirmed
        );
        f.assert_locked();
        assert!(f.protocol.take_delete().is_none());
        f.release(if first == Owner::Adapter {
            Owner::Runtime
        } else {
            Owner::Adapter
        });
        f.journal.submit(&mut f.protocol);
        assert_eq!(
            f.journal.pending.as_ref().unwrap().record().stage,
            Stage::DeleteIntent
        );
        assert!(f.protocol.take_delete().is_none());
        f.journal.reply(&mut f.protocol, WriteOutcome::Committed);
        f.delete(DeleteOutcome::Deleted);
        f.assert_terminal(RunState::Failed, "kept");
        assert_eq!(
            f.provider.inspect().run.unwrap().cleanup,
            Cleanup::Confirmed
        );
    }
}

#[test]
fn wrong_cleanup_binding_and_duplicate_one_owner_never_forge_both() {
    let mut p = Protocol::active_fixture(INSTANCE, SLOT, 7, 0).unwrap();
    p.request_revoke(CleanupNeed::Required).unwrap();
    let binding = p.binding();
    for wrong in [
        Binding {
            instance: INSTANCE + 1,
            ..binding
        },
        Binding {
            slot: SLOT + 1,
            ..binding
        },
        Binding {
            generation: 8,
            ..binding
        },
        Binding {
            operation: 2,
            ..binding
        },
    ] {
        assert_eq!(
            p.cleanup_ack(wrong, Owner::Adapter),
            Err(Error::WrongTicket)
        );
    }
    p.cleanup_ack(binding, Owner::Adapter).unwrap();
    p.cleanup_ack(binding, Owner::Adapter).unwrap();
    assert!(!p.cleanup_complete());
    let ticket = p.take_write().unwrap();
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    assert_eq!(p.state(), State::AwaitingCleanup);
    assert!(p.take_delete().is_none());
}

#[test]
fn failed_or_uncertain_delete_intent_write_never_emits_delete() {
    for outcome in [WriteOutcome::Failed, WriteOutcome::Uncertain] {
        let (mut p, ticket) = at_write(Stage::DeleteIntent);
        p.complete_write(ticket, outcome).unwrap();
        assert!(p.take_delete().is_none());
        assert!(p.take_write().is_none());
        assert!(p.durable_acceptance());
        assert_eq!(p.record().unwrap().stage, Stage::RevokeIntent);
        assert_eq!(p.registration_generation(), Err(Error::Locked));
    }
}

#[test]
fn failed_or_uncertain_delete_stays_locked_across_restart_without_retry() {
    for outcome in [DeleteOutcome::Failed, DeleteOutcome::Uncertain] {
        let (mut p, ticket) = at_write(Stage::DeleteIntent);
        p.complete_write(ticket, WriteOutcome::Committed).unwrap();
        let delete = p.take_delete().unwrap();
        p.complete_delete(delete, outcome).unwrap();
        assert_eq!(
            p.state(),
            if outcome == DeleteOutcome::Failed {
                State::Failed
            } else {
                State::Uncertain
            }
        );
        assert_eq!(
            p.request_revoke(CleanupNeed::NotRequired),
            Err(Error::Locked)
        );
        assert!(p.take_delete().is_none() && p.take_write().is_none());
        let snapshot = p.record();
        let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
        assert_eq!(restarted.state(), State::RecoveryLocked);
        assert!(!restarted.admission_allowed());
        assert_eq!(restarted.registration_generation(), Err(Error::Locked));
        assert!(restarted.take_delete().is_none() && restarted.take_write().is_none());
    }
}

#[test]
fn deleted_reply_alone_does_not_confirm_absence_or_enable_registration() {
    let (mut p, ticket) = at_write(Stage::DeleteIntent);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let delete = p.take_delete().unwrap();
    p.complete_delete(delete, DeleteOutcome::Deleted).unwrap();
    assert_eq!(p.state(), State::AwaitingAbsence);
    assert!(p.take_write().is_none() && p.take_delete().is_none());
    assert_eq!(p.registration_generation(), Err(Error::Locked));
    assert_eq!(
        p.reconcile_absence(evidence(&p, CleanupProof::Unconfirmed)),
        Err(Error::EvidenceMismatch)
    );
    assert_eq!(p.state(), State::AwaitingAbsence);
}

#[test]
fn failed_or_uncertain_confirmation_commit_never_enables_registration() {
    for outcome in [WriteOutcome::Failed, WriteOutcome::Uncertain] {
        let (mut p, ticket) = at_write(Stage::Confirmed);
        p.complete_write(ticket, outcome).unwrap();
        assert_eq!(p.registration_generation(), Err(Error::Locked));
        assert_eq!(p.record().unwrap().stage, Stage::DeleteIntent);
        assert!(p.take_write().is_none() && p.take_delete().is_none());
    }
}

#[test]
fn crash_before_and_after_every_write_uses_only_committed_snapshot() {
    for stage in [Stage::RevokeIntent, Stage::DeleteIntent, Stage::Confirmed] {
        for committed_before_reply in [false, true] {
            let (p, ticket) = at_write(stage);
            let old_callback = copy_write(&ticket);
            let snapshot = if committed_before_reply {
                Some(ticket.record())
            } else {
                p.record()
            };
            drop(ticket);
            drop(p);
            let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
            assert_eq!(
                restarted.state(),
                if snapshot.is_some() {
                    State::RecoveryLocked
                } else {
                    State::Quarantined
                }
            );
            assert!(!restarted.admission_allowed());
            assert!(!restarted.durable_acceptance());
            assert_eq!(restarted.registration_generation(), Err(Error::Locked));
            assert_eq!(
                restarted.complete_write(old_callback, WriteOutcome::Committed),
                Err(Error::WrongTicket)
            );
            assert!(restarted.take_write().is_none() && restarted.take_delete().is_none());
        }
    }
}

#[test]
fn lost_delete_reply_never_replays_a_delete_on_restart() {
    let (mut p, ticket) = at_write(Stage::DeleteIntent);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let delete = p.take_delete().unwrap();
    let stale = DeleteTicket { key: delete.key };
    let snapshot = p.record();
    drop(delete);
    drop(p);
    let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
    assert_eq!(
        restarted.complete_delete(stale, DeleteOutcome::Deleted),
        Err(Error::WrongTicket)
    );
    assert!(restarted.take_delete().is_none());
    assert_eq!(
        restarted.request_revoke(CleanupNeed::NotRequired),
        Err(Error::Locked)
    );
}

#[test]
fn missing_corrupt_rollback_and_mismatched_history_remain_quarantined() {
    let (_, ticket) = at_write(Stage::RevokeIntent);
    let valid = ticket.record();
    let bad = [
        (None, history(Some(valid))),
        (Some(valid), Continuity::Missing),
        (Some(valid), Continuity::Corrupt),
        (Some(valid), Continuity::RolledBack),
        (
            Some(valid),
            Continuity::Verified {
                generation: 8,
                operation: 1,
            },
        ),
        (
            Some(valid),
            Continuity::Verified {
                generation: 7,
                operation: 2,
            },
        ),
        (
            Some(Record {
                schema: SCHEMA + 1,
                ..valid
            }),
            history(Some(valid)),
        ),
        (
            Some(Record {
                slot: SLOT + 1,
                ..valid
            }),
            history(Some(valid)),
        ),
        (
            Some(Record {
                generation: 0,
                ..valid
            }),
            Continuity::Verified {
                generation: 0,
                operation: 1,
            },
        ),
        (
            Some(Record {
                operation: 0,
                ..valid
            }),
            Continuity::Verified {
                generation: 7,
                operation: 0,
            },
        ),
        (
            Some(Record {
                stage: Stage::DeleteIntent,
                cleanup_required: true,
                ..valid
            }),
            history(Some(valid)),
        ),
        (
            Some(Record {
                stage: Stage::Confirmed,
                cleanup_required: true,
                ..valid
            }),
            history(Some(valid)),
        ),
    ];
    for (snapshot, continuity) in bad {
        let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, continuity);
        assert_eq!(restarted.state(), State::Quarantined);
        assert!(!restarted.admission_allowed());
        assert!(restarted
            .reconcile_absence(evidence(&restarted, CleanupProof::BothReleased))
            .is_err());
        assert!(restarted.take_write().is_none() && restarted.take_delete().is_none());
    }
}

#[test]
fn previous_confirmed_record_still_requires_fresh_reconciliation_commit() {
    let (mut p, ticket) = at_write(Stage::Confirmed);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let snapshot = p.record();
    let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
    assert_eq!(restarted.state(), State::RecoveryLocked);
    assert_eq!(restarted.registration_generation(), Err(Error::Locked));
    assert_eq!(
        restarted.cleanup_ack(p.binding(), Owner::Adapter),
        Err(Error::WrongTicket)
    );
    assert_eq!(
        restarted.reconcile_absence(evidence(&p, CleanupProof::NotRequired)),
        Err(Error::EvidenceMismatch)
    );
    restarted
        .reconcile_absence(evidence(&restarted, CleanupProof::NotRequired))
        .unwrap();
    assert_eq!(restarted.registration_generation(), Err(Error::Locked));
    let confirm = restarted.take_write().unwrap();
    assert_eq!(confirm.record().stage, Stage::Confirmed);
    restarted
        .complete_write(confirm, WriteOutcome::Committed)
        .unwrap();
    assert_eq!(restarted.registration_generation(), Ok(8));
    assert!(!restarted.admission_allowed());
    assert!(restarted.take_delete().is_none());
}

#[test]
fn restarted_cleanup_requires_fresh_owner_proof_not_old_ack_or_absence_alone() {
    let mut p = Protocol::active_fixture(INSTANCE, SLOT, 7, 0).unwrap();
    p.request_revoke(CleanupNeed::Required).unwrap();
    let ticket = p.take_write().unwrap();
    let snapshot = Some(ticket.record());
    let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
    assert_eq!(
        restarted.cleanup_ack(p.binding(), Owner::Runtime),
        Err(Error::WrongTicket)
    );
    assert_eq!(
        restarted.cleanup_ack(restarted.binding(), Owner::Runtime),
        Err(Error::Locked)
    );
    for proof in [CleanupProof::NotRequired, CleanupProof::Unconfirmed] {
        assert_eq!(
            restarted.reconcile_absence(evidence(&restarted, proof)),
            Err(Error::EvidenceMismatch)
        );
    }
    restarted
        .reconcile_absence(evidence(&restarted, CleanupProof::BothReleased))
        .unwrap();
    assert!(restarted.take_delete().is_none());
    assert_eq!(
        restarted.take_write().unwrap().record().stage,
        Stage::Confirmed
    );
}

#[test]
fn wrong_absence_binding_never_reconciles_live_operation() {
    let (mut p, ticket) = at_write(Stage::DeleteIntent);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let delete = p.take_delete().unwrap();
    p.complete_delete(delete, DeleteOutcome::Deleted).unwrap();
    let b = p.binding();
    for wrong in [
        Binding {
            instance: b.instance + 1,
            ..b
        },
        Binding {
            slot: b.slot + 1,
            ..b
        },
        Binding {
            generation: b.generation + 1,
            ..b
        },
        Binding {
            operation: b.operation + 1,
            ..b
        },
    ] {
        let evidence = AbsenceEvidence::fixture(
            wrong.instance,
            wrong.slot,
            wrong.generation,
            wrong.operation,
            CleanupProof::NotRequired,
        );
        assert_eq!(p.reconcile_absence(evidence), Err(Error::EvidenceMismatch));
        assert!(p.take_write().is_none());
    }
}

#[test]
fn stale_write_fields_and_duplicate_reply_do_not_consume_current_operation() {
    let (mut p, ticket) = at_write(Stage::RevokeIntent);
    for field in 0..8 {
        let mut wrong = copy_write(&ticket);
        match field {
            0 => wrong.key.binding.instance += 1,
            1 => wrong.key.binding.slot += 1,
            2 => wrong.key.binding.generation += 1,
            3 => wrong.key.binding.operation += 1,
            4 => wrong.key.sequence += 1,
            5 => wrong.record.stage = Stage::Confirmed,
            6 => wrong.record.slot += 1,
            _ => wrong.record.cleanup_required = !wrong.record.cleanup_required,
        }
        assert_eq!(
            p.complete_write(wrong, WriteOutcome::Committed),
            Err(Error::WrongTicket)
        );
        assert_eq!(p.state(), State::AwaitingIntent);
    }
    let duplicate = copy_write(&ticket);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    assert_eq!(
        p.complete_write(duplicate, WriteOutcome::Committed),
        Err(Error::WrongTicket)
    );
    assert_eq!(p.state(), State::AwaitingDeleteIntent);
    assert!(p.take_delete().is_none());
}

#[test]
fn stale_delete_ticket_and_duplicate_reply_cannot_create_confirmation() {
    let (mut p, ticket) = at_write(Stage::DeleteIntent);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let delete = p.take_delete().unwrap();
    for field in 0..5 {
        let mut wrong = DeleteTicket { key: delete.key };
        match field {
            0 => wrong.key.binding.instance += 1,
            1 => wrong.key.binding.slot += 1,
            2 => wrong.key.binding.generation += 1,
            3 => wrong.key.binding.operation += 1,
            _ => wrong.key.sequence += 1,
        }
        assert_eq!(
            p.complete_delete(wrong, DeleteOutcome::Deleted),
            Err(Error::WrongTicket)
        );
        assert_eq!(p.state(), State::Deleting);
    }
    let duplicate = DeleteTicket { key: delete.key };
    p.complete_delete(delete, DeleteOutcome::Deleted).unwrap();
    assert_eq!(
        p.complete_delete(duplicate, DeleteOutcome::Deleted),
        Err(Error::WrongTicket)
    );
    assert_eq!(p.state(), State::AwaitingAbsence);
    assert!(p.take_write().is_none());
}

#[test]
fn explicit_absence_after_failure_never_retries_delete_or_duplicates_generation() {
    let (mut p, ticket) = at_write(Stage::DeleteIntent);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    let delete = p.take_delete().unwrap();
    p.complete_delete(delete, DeleteOutcome::Uncertain).unwrap();
    p.reconcile_absence(evidence(&p, CleanupProof::NotRequired))
        .unwrap();
    assert_eq!(
        p.reconcile_absence(evidence(&p, CleanupProof::NotRequired)),
        Err(Error::Locked)
    );
    let confirm = p.take_write().unwrap();
    p.complete_write(confirm, WriteOutcome::Committed).unwrap();
    assert_eq!(p.registration_generation(), Ok(8));
    assert_eq!(p.registration_generation(), Ok(8));
    assert_eq!(p.operation(), 1);
    assert!(!p.admission_allowed());
    assert!(p.take_delete().is_none() && p.take_write().is_none());
}

#[test]
fn counter_exhaustion_is_fail_closed_and_does_not_skip_host_abort() {
    let mut f = RunFixture::new(Some(true));
    f.protocol = Protocol::active_fixture(INSTANCE, SLOT, 1, u64::MAX).unwrap();
    assert_eq!(f.revoke(), Err(Error::CounterExhausted));
    assert!(f.provider.abort_requested());
    f.release(Owner::Adapter);
    f.release(Owner::Runtime);
    f.assert_terminal(RunState::Cancelled, "kept");
    assert_eq!(f.protocol.state(), State::Exhausted);
    assert_eq!(
        f.protocol
            .reconcile_absence(evidence(&f.protocol, CleanupProof::BothReleased)),
        Err(Error::Locked)
    );
    assert_eq!(f.protocol.registration_generation(), Err(Error::Locked));
    assert!(f.protocol.take_write().is_none() && f.protocol.take_delete().is_none());

    let (mut p, ticket) = at_write_generation(Stage::Confirmed, u64::MAX);
    p.complete_write(ticket, WriteOutcome::Committed).unwrap();
    assert_eq!(p.registration_generation(), Err(Error::CounterExhausted));
    assert!(!p.admission_allowed());

    let mut p = Protocol::active_fixture(INSTANCE, SLOT, 7, 0).unwrap();
    p.sequence = u64::MAX;
    assert_eq!(
        p.request_revoke(CleanupNeed::NotRequired),
        Err(Error::CounterExhausted)
    );
    assert_eq!(p.state(), State::Exhausted);
    assert_eq!(
        p.reconcile_absence(evidence(&p, CleanupProof::NotRequired)),
        Err(Error::Locked)
    );
    assert!(!p.admission_allowed());
    assert!(p.take_write().is_none());
}

#[test]
fn final_operation_id_cannot_be_reused_for_registration_after_confirmation() {
    let mut p = Protocol::active_fixture(INSTANCE, SLOT, 7, u64::MAX - 1).unwrap();
    p.request_revoke(CleanupNeed::NotRequired).unwrap();
    let intent = p.take_write().unwrap();
    p.complete_write(intent, WriteOutcome::Committed).unwrap();
    let delete_intent = p.take_write().unwrap();
    p.complete_write(delete_intent, WriteOutcome::Committed)
        .unwrap();
    let delete = p.take_delete().unwrap();
    p.complete_delete(delete, DeleteOutcome::Deleted).unwrap();
    p.reconcile_absence(evidence(&p, CleanupProof::NotRequired))
        .unwrap();
    let confirm = p.take_write().unwrap();
    p.complete_write(confirm, WriteOutcome::Committed).unwrap();
    assert_eq!(p.operation(), u64::MAX);
    assert_eq!(p.registration_generation(), Err(Error::CounterExhausted));
    let snapshot = p.record();
    let mut restarted = Protocol::restore(INSTANCE + 1, SLOT, snapshot, history(snapshot));
    restarted
        .reconcile_absence(evidence(&restarted, CleanupProof::NotRequired))
        .unwrap();
    let confirm = restarted.take_write().unwrap();
    restarted
        .complete_write(confirm, WriteOutcome::Committed)
        .unwrap();
    assert_eq!(
        restarted.registration_generation(),
        Err(Error::CounterExhausted)
    );
    assert!(!restarted.admission_allowed());
}

#[test]
fn fixture_does_not_enable_product_credentials_or_live_dispatch() {
    assert_eq!(live_dispatch(), Err(ErrorCode::ProviderDisabled));
    assert_eq!(DisabledCredentials.state(), CredentialState::Disabled);
    assert!(DisabledCredentials.resolve().is_err());
    assert!(Protocol::active_fixture(0, SLOT, 7, 0).is_err());
    assert!(Protocol::active_fixture(INSTANCE, 0, 7, 0).is_err());
    assert!(Protocol::active_fixture(INSTANCE, SLOT, 0, 0).is_err());
    let f = RunFixture::new(Some(true));
    assert_eq!(
        f.provider
            .events()
            .iter()
            .filter(|event| matches!(event.payload, Payload::ResponseDelta(_)))
            .count(),
        1
    );
}

#[test]
fn available_resolver_cannot_override_missing_history_lock() {
    let mut f = RunFixture::new(None);
    assert_eq!(f.provider.credentials.state(), CredentialState::Available);
    f.protocol = Protocol::restore(INSTANCE + 1, SLOT, None, Continuity::Missing);
    assert_eq!(f.protocol.state(), State::Quarantined);
    f.assert_locked();
    assert!(matches!(
        f.provider.credentials,
        MockResolver::Available { resolutions: 0, .. }
    ));
    assert_eq!(
        f.protocol.request_revoke(CleanupNeed::NotRequired),
        Err(Error::Locked)
    );
    assert!(f.protocol.take_write().is_none() && f.protocol.take_delete().is_none());
    assert!(f.deletions.is_empty());
}
