use crate::{error::KmsError, types::KeyId};

pub trait Verifier: Send + Sync {
    /// Verify the signature for `msg` under `kid`'s public key.
    fn verify(&self, kid: &KeyId, msg: &[u8], sig: &[u8]) -> Result<bool, KmsError>;
}
