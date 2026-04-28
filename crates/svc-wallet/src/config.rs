//! RO:WHAT — Runtime and invariant configuration for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES/GOV. Centralizes hard bounds before handlers and ledger IO use them.
//! RO:INTERACTS — dto validation, middleware limits, idempotency store, ledger adapter.
//! RO:INVARIANTS — body≤1MiB by default; decompress≤10x; timeout=5s; inflight bounded; no floats.
//! RO:METRICS — config-derived values may be surfaced as gauges by metrics.rs later.
//! RO:CONFIG — this file is the config contract; env/file loaders can map into WalletConfig.
//! RO:SECURITY — amnesia mode forbids wallet-local durable state; cap verification remains mandatory on mutations.
//! RO:TEST — config_default_is_valid; config_rejects_invalid_bounds.

use std::time::Duration;

use crate::errors::{WalletError, WalletResult};

/// Default HTTP/OAP request body ceiling: 1 MiB.
pub const DEFAULT_MAX_BODY_BYTES: usize = 1_048_576;
/// Default decompression ratio ceiling.
pub const DEFAULT_MAX_DECOMP_RATIO: u32 = 10;
/// Default request timeout in milliseconds.
pub const DEFAULT_REQ_TIMEOUT_MS: u64 = 5_000;
/// Default in-flight request ceiling.
pub const DEFAULT_MAX_INFLIGHT: usize = 512;
/// Default balance-cache staleness window.
pub const DEFAULT_STALENESS_WINDOW_MS: u64 = 250;
/// Default idempotency TTL.
pub const DEFAULT_IDEMPOTENCY_TTL_SECS: u64 = 24 * 60 * 60;
/// Default operation amount ceiling from the docs: 10^20 minor units.
pub const DEFAULT_MAX_AMOUNT_PER_OP: u128 = 100_000_000_000_000_000_000;
/// Default running account ceiling: u128::MAX minus safety headroom.
pub const DEFAULT_MAX_ACCOUNT_TOTAL: u128 = u128::MAX - 1_000_000_000;
/// First accepted debit-side nonce.
pub const NONCE_START: u64 = 1;
/// Default ROC asset symbol used until multi-asset policy is wired.
pub const DEFAULT_ASSET: &str = "roc";

/// Wallet runtime config.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletConfig {
    /// Asset symbol expected on v1 requests.
    pub asset: String,
    /// Whether this node is in amnesia mode.
    pub amnesia: bool,
    /// Maximum encoded request body bytes.
    pub max_body_bytes: usize,
    /// Maximum decompression expansion ratio.
    pub max_decomp_ratio: u32,
    /// Request timeout in milliseconds.
    pub req_timeout_ms: u64,
    /// Maximum in-flight requests.
    pub max_inflight: usize,
    /// Read cache staleness window in milliseconds.
    pub default_staleness_ms: u64,
    /// Idempotency TTL in seconds.
    pub idempotency_ttl_secs: u64,
    /// Maximum amount allowed for a single operation.
    pub max_amount_per_op: u128,
    /// Maximum allowed account running total.
    pub max_account_total: u128,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            asset: DEFAULT_ASSET.to_string(),
            amnesia: true,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            max_decomp_ratio: DEFAULT_MAX_DECOMP_RATIO,
            req_timeout_ms: DEFAULT_REQ_TIMEOUT_MS,
            max_inflight: DEFAULT_MAX_INFLIGHT,
            default_staleness_ms: DEFAULT_STALENESS_WINDOW_MS,
            idempotency_ttl_secs: DEFAULT_IDEMPOTENCY_TTL_SECS,
            max_amount_per_op: DEFAULT_MAX_AMOUNT_PER_OP,
            max_account_total: DEFAULT_MAX_ACCOUNT_TOTAL,
        }
    }
}

impl WalletConfig {
    /// Validate hard invariant bounds.
    pub fn validate(&self) -> WalletResult<()> {
        if self.asset.trim().is_empty() || self.asset.len() > 32 {
            return Err(WalletError::bad_request("asset must be 1..=32 bytes"));
        }
        if self.max_body_bytes == 0 || self.max_body_bytes > DEFAULT_MAX_BODY_BYTES {
            return Err(WalletError::limits_exceeded(
                "max_body_bytes must be 1..=1_048_576",
            ));
        }
        if self.max_decomp_ratio == 0 || self.max_decomp_ratio > DEFAULT_MAX_DECOMP_RATIO {
            return Err(WalletError::limits_exceeded(
                "max_decomp_ratio must be 1..=10",
            ));
        }
        if self.req_timeout_ms == 0 {
            return Err(WalletError::bad_request("req_timeout_ms must be > 0"));
        }
        if self.max_inflight == 0 {
            return Err(WalletError::bad_request("max_inflight must be > 0"));
        }
        if self.idempotency_ttl_secs == 0 {
            return Err(WalletError::bad_request("idempotency_ttl_secs must be > 0"));
        }
        if self.max_amount_per_op == 0 {
            return Err(WalletError::bad_request("max_amount_per_op must be > 0"));
        }
        if self.max_account_total == 0 {
            return Err(WalletError::bad_request("max_account_total must be > 0"));
        }
        Ok(())
    }

    /// Return idempotency TTL as Duration.
    pub fn idempotency_ttl(&self) -> Duration {
        Duration::from_secs(self.idempotency_ttl_secs)
    }

    /// Return request timeout as Duration.
    pub fn request_timeout(&self) -> Duration {
        Duration::from_millis(self.req_timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        WalletConfig::default().validate().unwrap();
    }

    #[test]
    fn rejects_excessive_decompression_ratio() {
        let cfg = WalletConfig {
            max_decomp_ratio: 11,
            ..WalletConfig::default()
        };
        assert!(cfg.validate().is_err());
    }
}
