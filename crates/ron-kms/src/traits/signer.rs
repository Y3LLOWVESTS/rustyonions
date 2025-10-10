// Signer trait scaffold
use crate::error::KmResult;

pub trait Signer {
    fn sign(&self, _msg: &[u8]) -> KmResult<Vec<u8>>;
}
