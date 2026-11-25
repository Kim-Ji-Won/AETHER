//! Offline protocol model, compiled only for tests. No disk, Keychain, IPC or worker.
//! The caller must revoke consent and request Host cancel/local abort immediately
//! after request_revoke, before polling effects. Waiting for a journal effect must
//! never retain the Host lock or suspend its clock/cleanup processing.
//!
//! Commit and absence observations below are fixture assumptions, not proof of OS
//! durability or authority. A future native owner must establish those facts and
//! issue fresh instance identities; no production issuer is provided here.

pub(crate) const SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    RevokeIntent,
    DeleteIntent,
    Confirmed,
}

// Deliberately no Debug, Serde, key material, request content or credential hash.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Record {
    pub(crate) schema: u32,
    pub(crate) slot: u64,
    pub(crate) generation: u64,
    pub(crate) operation: u64,
    pub(crate) stage: Stage,
    pub(crate) cleanup_required: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Binding {
    pub(crate) instance: u64,
    pub(crate) slot: u64,
    pub(crate) generation: u64,
    pub(crate) operation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Active,
    AwaitingIntent,
    AwaitingCleanup,
    AwaitingDeleteIntent,
    DeleteReady,
    Deleting,
    AwaitingAbsence,
    AwaitingConfirmation,
    Confirmed,
    Failed,
    Exhausted,
    Uncertain,
    RecoveryLocked,
    Quarantined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidInput,
    WrongTicket,
    Locked,
    EvidenceMismatch,
    CounterExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CleanupNeed {
    Required,
    NotRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CleanupProof {
    NotRequired,
    BothReleased,
    Unconfirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Adapter,
    Runtime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WriteOutcome {
    Committed,
    Failed,
    Uncertain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeleteOutcome {
    Deleted,
    Failed,
    Uncertain,
}

/// Verified is a test precondition supplied by a trusted history observer. This
/// model does not implement rollback detection or a process-wide owner lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Continuity {
    Verified { generation: u64, operation: u64 },
    Missing,
    Corrupt,
    RolledBack,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TicketKey {
    binding: Binding,
    sequence: u64,
}

// Tickets are move-only. Even a duplicated test callback is checked against the
// exact outstanding key and record before it can mutate protocol state.
pub(crate) struct WriteTicket {
    key: TicketKey,
    record: Record,
}
impl WriteTicket {
    pub(crate) fn record(&self) -> Record {
        self.record
    }
}

pub(crate) struct DeleteTicket {
    key: TicketKey,
}
impl DeleteTicket {
    pub(crate) fn binding(&self) -> Binding {
        self.key.binding
    }
    pub(crate) fn record(&self) -> Record {
        Record {
            schema: SCHEMA,
            slot: self.key.binding.slot,
            generation: self.key.binding.generation,
            operation: self.key.binding.operation,
            stage: Stage::DeleteIntent,
            cleanup_required: false,
        }
    }
}

/// Assumes the currently authorized original owner observed exact-slot absence,
/// established history continuity and established local cleanup. No arbitrary
/// renderer flag, OS not-found from another group, or stale ACK can mint this.
/// The only constructor is an explicit fixture assumption, not a native issuer.
pub(crate) struct AbsenceEvidence {
    binding: Binding,
    cleanup: CleanupProof,
}
impl AbsenceEvidence {
    #[cfg(test)]
    pub(crate) fn fixture(
        instance: u64,
        slot: u64,
        generation: u64,
        operation: u64,
        cleanup: CleanupProof,
    ) -> Self {
        Self {
            binding: Binding {
                instance,
                slot,
                generation,
                operation,
            },
            cleanup,
        }
    }
}

pub(crate) struct Protocol {
    binding: Binding,
    state: State,
    trusted_history: bool,
    cleanup_required: bool,
    adapter: bool,
    runtime: bool,
    accepted: bool,
    committed: Option<Record>,
    sequence: u64,
    write: Option<WriteTicket>,
    writing: Option<(TicketKey, Record)>,
    delete: Option<DeleteTicket>,
    deleting: Option<TicketKey>,
}

impl Protocol {
    fn locked(instance: u64, slot: u64) -> Self {
        Self {
            binding: Binding {
                instance,
                slot,
                generation: 0,
                operation: 0,
            },
            state: State::Quarantined,
            trusted_history: false,
            cleanup_required: true,
            adapter: false,
            runtime: false,
            accepted: false,
            committed: None,
            sequence: 0,
            write: None,
            writing: None,
            delete: None,
            deleting: None,
        }
    }

    /// An already registered fixture. Production bootstrap cannot call this.
    #[cfg(test)]
    pub(crate) fn active_fixture(
        instance: u64,
        slot: u64,
        generation: u64,
        operation: u64,
    ) -> Result<Self, Error> {
        if instance == 0 || slot == 0 || generation == 0 {
            return Err(Error::InvalidInput);
        }
        let mut model = Self::locked(instance, slot);
        model.binding.generation = generation;
        model.binding.operation = operation;
        model.state = State::Active;
        model.trusted_history = true;
        Ok(model)
    }

    /// Every restart is locked and effect-free, including a valid Confirmed
    /// record. Absence, cleanup and the same owner's authority must be observed
    /// afresh. Invalid history cannot be repaired by an absence assertion.
    pub(crate) fn restore(
        instance: u64,
        expected_slot: u64,
        snapshot: Option<Record>,
        continuity: Continuity,
    ) -> Self {
        let mut model = Self::locked(instance, expected_slot);
        let Some(record) = snapshot else { return model };
        model.committed = Some(record);
        model.binding.generation = record.generation;
        model.binding.operation = record.operation;
        let stage_valid = record.stage == Stage::RevokeIntent || !record.cleanup_required;
        let valid = instance != 0
            && expected_slot != 0
            && record.schema == SCHEMA
            && record.slot == expected_slot
            && record.generation != 0
            && record.operation != 0
            && stage_valid
            && continuity
                == (Continuity::Verified {
                    generation: record.generation,
                    operation: record.operation,
                });
        if valid {
            model.state = State::RecoveryLocked;
            model.trusted_history = true;
        }
        model
    }

    pub(crate) fn state(&self) -> State {
        self.state
    }
    pub(crate) fn binding(&self) -> Binding {
        self.binding
    }
    pub(crate) fn operation(&self) -> u64 {
        self.binding.operation
    }
    pub(crate) fn record(&self) -> Option<Record> {
        self.committed
    }
    pub(crate) fn admission_allowed(&self) -> bool {
        self.state == State::Active
    }
    pub(crate) fn durable_acceptance(&self) -> bool {
        self.accepted
    }
    pub(crate) fn cleanup_complete(&self) -> bool {
        !self.cleanup_required || (self.adapter && self.runtime)
    }

    fn key(&mut self) -> Result<TicketKey, Error> {
        let Some(next) = self.sequence.checked_add(1) else {
            self.state = State::Exhausted;
            return Err(Error::CounterExhausted);
        };
        self.sequence = next;
        Ok(TicketKey {
            binding: self.binding,
            sequence: next,
        })
    }

    fn queue_write(&mut self, stage: Stage) -> Result<(), Error> {
        let key = self.key()?;
        let record = Record {
            schema: SCHEMA,
            slot: self.binding.slot,
            generation: self.binding.generation,
            operation: self.binding.operation,
            stage,
            cleanup_required: !self.cleanup_complete(),
        };
        self.write = Some(WriteTicket { key, record });
        self.state = match stage {
            Stage::RevokeIntent => State::AwaitingIntent,
            Stage::DeleteIntent => State::AwaitingDeleteIntent,
            Stage::Confirmed => State::AwaitingConfirmation,
        };
        Ok(())
    }

    /// Locks first. Duplicate requests neither allocate another operation nor
    /// retry a write/delete. Caller cancel/abort must precede take_write().
    pub(crate) fn request_revoke(&mut self, cleanup: CleanupNeed) -> Result<(), Error> {
        match self.state {
            State::Active => {}
            State::Failed
            | State::Exhausted
            | State::Uncertain
            | State::RecoveryLocked
            | State::Quarantined => {
                return Err(Error::Locked);
            }
            _ => return Ok(()),
        }
        self.state = State::AwaitingIntent;
        self.cleanup_required = cleanup == CleanupNeed::Required;
        let Some(operation) = self.binding.operation.checked_add(1) else {
            self.state = State::Exhausted;
            return Err(Error::CounterExhausted);
        };
        self.binding.operation = operation;
        self.adapter = false;
        self.runtime = false;
        self.queue_write(Stage::RevokeIntent)
    }

    pub(crate) fn cleanup_ack(&mut self, binding: Binding, owner: Owner) -> Result<(), Error> {
        if binding != self.binding {
            return Err(Error::WrongTicket);
        }
        if matches!(
            self.state,
            State::Active | State::RecoveryLocked | State::Quarantined
        ) {
            return Err(Error::Locked);
        }
        if !self.cleanup_required {
            return Err(Error::InvalidInput);
        }
        match owner {
            Owner::Adapter => self.adapter = true,
            Owner::Runtime => self.runtime = true,
        }
        // Cleanup continues even when a journal operation failed. Failure never
        // turns the next ACK into permission to retry or delete.
        self.maybe_delete_intent()
    }

    fn maybe_delete_intent(&mut self) -> Result<(), Error> {
        if self.state == State::AwaitingCleanup && self.cleanup_complete() {
            self.queue_write(Stage::DeleteIntent)?;
        }
        Ok(())
    }

    pub(crate) fn take_write(&mut self) -> Option<WriteTicket> {
        let ticket = self.write.take()?;
        self.writing = Some((ticket.key, ticket.record));
        Some(ticket)
    }

    pub(crate) fn complete_write(
        &mut self,
        ticket: WriteTicket,
        outcome: WriteOutcome,
    ) -> Result<(), Error> {
        if self.writing != Some((ticket.key, ticket.record)) {
            return Err(Error::WrongTicket);
        }
        self.writing = None;
        match outcome {
            WriteOutcome::Failed => {
                self.state = State::Failed;
                return Ok(());
            }
            WriteOutcome::Uncertain => {
                self.state = State::Uncertain;
                return Ok(());
            }
            WriteOutcome::Committed => {}
        }
        self.committed = Some(ticket.record);
        match ticket.record.stage {
            Stage::RevokeIntent => {
                self.accepted = true;
                self.state = State::AwaitingCleanup;
                self.maybe_delete_intent()?;
            }
            Stage::DeleteIntent => {
                // Only the committed intent releases a single exact-slot permit.
                let key = self.key()?;
                self.delete = Some(DeleteTicket { key });
                self.state = State::DeleteReady;
            }
            Stage::Confirmed => self.state = State::Confirmed,
        }
        Ok(())
    }

    pub(crate) fn take_delete(&mut self) -> Option<DeleteTicket> {
        let ticket = self.delete.take()?;
        self.deleting = Some(ticket.key);
        self.state = State::Deleting;
        Some(ticket)
    }

    pub(crate) fn complete_delete(
        &mut self,
        ticket: DeleteTicket,
        outcome: DeleteOutcome,
    ) -> Result<(), Error> {
        if self.deleting != Some(ticket.key) {
            return Err(Error::WrongTicket);
        }
        self.deleting = None;
        self.state = match outcome {
            // Delete's return value alone is not exact-slot absence evidence.
            DeleteOutcome::Deleted => State::AwaitingAbsence,
            DeleteOutcome::Failed => State::Failed,
            DeleteOutcome::Uncertain => State::Uncertain,
        };
        Ok(())
    }

    /// Explicit reconciliation never issues a deletion. A remaining item needs
    /// a separate owner decision, outside this fixture's authority. A live
    /// operation still requires its own two ACKs; restart requires fresh proof
    /// that the old owner's resources have actually retired, not synthetic ACKs.
    pub(crate) fn reconcile_absence(&mut self, evidence: AbsenceEvidence) -> Result<(), Error> {
        if !matches!(
            self.state,
            State::AwaitingAbsence | State::Failed | State::Uncertain | State::RecoveryLocked
        ) {
            return Err(Error::Locked);
        }
        let cleanup_valid = match evidence.cleanup {
            CleanupProof::Unconfirmed => false,
            CleanupProof::BothReleased => {
                self.state == State::RecoveryLocked
                    || (self.cleanup_required && self.cleanup_complete())
            }
            CleanupProof::NotRequired => {
                if self.state == State::RecoveryLocked {
                    self.committed
                        .is_some_and(|record| !record.cleanup_required)
                } else {
                    !self.cleanup_required
                }
            }
        };
        if !self.trusted_history || evidence.binding != self.binding || !cleanup_valid {
            return Err(Error::EvidenceMismatch);
        }
        self.cleanup_required = evidence.cleanup == CleanupProof::BothReleased;
        self.adapter = self.cleanup_required;
        self.runtime = self.cleanup_required;
        self.queue_write(Stage::Confirmed)
    }

    /// Eligibility only: this is not a registration API and cannot open admission.
    /// Registration/rotation intent and their crash recovery remain unimplemented.
    pub(crate) fn registration_generation(&self) -> Result<u64, Error> {
        if self.state != State::Confirmed {
            return Err(Error::Locked);
        }
        // A new registration also needs a fresh operation, even when the final
        // representable revocation operation itself committed successfully.
        self.binding
            .operation
            .checked_add(1)
            .ok_or(Error::CounterExhausted)?;
        self.binding
            .generation
            .checked_add(1)
            .ok_or(Error::CounterExhausted)
    }
}

#[cfg(test)]
mod tests;
