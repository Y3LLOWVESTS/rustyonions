//! RO:WHAT — Configuration for svc-storage, including paid-write wallet verifier knobs.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must default safely and allow explicit beta/prod modes.
//! RO:INTERACTS — main, http::routes::paid_object, policy::paid_write.
//! RO:INVARIANTS — paid mode is explicit; wallet mode fails closed on lookup/auth/parse errors; disabled never writes.
//! RO:METRICS — paid mode outcomes map to storage_paid_write_total status labels.
//! RO:CONFIG — RON_STORAGE_ADDR, RON_STORAGE_DATA_DIR, RON_STORAGE_MAX_BODY, paid wallet env vars.
//! RO:SECURITY — production should not use dev-header by accident; wallet mode requires receipt lookup.
//! RO:TEST — config tests below plus paid_write_policy/paid_write_verifier/http_client route contracts.

use anyhow::{bail, Context};
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Environment variable selecting the `/paid/o` verifier behavior.
pub const ENV_PAID_WRITE_VERIFIER_MODE: &str = "RON_STORAGE_PAID_WRITE_VERIFIER_MODE";

/// Environment variable for the wallet service base URL.
pub const ENV_WALLET_BASE_URL: &str = "RON_STORAGE_WALLET_BASE_URL";

/// Environment variable for the wallet receipt lookup bearer token.
///
/// Local dev can use `dev` if svc-wallet is running with its dev cap verifier.
pub const ENV_WALLET_BEARER: &str = "RON_STORAGE_WALLET_BEARER";

/// Environment variable for wallet receipt lookup timeout in milliseconds.
pub const ENV_WALLET_LOOKUP_TIMEOUT_MS: &str = "RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS";

/// Default wallet URL used for local dev wiring.
pub const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";

/// Default fail-fast timeout for wallet receipt lookup.
pub const DEFAULT_WALLET_LOOKUP_TIMEOUT_MS: u64 = 2_000;

/// Paid-write verifier mode for `/paid/o`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaidWriteVerifierMode {
    /// Current dev/test path: trust explicit wallet hold metadata headers.
    DevHeader,
    /// Production-shaped path: validate request headers against a wallet receipt lookup.
    WalletReceipt,
    /// Refuse paid writes entirely.
    Disabled,
}

impl PaidWriteVerifierMode {
    /// Stable config spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DevHeader => "dev-header",
            Self::WalletReceipt => "wallet-receipt",
            Self::Disabled => "disabled",
        }
    }

    /// Parse mode from environment, defaulting to dev-header for current local beta compatibility.
    pub fn from_env() -> anyhow::Result<Self> {
        match std::env::var(ENV_PAID_WRITE_VERIFIER_MODE) {
            Ok(value) => value.parse(),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(err) => Err(err).context(format!("reading {ENV_PAID_WRITE_VERIFIER_MODE}")),
        }
    }
}

impl Default for PaidWriteVerifierMode {
    fn default() -> Self {
        Self::DevHeader
    }
}

impl fmt::Display for PaidWriteVerifierMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaidWriteVerifierMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();

        match normalized.as_str() {
            "" => Ok(Self::default()),
            "dev" | "dev-header" | "dev_header" | "header" | "headers" => Ok(Self::DevHeader),
            "wallet"
            | "wallet-receipt"
            | "wallet_receipt"
            | "walletreceipt"
            | "wallet-receipt-verifier"
            | "wallet_receipt_verifier"
            | "production"
            | "prod" => Ok(Self::WalletReceipt),
            "disabled" | "disable" | "off" | "false" | "0" => Ok(Self::Disabled),
            other => bail!(
                "invalid {ENV_PAID_WRITE_VERIFIER_MODE}: {other}; expected dev-header, wallet-receipt, or disabled"
            ),
        }
    }
}

/// Return wallet base URL for wallet receipt lookups.
#[must_use]
pub fn wallet_receipt_base_url_from_env() -> String {
    std::env::var(ENV_WALLET_BASE_URL)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BASE_URL.to_string())
}

/// Return optional wallet bearer token for receipt lookups.
#[must_use]
pub fn wallet_receipt_bearer_from_env() -> Option<String> {
    std::env::var(ENV_WALLET_BEARER)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Return wallet receipt lookup timeout.
#[must_use]
pub fn wallet_receipt_lookup_timeout_from_env() -> Duration {
    let millis = std::env::var(ENV_WALLET_LOOKUP_TIMEOUT_MS)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(DEFAULT_WALLET_LOOKUP_TIMEOUT_MS);

    Duration::from_millis(millis)
}

/// Runtime config for svc-storage.
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP bind address.
    pub http_addr: SocketAddr,
    /// Storage data directory.
    pub data_dir: PathBuf,
    /// Maximum body size.
    pub max_body_bytes: u64,
    /// Paid-write verifier mode.
    pub paid_write_verifier_mode: PaidWriteVerifierMode,
    /// Wallet service base URL used by wallet receipt verifier mode.
    pub wallet_base_url: String,
    /// Wallet receipt lookup bearer token.
    pub wallet_bearer: Option<String>,
    /// Wallet receipt lookup timeout.
    pub wallet_lookup_timeout: Duration,
}

impl Config {
    /// Load config from environment.
    pub fn from_env() -> anyhow::Result<Self> {
        let http_addr_str =
            std::env::var("RON_STORAGE_ADDR").unwrap_or_else(|_| "127.0.0.1:5303".to_string());
        let http_addr = SocketAddr::from_str(&http_addr_str)
            .with_context(|| format!("invalid RON_STORAGE_ADDR: {http_addr_str}"))?;

        let data_dir = std::env::var("RON_STORAGE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/storage"));

        let max_body_bytes = std::env::var("RON_STORAGE_MAX_BODY")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(64 * 1024 * 1024);

        let paid_write_verifier_mode = PaidWriteVerifierMode::from_env()?;
        let wallet_base_url = wallet_receipt_base_url_from_env();
        let wallet_bearer = wallet_receipt_bearer_from_env();
        let wallet_lookup_timeout = wallet_receipt_lookup_timeout_from_env();

        Ok(Self {
            http_addr,
            data_dir,
            max_body_bytes,
            paid_write_verifier_mode,
            wallet_base_url,
            wallet_bearer,
            wallet_lookup_timeout,
        })
    }

    /// Read timeout.
    #[must_use]
    pub fn read_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    /// Write timeout.
    #[must_use]
    pub fn write_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paid_write_verifier_mode_accepts_stable_spellings() {
        assert_eq!(
            "dev-header".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::DevHeader
        );
        assert_eq!(
            "dev_header".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::DevHeader
        );
        assert_eq!(
            "wallet-receipt".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::WalletReceipt
        );
        assert_eq!(
            "wallet_receipt".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::WalletReceipt
        );
        assert_eq!(
            "production".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::WalletReceipt
        );
        assert_eq!(
            "disabled".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::Disabled
        );
        assert_eq!(
            "off".parse::<PaidWriteVerifierMode>().unwrap(),
            PaidWriteVerifierMode::Disabled
        );
    }

    #[test]
    fn paid_write_verifier_mode_rejects_unknown_spelling() {
        let err = "trust-me-bro"
            .parse::<PaidWriteVerifierMode>()
            .expect_err("unknown mode should reject");

        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains(ENV_PAID_WRITE_VERIFIER_MODE));
    }

    #[test]
    fn paid_write_verifier_mode_display_is_stable() {
        assert_eq!(PaidWriteVerifierMode::DevHeader.to_string(), "dev-header");
        assert_eq!(
            PaidWriteVerifierMode::WalletReceipt.to_string(),
            "wallet-receipt"
        );
        assert_eq!(PaidWriteVerifierMode::Disabled.to_string(), "disabled");
    }
}
