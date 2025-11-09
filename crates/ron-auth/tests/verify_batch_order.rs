// RO:WHAT   Batch verify order preservation on mixed good/bad tokens.
// RO:WHY    Ensure verify_many returns decisions in input order.
// RO:INVARIANTS No I/O; deterministic; avoid hard errors (Malformed/UnknownKid) in this test.

use ron_auth::{
    sign_and_encode_b64url, verify_many, CapabilityBuilder, Caveat, Decision, MacKey,
    MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};
use serde_cbor::Value;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone)]
struct StaticKeys;
impl MacKeyProvider for StaticKeys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        // Known good pair for tests
        if kid == "k1" && tid == "test" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

fn now() -> u64 {
    1_700_000_000
}

fn cfg() -> VerifierConfig {
    VerifierConfig {
        max_token_bytes: 4096,
        max_caveats: 64,
        clock_skew_secs: 60,
        soa_threshold: 8,
    }
}

fn ctx() -> RequestCtx {
    RequestCtx {
        now_unix_s: now(),
        method: "GET".into(),
        path: "/index/items/42".into(),
        peer_ip: Some(IpAddr::from_str("127.0.0.1").unwrap()),
        object_addr: None,
        tenant: "test".into(),
        amnesia: false,
        policy_digest_hex: Some("aud-demo".into()),
        extras: Value::Null,
    }
}

fn tok_allow() -> String {
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::PathPrefix("/index/".into()))
        .caveat(Caveat::Method(vec!["GET".into()]))
        .caveat(Caveat::Exp(now() + 300))
        .build();
    sign_and_encode_b64url(&cap, &StaticKeys).unwrap()
}

// Valid token that will evaluate to Deny (path/method mismatch) — NOT malformed, NOT unknown kid.
fn tok_deny() -> String {
    let scope = Scope {
        prefix: Some("/admin/".into()),
        methods: vec!["POST".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::PathPrefix("/admin/".into()))
        .caveat(Caveat::Method(vec!["POST".into()]))
        .caveat(Caveat::Exp(now() + 300))
        .build();
    sign_and_encode_b64url(&cap, &StaticKeys).unwrap()
}

#[test]
fn order_preserved_mixed() {
    // Mixed batch: Allow, Deny(valid), Allow, Allow
    let batch = vec![tok_allow(), tok_deny(), tok_allow(), tok_allow()];

    let decisions = verify_many(&cfg(), &batch, &ctx(), &StaticKeys).unwrap();
    assert_eq!(decisions.len(), 4);

    assert!(matches!(decisions[0], Decision::Allow { .. }));
    assert!(matches!(decisions[1], Decision::Deny { .. }));
    assert!(matches!(decisions[2], Decision::Allow { .. }));
    assert!(matches!(decisions[3], Decision::Allow { .. }));
}
