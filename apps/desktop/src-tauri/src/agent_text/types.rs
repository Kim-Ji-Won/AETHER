//! Closed, credential-free types. No arbitrary provider error/message/metadata fields.
pub const NAMESPACE: &str = "agent.text.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State { Queued, Running, CancelRequested, Succeeded, Failed, Cancelled }
impl State {
    pub fn terminal(self) -> bool { matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleanup { NotRequired, Pending, Confirmed, Unconfirmed }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason { UserCancel, AppShutdown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteOutcome { NotDispatched, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Admission, Dispatch, Stream, Cleanup, Lifecycle }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryAction { None, NewRequest, FixInput, Configure }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput, InputLimit, PolicyUnconfigured, ProviderDisabled, Busy, RateLimited, CostLimit,
    DuplicateRequest, RequestConflict, StaleRequest, StaleSession, InvalidIdentity,
    DuplicateEvent, EventConflict, StaleEvent, OutOfOrder, AlreadyTerminal,
    AuthRequired, PermissionDenied, ProviderRateLimit, QuotaExceeded, Offline,
    ProviderUnavailable, ProviderError, RunTimeout, IdleTimeout, OutputLimit,
    BufferLimit, ProtocolError, UnsupportedContent, RuntimeLost, CleanupUnconfirmed,
    CounterExhausted, ClockRegression,
}
impl ErrorCode {
    pub fn retry(self) -> RetryAction {
        use ErrorCode::*;
        match self {
            InvalidInput | InputLimit => RetryAction::FixInput,
            PolicyUnconfigured | ProviderDisabled | AuthRequired | PermissionDenied | QuotaExceeded => RetryAction::Configure,
            Busy | RateLimited | CostLimit | ProviderRateLimit | Offline | ProviderUnavailable |
            ProviderError | RunTimeout | IdleTimeout | RuntimeLost => RetryAction::NewRequest,
            _ => RetryAction::None,
        }
    }
    /// Classification only. Never accepts/logs raw HTTP bodies, headers or credentials.
    pub fn from_status(status: u16) -> Self {
        match status {
            401 => Self::AuthRequired, 403 => Self::PermissionDenied,
            429 => Self::ProviderRateLimit, 500..=599 => Self::ProviderUnavailable,
            _ => Self::ProviderError,
        }
    }
    pub(super) fn provider_failure(self) -> Self {
        match self {
            Self::AuthRequired | Self::PermissionDenied | Self::ProviderRateLimit |
            Self::QuotaExceeded | Self::Offline | Self::ProviderUnavailable |
            Self::ProviderError | Self::UnsupportedContent => self,
            _ => Self::ProviderError,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorProjection {
    pub code: ErrorCode,
    pub phase: Phase,
    pub retry_action: RetryAction,
    pub partial_output: bool,
    pub remote_outcome: RemoteOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiSession { pub host_instance_id: String, pub ui_context_epoch: u32 }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub host_instance_id: String,
    pub run_id: u32,
    pub request_id: u32,
    pub attempt_id: u32,
    pub session_generation: u32,
    pub correlation_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start { pub session: UiSession, pub request_id: u32, pub text: String }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    RunCreated, RunStarted, ResponseDelta(String),
    CancelRequested(CancelReason), RunSucceeded,
    RunFailed(ErrorProjection), RunCancelled { reason: CancelReason, remote_outcome: RemoteOutcome },
}
impl Payload {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::RunCreated => "RunCreated", Self::RunStarted => "RunStarted",
            Self::ResponseDelta(_) => "ResponseDelta", Self::CancelRequested(_) => "CancelRequested",
            Self::RunSucceeded => "RunSucceeded", Self::RunFailed(_) => "RunFailed",
            Self::RunCancelled { .. } => "RunCancelled",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub namespace: &'static str,
    pub identity: Identity,
    pub source_kind: &'static str,
    pub seq: u32,
    pub payload: Payload,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterPayload { TextDelta(String), Completed, Failed(ErrorCode) }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame { pub identity: Identity, pub source_seq: u32, pub payload: AdapterPayload }
impl Frame {
    pub(super) fn text_bytes(&self) -> usize {
        match &self.payload { AdapterPayload::TextDelta(text) => text.len(), _ => 0 }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupPart { Adapter, Runtime }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunSnapshot {
    pub identity: Identity, pub state: State, pub last_seq: u32, pub terminal_count: u8,
    pub text: String, pub error: Option<ErrorProjection>, pub cleanup: Cleanup,
    pub cancel_reason: Option<CancelReason>, pub remote_outcome: RemoteOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub namespace: &'static str, pub source_kind: &'static str,
    pub session: UiSession, pub session_generation: u32, pub snapshot_revision: u64,
    pub request_high_water: u32, pub closing: bool, pub run: Option<RunSnapshot>,
}

/// Explicit host policy, not product defaults or a real provider price/quota guarantee.
#[derive(Clone, Debug)]
pub struct Policy {
    pub max_input_bytes: usize, pub max_delta_bytes: usize, pub max_output_bytes: usize,
    pub max_deltas: u32, pub max_queue_frames: usize, pub max_queue_bytes: usize,
    pub run_ms: u64, pub idle_ms: u64, pub cleanup_ms: u64,
    pub rate_window_ms: u64, pub max_starts_per_window: u32, pub synthetic_budget: u32,
}
impl Policy {
    pub fn validate(&self) -> Result<(), ErrorCode> {
        // Hard allocation ceilings; not measured product budgets. Require all policy values.
        let sizes = [self.max_input_bytes, self.max_delta_bytes, self.max_output_bytes, self.max_queue_bytes];
        let times = [self.run_ms, self.idle_ms, self.cleanup_ms, self.rate_window_ms];
        if sizes.iter().any(|n| *n == 0 || *n > 1_048_576)
            || times.iter().any(|n| *n == 0 || *n > 86_400_000)
            || self.max_queue_frames == 0 || self.max_queue_frames > 1024
            || self.max_deltas == 0 || self.max_deltas > 65_536
            || self.max_starts_per_window == 0 || self.synthetic_budget == 0 {
            return Err(ErrorCode::PolicyUnconfigured);
        }
        Ok(())
    }
}
pub(super) fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
