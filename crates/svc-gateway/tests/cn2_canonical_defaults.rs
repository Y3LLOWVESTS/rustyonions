//! RO:WHAT — Proves svc-gateway defaults match the canonical CN-2 CrabNode topology.
//! RO:WHY — The real client gateway must own 8090 without colliding with svc-index on 5304.
//! RO:INTERACTS — svc-gateway Config defaults, Omnigate 9090, embedded storage 5303.
//! RO:INVARIANTS — client ingress 8090; Omnigate upstream 9090; storage upstream 5303.
//! RO:CONFIG — environment overrides are tested elsewhere and remain supported.
//! RO:SECURITY — canonical defaults remain loopback until explicit public exposure.
//! RO:TEST — cargo test -p svc-gateway --test cn2_canonical_defaults.

use svc_gateway::config::Config;

#[test]
fn canonical_crabnode_defaults_are_collision_free_and_target_real_upstreams() {
    let cfg = Config::default();

    assert_eq!(cfg.server.bind_addr, "127.0.0.1:8090");

    assert_eq!(cfg.upstreams.omnigate_base_url, "http://127.0.0.1:9090");

    assert_eq!(cfg.upstreams.storage_base_url, "http://127.0.0.1:5303");

    assert_ne!(
        cfg.server.bind_addr, "127.0.0.1:5304",
        "canonical gateway must not collide with svc-index"
    );
}
