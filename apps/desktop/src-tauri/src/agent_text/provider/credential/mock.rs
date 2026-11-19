//! Fixture variants have no constructor accepting key bytes, strings or OS handles.
use super::{CredentialError, CredentialResolver, CredentialState};

pub(crate) enum Sample { First, Replacement }
pub(crate) struct MockLease<'a> { pub(crate) sample: &'a Sample }

pub(crate) enum MockResolver {
    Missing,
    Available { sample: Sample, resolutions: u32 },
    AccessDenied,
    Unavailable,
    Revoked,
    Shutdown,
}

impl MockResolver {
    fn rejection(&self) -> CredentialError {
        match self {
            Self::Missing | Self::Revoked => CredentialError::Missing,
            Self::AccessDenied | Self::Available { .. } => CredentialError::AccessDenied,
            Self::Unavailable => CredentialError::Unavailable,
            Self::Shutdown => CredentialError::Disabled,
        }
    }
}

impl CredentialResolver for MockResolver {
    type Secret = Sample;
    type Lease<'a> = MockLease<'a>;

    fn state(&self) -> CredentialState {
        match self {
            Self::Missing => CredentialState::Missing,
            Self::Available { .. } => CredentialState::Available,
            Self::AccessDenied => CredentialState::AccessDenied,
            Self::Unavailable => CredentialState::Unavailable,
            Self::Revoked => CredentialState::Revoked,
            Self::Shutdown => CredentialState::Disabled,
        }
    }

    fn register(&mut self, sample: Sample) -> Result<(), CredentialError> {
        if !matches!(self, Self::Missing | Self::Revoked) {
            return Err(self.rejection());
        }
        *self = Self::Available { sample, resolutions: 0 };
        Ok(())
    }

    fn resolve(&mut self) -> Result<MockLease<'_>, CredentialError> {
        match self {
            Self::Available { sample, resolutions } => {
                *resolutions += 1;
                Ok(MockLease { sample })
            }
            _ => Err(self.rejection()),
        }
    }

    fn replace(&mut self, sample: Sample) -> Result<(), CredentialError> {
        if !matches!(self, Self::Available { .. }) { return Err(self.rejection()); }
        *self = Self::Available { sample, resolutions: 0 };
        Ok(())
    }

    fn revoke(&mut self) -> Result<(), CredentialError> {
        match self {
            Self::Available { .. } | Self::Revoked => { *self = Self::Revoked; Ok(()) }
            _ => Err(self.rejection()),
        }
    }

    fn shutdown(&mut self) { *self = Self::Shutdown; }
}
