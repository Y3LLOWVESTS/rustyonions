//! RO:WHAT — Canonical JSON payload + envelope (base64url, no pad).
//! RO:INVARIANTS — sorted keys; UTF-8 bytes; stable across languages.

use crate::{
    dto::{issue::IssueRequest, verify::Envelope},
    Config,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use time::OffsetDateTime;

pub fn canonical_payload(cfg: &Config, req: &IssueRequest) -> anyhow::Result<Value> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let ttl = req
        .ttl_s
        .unwrap_or(cfg.passport.default_ttl_s)
        .min(cfg.passport.max_ttl_s);
    let iat = now;
    let exp = now + ttl as i64;
    let nbf = req.nbf.unwrap_or(iat);

    // Basic payload; kid is stamped after sign via IssuerState (KMS head)
    let payload = json!({
        "iss": cfg.passport.issuer,
        "sub": req.sub,
        "aud": req.aud,
        "iat": iat,
        "exp": exp,
        "nbf": nbf,
        "scopes": req.scopes,
        "nonce": req.nonce,
        "ctx": req.ctx
    });

    Ok(payload)
}

pub fn envelope(payload: &Value, kid: &str, sig: &[u8]) -> anyhow::Result<Envelope> {
    let msg = serde_json::to_vec(payload)?;
    Ok(Envelope {
        alg: "Ed25519".into(),
        kid: kid.to_string(),
        sig_b64: URL_SAFE_NO_PAD.encode(sig),
        msg_b64: URL_SAFE_NO_PAD.encode(&msg),
    })
}

pub fn decode_envelope(env: &Envelope) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let msg = URL_SAFE_NO_PAD.decode(&env.msg_b64)?;
    let sig = URL_SAFE_NO_PAD.decode(&env.sig_b64)?;
    Ok((msg, sig))
}
