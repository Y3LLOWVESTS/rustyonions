//! RO:WHAT — Idempotency key derivation + header mapping helpers.
//! RO:WHY  — Give applications a deterministic, low-PII way to derive
//!           idempotency keys for “logical operations” (governance I-G1).
//! RO:INTERACTS — Uses `crate::config::IdemCfg`; will be used by planes
//!                (storage/mailbox/index) and transport wrappers.
//! RO:INVARIANTS —
//!   - Never generates two different keys for the same logical op.
//!   - Stable across process restarts (pure function of inputs).
//!   - No randomness; no dependency on wall-clock time.
//!   - No PII baked into the key string when a prefix is used.
//! RO:METRICS — None directly (planes may emit counters per idempotent call).
//! RO:CONFIG — Reads `IdemCfg { enabled, key_prefix }`.
//! RO:SECURITY — Keys are opaque 64-bit fingerprints; callers should avoid
//!               embedding raw PII into the “logical_key” input.
//! RO:TEST — Unit tests in this module (determinism + collision sanity).

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use crate::config::IdemCfg;

/// Default HTTP header name for idempotency keys.
///
/// This is a *convention*, not a hard requirement. Some hosts may
/// prefer a different header name at the gateway level.
pub const IDEMPOTENCY_HEADER: &str = "Idempotency-Key";

/// Opaque idempotency key value.
///
/// Semantically represented as a string, but we keep it wrapped so the
/// semantics remain clear and we can refine the format later without
/// breaking callers.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Access the underlying string.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the underlying string.
    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Convenience helper to turn this key into a `(header_name, value)` pair.
    #[inline]
    pub fn into_header(self) -> (String, String) {
        (IDEMPOTENCY_HEADER.to_string(), self.0)
    }
}

impl AsRef<str> for IdempotencyKey {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redact to avoid accidental key exposure in logs.
        f.debug_tuple("IdempotencyKey")
            .field(&"...redacted...")
            .finish()
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Derive an idempotency key for a logical operation.
///
/// Inputs:
/// - `cfg` — idempotency configuration (enable/disable, optional prefix).
/// - `method` — logical verb (e.g., "PUT", "POST"); case-insensitive.
/// - `endpoint` — stable endpoint identifier (path or logical name).
/// - `logical_key` — caller-defined identifier for the logical op
///   (e.g., order ID, manifest ID). May be `None` for simple cases.
///
/// Returns `None` when idempotency is disabled in config.
#[allow(dead_code)] // Public helper; may be used only by SDK consumers.
pub fn derive_idempotency_key(
    cfg: &IdemCfg,
    method: &str,
    endpoint: &str,
    logical_key: Option<&str>,
) -> Option<IdempotencyKey> {
    if !cfg.enabled {
        return None;
    }

    // Normalize inputs into a single logical string.
    let method_norm = method.to_ascii_uppercase();
    let endpoint_norm = endpoint.trim();
    let logical_norm = logical_key.unwrap_or("").trim();

    let fingerprint = stable_fingerprint(&format!(
        "{}\n{}\n{}",
        method_norm, endpoint_norm, logical_norm
    ));

    // Optional prefix helps keep keys non-PII even if logical_key has
    // some user-provided content.
    let prefix = cfg.key_prefix.as_deref().unwrap_or("ron"); // short + recognizable.

    let key = format!("{prefix}_{fingerprint:016x}");
    Some(IdempotencyKey(key))
}

/// Internal helper — 64-bit stable fingerprint using the standard hasher.
///
/// This is *not* cryptographic and is not intended for security; it’s
/// just a low-collision, deterministic fingerprint for idempotency.
#[allow(dead_code)] // Only used by `derive_idempotency_key` and tests.
fn stable_fingerprint(input: &str) -> u64 {
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_returns_none() {
        let cfg = IdemCfg {
            enabled: false,
            key_prefix: None,
        };
        let key = derive_idempotency_key(&cfg, "POST", "/storage/put", Some("abc"));
        assert!(key.is_none());
    }

    #[test]
    fn same_inputs_same_key() {
        let cfg = IdemCfg {
            enabled: true,
            key_prefix: Some("test".to_string()),
        };

        let a = derive_idempotency_key(&cfg, "POST", "/storage/put", Some("abc")).unwrap();
        let b = derive_idempotency_key(&cfg, "post", " /storage/put ", Some("abc")).unwrap();

        assert_eq!(a, b);
        // also assert AsRef/Display behave
        assert_eq!(a.as_ref(), b.as_str());
        assert_eq!(a.to_string(), b.to_string());
    }

    #[test]
    fn different_logical_key_changes_fingerprint() {
        let cfg = IdemCfg {
            enabled: true,
            key_prefix: Some("test".to_string()),
        };

        let a = derive_idempotency_key(&cfg, "POST", "/storage/put", Some("abc")).unwrap();
        let b = derive_idempotency_key(&cfg, "POST", "/storage/put", Some("def")).unwrap();

        assert_ne!(a, b);
    }
}
