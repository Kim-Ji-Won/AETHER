//! Interface only. No Keychain, environment, filesystem or secret-bearing test values.
//! A future resolver must use native user consent, bounded secret storage and audited
//! zeroization. Leases borrow the resolver: shutdown/replace cannot race a live lease.
//! Never implement Debug/Serialize/Clone for a real Secret/Lease or expose it via IPC.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialError { Disabled, Missing, AccessDenied, Unavailable }

pub trait CredentialResolver {
    type Secret;
    type Lease<'a> where Self: 'a;
    fn register(&mut self, secret: Self::Secret) -> Result<(), CredentialError>;
    fn resolve(&mut self) -> Result<Self::Lease<'_>, CredentialError>;
    fn replace(&mut self, secret: Self::Secret) -> Result<(), CredentialError>;
    fn revoke(&mut self) -> Result<(), CredentialError>;
    /// Release owned native handles/secret memory; no promise of erasing OS swap or copies.
    fn shutdown(&mut self);
}

pub struct DisabledCredentials;
impl CredentialResolver for DisabledCredentials {
    type Secret = std::convert::Infallible;
    type Lease<'a> = std::convert::Infallible;
    fn register(&mut self, never: Self::Secret) -> Result<(), CredentialError> { match never {} }
    fn resolve(&mut self) -> Result<Self::Lease<'_>, CredentialError> { Err(CredentialError::Disabled) }
    fn replace(&mut self, never: Self::Secret) -> Result<(), CredentialError> { match never {} }
    fn revoke(&mut self) -> Result<(), CredentialError> { Err(CredentialError::Disabled) }
    fn shutdown(&mut self) {}
}
