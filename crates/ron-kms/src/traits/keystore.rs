// Keystore trait scaffold
use crate::{types::KeyId, error::KmResult};

pub trait KeyStore {
    fn create(&self) -> KmResult<KeyId>;
}
