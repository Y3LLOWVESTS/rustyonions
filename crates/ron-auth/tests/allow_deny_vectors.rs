// RO:WHAT   Minimal allow/deny sanity for ron-auth core.
// RO:WHY    Catch regressions fast: MAC path, bounds, caveats.
// RO:INVARIANTS BLAKE3 only; no I/O; deterministic CBOR+Base64URL.

use ron_auth::{
    sign_and_encode_b64url, verify_token, AuthError, CapabilityBuilder, Caveat, Decision,
    DenyReason, MacKey, MacKeyProvider, RequestCtx, Scope, VerifierConfig,
};
use serde_cbor::Value;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone)]
struct StaticKeys;
impl MacKeyProvider for StaticKeys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        // One fixed 32B key for (tid="test", kid="k1")
        if kid == "k1" && tid == "test" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

fn now() -> u64 {
    // Stable-ish test timestamp
    1_700_000_000
}

fn base_cfg() -> VerifierConfig {
    VerifierConfig {
        max_token_bytes: 4096,
        max_caveats: 64,
        clock_skew_secs: 60,
        soa_threshold: 8, // NEW: crossover for streaming vs SoA
    }
}

fn base_ctx() -> RequestCtx {
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

#[test]
fn allow_happy_path() {
    // Scope allows GET under /index/
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };

    // Build a capability with audience + path prefix + method + tenant + exp
    let mut cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::PathPrefix("/index/".into()))
        .caveat(Caveat::Method(vec!["GET".into()]))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::Exp(now() + 300))
        .build();

    // Sign + encode
    let tok = sign_and_encode_b64url(&mut cap, &StaticKeys).expect("sign");

    // Verify
    let decision = verify_token(&base_cfg(), &tok, &base_ctx(), &StaticKeys).expect("verify ok");
    match decision {
        Decision::Allow { scope } => {
            assert_eq!(scope.prefix.as_deref(), Some("/index/"));
        }
        _ => panic!("expected Allow"),
    }
}

#[test]
fn deny_method_not_allowed() {
    // Scope allows GET under /index/
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };

    let mut cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::PathPrefix("/index/".into()))
        .caveat(Caveat::Method(vec!["GET".into()]))
        .caveat(Caveat::Tenant("test".into()))
        .caveat(Caveat::Exp(now() + 300))
        .build();

    let tok = sign_and_encode_b64url(&mut cap, &StaticKeys).expect("sign");

    // Change method in context to POST to trigger deny
    let mut ctx = base_ctx();
    ctx.method = "POST".into();

    let decision = verify_token(&base_cfg(), &tok, &ctx, &StaticKeys).expect("verify ok");
    match decision {
        Decision::Deny { reasons } => {
            assert!(reasons.contains(&DenyReason::MethodNotAllowed));
        }
        _ => panic!("expected Deny"),
    }
}

#[test]
fn error_mac_mismatch() {
    // Tamper with token bytes after signing to ensure MAC mismatch is caught.
    let scope = Scope {
        prefix: None,
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let mut cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Exp(now() + 60))
        .build();
    let tok = sign_and_encode_b64url(&mut cap, &StaticKeys).expect("sign");

    // Flip one character safely within base64url alphabet
    let mut chars: Vec<u8> = tok.as_bytes().to_vec();
    // Find a position that is not '-' or '_' and flip it.
    let pos = chars
        .iter()
        .position(|&c| c != b'-' && c != b'_')
        .unwrap_or(0);
    chars[pos] = if chars[pos] != b'A' { b'A' } else { b'B' };
    let tampered = String::from_utf8(chars).unwrap();

    let err = verify_token(&base_cfg(), &tampered, &base_ctx(), &StaticKeys).unwrap_err();
    match err {
        AuthError::Malformed(_) | AuthError::MacMismatch => {} // either decode fails or MAC fails
        other => panic!("unexpected error: {:?}", other),
    }
}

#[test]
fn error_expired() {
    // Exp in the past triggers AuthError::Expired (hard error).
    let scope = Scope {
        prefix: None,
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let mut cap = CapabilityBuilder::new(scope, "test", "k1")
        .caveat(Caveat::Exp(now() - 3600))
        .build();
    let tok = sign_and_encode_b64url(&mut cap, &StaticKeys).expect("sign");

    let err = verify_token(&base_cfg(), &tok, &base_ctx(), &StaticKeys).unwrap_err();
    matches!(err, AuthError::Expired);
}
