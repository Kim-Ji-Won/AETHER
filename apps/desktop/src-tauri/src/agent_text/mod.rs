//! Runtime-independent host ledger. Exclusive `&mut Host` is the
//! linearization boundary. No I/O, timers, global state, credentials or IPC registration.
mod types;
pub mod wire;
pub mod synthetic;
pub mod provider;
pub use types::*;

struct Run {
    view: RunSnapshot,
    input: String,
    last_frame: Option<Frame>,
    deltas: u32,
    run_deadline: u64,
    idle_deadline: Option<u64>,
    cleanup_deadline: Option<u64>,
    completed: bool,
    dispatched: bool,
    adapter_released: bool,
    runtime_released: bool,
}

pub struct Host {
    source_kind: &'static str,
    instance: String,
    legacy_epoch: Option<u32>,
    ui_epoch: u32,
    generation: u32,
    next_run: u32,
    next_attempt: u32,
    high_water: u32,
    revision: u64,
    now: u64,
    closing: bool,
    policy: Policy,
    window_start: u64,
    window_count: u32,
    budget_left: u32,
    run: Option<Run>,
    events: Vec<Event>,
}

impl Host {
    /// `instance` must come from the trusted host lifecycle, never client JSON.
    pub fn new(instance: &str, policy: Policy) -> Result<Self, ErrorCode> {
        if !valid_id(instance) { return Err(ErrorCode::InvalidIdentity); }
        policy.validate()?;
        Ok(Self {
            source_kind: "synthetic",
            instance: instance.into(), legacy_epoch: None, ui_epoch: 0, generation: 1,
            next_run: 0, next_attempt: 0, high_water: 0, revision: 0, now: 0,
            closing: false, window_start: 0, window_count: 0,
            budget_left: policy.synthetic_budget, policy, run: None, events: Vec::new(),
        })
    }

    /// Host-only bridge entry, NOT an IPC command or proof of a lifecycle check.
    /// Future integration must call this under the existing lifecycle Security Gate.
    /// Existing legacy epoch 0 remains 0 there; this namespace maps it to wire 1.
    pub fn attach(&mut self, instance: &str, legacy_epoch: u32, lifecycle_live: bool) -> Result<UiSession, ErrorCode> {
        if self.closing || !lifecycle_live || instance != self.instance { return Err(ErrorCode::StaleSession); }
        if self.legacy_epoch.is_some_and(|previous| legacy_epoch < previous) { return Err(ErrorCode::StaleSession); }
        let wire_epoch = legacy_epoch.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        if self.legacy_epoch != Some(legacy_epoch) {
            self.capacity()?;
            self.legacy_epoch = Some(legacy_epoch);
            self.ui_epoch = wire_epoch;
            self.bump();
        }
        Ok(self.session())
    }
    fn session(&self) -> UiSession {
        UiSession { host_instance_id: self.instance.clone(), ui_context_epoch: self.ui_epoch }
    }
    fn check_session(&self, session: &UiSession) -> Result<(), ErrorCode> {
        if self.closing || self.legacy_epoch.is_none() || *session != self.session() {
            return Err(ErrorCode::StaleSession);
        }
        Ok(())
    }
    pub fn snapshot(&self, session: &UiSession) -> Result<Snapshot, ErrorCode> {
        self.check_session(session)?;
        Ok(self.inspect())
    }
    /// Trusted owner inspection, including closing; IPC callers still need the lifecycle gate.
    pub fn inspect(&self) -> Snapshot {
        Snapshot {
            namespace: NAMESPACE, source_kind: self.source_kind, session: self.session(),
            session_generation: self.generation, snapshot_revision: self.revision,
            request_high_water: self.high_water, closing: self.closing,
            run: self.run.as_ref().map(|r| r.view.clone()),
        }
    }
    /// Host inspection, including shutdown. Not exposed to an unvalidated UI.
    pub fn current(&self) -> Option<&RunSnapshot> { self.run.as_ref().map(|r| &r.view) }
    pub fn events(&self) -> &[Event] { &self.events }
    pub fn policy(&self) -> &Policy { &self.policy }
    pub fn now_ms(&self) -> u64 { self.now }
    fn capacity(&self) -> Result<(), ErrorCode> {
        // Reserve revision space for a command and its possible terminal/cleanup events.
        if self.revision > u64::MAX - 8 { Err(ErrorCode::CounterExhausted) } else { Ok(()) }
    }
    fn bump(&mut self) { self.revision += 1; }
    fn emit(&mut self, payload: Payload) {
        let run = self.run.as_mut().expect("host transition requires admitted run");
        // At most max_deltas + Created/Started/CancelRequested/terminal, validated policy.
        run.view.last_seq += 1;
        self.events.push(Event { namespace: NAMESPACE, identity: run.view.identity.clone(),
            source_kind: self.source_kind, seq: run.view.last_seq, payload });
        self.bump();
    }
    fn check_identity(&self, id: &Identity) -> Result<(), ErrorCode> {
        let run = self.run.as_ref().ok_or(ErrorCode::InvalidIdentity)?;
        if &run.view.identity != id || id.session_generation != self.generation {
            return Err(ErrorCode::InvalidIdentity);
        }
        Ok(())
    }

    pub fn start(&mut self, start: Start) -> Result<Identity, ErrorCode> {
        self.start_source(start, "synthetic")
    }
    // Native child module only; no source field is accepted from Renderer IPC.
    fn start_source(&mut self, start: Start, source: &'static str) -> Result<Identity, ErrorCode> {
        self.check_session(&start.session)?;
        self.capacity()?;
        if start.request_id == 0 { return Err(ErrorCode::InvalidIdentity); }
        if start.text.len() > self.policy.max_input_bytes { return Err(ErrorCode::InputLimit); }
        if start.text.trim().is_empty() { return Err(ErrorCode::InvalidInput); }
        if start.request_id < self.high_water { return Err(ErrorCode::StaleRequest); }
        if start.request_id == self.high_water {
            return Err(if self.run.as_ref().is_some_and(|r| r.input == start.text) {
                ErrorCode::DuplicateRequest
            } else { ErrorCode::RequestConflict });
        }
        if self.run.as_ref().is_some_and(|r| !r.view.state.terminal()
            || matches!(r.view.cleanup, Cleanup::Pending | Cleanup::Unconfirmed)) {
            return Err(ErrorCode::Busy);
        }
        let run_id = self.next_run.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        let attempt_id = self.next_attempt.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        let reset_window = self.now - self.window_start >= self.policy.rate_window_ms;
        let count = if reset_window { 0 } else { self.window_count };
        if count >= self.policy.max_starts_per_window { return Err(ErrorCode::RateLimited); }
        if source == "synthetic" && self.budget_left == 0 { return Err(ErrorCode::CostLimit); }
        let identity = Identity { host_instance_id: self.instance.clone(), run_id,
            request_id: start.request_id, attempt_id, session_generation: self.generation,
            correlation_id: format!("run-{run_id}-attempt-{attempt_id}") };
        // Commit admission only after all checks. Rejections do not consume IDs or quota.
        self.source_kind = source;
        self.next_run = run_id; self.next_attempt = attempt_id; self.high_water = start.request_id;
        if reset_window { self.window_start = self.now; }
        self.window_count = count + 1;
        if source == "synthetic" { self.budget_left -= 1; }
        self.events.clear();
        self.run = Some(Run {
            view: RunSnapshot { identity: identity.clone(), state: State::Queued, last_seq: 0,
                terminal_count: 0, text: String::new(), error: None, cleanup: Cleanup::NotRequired,
                cancel_reason: None, remote_outcome: RemoteOutcome::NotDispatched },
            input: start.text, last_frame: None, deltas: 0, run_deadline: self.now + self.policy.run_ms,
            idle_deadline: None, cleanup_deadline: None, completed: false, dispatched: false,
            adapter_released: false, runtime_released: false,
        });
        self.emit(Payload::RunCreated);
        Ok(identity)
    }
    /// Consumes the one dispatch gate. Synthetic adapter construction follows this call.
    pub fn dispatch(&mut self, id: &Identity) -> Result<(), ErrorCode> {
        self.check_identity(id)?; self.capacity()?;
        if self.closing { return Err(ErrorCode::StaleSession); }
        let run = self.run.as_mut().unwrap();
        if run.view.state != State::Queued { return Err(ErrorCode::AlreadyTerminal); }
        run.view.state = State::Running; run.dispatched = true;
        run.view.remote_outcome = RemoteOutcome::Unknown;
        run.view.cleanup = Cleanup::Pending;
        run.idle_deadline = Some(self.now + self.policy.idle_ms);
        self.emit(Payload::RunStarted);
        Ok(())
    }
    pub fn cancel(&mut self, session: &UiSession, id: &Identity) -> Result<(), ErrorCode> {
        self.check_session(session)?; self.check_identity(id)?; self.capacity()?;
        self.request_cancel(CancelReason::UserCancel)
    }
    fn request_cancel(&mut self, reason: CancelReason) -> Result<(), ErrorCode> {
        let run = self.run.as_mut().ok_or(ErrorCode::InvalidIdentity)?;
        match run.view.state {
            State::CancelRequested | State::Cancelled => return Ok(()),
            State::Succeeded | State::Failed => return Err(ErrorCode::AlreadyTerminal),
            _ => {}
        }
        run.view.state = State::CancelRequested;
        run.view.cancel_reason = Some(reason);
        run.idle_deadline = None;
        if run.dispatched && run.cleanup_deadline.is_none() {
            run.cleanup_deadline = Some(self.now + self.policy.cleanup_ms);
        }
        self.emit(Payload::CancelRequested(reason));
        // C01: no adapter/Runtime exists yet. No fabricated release/abort/ACK required.
        self.settle();
        Ok(())
    }
    fn settle(&mut self) {
        let run = self.run.as_mut().unwrap();
        let clean = !run.dispatched || (run.adapter_released && run.runtime_released);
        if !clean { return; }
        let cleanup = if run.dispatched { Cleanup::Confirmed } else { Cleanup::NotRequired };
        let changed = run.view.cleanup != cleanup;
        run.view.cleanup = cleanup;
        run.cleanup_deadline = None;
        let payload = match run.view.state {
            State::CancelRequested => {
                run.view.state = State::Cancelled; run.view.terminal_count = 1;
                Some(Payload::RunCancelled { reason: run.view.cancel_reason.unwrap(), remote_outcome: run.view.remote_outcome })
            }
            State::Running if run.completed => {
                run.view.state = State::Succeeded; run.view.terminal_count = 1;
                Some(Payload::RunSucceeded)
            }
            _ => None,
        };
        if let Some(payload) = payload { self.emit(payload); } else if changed { self.bump(); }
    }
    fn fail(&mut self, code: ErrorCode, phase: Phase) {
        let Some(run) = self.run.as_mut() else { return; };
        if run.view.state.terminal() { return; }
        let error = ErrorProjection { code, phase, retry_action: code.retry(),
            partial_output: !run.view.text.is_empty(), remote_outcome: run.view.remote_outcome };
        run.view.state = State::Failed; run.view.terminal_count = 1;
        run.view.error = Some(error.clone()); run.idle_deadline = None;
        if run.dispatched && !(run.adapter_released && run.runtime_released) {
            run.view.cleanup = if code == ErrorCode::CleanupUnconfirmed { Cleanup::Unconfirmed } else { Cleanup::Pending };
            if run.cleanup_deadline.is_none() { run.cleanup_deadline = Some(self.now + self.policy.cleanup_ms); }
        }
        self.emit(Payload::RunFailed(error));
    }

    pub fn ingest(&mut self, frame: Frame) -> Result<(), ErrorCode> {
        self.check_identity(&frame.identity)?; self.capacity()?;
        if frame.source_seq == 0 { return self.protocol_failure(ErrorCode::InvalidIdentity); }
        let run = self.run.as_ref().unwrap();
        if let Some(previous) = &run.last_frame {
            if frame.source_seq == previous.source_seq {
                return if frame == *previous { Err(ErrorCode::DuplicateEvent) }
                    else { self.protocol_failure(ErrorCode::EventConflict) };
            }
            if frame.source_seq < previous.source_seq { return Err(ErrorCode::StaleEvent); }
        }
        if run.view.state.terminal() || run.view.state == State::CancelRequested { return Err(ErrorCode::AlreadyTerminal); }
        let next = run.last_frame.as_ref().map_or(1, |f| f.source_seq + 1);
        if frame.source_seq != next { return self.protocol_failure(ErrorCode::OutOfOrder); }
        if run.view.state != State::Running || run.completed { return self.protocol_failure(ErrorCode::ProtocolError); }
        match &frame.payload {
            AdapterPayload::TextDelta(text) => {
                if text.is_empty() { return self.protocol_failure(ErrorCode::ProtocolError); }
                if text.len() > self.policy.max_delta_bytes || run.view.text.len() + text.len() > self.policy.max_output_bytes
                    || run.deltas >= self.policy.max_deltas {
                    self.fail(ErrorCode::OutputLimit, Phase::Stream); return Err(ErrorCode::OutputLimit);
                }
                let run = self.run.as_mut().unwrap();
                run.view.text.push_str(text); run.deltas += 1;
                run.idle_deadline = Some(self.now + self.policy.idle_ms);
                self.emit(Payload::ResponseDelta(text.clone()));
            }
            AdapterPayload::Completed => {
                let run = self.run.as_mut().unwrap();
                run.completed = true; run.idle_deadline = None;
                run.cleanup_deadline = Some(self.now + self.policy.cleanup_ms);
                self.bump();
            }
            AdapterPayload::Failed(code) => self.fail(code.provider_failure(), Phase::Stream),
        }
        self.run.as_mut().unwrap().last_frame = Some(frame);
        self.settle();
        Ok(())
    }
    fn protocol_failure(&mut self, rejection: ErrorCode) -> Result<(), ErrorCode> {
        if self.run.as_ref().is_some_and(|r| r.view.state != State::CancelRequested) {
            self.fail(ErrorCode::ProtocolError, Phase::Stream);
        }
        Err(rejection)
    }
    /// Called only for a malformed/error input from an already bound, current stream.
    pub fn stream_error(&mut self, id: &Identity, code: ErrorCode) -> Result<(), ErrorCode> {
        self.check_identity(id)?; self.capacity()?;
        let state = self.run.as_ref().unwrap().view.state;
        if state.terminal() || state == State::CancelRequested { return Err(ErrorCode::AlreadyTerminal); }
        let safe = match code { ErrorCode::BufferLimit | ErrorCode::OutputLimit | ErrorCode::ProtocolError | ErrorCode::UnsupportedContent => code,
            _ => ErrorCode::ProviderError };
        self.fail(safe, Phase::Stream);
        Ok(())
    }
    /// Confirmed means owned local resource release, never remote billing cancellation.
    pub fn release(&mut self, id: &Identity, part: CleanupPart, confirmed: bool) -> Result<(), ErrorCode> {
        self.check_identity(id)?; self.capacity()?;
        let run = self.run.as_mut().unwrap();
        if !run.dispatched || !confirmed { return Ok(()); }
        match part { CleanupPart::Adapter => run.adapter_released = true, CleanupPart::Runtime => run.runtime_released = true }
        self.settle();
        Ok(())
    }
    /// The driver advances a monotonic clock before delivering inputs. No wall clock or sleep.
    pub fn advance_to(&mut self, now: u64) -> Result<(), ErrorCode> {
        if now < self.now { return Err(ErrorCode::ClockRegression); }
        // Reserve safe addition space for every validated deadline.
        if now > u64::MAX - 86_400_000 { return Err(ErrorCode::ClockRegression); }
        self.capacity()?; self.now = now;
        let Some(run) = self.run.as_mut() else { return Ok(()); };
        if run.view.state.terminal() {
            if run.view.cleanup == Cleanup::Pending && run.cleanup_deadline.is_some_and(|t| now >= t) {
                run.view.cleanup = Cleanup::Unconfirmed; self.bump();
            }
            return Ok(());
        }
        let timeout = if run.view.state == State::CancelRequested {
            run.cleanup_deadline.filter(|t| now >= *t).map(|_| (ErrorCode::CleanupUnconfirmed, Phase::Cleanup))
        } else {
            let mut deadlines = vec![(run.run_deadline, ErrorCode::RunTimeout, Phase::Dispatch)];
            if let Some(t) = run.idle_deadline { deadlines.push((t, ErrorCode::IdleTimeout, Phase::Stream)); }
            if let Some(t) = run.cleanup_deadline { deadlines.push((t, ErrorCode::CleanupUnconfirmed, Phase::Cleanup)); }
            deadlines.into_iter().min_by_key(|x| x.0).filter(|x| now >= x.0).map(|x| (x.1, x.2))
        };
        if let Some((code, phase)) = timeout { self.fail(code, phase); }
        Ok(())
    }
    pub fn shutdown(&mut self) -> Result<(), ErrorCode> {
        self.capacity()?;
        if !self.closing { self.closing = true; self.bump(); }
        if self.run.as_ref().is_some_and(|r| !r.view.state.terminal()) { self.request_cancel(CancelReason::AppShutdown)?; }
        Ok(())
    }
    /// Logical connection loss only; no A/B process or lifecycle implementation here.
    pub fn runtime_lost(&mut self) -> Result<(), ErrorCode> {
        self.capacity()?;
        let next = self.generation.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
        if self.run.as_ref().is_some_and(|r| r.view.state == State::CancelRequested) {
            // Cancellation already won; retain its cleanup deadline, don't change terminal reason.
        } else { self.fail(ErrorCode::RuntimeLost, Phase::Lifecycle); }
        self.generation = next; self.bump();
        Ok(())
    }
    /// Trusted owner confirmation after connection loss (old Runtime ACKs remain rejected).
    pub fn confirm_owned_cleanup(&mut self, run_id: u32) -> Result<(), ErrorCode> {
        self.capacity()?;
        let run = self.run.as_mut().ok_or(ErrorCode::InvalidIdentity)?;
        if run.view.identity.run_id != run_id { return Err(ErrorCode::InvalidIdentity); }
        if !run.view.state.terminal() && run.view.state != State::CancelRequested { return Err(ErrorCode::InvalidInput); }
        run.adapter_released = true; run.runtime_released = true;
        self.settle();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
