//! RO:WHAT — Timeout duration helpers for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/PERF. Route deadlines need one config-derived source.
//! RO:INTERACTS — config, future tower timeout layer.
//! RO:INVARIANTS — timeout must be nonzero and finite.
//! RO:METRICS — timeout rejects map to RETRY_LATER.
//! RO:CONFIG — WalletConfig.req_timeout_ms.
//! RO:SECURITY — none.
//! RO:TEST — timeout_matches_config.

use std::time::Duration;

use crate::config::WalletConfig;

/// Return request timeout duration.
pub fn request_timeout(cfg: &WalletConfig) -> Duration {
    cfg.request_timeout()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_matches_config() {
        let cfg = WalletConfig::default();
        assert_eq!(
            request_timeout(&cfg).as_millis(),
            cfg.req_timeout_ms as u128
        );
    }
}
