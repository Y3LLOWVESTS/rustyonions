use crate::{
    error::KmsError,
    types::{Alg, KeyId, KeyMeta},
};

/// Custody lifecycle — create/rotate/get metadata/attest (subset for core boot).
pub trait Keystore: Send + Sync {
    fn create_ed25519(&self, tenant: &str, purpose: &str) -> Result<KeyId, KmsError>;
    fn rotate(&self, kid: &KeyId) -> Result<KeyId, KmsError>;
    fn alg(&self, kid: &KeyId) -> Result<Alg, KmsError>;
    /// Public metadata about a key root.
    fn meta(&self, kid: &KeyId) -> Result<KeyMeta, KmsError>;
}
