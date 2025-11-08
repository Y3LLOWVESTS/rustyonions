//! Evaluator over SoA columns.

use super::soa::CaveatsSoA;
use crate::errors::{AuthError, DenyReason};
use crate::types::{RequestCtx, VerifierConfig};

pub fn eval_caveats_soa<'a>(
    cfg: &VerifierConfig,
    ctx: &RequestCtx,
    soa: CaveatsSoA<'a>,
    out: &mut Vec<DenyReason>,
) -> Result<(), AuthError> {
    let now = ctx.now_unix_s as i64;

    // Hard errors first.
    for exp in soa.exp.iter() {
        if now > (*exp as i64) + cfg.clock_skew_secs {
            return Err(AuthError::Expired);
        }
    }
    for nbf in soa.nbf.iter() {
        if now + cfg.clock_skew_secs < *nbf as i64 {
            return Err(AuthError::NotYetValid);
        }
    }

    // Audience
    for a in soa.aud.iter() {
        match &ctx.policy_digest_hex {
            Some(pd) if pd == a => {}
            _ => out.push(DenyReason::BadAudience),
        }
    }

    // Method (borrowed slice of Strings; compare as &str)
    for methods in soa.method.iter() {
        if !methods
            .iter()
            .any(|m| m.as_str().eq_ignore_ascii_case(&ctx.method))
        {
            out.push(DenyReason::MethodNotAllowed);
        }
    }

    // PathPrefix
    for pref in soa.path_prefix.iter() {
        if !ctx.path.starts_with(pref) {
            out.push(DenyReason::PathNotAllowed);
        }
    }

    // IpCidr (already parsed)
    for net in soa.ip_cidr.iter() {
        match (&ctx.peer_ip, net) {
            (Some(ip), Some(n)) if n.contains(ip) => {}
            _ => out.push(DenyReason::IpNotAllowed),
        }
    }

    // BytesLe
    if let Some(len) = extract_len_from_extras_for_soa(&ctx.extras) {
        for max in soa.bytes_le.iter() {
            if len > *max {
                out.push(DenyReason::BytesExceed);
            }
        }
    }

    // Rate (placeholder)
    for (_burst, _per_s) in soa.rate.iter() {
        // host policy may enforce elsewhere
    }

    // Tenant
    for t in soa.tenant.iter() {
        if *t != ctx.tenant {
            out.push(DenyReason::TenantMismatch);
        }
    }

    // Amnesia flag
    for flag in soa.amnesia.iter() {
        if *flag != ctx.amnesia {
            out.push(DenyReason::Custom("amnesia_mismatch".into()));
        }
    }

    // Governance policy digest
    for d in soa.gov_policy_digest.iter() {
        if ctx.policy_digest_hex.as_deref() != Some(*d) {
            out.push(DenyReason::Custom("gov_policy_digest_mismatch".into()));
        }
    }

    // Custom caveats are host-defined; keep informational
    let _ = soa.custom;

    Ok(())
}

// Small helper to reuse the same len-extractor without exposing internals widely.
pub(crate) fn extract_len_from_extras_for_soa(v: &serde_cbor::Value) -> Option<u64> {
    use serde_cbor::Value;
    match v {
        Value::Map(m) => {
            for (k, val) in m {
                if let Value::Text(s) = k {
                    if s == "len" {
                        if let Value::Integer(i) = val {
                            if *i >= 0 {
                                return Some(*i as u64);
                            }
                        } else if let Value::Float(f) = val {
                            if *f >= 0.0 {
                                return Some(*f as u64);
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}
