//! Controlled synthetic text stream; no URL, network client, process, key or I/O.
use std::collections::VecDeque;
use super::*;

pub struct SyntheticAdapter {
    identity: Identity,
    queue: VecDeque<Frame>,
    queued_bytes: usize,
    max_frames: usize,
    max_bytes: usize,
    max_delta_bytes: usize,
    next_source_seq: u32,
    completion_enqueued: bool,
    closed: bool,
    abort_count: u32,
}
impl SyntheticAdapter {
    /// Single dispatch is consumed by Host, before any synthetic resource is constructed.
    pub fn dispatch(host: &mut Host, identity: &Identity) -> Result<Self, ErrorCode> {
        if host.source_kind != "synthetic" { return Err(ErrorCode::InvalidIdentity); }
        host.dispatch(identity)?;
        Ok(Self { identity: identity.clone(), queue: VecDeque::new(), queued_bytes: 0,
            max_frames: host.policy().max_queue_frames, max_bytes: host.policy().max_queue_bytes,
            max_delta_bytes: host.policy().max_delta_bytes, next_source_seq: 0,
            completion_enqueued: false, closed: false, abort_count: 0 })
    }
    pub fn enqueue(&mut self, host: &mut Host, payload: AdapterPayload) -> Result<(), ErrorCode> {
        host.check_identity(&self.identity)?;
        if self.closed || self.completion_enqueued { return Err(ErrorCode::AlreadyTerminal); }
        if host.current().is_some_and(|r| r.state != State::Running) { return Err(ErrorCode::AlreadyTerminal); }
        let bytes = match &payload { AdapterPayload::TextDelta(text) => text.len(), _ => 0 };
        if bytes > self.max_delta_bytes {
            host.stream_error(&self.identity, ErrorCode::OutputLimit)?; return Err(ErrorCode::OutputLimit);
        }
        if self.queue.len() >= self.max_frames || bytes > self.max_bytes - self.queued_bytes {
            host.stream_error(&self.identity, ErrorCode::BufferLimit)?;
            return Err(ErrorCode::BufferLimit);
        }
        self.next_source_seq = self.next_source_seq.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        self.completion_enqueued = matches!(payload, AdapterPayload::Completed | AdapterPayload::Failed(_));
        self.queued_bytes += bytes;
        self.queue.push_back(Frame { identity: self.identity.clone(), source_seq: self.next_source_seq, payload });
        Ok(())
    }
    /// Explicit barrier release: exactly one queued frame, no sleeps or background task.
    pub fn step(&mut self, host: &mut Host) -> Result<bool, ErrorCode> {
        host.check_identity(&self.identity)?;
        if self.closed { return Ok(false); }
        let Some(frame) = self.queue.pop_front() else { return Ok(false); };
        self.queued_bytes -= frame.text_bytes();
        host.ingest(frame)?;
        Ok(true)
    }
    pub fn abort(&mut self, host: &mut Host) -> Result<(), ErrorCode> {
        host.check_identity(&self.identity)?;
        // A normal running stream may not be silently aborted without a host decision.
        if host.current().is_some_and(|r| r.state == State::Running) { return Err(ErrorCode::InvalidInput); }
        if !self.closed { self.abort_count += 1; }
        self.closed = true; self.queue.clear(); self.queued_bytes = 0;
        host.release(&self.identity, CleanupPart::Adapter, true)
    }
    /// Local close after the host accepted Completed/Failed. Runtime ACK remains separate.
    pub fn close(&mut self, host: &mut Host) -> Result<(), ErrorCode> {
        host.check_identity(&self.identity)?;
        if !host.run.as_ref().is_some_and(|r| r.completed || r.view.state.terminal()) {
            return Err(ErrorCode::InvalidInput);
        }
        self.closed = true; self.queue.clear(); self.queued_bytes = 0;
        host.release(&self.identity, CleanupPart::Adapter, true)
    }
    pub fn abort_count(&self) -> u32 { self.abort_count }
    /// Stream termination is not completion. Report a missing Completed frame as
    /// a protocol failure, then release only this adapter's local resource.
    pub fn end_of_stream(&mut self, host: &mut Host) -> Result<(), ErrorCode> {
        host.check_identity(&self.identity)?;
        if host.current().is_some_and(|r| r.state == State::CancelRequested) {
            return self.abort(host);
        }
        if host.run.as_ref().is_some_and(|r| !r.completed && !r.view.state.terminal()) {
            host.stream_error(&self.identity, ErrorCode::ProtocolError)?;
        }
        self.close(host)
    }
    pub fn pending_frames(&self) -> usize { self.queue.len() }
    pub fn pending_bytes(&self) -> usize { self.queued_bytes }
}
