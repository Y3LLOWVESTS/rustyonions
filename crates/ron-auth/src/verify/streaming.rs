//! Streaming evaluator with early short-circuit for Exp/Nbf.

use crate::errors::{AuthError, DenyReason};
use crate::types::{Caveat, RequestCtx, VerifierConfig};
use ipnet::IpNet;
use serde_cbor::Value;
use std::str::FromStr;

pub fn eval_caveats_streaming(
    cfg: &VerifierConfig,
    ctx: &RequestCtx,
    caveats: &[Caveat],
    out: &mut Vec<DenyReason>,
) -> Result<(), AuthError> {
    let now = ctx.now_unix_s as i64;
    let mut need_len: Option<u64> = None;

    for c in caveats {
        match c {
            Caveat::Exp(v) => {
                if now > (*v as i64) + cfg.clock_skew_secs {
                    return Err(AuthError::Expired);
                }
            }
            Caveat::Nbf(v) => {
                if now + cfg.clock_skew_secs < *v as i64 {
                    return Err(AuthError::NotYetValid);
                }
            }
            Caveat::Aud(a) => {
                if ctx.policy_digest_hex.as_deref() != Some(a.as_str()) {
                    out.push(DenyReason::BadAudience);
                }
            }
            Caveat::Method(ms) => {
                if !ms.iter().any(|m| m.eq_ignore_ascii_case(&ctx.method)) {
                    out.push(DenyReason::MethodNotAllowed);
                }
            }
            Caveat::PathPrefix(pref) => {
                if !ctx.path.starts_with(pref) {
                    out.push(DenyReason::PathNotAllowed);
                }
            }
            Caveat::IpCidr(s) => match (&ctx.peer_ip, IpNet::from_str(s)) {
                (Some(ip), Ok(net)) if net.contains(ip) => {}
                _ => out.push(DenyReason::IpNotAllowed),
            },
            Caveat::BytesLe(max) => {
                if need_len.is_none() {
                    need_len = extract_len_from_extras(&ctx.extras);
                }
                if let Some(len) = need_len {
                    if len > *max {
                        out.push(DenyReason::BytesExceed);
                    }
                }
            }
            Caveat::Rate { .. } => {} // informational
            Caveat::Tenant(t) => {
                if t != &ctx.tenant {
                    out.push(DenyReason::TenantMismatch);
                }
            }
            Caveat::Amnesia(flag) => {
                if *flag != ctx.amnesia {
                    out.push(DenyReason::Custom("amnesia_mismatch".into()));
                }
            }
            Caveat::GovPolicyDigest(d) => {
                if ctx.policy_digest_hex.as_deref() != Some(d.as_str()) {
                    out.push(DenyReason::Custom("gov_policy_digest_mismatch".into()));
                }
            }
            Caveat::Custom { .. } => {}
        }
    }
    Ok(())
}

fn extract_len_from_extras(v: &Value) -> Option<u64> {
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
