//! RO:WHAT — Routing table over buckets
//! RO:WHY — Find closest peers; Concerns: PERF
use super::{bucket::KBucket, id::NodeId};

pub struct RoutingTable {
    buckets: Vec<KBucket>,
    _k: usize, // kept for shape; prefixed to avoid dead_code warning until used
}

impl RoutingTable {
    pub fn new(k: usize) -> Self {
        // 256-bit space → 256 buckets (MVP)
        let buckets = (0..256).map(|_| KBucket::new(k)).collect();
        Self { buckets, _k: k }
    }

    pub fn observe(&self, me: NodeId, peer: NodeId) {
        let dist = me.distance(&peer);
        let idx = leading_zeros(&dist) as usize;
        let idx = idx.min(self.buckets.len() - 1);
        self.buckets[idx].touch(peer);
    }

    pub fn closest(&self, _me: NodeId, _target: NodeId, n: usize) -> Vec<NodeId> {
        // MVP: concat from all buckets; refine in phase 2
        let mut out = Vec::with_capacity(n);
        for b in &self.buckets {
            for id in b.snapshot() {
                out.push(id);
                if out.len() == n {
                    return out;
                }
            }
        }
        out
    }
}

fn leading_zeros(bytes: &[u8; 32]) -> u32 {
    for (i, b) in bytes.iter().enumerate() {
        if *b != 0 {
            return (i as u32) * 8 + b.leading_zeros();
        }
    }
    256
}
