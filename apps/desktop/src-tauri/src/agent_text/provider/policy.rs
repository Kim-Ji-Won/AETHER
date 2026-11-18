//! Offline policy arithmetic, NOT model pricing, a tokenizer or a billing hard cap.
use super::super::ErrorCode;
use super::error::ProviderError;
use serde_json::{json, Value};

pub const ENDPOINT: &str = "https://api.openai.com/v1/responses";

#[derive(Clone)]
pub struct ModelPolicy {
    pub id: String,
    pub max_input_units: u64,
    pub max_output_tokens: u64,
    /// Fictional integer units per token. Real pricing/tier/cache rules are NOT configured.
    pub input_unit_price: u64,
    pub output_unit_price: u64,
}
pub struct OfflinePolicy {
    pub models: Vec<ModelPolicy>,
    pub total_cost_units: u64,
    pub max_requests_per_window: u32,
    pub rate_window_ms: u64,
    pub max_sse_frame_bytes: usize,
    pub max_stream_bytes: usize,
}
impl OfflinePolicy {
    pub fn validate(&self) -> Result<(), ErrorCode> {
        if self.models.is_empty() || self.models.len() > 16 || self.total_cost_units == 0
            || self.max_requests_per_window == 0 || self.rate_window_ms == 0
            || self.rate_window_ms > 86_400_000 || !(256..=1_048_576).contains(&self.max_sse_frame_bytes)
            || self.max_stream_bytes < self.max_sse_frame_bytes || self.max_stream_bytes > 16_777_216 {
            return Err(ErrorCode::PolicyUnconfigured);
        }
        for (i,m) in self.models.iter().enumerate() {
            if m.id.is_empty() || m.id.len() > 128
                || !m.id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b'-'|b'_'|b'.'))
                || m.max_input_units == 0 || m.max_output_tokens == 0
                || m.max_input_units > 1_048_576 || m.max_output_tokens > 1_048_576
                || m.input_unit_price == 0 || m.output_unit_price == 0
                || self.models[..i].iter().any(|prev| prev.id == m.id) {
                return Err(ErrorCode::PolicyUnconfigured);
            }
            m.cost(m.max_input_units, m.max_output_tokens).ok_or(ErrorCode::PolicyUnconfigured)?;
        }
        Ok(())
    }
}
impl ModelPolicy {
    fn cost(&self, input: u64, output: u64) -> Option<u64> {
        input.checked_mul(self.input_unit_price)?.checked_add(output.checked_mul(self.output_unit_price)?)
    }
}

/// Memory-only preview; no Debug/Serialize or transport. Never accepts URL/headers/tools.
pub struct RequestPreview {
    model: ModelPolicy,
    text: String,
    pub estimated_input_units: u64,
    pub reserved_cost_units: u64,
}
impl RequestPreview {
    pub(super) fn model_id(&self) -> &str { &self.model.id }
    pub fn endpoint(&self) -> &'static str { ENDPOINT }
    /// Explicit native inspection for offline tests/future transport. Contains user text;
    /// must never be logged or sent to diagnostics. No credential or Authorization field.
    pub fn body(&self) -> Value {
        json!({"model": self.model.id, "input": [{"role":"user","content":[{"type":"input_text","text":self.text}]}],
            "stream":true,"store":false,"background":false,"tools":[],"tool_choice":"none",
            "parallel_tool_calls":false,"max_output_tokens":self.model.max_output_tokens})
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Usage { pub input_tokens: u64, pub output_tokens: u64, pub total_tokens: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accounting {
    pub reserved_cost_units: u64,
    pub reported_usage: Option<Usage>,
    pub reported_cost_units: Option<u64>,
    pub policy_exceeded: bool,
    /// Usage is provider-reported, not an invoice; local cleanup never settles billing.
    pub billing_confirmed: bool,
}
pub(super) struct Ledger {
    pub policy: OfflinePolicy,
    spent: u64,
    window_start: u64,
    count: u32,
    last_now: u64,
    active_model: Option<ModelPolicy>,
    pub accounting: Option<Accounting>,
}
impl Ledger {
    pub fn new(policy: OfflinePolicy) -> Result<Self, ErrorCode> {
        policy.validate()?;
        Ok(Self { policy, spent:0, window_start:0, count:0, last_now:0, active_model:None, accounting:None })
    }
    /// Pure check. The driver reserves only after Host admission succeeds.
    pub fn prepare(&self, now: u64, model: &str, text: &str) -> Result<RequestPreview, ErrorCode> {
        if now < self.last_now { return Err(ErrorCode::ClockRegression); }
        let model = self.policy.models.iter().find(|m| m.id == model).ok_or(ErrorCode::PolicyUnconfigured)?;
        if text.trim().is_empty() { return Err(ErrorCode::InvalidInput); }
        // UTF-8 bytes as explicit fictional input units, not a claim of actual token count.
        let units = text.len() as u64;
        if units > model.max_input_units { return Err(ErrorCode::InputLimit); }
        let cost = model.cost(units, model.max_output_tokens).ok_or(ErrorCode::CostLimit)?;
        if cost > self.policy.total_cost_units.saturating_sub(self.spent) { return Err(ErrorCode::CostLimit); }
        if now - self.window_start < self.policy.rate_window_ms && self.count >= self.policy.max_requests_per_window {
            return Err(ErrorCode::RateLimited);
        }
        Ok(RequestPreview { model:model.clone(), text:text.into(), estimated_input_units:units, reserved_cost_units:cost })
    }
    pub fn reserve(&mut self, now: u64, request: &RequestPreview) {
        if now - self.window_start >= self.policy.rate_window_ms { self.window_start=now; self.count=0; }
        self.last_now=now; self.count+=1; self.spent+=request.reserved_cost_units;
        self.active_model=Some(request.model.clone());
        self.accounting=Some(Accounting { reserved_cost_units:request.reserved_cost_units, reported_usage:None,
            reported_cost_units:None, policy_exceeded:false, billing_confirmed:false });
    }
    pub fn record(&mut self, usage: Usage) -> Result<(), ProviderError> {
        let a=self.accounting.as_mut().ok_or(ProviderError::UsageInvalid)?;
        if a.reported_usage.is_some() || usage.input_tokens.checked_add(usage.output_tokens)!=Some(usage.total_tokens) {
            return Err(ProviderError::UsageInvalid);
        }
        let m=self.active_model.as_ref().ok_or(ProviderError::UsageInvalid)?;
        let cost=m.cost(usage.input_tokens,usage.output_tokens).ok_or(ProviderError::UsageInvalid)?;
        a.reported_usage=Some(usage); a.reported_cost_units=Some(cost);
        a.policy_exceeded=usage.input_tokens>m.max_input_units || usage.output_tokens>m.max_output_tokens || cost>a.reserved_cost_units;
        // Never refund uncertain reservations. Account for an overrun before admitting more.
        self.spent=self.spent.saturating_add(cost.saturating_sub(a.reserved_cost_units));
        if a.policy_exceeded { Err(ProviderError::UsageInvalid) } else { Ok(()) }
    }
}
