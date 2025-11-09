// RO:WHAT   IpCidr caveat sanity (allow/deny).
// RO:WHY    Lock CIDR parsing and membership semantics.
// RO:INVARIANTS Pure; BLAKE3; deterministic; no I/O.

use ron_auth::{
    sign_and_encode_b64url, verify_token, CapabilityBuilder, Caveat, Decision, DenyReason, MacKey,
    MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};
use serde_cbor::Value;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone)]
struct StaticKeys;
impl MacKeyProvider for StaticKeys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
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
        soa_threshold: 8, // crossover for streaming vs SoA
    }
}

fn ctx_with_ip(ip: &str) -> RequestCtx {
    RequestCtx {
        now_unix_s: now(),
        method: "GET".into(),
        path: "/".into(),
        peer_ip: Some(IpAddr::from_str(ip).unwrap()),
        object_addr: None,
        tenant: "test".into(),
        amnesia: false,
        policy_digest_hex: Some("aud-demo".into()),
        extras: Value::Null,
    }
}

#[test]
fn ip_cidr_allow_ipv4() {
    let scope = Scope {
        prefix: None,
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::IpCidr("192.168.1.0/24".into()))
        .caveat(Caveat::Exp(now() + 60))
        .build();
    let tok = sign_and_encode_b64url(&cap, &StaticKeys).unwrap();

    let dec = verify_token(&cfg(), &tok, &ctx_with_ip("192.168.1.42"), &StaticKeys).unwrap();
    assert!(matches!(dec, Decision::Allow { .. }));
}

#[test]
fn ip_cidr_deny_outside() {
    let scope = Scope {
        prefix: None,
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::IpCidr("10.0.0.0/8".into()))
        .caveat(Caveat::Exp(now() + 60))
        .build();
    let tok = sign_and_encode_b64url(&cap, &StaticKeys).unwrap();

    let dec = verify_token(&cfg(), &tok, &ctx_with_ip("192.168.1.42"), &StaticKeys).unwrap();
    match dec {
        Decision::Deny { reasons } => assert!(reasons.contains(&DenyReason::IpNotAllowed)),
        _ => panic!("expected Deny"),
    }
}

#[test]
fn ip_cidr_deny_malformed() {
    let scope = Scope {
        prefix: None,
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::IpCidr("not-a-cidr".into()))
        .caveat(Caveat::Exp(now() + 60))
        .build();
    let tok = sign_and_encode_b64url(&cap, &StaticKeys).unwrap();

    let dec = verify_token(&cfg(), &tok, &ctx_with_ip("127.0.0.1"), &StaticKeys).unwrap();
    match dec {
        Decision::Deny { reasons } => assert!(reasons.contains(&DenyReason::IpNotAllowed)),
        _ => panic!("expected Deny"),
    }
}
