//! RO:WHAT — Request body and inflight limit helpers for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: SEC/RES/PERF. Admission limits must be checked before expensive work.
//! RO:INTERACTS — config, future tower body-limit layer.
//! RO:INVARIANTS — default body cap is 1MiB; inflight must be bounded.
//! RO:METRICS — callers record LIMITS_EXCEEDED rejects.
//! RO:CONFIG — WalletConfig.max_body_bytes and max_inflight.
//! RO:SECURITY — protects against oversized body abuse.
//! RO:TEST — rejects_body_over_cap.

use crate::{
    config::WalletConfig,
    errors::{WalletError, WalletResult},
};

/// Validate a request body length against config.
pub fn check_body_len(cfg: &WalletConfig, len: usize) -> WalletResult<()> {
    if len > cfg.max_body_bytes {
        return Err(WalletError::limits_exceeded("request body exceeds cap"));
    }
    Ok(())
}

/// Validate an inflight count against config.
pub fn check_inflight(cfg: &WalletConfig, inflight: usize) -> WalletResult<()> {
    if inflight >= cfg.max_inflight {
        return Err(WalletError::new(
            crate::errors::WalletErrorCode::Busy,
            "wallet inflight limit reached",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_body_over_cap() {
        let cfg = WalletConfig::default();
        assert!(check_body_len(&cfg, cfg.max_body_bytes).is_ok());
        assert!(check_body_len(&cfg, cfg.max_body_bytes + 1).is_err());
    }
}
