// KEM trait scaffold
use crate::error::KmResult;

pub trait Kem {
    fn encap(&self, _peer_pub: &[u8]) -> KmResult<(Vec<u8>, Vec<u8>)>;
    fn decap(&self, _ct: &[u8]) -> KmResult<Vec<u8>>;
}
