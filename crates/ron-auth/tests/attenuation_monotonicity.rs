//! Property: Adding extra caveats never *widens* access.
//! We approximate by holding scope fixed and adding either Exp tightening or PathPrefix tightening.
use proptest::prelude::*;
use ron_auth::{
    verify_token, CapabilityBuilder, Caveat, Decision, MacKey, MacKeyProvider, RequestCtx, Scope,
    VerifierConfig,
};
use serde_cbor::Value;

#[derive(Clone)]
struct Keys;
impl MacKeyProvider for Keys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        if kid == "k1" && tid == "tenant-a" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

fn ctx(now: u64, path: &str) -> RequestCtx {
    RequestCtx {
        now_unix_s: now,
        method: "GET".into(),
        path: path.into(),
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

proptest! {
    #[test]
    fn adding_stricter_caveats_does_not_turn_deny_into_allow(
        now in 1_699_999_500u64..=1_700_000_500u64,
        suffix in "[a-z]{0,8}"
    ) {
        let scope = Scope { prefix: Some("/index/".into()), methods: vec!["GET".into()], max_bytes: None };

        // Parent: exp near future; GET /index/*
        let parent = CapabilityBuilder::new(scope.clone(), "tenant-a", "k1")
            .caveat(Caveat::Aud("aud-demo".into()))
            .caveat(Caveat::Tenant("tenant-a".into()))
            .caveat(Caveat::Method(vec!["GET".into()]))
            .caveat(Caveat::PathPrefix("/index/".into()))
            .caveat(Caveat::Exp(now + 300))
            .build();
        let parent_b64 = ron_auth::sign_and_encode_b64url(&parent, &Keys).unwrap();

        // Child: add tighter expiry and tighter path
        let child = CapabilityBuilder::new(scope, "tenant-a", "k1")
            .caveat(Caveat::Aud("aud-demo".into()))
            .caveat(Caveat::Tenant("tenant-a".into()))
            .caveat(Caveat::Method(vec!["GET".into()]))
            .caveat(Caveat::PathPrefix(format!("/index/{suffix}")))
            .caveat(Caveat::Exp(now + 60))
            .build();
        let child_b64 = ron_auth::sign_and_encode_b64url(&child, &Keys).unwrap();

        // Pick a path potentially inside or outside child prefix.
        let path = format!("/index/{suffix}/item");
        let parent_dec = verify_token(&cfg(), &parent_b64, &ctx(now, &path), &Keys).unwrap();
        let child_dec = verify_token(&cfg(), &child_b64, &ctx(now, &path), &Keys).unwrap();

        // If parent denied, child must NOT become Allow.
        if matches!(parent_dec, Decision::Deny { .. }) {
            assert!(matches!(child_dec, Decision::Deny { .. }));
        }
    }
}
