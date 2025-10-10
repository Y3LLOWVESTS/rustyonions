// Verifier trait scaffold
use crate::error::KmResult;

pub trait Verifier {
    fn verify(&self, _msg: &[u8], _sig: &[u8]) -> KmResult<bool>;
}
