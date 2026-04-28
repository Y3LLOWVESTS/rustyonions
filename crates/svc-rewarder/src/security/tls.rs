//! RO:WHAT — TLS configuration seam for svc-rewarder.
//! RO:WHY — Pillar 12; Concerns: SEC/RES. Direct TLS is feature-gated and normally fronted by gateway.
//! RO:INTERACTS — config::TlsConfig and future rustls server setup.
//! RO:INVARIANTS — TLS enabled requires cert/key validation before bind.
//! RO:METRICS — TLS failures counted in future transport metrics.
//! RO:CONFIG — tls.enabled, cert_path, key_path.
//! RO:SECURITY — no key material loaded in batch 1 placeholder.
//! RO:TEST — config validation covers missing paths.

use crate::config::TlsConfig;
use crate::{Result, RewarderError};

/// Validate TLS feature availability for batch 1.
pub fn validate_tls_runtime(cfg: &TlsConfig) -> Result<()> {
    if cfg.enabled && !cfg!(feature = "tls") {
        return Err(RewarderError::Config(
            "tls.enabled requires building svc-rewarder with --features tls".into(),
        ));
    }
    Ok(())
}
