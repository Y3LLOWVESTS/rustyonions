//! Minimal, zero-IO metrics hook for ron-auth.
//!
//! - Default is NO-OP (no allocations, no locks on hot path).
//! - Hosts may register a recorder once (e.g., Prometheus in svc-passport).
//! - This crate never depends on prometheus/tokio/etc.

use crate::errors::AuthError;
use std::sync::OnceLock;

pub trait MetricsRecorder: Send + Sync + 'static {
    /// Counter add (monotonic). Example: "ron_auth_verify_allow_total"
    fn counter_add(&self, name: &'static str, by: u64);
    /// Histogram observe (nanoseconds, counts, sizes, etc.).
    fn histogram_observe(&self, name: &'static str, value: u64);
    /// Gauge set (rarely used here).
    fn gauge_set(&self, _name: &'static str, _value: i64) {
        // optional to implement
    }
}

struct Nop;
impl MetricsRecorder for Nop {
    #[inline]
    fn counter_add(&self, _name: &'static str, _by: u64) {}
    #[inline]
    fn histogram_observe(&self, _name: &'static str, _value: u64) {}
    #[inline]
    fn gauge_set(&self, _name: &'static str, _value: i64) {}
}

static REC: OnceLock<&'static dyn MetricsRecorder> = OnceLock::new();
static NOP: Nop = Nop;

/// One-time hook called by hosts (e.g., svc-passport) to install a recorder.
/// Safe to call at startup; subsequent calls are ignored.
pub fn set_recorder(rec: &'static dyn MetricsRecorder) {
    let _ = REC.set(rec);
}

#[inline]
fn rec() -> &'static dyn MetricsRecorder {
    REC.get().copied().unwrap_or(&NOP)
}

// ---------- Convenience shims used by the pipeline ----------

#[inline]
pub fn counter_inc(name: &'static str) {
    rec().counter_add(name, 1);
}
#[inline]
pub fn counter_add(name: &'static str, by: u64) {
    rec().counter_add(name, by);
}
#[inline]
pub fn hist_ns(name: &'static str, v: u64) {
    rec().histogram_observe(name, v);
}
#[inline]
pub fn gauge(name: &'static str, v: i64) {
    rec().gauge_set(name, v);
}

// Grouped helpers used at error/decision sites:

pub const C_ALLOW: &'static str = "ron_auth_verify_allow_total";
pub const C_DENY: &'static str = "ron_auth_verify_deny_total";

pub const C_ERR_MALFORMED: &'static str = "ron_auth_err_malformed_total";
pub const C_ERR_BOUNDS: &'static str = "ron_auth_err_bounds_total";
pub const C_ERR_UNKNOWN_KID: &'static str = "ron_auth_err_unknown_kid_total";
pub const C_ERR_MAC: &'static str = "ron_auth_err_mac_mismatch_total";
pub const C_ERR_EXPIRED: &'static str = "ron_auth_err_expired_total";
pub const C_ERR_NOTYET: &'static str = "ron_auth_err_not_yet_valid_total";
pub const C_ERR_POLICY: &'static str = "ron_auth_err_policy_total";

pub const H_BATCH_SIZE: &'static str = "ron_auth_verify_batch_size";
pub const H_CAVEATS_PER_CAP: &'static str = "ron_auth_caveats_per_cap";

/// Increment a counter by error type (call *before* returning the error).
#[inline]
pub fn bump_error(e: &AuthError) {
    match e {
        AuthError::Malformed(_) => counter_inc(C_ERR_MALFORMED),
        AuthError::Bounds => counter_inc(C_ERR_BOUNDS),
        AuthError::UnknownKid => counter_inc(C_ERR_UNKNOWN_KID),
        AuthError::MacMismatch => counter_inc(C_ERR_MAC),
        AuthError::Expired => counter_inc(C_ERR_EXPIRED),
        AuthError::NotYetValid => counter_inc(C_ERR_NOTYET),
        AuthError::PolicyDeny => counter_inc(C_ERR_POLICY),
    }
}

/// Record per-capacity caveat count (cheap int, helps crossover tuning).
#[inline]
pub fn observe_caveats(n: usize) {
    rec().histogram_observe(H_CAVEATS_PER_CAP, n as u64);
}
