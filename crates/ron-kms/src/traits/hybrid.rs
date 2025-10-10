// Hybrid trait scaffold
use crate::error::KmResult;

pub trait Hybrid {
    fn wrap(&self, _pt: &[u8]) -> KmResult<Vec<u8>>;
    fn unwrap_(&self, _ct: &[u8]) -> KmResult<Vec<u8>>;
}
