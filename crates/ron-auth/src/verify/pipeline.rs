//! RO:WHAT  Token verification pipeline (pure, sync) with hybrid eval.
//! RO:WHY   Keep early short-circuit cost for common tiny tokens; use SoA for large sets.
//! RO:INVARS No I/O; strict bounds; constant-time MAC compare; BLAKE3 only.

use super::{soa::CaveatsSoA, soa_eval::eval_caveats_soa, streaming::eval_caveats_streaming};
use crate::cbor::decode_b64url_cbor_capability_with_buf;
use crate::errors::{AuthError, DenyReason};
use crate::mac::{compute_mac, macs_equal};
use crate::types::{Decision, MacKeyProvider, RequestCtx, VerifierConfig};
use core::mem;

const SOA_THRESHOLD: usize = 8; // <=8: streaming; >8: SoA

/// Verify a single Base64URL token.
pub fn verify_token(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Decision, AuthError> {
    let mut scratch = Vec::with_capacity(1024);
    verify_one_with_buf(cfg, token_b64url, ctx, keys, &mut scratch)
}

/// Verify many Base64URL tokens with amortized buffer reuse; returns a Vec.
/// See `verify_many_into` to reuse the output buffer.
pub fn verify_many(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Vec<Decision>, AuthError> {
    let mut out = Vec::with_capacity(tokens_b64url.len());
    verify_many_into(cfg, tokens_b64url, ctx, keys, &mut out)?;
    Ok(out)
}

/// Same as `verify_many` but writes decisions into `out` (clears it first).
pub fn verify_many_into(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    let mut scratch = Vec::with_capacity(1024);
    let mut reasons = Vec::<DenyReason>::new();
    out.clear();
    out.reserve(tokens_b64url.len());

    for tok in tokens_b64url {
        let cap = decode_b64url_cbor_capability_with_buf(tok, cfg.max_token_bytes, &mut scratch)?;
        if cap.caveats.len() > cfg.max_caveats {
            return Err(AuthError::Bounds);
        }

        let key = keys
            .key_for(&cap.kid, &cap.tid)
            .ok_or(AuthError::UnknownKid)?;

        // MAC over original caveat order (domain fixed)
        let expect = compute_mac(&key, &cap);
        if !macs_equal(&expect, &cap.mac) {
            return Err(AuthError::MacMismatch);
        }

        reasons.clear();

        if cap.caveats.len() <= SOA_THRESHOLD {
            eval_caveats_streaming(cfg, ctx, &cap.caveats, &mut reasons)?;
        } else {
            eval_caveats_soa(cfg, ctx, CaveatsSoA::from_slice(&cap.caveats), &mut reasons)?;
        }

        if reasons.is_empty() {
            out.push(Decision::Allow { scope: cap.scope });
        } else {
            let taken = mem::take(&mut reasons);
            out.push(Decision::Deny { reasons: taken });
        }
    }

    Ok(())
}

fn verify_one_with_buf(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
    scratch: &mut Vec<u8>,
) -> Result<Decision, AuthError> {
    let cap = decode_b64url_cbor_capability_with_buf(token_b64url, cfg.max_token_bytes, scratch)?;
    if cap.caveats.len() > cfg.max_caveats {
        return Err(AuthError::Bounds);
    }

    let key = keys
        .key_for(&cap.kid, &cap.tid)
        .ok_or(AuthError::UnknownKid)?;

    // MAC over original caveat order
    let expect = compute_mac(&key, &cap);
    if !macs_equal(&expect, &cap.mac) {
        return Err(AuthError::MacMismatch);
    }

    let mut reasons = Vec::new();
    if cap.caveats.len() <= SOA_THRESHOLD {
        eval_caveats_streaming(cfg, ctx, &cap.caveats, &mut reasons)?;
    } else {
        eval_caveats_soa(cfg, ctx, CaveatsSoA::from_slice(&cap.caveats), &mut reasons)?;
    }

    if reasons.is_empty() {
        Ok(Decision::Allow { scope: cap.scope })
    } else {
        Ok(Decision::Deny { reasons })
    }
}

/* -------- Benchmark-only hard toggles (opt-in from benches) -------- */

#[allow(dead_code)]
pub fn verify_token_streaming_only(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Decision, AuthError> {
    let mut scratch = Vec::with_capacity(1024);
    let cap =
        decode_b64url_cbor_capability_with_buf(token_b64url, cfg.max_token_bytes, &mut scratch)?;
    if cap.caveats.len() > cfg.max_caveats {
        return Err(AuthError::Bounds);
    }
    let key = keys
        .key_for(&cap.kid, &cap.tid)
        .ok_or(AuthError::UnknownKid)?;
    let expect = compute_mac(&key, &cap);
    if !macs_equal(&expect, &cap.mac) {
        return Err(AuthError::MacMismatch);
    }
    let mut reasons = Vec::new();
    eval_caveats_streaming(cfg, ctx, &cap.caveats, &mut reasons)?;
    if reasons.is_empty() {
        Ok(Decision::Allow { scope: cap.scope })
    } else {
        Ok(Decision::Deny { reasons })
    }
}

#[allow(dead_code)]
pub fn verify_token_soa_only(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Decision, AuthError> {
    let mut scratch = Vec::with_capacity(1024);
    let cap =
        decode_b64url_cbor_capability_with_buf(token_b64url, cfg.max_token_bytes, &mut scratch)?;
    if cap.caveats.len() > cfg.max_caveats {
        return Err(AuthError::Bounds);
    }
    let key = keys
        .key_for(&cap.kid, &cap.tid)
        .ok_or(AuthError::UnknownKid)?;
    let expect = compute_mac(&key, &cap);
    if !macs_equal(&expect, &cap.mac) {
        return Err(AuthError::MacMismatch);
    }
    let mut reasons = Vec::new();
    eval_caveats_soa(cfg, ctx, CaveatsSoA::from_slice(&cap.caveats), &mut reasons)?;
    if reasons.is_empty() {
        Ok(Decision::Allow { scope: cap.scope })
    } else {
        Ok(Decision::Deny { reasons })
    }
}

#[allow(dead_code)]
pub fn verify_many_streaming_only(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Vec<Decision>, AuthError> {
    let mut out = Vec::with_capacity(tokens_b64url.len());
    let mut scratch = Vec::with_capacity(1024);
    let mut reasons = Vec::<DenyReason>::new();

    for tok in tokens_b64url {
        let cap = decode_b64url_cbor_capability_with_buf(tok, cfg.max_token_bytes, &mut scratch)?;
        if cap.caveats.len() > cfg.max_caveats {
            return Err(AuthError::Bounds);
        }
        let key = keys
            .key_for(&cap.kid, &cap.tid)
            .ok_or(AuthError::UnknownKid)?;
        let expect = compute_mac(&key, &cap);
        if !macs_equal(&expect, &cap.mac) {
            return Err(AuthError::MacMismatch);
        }
        reasons.clear();
        eval_caveats_streaming(cfg, ctx, &cap.caveats, &mut reasons)?;
        if reasons.is_empty() {
            out.push(Decision::Allow { scope: cap.scope });
        } else {
            let taken = mem::take(&mut reasons);
            out.push(Decision::Deny { reasons: taken });
        }
    }
    Ok(out)
}

#[allow(dead_code)]
pub fn verify_many_soa_only(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &impl MacKeyProvider,
) -> Result<Vec<Decision>, AuthError> {
    let mut out = Vec::with_capacity(tokens_b64url.len());
    let mut scratch = Vec::with_capacity(1024);
    let mut reasons = Vec::<DenyReason>::new();

    for tok in tokens_b64url {
        let cap = decode_b64url_cbor_capability_with_buf(tok, cfg.max_token_bytes, &mut scratch)?;
        if cap.caveats.len() > cfg.max_caveats {
            return Err(AuthError::Bounds);
        }
        let key = keys
            .key_for(&cap.kid, &cap.tid)
            .ok_or(AuthError::UnknownKid)?;
        let expect = compute_mac(&key, &cap);
        if !macs_equal(&expect, &cap.mac) {
            return Err(AuthError::MacMismatch);
        }
        reasons.clear();
        eval_caveats_soa(cfg, ctx, CaveatsSoA::from_slice(&cap.caveats), &mut reasons)?;
        if reasons.is_empty() {
            out.push(Decision::Allow { scope: cap.scope });
        } else {
            let taken = mem::take(&mut reasons);
            out.push(Decision::Deny { reasons: taken });
        }
    }
    Ok(out)
}
