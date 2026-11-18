//! Offline native admission only. Production has no approval issuer and no live transport.
use super::credential::{CredentialError, CredentialResolver, CredentialState};
use super::{Cleanup, ErrorCode, Identity, OfflineProvider, RequestPreview, Start, UiSession};

#[derive(Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Host(ErrorCode),
    Credential(CredentialError),
    ConsentRequired,
    BindingChanged,
}

impl From<ErrorCode> for AdmissionError {
    fn from(error: ErrorCode) -> Self { Self::Host(error) }
}

impl From<CredentialError> for AdmissionError {
    fn from(error: CredentialError) -> Self { Self::Credential(error) }
}

// Exact, bounded, memory-only native request. No Debug, Clone, Serde or wire conversion.
#[derive(PartialEq)]
struct Binding {
    identity: Identity,
    session: UiSession,
    endpoint: &'static str,
    body: serde_json::Value,
    credential_generation: u64,
    policy_version: u64,
}

pub(super) struct ConsentState {
    approved: Option<Binding>,
    credential_generation: u64,
    policy_version: u64,
    revocation_error: Option<CredentialError>,
    pub(super) awaiting_new_context: bool,
}

impl ConsentState {
    pub(super) fn new() -> Self {
        Self { approved: None, credential_generation: 0, policy_version: 1,
            revocation_error: None, awaiting_new_context: false }
    }
    pub(super) fn withdraw(&mut self) { self.approved = None; }
}

impl<R: CredentialResolver> OfflineProvider<R> {
    /// A pending pre-run approval can be cancelled without creating a run/event.
    pub fn withdraw_consent(&mut self) -> Result<(), ErrorCode> {
        self.consent.withdraw();
        if self.host.current().is_some_and(|run| !run.state.terminal()) {
            self.host.request_cancel(super::CancelReason::UserCancel)?;
            if self.local_closed { self.request = None; } else { self.abort_requested = true; }
        }
        Ok(())
    }

    /// A native policy revision invalidates approval even if the content later changes back.
    pub fn advance_policy_version(&mut self) -> Result<(), ErrorCode> {
        self.withdraw_consent()?;
        self.consent.policy_version = self.consent.policy_version.checked_add(1)
            .ok_or(ErrorCode::CounterExhausted)?;
        Ok(())
    }

    fn credential_edit(&mut self) -> Result<u64, AdmissionError> {
        self.consent.withdraw();
        if self.host.closing { return Err(CredentialError::Disabled.into()); }
        if self.host.current().is_some_and(|run| !run.state.terminal()
            || matches!(run.cleanup, Cleanup::Pending | Cleanup::Unconfirmed)) {
            return Err(ErrorCode::Busy.into());
        }
        Ok(self.consent.credential_generation.checked_add(1).ok_or(ErrorCode::CounterExhausted)?)
    }

    pub fn register_credential(&mut self, secret: R::Secret) -> Result<(), AdmissionError> {
        let generation = self.credential_edit()?;
        self.credentials.register(secret)?;
        self.consent.credential_generation = generation;
        self.consent.revocation_error = None;
        Ok(())
    }

    pub fn rotate_credential(&mut self, secret: R::Secret) -> Result<(), AdmissionError> {
        let generation = self.credential_edit()?;
        if let Some(error) = self.consent.revocation_error { return Err(error.into()); }
        self.credentials.replace(secret)?;
        self.consent.credential_generation = generation;
        Ok(())
    }

    pub fn revoke_credential(&mut self) -> Result<(), AdmissionError> {
        let revoked = self.credentials.revoke();
        self.consent.revocation_error = Some(revoked.err().unwrap_or(CredentialError::Missing));
        self.withdraw_consent()?;
        Ok(revoked?)
    }

    fn prepare_binding(&mut self, start: &Start, model: &str, now: u64)
        -> Result<(RequestPreview, Binding), AdmissionError>
    {
        self.advance_to(now)?;
        if self.consent.awaiting_new_context { return Err(ErrorCode::StaleSession.into()); }
        let (identity, _) = self.host.check_start(start, "provider")?;
        let request = self.ledger.prepare(now, model, &start.text)?;
        if let Some(error) = self.consent.revocation_error { return Err(error.into()); }
        match self.credentials.state() {
            CredentialState::Available => {}
            CredentialState::Disabled => return Err(CredentialError::Disabled.into()),
            CredentialState::Missing | CredentialState::Revoked => return Err(CredentialError::Missing.into()),
            CredentialState::AccessDenied => return Err(CredentialError::AccessDenied.into()),
            CredentialState::Unavailable => return Err(CredentialError::Unavailable.into()),
        }
        let binding = Binding {
            identity, session: start.session.clone(), endpoint: request.endpoint(),
            body: request.body(), credential_generation: self.consent.credential_generation,
            policy_version: self.consent.policy_version,
        };
        Ok((request, binding))
    }

    // Models the native approval action. No production constructor, IPC flag or model output
    // can place a binding in the approved slot.
    #[cfg(test)]
    fn approve_fixture(&mut self, start: &Start, model: &str, now: u64) -> Result<(), AdmissionError> {
        self.consent.withdraw();
        let (_, binding) = self.prepare_binding(start, model, now)?;
        self.consent.approved = Some(binding);
        Ok(())
    }

    /// Check and consume in one exclusive operation. Every attempt burns the previous
    /// approval, including a rejected edit. No Host IDs or ledger units are spent on rejection.
    pub fn start_consented(&mut self, start: Start, model: &str, now: u64)
        -> Result<Identity, AdmissionError>
    {
        let approved = self.consent.approved.take();
        let (request, current) = self.prepare_binding(&start, model, now)?;
        let approved = approved.ok_or(AdmissionError::ConsentRequired)?;
        if approved != current { return Err(AdmissionError::BindingChanged); }
        // Only the matching one-time approval may resolve. The lease ends here;
        // this offline operation sends nothing and cannot retain a secret handle.
        drop(self.credentials.resolve()?);
        Ok(self.commit_start(start, request, now)?)
    }
}

#[cfg(test)]
mod tests;
