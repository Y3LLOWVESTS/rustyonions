//! Minimal end-to-end example: verify one token, then a batch.
use ron_auth::{
    verify_many, verify_token, Decision, MacKey, MacKeyProvider, RequestCtx, VerifierConfig,
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
        policy_digest_hex: None,
        extras: Value::Null,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = VerifierConfig::with_defaults();

    // Single
    let token = std::env::args().nth(1).expect("pass b64 token");
    let d: Decision = verify_token(&cfg, &token, &ctx(), &StaticKeys)?;
    println!("single: {d:?}");

    // Batch
    let batch = vec![token.clone(), token.clone(), token];
    let out = verify_many(&cfg, &batch, &ctx(), &StaticKeys)?;
    println!("batch: {out:?}");

    Ok(())
}
