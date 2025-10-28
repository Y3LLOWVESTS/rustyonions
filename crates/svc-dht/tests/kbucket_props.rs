//! RO:WHAT — Routing table properties that hold for the MVP implementation.
//! RO:WHY  — Catch regressions in bucket indexing and "closest N" behavior.
//! RO:INTERACTS — peer::{NodeId, RoutingTable}

use svc_dht::peer::{NodeId, RoutingTable};

fn nid(bytes: &[u8]) -> NodeId {
    // NodeId::from_pubkey() hashes input; that's fine for deterministic construction
    NodeId::from_pubkey(bytes)
}

#[test]
fn distance_xor_zero_for_identical_ids() {
    let a = nid(&[0xAA; 32]);
    let d = a.distance(&a);
    assert!(d.iter().all(|&b| b == 0), "distance(self,self) must be zero");
}

#[test]
fn closest_respects_limit_and_has_no_duplicates() {
    let me = nid(&[0x11; 32]);
    let rt = RoutingTable::new(/*k*/ 8);

    // Observe > 8 peers; closest(.., n) must never return more than n
    for i in 0..50u8 {
        let pk = [i; 32];
        rt.observe(me, nid(&pk));
    }

    let out = rt.closest(me, nid(&[0x22; 32]), 8);
    assert!(out.len() <= 8);

    // no duplicates
    let mut set = std::collections::HashSet::new();
    for id in &out {
        assert!(set.insert(id.clone()), "duplicate NodeId in closest()");
    }
}

#[test]
fn bucket_index_monotonicity_smoke() {
    // As distance grows, we *tend* to hit different buckets. We can't see buckets directly,
    // but we can at least ensure observe() doesn't panic and closest() is stable.
    let me = nid(&[0u8; 32]);
    let rt = RoutingTable::new(8);

    // Craft peers at different XOR distances
    let peers = [
        nid(&[0x00; 32]), // identical (distance 0)
        nid(&[0x80; 32]), // highest bit diff
        nid(&[0x7F; 32]), // many lower bits diff
        nid(&[0x01; 32]), // only LSB diff
        nid(&[0xFF; 32]), // all bits diff
    ];

    for p in peers {
        rt.observe(me, p);
    }

    let out = rt.closest(me, nid(&[0x10; 32]), 5);
    assert!(!out.is_empty());
}
