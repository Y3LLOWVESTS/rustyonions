//! RO:WHAT — Compact `NodeId` and XOR distance
//! RO:WHY  — Kademlia math; Concerns: PERF/RES

use blake3::hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId([u8; 32]);

impl NodeId {
    #[inline]
    pub fn from_pubkey(pk: &[u8]) -> Self {
        let h = hash(pk);
        Self(*h.as_bytes())
    }

    /// XOR distance between two node IDs.
    #[inline]
    pub fn distance(&self, other: &Self) -> [u8; 32] {
        let mut out = [0u8; 32];
        // Avoid index-based loop to satisfy clippy::needless_range_loop.
        for (dst, (&a, &b)) in out.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *dst = a ^ b;
        }
        out
    }

    /// Optional helpers (handy in tests/callers).
    #[inline]
    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    #[inline]
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }
}
