//! Closed post-JSON-parse schema boundary; NOT a pre-allocation transport limit.
//! No command is registered with Tauri. Existing lifecycle validation must precede
//! these parsers when a later phase connects IPC.
use tauri::ipc::InvokeBody;
use super::*;

impl Start {
    pub fn parse(body: &InvokeBody, max_input_bytes: usize) -> Result<Self, ErrorCode> {
        let InvokeBody::Json(j) = body else { return Err(ErrorCode::InvalidInput); };
        let keys = ["namespace", "host_instance_id", "ui_context_epoch", "request_id", "text"];
        let obj = j.as_object().ok_or(ErrorCode::InvalidInput)?;
        if obj.len() != keys.len() || keys.iter().any(|key| !obj.contains_key(*key))
            || j["namespace"].as_str() != Some(NAMESPACE) { return Err(ErrorCode::InvalidInput); }
        let number = |key: &str| j[key].as_u64().and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n != 0).ok_or(ErrorCode::InvalidIdentity);
        let instance = j["host_instance_id"].as_str().filter(|s| valid_id(s)).ok_or(ErrorCode::InvalidIdentity)?;
        let text = j["text"].as_str().ok_or(ErrorCode::InvalidInput)?;
        if text.len() > max_input_bytes { return Err(ErrorCode::InputLimit); }
        if text.trim().is_empty() { return Err(ErrorCode::InvalidInput); }
        Ok(Self { session: UiSession { host_instance_id: instance.into(), ui_context_epoch: number("ui_context_epoch")? },
            request_id: number("request_id")?, text: text.into() })
    }
}

#[derive(Clone, Debug)]
pub struct Cancel {
    pub session: UiSession, pub run_id: u32, pub request_id: u32,
    pub attempt_id: u32, pub session_generation: u32,
}
impl Cancel {
    pub fn parse(body: &InvokeBody) -> Result<Self, ErrorCode> {
        let InvokeBody::Json(j) = body else { return Err(ErrorCode::InvalidInput); };
        let keys = ["namespace", "host_instance_id", "ui_context_epoch", "run_id", "request_id", "attempt_id", "session_generation"];
        let obj = j.as_object().ok_or(ErrorCode::InvalidInput)?;
        if obj.len() != keys.len() || keys.iter().any(|key| !obj.contains_key(*key))
            || j["namespace"].as_str() != Some(NAMESPACE) { return Err(ErrorCode::InvalidInput); }
        let number = |key: &str| j[key].as_u64().and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n != 0).ok_or(ErrorCode::InvalidIdentity);
        let instance = j["host_instance_id"].as_str().filter(|s| valid_id(s)).ok_or(ErrorCode::InvalidIdentity)?;
        Ok(Self { session: UiSession { host_instance_id: instance.into(), ui_context_epoch: number("ui_context_epoch")? },
            run_id: number("run_id")?, request_id: number("request_id")?, attempt_id: number("attempt_id")?,
            session_generation: number("session_generation")? })
    }
}
pub fn snapshot_session(body: &InvokeBody) -> Result<UiSession, ErrorCode> {
    let InvokeBody::Json(j) = body else { return Err(ErrorCode::InvalidInput); };
    let keys = ["namespace", "host_instance_id", "ui_context_epoch"];
    let obj = j.as_object().ok_or(ErrorCode::InvalidInput)?;
    if obj.len() != keys.len() || keys.iter().any(|key| !obj.contains_key(*key))
        || j["namespace"].as_str() != Some(NAMESPACE) { return Err(ErrorCode::InvalidInput); }
    let instance = j["host_instance_id"].as_str().filter(|s| valid_id(s)).ok_or(ErrorCode::InvalidIdentity)?;
    let epoch = j["ui_context_epoch"].as_u64().and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n != 0).ok_or(ErrorCode::InvalidIdentity)?;
    Ok(UiSession { host_instance_id: instance.into(), ui_context_epoch: epoch })
}
impl Host {
    pub fn cancel_command(&mut self, command: Cancel) -> Result<(), ErrorCode> {
        self.check_session(&command.session)?;
        let id = self.current().ok_or(ErrorCode::InvalidIdentity)?.identity.clone();
        if (command.run_id, command.request_id, command.attempt_id, command.session_generation)
            != (id.run_id, id.request_id, id.attempt_id, id.session_generation) { return Err(ErrorCode::InvalidIdentity); }
        self.cancel(&command.session, &id)
    }
}

impl Frame {
    /// The host separately binds the sender to an owned stream. A parsed identity
    /// alone is never authorization to fail an unrelated active run.
    pub fn parse(body: &InvokeBody, max_delta_bytes: usize) -> Result<Self, ErrorCode> {
        let InvokeBody::Json(j) = body else { return Err(ErrorCode::ProtocolError); };
        let keys = ["namespace", "host_instance_id", "run_id", "request_id", "attempt_id", "session_generation",
            "correlation_id", "source_kind", "source_seq", "type", "payload"];
        let obj = j.as_object().ok_or(ErrorCode::ProtocolError)?;
        if obj.len() != keys.len() || keys.iter().any(|key| !obj.contains_key(*key))
            || j["namespace"].as_str() != Some(NAMESPACE) { return Err(ErrorCode::ProtocolError); }
        if j["source_kind"].as_str() != Some("synthetic") { return Err(ErrorCode::InvalidIdentity); }
        let number = |key: &str| j[key].as_u64().and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n != 0).ok_or(ErrorCode::InvalidIdentity);
        let id = |key: &str| j[key].as_str().filter(|s| valid_id(s)).map(str::to_owned).ok_or(ErrorCode::InvalidIdentity);
        let identity = Identity { host_instance_id: id("host_instance_id")?, run_id: number("run_id")?,
            request_id: number("request_id")?, attempt_id: number("attempt_id")?,
            session_generation: number("session_generation")?, correlation_id: id("correlation_id")? };
        let p = j["payload"].as_object().ok_or(ErrorCode::ProtocolError)?;
        let payload = match j["type"].as_str() {
            Some("TextDelta") if p.len() == 1 && p.contains_key("text") => {
                let text = p["text"].as_str().filter(|s| !s.is_empty()).ok_or(ErrorCode::ProtocolError)?;
                if text.len() > max_delta_bytes { return Err(ErrorCode::OutputLimit); }
                AdapterPayload::TextDelta(text.into())
            }
            Some("Completed") if p.is_empty() => AdapterPayload::Completed,
            Some("Failed") if p.len() == 1 && p.contains_key("code") => {
                let code = match p["code"].as_str().ok_or(ErrorCode::ProtocolError)? {
                    "AUTH_REQUIRED" => ErrorCode::AuthRequired, "PERMISSION_DENIED" => ErrorCode::PermissionDenied,
                    "PROVIDER_RATE_LIMIT" => ErrorCode::ProviderRateLimit, "QUOTA_EXCEEDED" => ErrorCode::QuotaExceeded,
                    "OFFLINE" => ErrorCode::Offline, "PROVIDER_UNAVAILABLE" => ErrorCode::ProviderUnavailable,
                    "UNSUPPORTED_CONTENT" => ErrorCode::UnsupportedContent, _ => ErrorCode::ProviderError,
                };
                AdapterPayload::Failed(code)
            }
            _ => return Err(ErrorCode::ProtocolError),
        };
        Ok(Self { identity, source_seq: number("source_seq")?, payload })
    }
}

/// A bounded incremental decoder for a future byte-stream boundary. UTF-8 only;
/// synthetic adapter below normally uses already valid Rust strings.
pub struct Utf8Decoder { pending: Vec<u8>, cap: usize }
impl Utf8Decoder {
    pub fn new(cap: usize) -> Result<Self, ErrorCode> {
        if cap == 0 || cap > 1_048_576 { return Err(ErrorCode::PolicyUnconfigured); }
        Ok(Self { pending: Vec::new(), cap })
    }
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Option<String>, ErrorCode> {
        if bytes.len() > self.cap - self.pending.len() { self.pending.clear(); return Err(ErrorCode::BufferLimit); }
        self.pending.extend_from_slice(bytes);
        let length = match std::str::from_utf8(&self.pending) {
            Ok(_) => self.pending.len(),
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => { self.pending.clear(); return Err(ErrorCode::ProtocolError); }
        };
        if length == 0 { return Ok(None); }
        let text = std::str::from_utf8(&self.pending[..length]).unwrap().to_owned();
        self.pending.drain(..length);
        Ok(Some(text))
    }
    pub fn finish(&self) -> Result<(), ErrorCode> {
        if self.pending.is_empty() { Ok(()) } else { Err(ErrorCode::ProtocolError) }
    }
}
