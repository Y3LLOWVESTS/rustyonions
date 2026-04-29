//! RO:WHAT — Configuration for svc-storage, including paid-write, settlement, and accounting-export knobs.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must default safely and make beta/prod modes explicit.
//! RO:INTERACTS — main, http::routes::paid_object, accounting::exporter, policy::{paid_write,settlement}.
//! RO:INVARIANTS — paid mode explicit; wallet mode fail-closed; settlement/export opt-in; disabled never writes.
//! RO:METRICS — paid/accounting mode outcomes map to storage_* metrics.
//! RO:CONFIG — RON_STORAGE_ADDR, RON_STORAGE_DATA_DIR, max body, wallet, settlement, accounting env vars.
//! RO:SECURITY — production should not use dev-header accidentally; exporters require explicit base URL.
//! RO:TEST — config tests plus paid_write_policy/paid_write_verifier/paid_write_settlement/accounting_export tests.

use anyhow::{bail, Context};
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

/// Environment variable selecting the `/paid/o` verifier behavior.
pub const ENV_PAID_WRITE_VERIFIER_MODE: &str = "RON_STORAGE_PAID_WRITE_VERIFIER_MODE";

/// Environment variable selecting optional post-write settlement behavior.
pub const ENV_PAID_SETTLEMENT_MODE: &str = "RON_STORAGE_PAID_SETTLEMENT_MODE";

/// Environment variable for the paid-storage settlement recipient/payee account.
pub const ENV_PAID_SETTLEMENT_PAYEE: &str = "RON_STORAGE_PAID_SETTLEMENT_PAYEE";

/// Environment variable for the wallet service base URL.
pub const ENV_WALLET_BASE_URL: &str = "RON_STORAGE_WALLET_BASE_URL";

/// Environment variable for the wallet receipt lookup / settlement bearer token.
pub const ENV_WALLET_BEARER: &str = "RON_STORAGE_WALLET_BEARER";

/// Environment variable for wallet receipt lookup / settlement timeout in milliseconds.
pub const ENV_WALLET_LOOKUP_TIMEOUT_MS: &str = "RON_STORAGE_WALLET_LOOKUP_TIMEOUT_MS";

/// Environment variable selecting optional accounting export behavior.
pub const ENV_ACCOUNTING_EXPORT_MODE: &str = "RON_STORAGE_ACCOUNTING_EXPORT_MODE";

/// Environment variable for accounting service base URL.
pub const ENV_ACCOUNTING_BASE_URL: &str = "RON_STORAGE_ACCOUNTING_BASE_URL";

/// Environment variable for accounting export bearer token.
pub const ENV_ACCOUNTING_BEARER: &str = "RON_STORAGE_ACCOUNTING_BEARER";

/// Environment variable for accounting export timeout in milliseconds.
pub const ENV_ACCOUNTING_TIMEOUT_MS: &str = "RON_STORAGE_ACCOUNTING_TIMEOUT_MS";

/// Default wallet URL used for local dev wiring.
pub const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";

/// Default accounting URL used for local dev wiring.
pub const DEFAULT_ACCOUNTING_BASE_URL: &str = "http://127.0.0.1:9600";

/// Default paid-storage settlement payee account.
pub const DEFAULT_PAID_SETTLEMENT_PAYEE: &str = "svc_storage";

/// Default fail-fast timeout for wallet receipt lookup and settlement calls.
pub const DEFAULT_WALLET_LOOKUP_TIMEOUT_MS: u64 = 2_000;

/// Default fail-fast timeout for accounting exports.
pub const DEFAULT_ACCOUNTING_TIMEOUT_MS: u64 = 2_000;

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

/// Paid-storage post-write settlement mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaidSettlementMode {
    /// Do not call wallet capture/release from svc-storage.
    Disabled,
    /// After a successful paid write, call wallet capture and release through svc-wallet HTTP API.
    WalletCapture,
}

impl PaidSettlementMode {
    /// Stable config spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::WalletCapture => "wallet-capture",
        }
    }

    /// Parse mode from environment, defaulting to disabled so existing dev/test paths do not mutate wallet.
    pub fn from_env() -> anyhow::Result<Self> {
        match std::env::var(ENV_PAID_SETTLEMENT_MODE) {
            Ok(value) => value.parse(),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(err) => Err(err).context(format!("reading {ENV_PAID_SETTLEMENT_MODE}")),
        }
    }
}

impl Default for PaidSettlementMode {
    fn default() -> Self {
        Self::Disabled
    }
}

impl fmt::Display for PaidSettlementMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaidSettlementMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();

        match normalized.as_str() {
            "" => Ok(Self::default()),
            "disabled" | "disable" | "off" | "false" | "0" | "none" => Ok(Self::Disabled),
            "wallet" | "wallet-capture" | "wallet_capture" | "capture" | "capture-release"
            | "capture_release" | "wallet-settlement" | "wallet_settlement" | "settle"
            | "settlement" => Ok(Self::WalletCapture),
            other => bail!(
                "invalid {ENV_PAID_SETTLEMENT_MODE}: {other}; expected disabled or wallet-capture"
            ),
        }
    }
}

/// Accounting export mode for usage events emitted by paid storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountingExportMode {
    /// Do not export usage events; still return them in the paid write response.
    Disabled,
    /// POST usage events to an accounting HTTP adapter.
    Http,
}

impl AccountingExportMode {
    /// Stable config spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Http => "http",
        }
    }

    /// Parse mode from environment, defaulting to disabled so accounting outages do not affect dev runs.
    pub fn from_env() -> anyhow::Result<Self> {
        match std::env::var(ENV_ACCOUNTING_EXPORT_MODE) {
            Ok(value) => value.parse(),
            Err(std::env::VarError::NotPresent) => Ok(Self::default()),
            Err(err) => Err(err).context(format!("reading {ENV_ACCOUNTING_EXPORT_MODE}")),
        }
    }
}

impl Default for AccountingExportMode {
    fn default() -> Self {
        Self::Disabled
    }
}

impl fmt::Display for AccountingExportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AccountingExportMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value.trim().to_ascii_lowercase();

        match normalized.as_str() {
            "" => Ok(Self::default()),
            "disabled" | "disable" | "off" | "false" | "0" | "none" => Ok(Self::Disabled),
            "http" | "https" | "accounting-http" | "accounting_http" | "remote" => Ok(Self::Http),
            other => {
                bail!("invalid {ENV_ACCOUNTING_EXPORT_MODE}: {other}; expected disabled or http")
            }
        }
    }
}

/// Return wallet base URL for wallet receipt and settlement calls.
#[must_use]
pub fn wallet_receipt_base_url_from_env() -> String {
    std::env::var(ENV_WALLET_BASE_URL)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BASE_URL.to_string())
}

/// Return optional wallet bearer token for receipt and settlement calls.
#[must_use]
pub fn wallet_receipt_bearer_from_env() -> Option<String> {
    std::env::var(ENV_WALLET_BEARER)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Return wallet receipt lookup / settlement timeout.
#[must_use]
pub fn wallet_receipt_lookup_timeout_from_env() -> Duration {
    let millis = std::env::var(ENV_WALLET_LOOKUP_TIMEOUT_MS)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(DEFAULT_WALLET_LOOKUP_TIMEOUT_MS);

    Duration::from_millis(millis)
}

/// Return configured paid-storage settlement payee account.
#[must_use]
pub fn paid_settlement_payee_from_env() -> String {
    std::env::var(ENV_PAID_SETTLEMENT_PAYEE)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PAID_SETTLEMENT_PAYEE.to_string())
}

/// Return accounting base URL for HTTP usage event export.
#[must_use]
pub fn accounting_export_base_url_from_env() -> String {
    std::env::var(ENV_ACCOUNTING_BASE_URL)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ACCOUNTING_BASE_URL.to_string())
}

/// Return optional accounting bearer token.
#[must_use]
pub fn accounting_export_bearer_from_env() -> Option<String> {
    std::env::var(ENV_ACCOUNTING_BEARER)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Return accounting export timeout.
#[must_use]
pub fn accounting_export_timeout_from_env() -> Duration {
    let millis = std::env::var(ENV_ACCOUNTING_TIMEOUT_MS)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or(DEFAULT_ACCOUNTING_TIMEOUT_MS);

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
    /// Paid post-write settlement mode.
    pub paid_settlement_mode: PaidSettlementMode,
    /// Paid-storage settlement payee.
    pub paid_settlement_payee: String,
    /// Wallet service base URL used by wallet receipt verifier and settlement mode.
    pub wallet_base_url: String,
    /// Wallet receipt lookup / settlement bearer token.
    pub wallet_bearer: Option<String>,
    /// Wallet receipt lookup / settlement timeout.
    pub wallet_lookup_timeout: Duration,
    /// Accounting export mode.
    pub accounting_export_mode: AccountingExportMode,
    /// Accounting base URL.
    pub accounting_base_url: String,
    /// Accounting bearer token.
    pub accounting_bearer: Option<String>,
    /// Accounting export timeout.
    pub accounting_timeout: Duration,
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
        let paid_settlement_mode = PaidSettlementMode::from_env()?;
        let paid_settlement_payee = paid_settlement_payee_from_env();
        let wallet_base_url = wallet_receipt_base_url_from_env();
        let wallet_bearer = wallet_receipt_bearer_from_env();
        let wallet_lookup_timeout = wallet_receipt_lookup_timeout_from_env();
        let accounting_export_mode = AccountingExportMode::from_env()?;
        let accounting_base_url = accounting_export_base_url_from_env();
        let accounting_bearer = accounting_export_bearer_from_env();
        let accounting_timeout = accounting_export_timeout_from_env();

        Ok(Self {
            http_addr,
            data_dir,
            max_body_bytes,
            paid_write_verifier_mode,
            paid_settlement_mode,
            paid_settlement_payee,
            wallet_base_url,
            wallet_bearer,
            wallet_lookup_timeout,
            accounting_export_mode,
            accounting_base_url,
            accounting_bearer,
            accounting_timeout,
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

    #[test]
    fn paid_settlement_mode_accepts_stable_spellings() {
        assert_eq!(
            "disabled".parse::<PaidSettlementMode>().unwrap(),
            PaidSettlementMode::Disabled
        );
        assert_eq!(
            "off".parse::<PaidSettlementMode>().unwrap(),
            PaidSettlementMode::Disabled
        );
        assert_eq!(
            "wallet-capture".parse::<PaidSettlementMode>().unwrap(),
            PaidSettlementMode::WalletCapture
        );
        assert_eq!(
            "wallet_capture".parse::<PaidSettlementMode>().unwrap(),
            PaidSettlementMode::WalletCapture
        );
        assert_eq!(
            "capture-release".parse::<PaidSettlementMode>().unwrap(),
            PaidSettlementMode::WalletCapture
        );
    }

    #[test]
    fn paid_settlement_mode_rejects_unknown_spelling() {
        let err = "settle-on-solana"
            .parse::<PaidSettlementMode>()
            .expect_err("unknown mode should reject");

        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains(ENV_PAID_SETTLEMENT_MODE));
    }

    #[test]
    fn paid_settlement_mode_display_is_stable() {
        assert_eq!(PaidSettlementMode::Disabled.to_string(), "disabled");
        assert_eq!(
            PaidSettlementMode::WalletCapture.to_string(),
            "wallet-capture"
        );
    }

    #[test]
    fn accounting_export_mode_accepts_stable_spellings() {
        assert_eq!(
            "disabled".parse::<AccountingExportMode>().unwrap(),
            AccountingExportMode::Disabled
        );
        assert_eq!(
            "off".parse::<AccountingExportMode>().unwrap(),
            AccountingExportMode::Disabled
        );
        assert_eq!(
            "http".parse::<AccountingExportMode>().unwrap(),
            AccountingExportMode::Http
        );
        assert_eq!(
            "accounting-http".parse::<AccountingExportMode>().unwrap(),
            AccountingExportMode::Http
        );
    }

    #[test]
    fn accounting_export_mode_rejects_unknown_spelling() {
        let err = "write-ledger-directly"
            .parse::<AccountingExportMode>()
            .expect_err("unknown mode should reject");

        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains(ENV_ACCOUNTING_EXPORT_MODE));
    }

    #[test]
    fn accounting_export_mode_display_is_stable() {
        assert_eq!(AccountingExportMode::Disabled.to_string(), "disabled");
        assert_eq!(AccountingExportMode::Http.to_string(), "http");
    }
}
