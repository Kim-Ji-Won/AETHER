//! Native, memory-only adapter. The caller owns the Host, input capability and cleanup.
use super::*;

pub struct ProviderBudget(Ledger);

impl ProviderBudget {
    pub fn new(policy: OfflinePolicy) -> Result<Self, ErrorCode> {
        Ok(Self(Ledger::new(policy)?))
    }

    pub fn accounting(&self) -> Option<Accounting> { self.0.accounting }

    pub fn admit(&mut self, host: &mut Host, start: Start, model: &str) -> Result<ProviderAttempt, ErrorCode> {
        host.check_start(&start, "provider")?;
        let request = self.0.prepare(host.now_ms(), model, &start.text)?;
        let identity = admit(host, &mut self.0, start, &request, host.now_ms())?;
        Ok(ProviderAttempt { identity, request: Some(request), stream: None })
    }
}

pub(super) fn admit(host: &mut Host, ledger: &mut Ledger, start: Start, request: &RequestPreview, now: u64)
    -> Result<Identity, ErrorCode>
{
    let id = host.start_source(start, "provider")?;
    ledger.reserve(now, request);
    Ok(id)
}

pub struct ProviderAttempt {
    identity: Identity,
    request: Option<RequestPreview>,
    stream: Option<Stream>,
}

impl ProviderAttempt {
    pub fn identity(&self) -> &Identity { &self.identity }

    fn bound(&self, host: &Host) -> Result<(), ErrorCode> {
        if host.source_kind != "provider" { return Err(ErrorCode::InvalidIdentity); }
        host.check_identity(&self.identity)
    }

    pub fn dispatch(&mut self, host: &mut Host, budget: &ProviderBudget) -> Result<(), ErrorCode> {
        self.bound(host)?;
        self.stream = Some(Stream::dispatch(host, &self.identity, &budget.0, self.request.as_ref())?);
        self.request = None;
        Ok(())
    }

    /// True signals explicit completion. The owner must drop its reader before ACK.
    pub fn feed(&mut self, host: &mut Host, budget: &mut ProviderBudget, bytes: &[u8]) -> Result<bool, ErrorCode> {
        self.bound(host)?;
        if host.current().is_none_or(|run| run.state != State::Running) {
            return Err(ErrorCode::AlreadyTerminal);
        }
        let stream = self.stream.as_mut().ok_or(ErrorCode::AlreadyTerminal)?;
        let result = stream.feed(host, &self.identity, &mut budget.0, bytes).map_err(|e| e.apply(host));
        if !matches!(result, Ok(false)) { self.stream = None; }
        result
    }

    pub fn eof(&mut self, host: &mut Host) -> Result<(), ErrorCode> {
        self.bound(host)?;
        if host.current().is_none_or(|run| run.state != State::Running) {
            return Err(ErrorCode::AlreadyTerminal);
        }
        let stream = self.stream.take().ok_or(ErrorCode::AlreadyTerminal)?;
        Err(reject(host, stream.eof_error()))
    }

    pub fn discard(&mut self) {
        self.stream = None;
        self.request = None;
    }
}

pub(super) enum FeedError { Provider(ProviderError), Host(ErrorCode) }

impl From<ProviderError> for FeedError {
    fn from(e: ProviderError) -> Self { Self::Provider(e) }
}
impl From<ErrorCode> for FeedError {
    fn from(e: ErrorCode) -> Self { Self::Host(e) }
}
impl FeedError {
    pub(super) fn apply(self, host: &mut Host) -> ErrorCode {
        match self {
            Self::Provider(e) => reject(host, e),
            Self::Host(code) => code,
        }
    }
}

pub(super) fn reject(host: &mut Host, e: ProviderError) -> ErrorCode {
    let ingress = e.host_code();
    let terminal = if e == ProviderError::Duplicate { ErrorCode::ProtocolError } else { ingress };
    host.fail(terminal, Phase::Stream);
    ingress
}

impl Stream {
    pub(super) fn dispatch(host: &mut Host, id: &Identity, ledger: &Ledger, request: Option<&RequestPreview>) -> Result<Self, ErrorCode> {
        host.dispatch(id)?;
        let request = request.ok_or(ErrorCode::PolicyUnconfigured)?;
        Ok(Self {
            decoder: Decoder::new(ledger.policy.max_sse_frame_bytes, ledger.policy.max_stream_bytes),
            mapper: Mapper::new(host.policy.max_output_bytes, host.policy.max_delta_bytes, request.model_id()),
            canonical_seq: 0, events: 0,
        })
    }

    pub(super) fn eof_error(&self) -> ProviderError {
        self.decoder.finish().err().unwrap_or(ProviderError::Incomplete)
    }

    pub(super) fn feed(&mut self, host: &mut Host, id: &Identity, ledger: &mut Ledger, bytes: &[u8]) -> Result<bool, FeedError> {
        for byte in bytes {
            let Some(message) = self.decoder.push(*byte)? else { continue };
            self.events += 1;
            if self.events > transport::MAX_EVENTS { return Err(ProviderError::BufferLimit.into()); }
            let payload = match self.mapper.accept(message)? {
                Mapped::Metadata => continue,
                Mapped::Delta(text) => AdapterPayload::TextDelta(text),
                Mapped::Completed(usage) => {
                    ledger.record(usage)?;
                    AdapterPayload::Completed
                }
                Mapped::Failed(error, usage) => {
                    if let Some(usage) = usage { ledger.record(usage)?; }
                    return Err(error.into());
                }
            };
            let completed = matches!(payload, AdapterPayload::Completed);
            self.canonical_seq = self.canonical_seq.checked_add(1).ok_or(ErrorCode::CounterExhausted)?;
            host.ingest(Frame {identity: id.clone(), source_seq: self.canonical_seq, payload})?;
            if completed { return Ok(true); }
        }
        Ok(false)
    }
}
