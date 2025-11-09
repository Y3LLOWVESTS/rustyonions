//! RO:WHAT  Token verification pipeline (pure, sync) with hybrid eval + optional parallel batch.
//! RO:WHY   Keep early short-circuit cost for common tiny tokens; SoA for larger sets;
//!          add feature-gated parallelism for big batches while preserving order.
//! RO:INVARIANTS No I/O; strict bounds; constant-time MAC compare; BLAKE3 only.

use super::{soa::CaveatsSoA, soa_eval::eval_caveats_soa, streaming::eval_caveats_streaming};
use crate::cbor::decode_b64url_cbor_capability_with_buf;
use crate::errors::{AuthError, DenyReason};
use crate::mac::{compute_mac, macs_equal};
use crate::types::{Decision, MacKeyProvider, RequestCtx, VerifierConfig};

use smallvec::SmallVec;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Parallel shard kickoff threshold (avoid thread-pool overhead on small batches).
#[cfg(feature = "parallel")]
const PAR_MIN_BATCH: usize = 64;

/// Sample size used to estimate a per-batch effective threshold.
const THRESH_SAMPLE: usize = 6;

/// Heuristic: estimate an effective crossover threshold from a tiny sample of the batch.
///
/// Optimization: if we're going to take the parallel path (large batch), we avoid
/// double-decoding a sample and simply bias strongly toward the streaming evaluator,
/// which is what heavy caveat sets prefer on this machine.
fn estimate_effective_threshold(cfg: &VerifierConfig, tokens_b64url: &[String]) -> usize {
    // Fast bail-out: huge batches will likely run in parallel; skip sampling work.
    #[cfg(feature = "parallel")]
    {
        if tokens_b64url.len() >= PAR_MIN_BATCH {
            return usize::MAX / 2; // bias to streaming, avoids extra sample decodes
        }
    }

    let sample_n = tokens_b64url.len().min(THRESH_SAMPLE);
    if sample_n == 0 {
        return cfg.soa_threshold;
    }

    let mut scratch = Vec::with_capacity(512);
    let mut lens: SmallVec<[usize; 8]> = SmallVec::new();

    for tok in &tokens_b64url[..sample_n] {
        match decode_b64url_cbor_capability_with_buf(tok, cfg.max_token_bytes, &mut scratch) {
            Ok(cap) => lens.push(cap.caveats.len()),
            Err(_) => return cfg.soa_threshold, // fall back if malformed appears in sample
        }
    }

    if lens.is_empty() {
        return cfg.soa_threshold;
    }
    lens.sort_unstable();
    let median = lens[lens.len() / 2];

    if median > 20 {
        usize::MAX / 2 // bias strongly toward streaming for heavy caveat sets
    } else {
        cfg.soa_threshold
    }
}

/// Verify a single Base64URL token.
pub fn verify_token<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Decision, AuthError> {
    let mut scratch = Vec::with_capacity(1024);
    verify_one_with_buf_thresh(cfg, cfg.soa_threshold, token_b64url, ctx, keys, &mut scratch)
}

/// Verify many Base64URL tokens with amortized buffer reuse; returns a Vec.
/// See `verify_many_into` to reuse the output buffer.
pub fn verify_many<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
) -> Result<Vec<Decision>, AuthError> {
    let mut out = Vec::with_capacity(tokens_b64url.len());
    verify_many_into(cfg, tokens_b64url, ctx, keys, &mut out)?;
    Ok(out)
}

/// Same as `verify_many` but writes decisions into `out` (clears it first).
///
/// When built with `--features parallel` and the batch size is large enough,
/// this will shard across the Rayon pool while preserving output order.
/// Otherwise it falls back to the sequential pipeline.
pub fn verify_many_into<K: MacKeyProvider + Sync>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
    out: &mut Vec<Decision>,
) -> Result<(), AuthError> {
    out.clear();
    out.reserve(tokens_b64url.len());

    // Fast-path: empty
    if tokens_b64url.is_empty() {
        return Ok(());
    }

    // Decide an effective threshold from a tiny sample of the batch (or skip on big batches).
    let effective_threshold = estimate_effective_threshold(cfg, tokens_b64url);

    // Parallel order-preserving path (feature-gated + threshold)
    #[cfg(feature = "parallel")]
    {
        if tokens_b64url.len() >= PAR_MIN_BATCH {
            let decisions: Result<Vec<Decision>, AuthError> = tokens_b64url
                .par_iter()
                .map(|tok| {
                    // Slightly larger scratch to reduce growth on heavy tokens.
                    let mut scratch = Vec::with_capacity(2048);
                    verify_one_with_buf_thresh(cfg, effective_threshold, tok, ctx, keys, &mut scratch)
                })
                .collect();
            out.extend(decisions?);
            return Ok(());
        }
    }

    // Sequential fallback (and default when `parallel` feature is off).
    let mut scratch = Vec::with_capacity(1024);
    let mut reasons: Vec<DenyReason> = Vec::new();

    for tok in tokens_b64url {
        let cap =
            decode_b64url_cbor_capability_with_buf(tok, cfg.max_token_bytes, &mut scratch)?;
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

        if cap.caveats.len() <= effective_threshold {
            eval_caveats_streaming(cfg, ctx, &cap.caveats, &mut reasons)?;
        } else {
            eval_caveats_soa(cfg, ctx, CaveatsSoA::from_slice(&cap.caveats), &mut reasons)?;
        }

        if reasons.is_empty() {
            out.push(Decision::Allow { scope: cap.scope });
        } else {
            out.push(Decision::Deny { reasons: core::mem::take(&mut reasons) });
        }
    }

    Ok(())
}

fn verify_one_with_buf_thresh<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    threshold: usize,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
    scratch: &mut Vec<u8>,
) -> Result<Decision, AuthError> {
    let cap =
        decode_b64url_cbor_capability_with_buf(token_b64url, cfg.max_token_bytes, scratch)?;
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

    let mut reasons: Vec<DenyReason> = Vec::new();
    if cap.caveats.len() <= threshold {
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

#[cfg(feature = "bench-eval-modes")]
#[allow(dead_code)]
pub fn verify_token_streaming_only<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
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

#[cfg(feature = "bench-eval-modes")]
#[allow(dead_code)]
pub fn verify_token_soa_only<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    token_b64url: &str,
    ctx: &RequestCtx,
    keys: &K,
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

#[cfg(feature = "bench-eval-modes")]
#[allow(dead_code)]
pub fn verify_many_streaming_only<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
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
            out.push(Decision::Deny { reasons: core::mem::take(&mut reasons) });
        }
    }
    Ok(out)
}

#[cfg(feature = "bench-eval-modes")]
#[allow(dead_code)]
pub fn verify_many_soa_only<K: MacKeyProvider>(
    cfg: &VerifierConfig,
    tokens_b64url: &[String],
    ctx: &RequestCtx,
    keys: &K,
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
            out.push(Decision::Deny { reasons: core::mem::take(&mut reasons) });
        }
    }
    Ok(out)
}
