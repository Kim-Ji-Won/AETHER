//! Allowlisted classifications only. Never retain body/message/param/header/URL.
use super::super::ErrorCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderError {
    InvalidRequest, Authentication, Permission, RateLimit, Quota, Credit, SpendLimit,
    Unknown429, Server, Offline, Disconnected, MalformedSse, Incomplete,
    UnsupportedContent, OutputLimit, BufferLimit, Duplicate, Sequence, Identity,
    LateEvent, UsageInvalid, Unknown,
}
impl ProviderError {
    /// Bounded future HTTP ingress. Raw bodies/URLs/headers are never projected.
    /// Invalid/oversized error bodies retain only a safe status classification.
    pub fn http_body(status: u16, body: &[u8]) -> Self {
        if body.len() > 8192 { return Self::http(status, None, None); }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) else {
            return Self::http(status, None, None);
        };
        let error = &value["error"];
        Self::http(status, error["code"].as_str(), error["type"].as_str())
    }
    pub fn http(status: u16, code: Option<&str>, kind: Option<&str>) -> Self {
        match status {
            400 => Self::InvalidRequest,
            401 => Self::Authentication,
            403 => Self::Permission,
            429 => match code {
                Some("credit_balance_exhausted") => Self::Credit,
                Some("organization_spend_limit_exceeded" | "project_spend_limit_exceeded") => Self::SpendLimit,
                Some("insufficient_quota" | "organization_usage_limit_exceeded") => Self::Quota,
                Some("rate_limit_exceeded" | "slow_down") => Self::RateLimit,
                _ if kind == Some("insufficient_quota") => Self::Quota,
                _ if kind == Some("rate_limit_error") => Self::RateLimit,
                _ => Self::Unknown429,
            },
            500..=599 => Self::Server,
            _ => Self::Unknown,
        }
    }
    pub(super) fn stream(code: Option<&str>) -> Self {
        match code {
            Some("server_error" | "server_is_overloaded") => Self::Server,
            Some("rate_limit_exceeded" | "slow_down") => Self::RateLimit,
            Some("credit_balance_exhausted") => Self::Credit,
            Some("organization_spend_limit_exceeded" | "project_spend_limit_exceeded") => Self::SpendLimit,
            Some("insufficient_quota" | "organization_usage_limit_exceeded") => Self::Quota,
            Some("invalid_api_key") => Self::Authentication,
            Some("invalid_prompt" | "invalid_request_error") => Self::InvalidRequest,
            _ => Self::Unknown,
        }
    }
    /// Existing canonical/UI vocabulary remains closed. Native classification keeps
    /// Credit/Spend/Quota distinct; P2.2b must explicitly version any UI expansion.
    pub fn host_code(self) -> ErrorCode {
        match self {
            Self::Authentication => ErrorCode::AuthRequired,
            Self::Permission => ErrorCode::PermissionDenied,
            Self::RateLimit => ErrorCode::ProviderRateLimit,
            Self::Quota | Self::Credit | Self::SpendLimit => ErrorCode::QuotaExceeded,
            Self::Server => ErrorCode::ProviderUnavailable,
            Self::Offline | Self::Disconnected => ErrorCode::Offline,
            Self::UnsupportedContent => ErrorCode::UnsupportedContent,
            Self::OutputLimit => ErrorCode::OutputLimit,
            Self::BufferLimit => ErrorCode::BufferLimit,
            Self::MalformedSse | Self::Sequence | Self::Identity | Self::UsageInvalid => ErrorCode::ProtocolError,
            Self::Duplicate => ErrorCode::DuplicateEvent,
            Self::LateEvent => ErrorCode::AlreadyTerminal,
            Self::InvalidRequest | Self::Incomplete | Self::Unknown429 | Self::Unknown => ErrorCode::ProviderError,
        }
    }
}
