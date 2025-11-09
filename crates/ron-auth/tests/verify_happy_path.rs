//! Happy-path verification: small token that should ALLOW.
use ron_auth::{
    verify_token, CapabilityBuilder, Caveat, Decision, MacKey, MacKeyProvider, RequestCtx, Scope,
    VerifierConfig,
};
use serde_cbor::Value;

#[derive(Clone)]
struct StaticKeys;
impl MacKeyProvider for StaticKeys {
    fn key_for(&self, kid: &str, tid: &str) -> Option<MacKey> {
        if kid == "k1" && tid == "tenant-a" {
            Some(MacKey(*b"0123456789abcdef0123456789abcdef"))
        } else {
            None
        }
    }
}

fn ctx() -> RequestCtx {
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
        // keep your hybrid crossover knob (as in benches)
        soa_threshold: 8,
    }
}

#[test]
fn allow_small_token() {
    // Build a minimal “allow GET /index/*” token for tenant-a, kid k1
    let scope = Scope {
        prefix: Some("/index/".into()),
        methods: vec!["GET".into()],
        max_bytes: None,
    };
    let cap = CapabilityBuilder::new(scope, "tenant-a", "k1")
        .caveat(Caveat::Aud("aud-demo".into()))
        .caveat(Caveat::Tenant("tenant-a".into()))
        .caveat(Caveat::PathPrefix("/index/".into()))
        .caveat(Caveat::Method(vec!["GET".into()]))
        .caveat(Caveat::Exp(ctx().now_unix_s + 600))
        .build();

    // Use the same signing helper your benches use.
    let token_b64 = ron_auth::sign_and_encode_b64url(&cap, &StaticKeys).expect("sign");

    let d = verify_token(&cfg(), &token_b64, &ctx(), &StaticKeys).expect("verify");
    match d {
        Decision::Allow { scope } => {
            assert_eq!(scope.prefix.as_deref(), Some("/index/"));
            assert!(scope.methods.iter().any(|m| m == "GET"));
        }
        _ => panic!("expected Allow, got {d:?}"),
    }
}
