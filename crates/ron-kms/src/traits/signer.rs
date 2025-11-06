use crate::{error::KmsError, types::KeyId};

pub trait Signer: Send + Sync {
    /// Sign message bytes with the private key designated by `kid`.
    fn sign(&self, kid: &KeyId, msg: &[u8]) -> Result<Vec<u8>, KmsError>;
}
