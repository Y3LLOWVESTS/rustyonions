//! Error-path sanity: UnknownKid, Expired, Malformed (tuple variant), and basic shape.
use ron_auth::{
    verify_token, AuthError, CapabilityBuilder, MacKey, MacKeyProvider, RequestCtx, Scope,
    VerifierConfig,
};
use serde_cbor::Value;

#[derive(Clone)]
struct KeysOk;
impl MacKeyProvider for KeysOk {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        if kid == "k1" && tid == "tenant-a" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct KeysEmpty;
impl MacKeyProvider for KeysEmpty {
    fn key_for(&self, _kid: &str, _tid: &str) -> Option<MacKey> {
        None
    }
}

fn base_ctx() -> RequestCtx {
    RequestCtx {
        now_unix_s: 1_700_000_000,
        method: "GET".into(),
        path: "/index/abc".into(),
        peer_ip: None,
        object_addr: None,
        tenant: "tenant-a".into(),
        amnesia: false,
        policy_digest_hex: Some("aud-demo".into()),
        extras: Value::Null,
    }
}

fn cfg() -> VerifierConfig {
    VerifierConfig {
        max_token_bytes: 4096,
        max_caveats: 128,
        clock_skew_secs: 60,
        soa_threshold: 8,
    }
}

fn signed_ok() -> String {
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "tenant-a", "k1")
        .caveat(ron_auth::Caveat::Aud("aud-demo".into()))
        .caveat(ron_auth::Caveat::Tenant("tenant-a".into()))
        .caveat(ron_auth::Caveat::PathPrefix("/index/".into()))
        .caveat(ron_auth::Caveat::Method(vec!["GET".into()]))
        .caveat(ron_auth::Caveat::Exp(base_ctx().now_unix_s + 60))
        .build();
    ron_auth::sign_and_encode_b64url(&cap, &KeysOk).unwrap()
}

#[test]
fn unknown_kid() {
    let tok = signed_ok();
    let err = verify_token(&cfg(), &tok, &base_ctx(), &KeysEmpty).unwrap_err();
    // tuple variant carries a &'static str
    assert!(matches!(err, AuthError::UnknownKid));
}

#[test]
fn expired() {
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "tenant-a", "k1")
        .caveat(ron_auth::Caveat::Aud("aud-demo".into()))
        .caveat(ron_auth::Caveat::Tenant("tenant-a".into()))
        .caveat(ron_auth::Caveat::Exp(base_ctx().now_unix_s - 61))
        .build();
    let tok = ron_auth::sign_and_encode_b64url(&cap, &KeysOk).unwrap();
    let err = verify_token(&cfg(), &tok, &base_ctx(), &KeysOk).unwrap_err();
    assert!(matches!(err, AuthError::Expired));
}

#[test]
fn malformed_base64() {
    let bad = "!!!this-is-not-base64url!!!";
    let err = verify_token(&cfg(), bad, &base_ctx(), &KeysOk).unwrap_err();
    // tuple variant requires payload pattern
    assert!(matches!(err, AuthError::Malformed(_)));
}
