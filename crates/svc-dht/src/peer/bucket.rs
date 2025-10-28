//! RO:WHAT — Single-writer Kademlia bucket (MVP)
//! RO:WHY — Enforce single-writer discipline; Concerns: RES
use super::id::NodeId;
use parking_lot::Mutex;

pub struct KBucket {
    k: usize,
    // single-writer: interior mut guarded, not held across await in higher layers
    inner: Mutex<Vec<NodeId>>,
}

impl KBucket {
    pub fn new(k: usize) -> Self {
        Self { k, inner: Mutex::new(Vec::with_capacity(k)) }
    }

    pub fn touch(&self, id: NodeId) {
        let mut g = self.inner.lock();
        if let Some(pos) = g.iter().position(|x| *x == id) {
            let n = g.remove(pos);
            g.insert(0, n);
            return;
        }
        if g.len() < self.k {
            g.insert(0, id);
        } else {
            // naive eviction: drop tail (older)
            g.pop();
            g.insert(0, id);
        }
    }

    pub fn snapshot(&self) -> Vec<NodeId> {
        self.inner.lock().clone()
    }
}
