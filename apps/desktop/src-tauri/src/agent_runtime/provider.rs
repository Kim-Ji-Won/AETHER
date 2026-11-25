//! The native reader and producer context have independent lifetimes.
use super::*;
use contract::provider::adapter::{ProviderAttempt, ProviderBudget};
use std::convert::Infallible;
use std::sync::mpsc::{Receiver, TryRecvError};

pub(super) struct ProviderOwner {
    pub(super) attempt: ProviderAttempt,
    reader: Option<Receiver<Vec<u8>>>,
    context: Option<Receiver<Infallible>>,
}

impl ProviderOwner {
    #[cfg(test)]
    pub(super) fn queued(attempt: ProviderAttempt) -> Self {
        Self { attempt, reader: None, context: None }
    }

    #[cfg(test)]
    pub(super) fn dispatch(&mut self, host: &mut Host, budget: &ProviderBudget, id: &Identity)
        -> Result<FixtureProducer, ErrorCode>
    {
        if id != self.attempt.identity() { return Err(ErrorCode::InvalidIdentity); }
        self.attempt.dispatch(host, budget)?;
        let (sender, reader) = std::sync::mpsc::sync_channel(1);
        let (guard, context) = std::sync::mpsc::channel();
        self.reader = Some(reader);
        self.context = Some(context);
        Ok(FixtureProducer { sender, _guard: guard })
    }

    fn detach(&mut self) -> bool {
        let had_reader = self.reader.take().is_some();
        self.attempt.discard();
        had_reader
    }

    pub(super) fn context_lost(&mut self) {
        // Release the actual queue/parser now. The old context may still be alive.
        self.detach();
    }

    fn close_reader(&mut self, host: &mut Host) -> Result<(), ErrorCode> {
        if self.detach() { host.release(self.attempt.identity(), CleanupPart::Adapter, true)?; }
        Ok(())
    }

    pub(super) fn tick(&mut self, host: &mut Host, lost_run: &mut Option<u32>) -> Result<(), ErrorCode> {
        if host.current().is_some_and(|r| r.state == State::CancelRequested || r.state.terminal()) {
            self.close_reader(host)?;
        }
        // Dropping every producer guard is evidence that its context has retired.
        // A page replacement or a cancel request is insufficient.
        if self.reader.is_none() && self.context.as_ref().is_some_and(|c|
            matches!(c.try_recv(), Err(TryRecvError::Disconnected))) {
            let id = self.attempt.identity();
            if *lost_run == Some(id.run_id) {
                host.confirm_owned_cleanup(id.run_id)?;
                *lost_run = None;
            } else {
                host.release(id, CleanupPart::Runtime, true)?;
            }
            self.context = None;
        }
        Ok(())
    }

    pub(super) fn pump(&mut self, host: &mut Host, budget: &mut ProviderBudget) -> Result<(), ErrorCode> {
        let Some(reader) = &self.reader else { return Ok(()) };
        let result = match reader.try_recv() {
            Ok(bytes) => self.attempt.feed(host, budget, &bytes),
            Err(TryRecvError::Empty) => return Ok(()),
            Err(TryRecvError::Disconnected) => self.attempt.eof(host).map(|_| true),
        };
        if !matches!(result, Ok(false)) { self.close_reader(host)?; }
        result.map(|_| ())
    }
}

#[cfg(test)]
pub(super) struct FixtureProducer {
    sender: std::sync::mpsc::SyncSender<Vec<u8>>,
    _guard: std::sync::mpsc::Sender<Infallible>,
}

#[cfg(test)]
impl FixtureProducer {
    pub(super) fn send(&self, bytes: Vec<u8>) -> Result<(), ErrorCode> {
        if bytes.len() > 16 * 1024 { return Err(ErrorCode::BufferLimit); }
        self.sender.try_send(bytes).map_err(|e| match e {
            std::sync::mpsc::TrySendError::Full(_) => ErrorCode::Busy,
            std::sync::mpsc::TrySendError::Disconnected(_) => ErrorCode::InvalidIdentity,
        })
    }
}
