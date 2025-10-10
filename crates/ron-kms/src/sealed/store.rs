// Sealed store trait scaffold
use crate::error::KmResult;

pub trait SealedStore {
    fn put(&self, _blob: &[u8]) -> KmResult<()>;
}
