//! RO:WHAT — Decompression ratio guard helper for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: SEC/RES. Prevents compression-bomb expansion before JSON parsing.
//! RO:INTERACTS — config, future tower decompression layer.
//! RO:INVARIANTS — expansion ratio must stay ≤ WalletConfig.max_decomp_ratio.
//! RO:METRICS — callers record LIMITS_EXCEEDED rejects.
//! RO:CONFIG — WalletConfig.max_decomp_ratio.
//! RO:SECURITY — no payload contents are logged.
//! RO:TEST — rejects_ratio_over_cap.

use crate::{
    config::WalletConfig,
    errors::{WalletError, WalletResult},
};

/// Check decompression expansion ratio with integer ceiling math.
pub fn check_decompression_ratio(
    cfg: &WalletConfig,
    compressed_bytes: usize,
    decompressed_bytes: usize,
) -> WalletResult<()> {
    if compressed_bytes == 0 {
        return Err(WalletError::bad_request("compressed size must be > 0"));
    }

    let allowed = compressed_bytes.saturating_mul(cfg.max_decomp_ratio as usize);
    if decompressed_bytes > allowed {
        return Err(WalletError::limits_exceeded(
            "decompressed body exceeds ratio cap",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ratio_over_cap() {
        let cfg = WalletConfig::default();
        assert!(check_decompression_ratio(&cfg, 10, 100).is_ok());
        assert!(check_decompression_ratio(&cfg, 10, 101).is_err());
    }
}
