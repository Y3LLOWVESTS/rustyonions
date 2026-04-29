//! RO:WHAT — Wallet-backed capture/release adapter for paid storage settlement.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid writes need hold → capture → release without direct ledger mutation.
//! RO:INTERACTS — svc-wallet POST /v1/capture and /v1/release, paid_write::PaidWriteProof, /paid/o route.
//! RO:INVARIANTS — integer minor units only; capture cannot exceed hold; wallet remains mutation front-door.
//! RO:METRICS — route maps settlement failures into paid-write status labels.
//! RO:CONFIG — wallet base URL, bearer, timeout, settlement payee, settlement mode, optional ROC economics pricing.
//! RO:SECURITY — bearer is only sent to wallet; idempotency keys are deterministic and bounded.
//! RO:TEST — tests/paid_write_settlement.rs, tests/paid_write_economics.rs, scripts/web3_paid_storage_live_smoke.sh.

use std::{error::Error, fmt, time::Duration};

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use crate::policy::{
    economics::legacy_paid_storage_capture_amount,
    paid_write::{paid_storage_context_idem, PaidWriteProof, WalletReceipt},
};

/// Stable paid-storage settlement mode label for response payloads.
pub const SETTLEMENT_MODE_WALLET_CAPTURE: &str = "wallet-capture";

/// Actual paid storage settlement plan derived from a validated wallet hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaidStorageSettlementPlan {
    /// Body-derived object CID.
    pub cid: String,
    /// Source hold wallet transaction ID.
    pub hold_txid: String,
    /// Paying account.
    pub payer: String,
    /// Escrow account that currently holds funds.
    pub escrow: String,
    /// Capture recipient/payee account.
    pub payee: String,
    /// Asset symbol.
    pub asset: String,
    /// Held maximum amount.
    pub held_amount_minor: u128,
    /// Actual amount to capture.
    pub capture_amount_minor: u128,
    /// Remainder to release after capture.
    pub release_amount_minor: u128,
}

impl PaidStorageSettlementPlan {
    /// Build a deterministic paid-storage settlement plan using the legacy beta pricing model.
    pub fn from_paid_write(
        proof: &PaidWriteProof,
        cid: &str,
        bytes_stored: u64,
        payee: impl Into<String>,
    ) -> Result<Self, PaidSettlementError> {
        let capture_amount_minor = legacy_paid_storage_capture_amount(bytes_stored);
        Self::from_paid_write_with_capture_amount(proof, cid, capture_amount_minor, payee)
    }

    /// Build a deterministic paid-storage settlement plan using an already-computed capture amount.
    ///
    /// This is used when the route prices the action from `configs/roc-economics.toml`.
    pub fn from_paid_write_with_capture_amount(
        proof: &PaidWriteProof,
        cid: &str,
        capture_amount_minor: u128,
        payee: impl Into<String>,
    ) -> Result<Self, PaidSettlementError> {
        paid_storage_context_idem(
            cid,
            &proof.payer,
            &proof.escrow,
            &proof.asset,
            proof.estimate_minor,
        )
        .map_err(|err| PaidSettlementError::payment_required(err.to_string()))?;

        let payee = required_text(payee.into(), "settlement payee")?;
        if payee == proof.escrow {
            return Err(PaidSettlementError::payment_required(
                "settlement payee cannot be the escrow account",
            ));
        }

        if capture_amount_minor == 0 {
            return Err(PaidSettlementError::payment_required(
                "paid storage capture amount must be greater than zero",
            ));
        }

        if capture_amount_minor > proof.estimate_minor {
            return Err(PaidSettlementError::payment_required(format!(
                "paid hold estimate {} is below actual paid-storage cost {}",
                proof.estimate_minor, capture_amount_minor
            )));
        }

        Ok(Self {
            cid: cid.to_string(),
            hold_txid: proof.txid.clone(),
            payer: proof.payer.clone(),
            escrow: proof.escrow.clone(),
            payee,
            asset: proof.asset.clone(),
            held_amount_minor: proof.estimate_minor,
            capture_amount_minor,
            release_amount_minor: proof.estimate_minor - capture_amount_minor,
        })
    }

    /// Idempotency key for capture.
    #[must_use]
    pub fn capture_idem(&self) -> String {
        settlement_idem("capture", self, self.capture_amount_minor)
    }

    /// Idempotency key for normal release after successful capture.
    #[must_use]
    pub fn release_idem(&self) -> String {
        settlement_idem("release", self, self.release_amount_minor)
    }

    /// Idempotency key for release after storage write failure before capture.
    #[must_use]
    pub fn failed_write_release_idem(&self) -> String {
        settlement_idem("failed_release", self, self.held_amount_minor)
    }
}

/// Paid-storage settlement response returned by `/paid/o` when settlement is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaidStorageSettlement {
    /// Stable settlement mode label.
    pub mode: &'static str,
    /// Captured amount in minor units.
    pub capture_amount_minor: String,
    /// Released amount in minor units.
    pub release_amount_minor: String,
    /// Wallet capture receipt.
    pub capture_receipt: WalletReceipt,
    /// Wallet release receipt when a remainder existed.
    pub release_receipt: Option<WalletReceipt>,
}

/// Wallet settlement HTTP client.
#[derive(Debug, Clone)]
pub struct WalletSettlementHttpClient {
    client: reqwest::Client,
    base_url: String,
    bearer: Option<String>,
}

impl WalletSettlementHttpClient {
    /// Build a wallet settlement client.
    pub fn new(
        base_url: impl Into<String>,
        timeout: Duration,
        bearer: Option<String>,
    ) -> Result<Self, PaidSettlementError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| {
                PaidSettlementError::settlement_failed(format!(
                    "failed to build wallet settlement HTTP client: {err}"
                ))
            })?;

        let base_url = normalize_base_url(base_url.into())?;

        Ok(Self {
            client,
            base_url,
            bearer,
        })
    }

    /// Configured wallet base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Capture actual paid-storage cost and release the remainder, if any.
    pub async fn settle_paid_storage(
        &self,
        plan: &PaidStorageSettlementPlan,
    ) -> Result<PaidStorageSettlement, PaidSettlementError> {
        let capture_receipt = self.capture(plan).await?;
        let release_receipt = if plan.release_amount_minor > 0 {
            Some(self.release_remainder(plan).await?)
        } else {
            None
        };

        Ok(PaidStorageSettlement {
            mode: SETTLEMENT_MODE_WALLET_CAPTURE,
            capture_amount_minor: plan.capture_amount_minor.to_string(),
            release_amount_minor: plan.release_amount_minor.to_string(),
            capture_receipt,
            release_receipt,
        })
    }

    /// Release the full hold after a storage write failure before capture.
    pub async fn release_failed_paid_storage(
        &self,
        plan: &PaidStorageSettlementPlan,
    ) -> Result<WalletReceipt, PaidSettlementError> {
        let idem = plan.failed_write_release_idem();
        let request = WalletTransferRequest {
            from: plan.escrow.clone(),
            to: plan.payer.clone(),
            asset: plan.asset.clone(),
            amount_minor: plan.held_amount_minor.to_string(),
            nonce: 1,
            idempotency_key: None,
            memo: Some(short_memo("paid_storage_failed_release", plan)),
        };

        let receipt = self.post_transfer("/v1/release", &idem, &request).await?;
        validate_settlement_receipt(
            &receipt,
            "release",
            &plan.escrow,
            &plan.payer,
            &plan.asset,
            plan.held_amount_minor,
        )?;

        Ok(receipt)
    }

    async fn capture(
        &self,
        plan: &PaidStorageSettlementPlan,
    ) -> Result<WalletReceipt, PaidSettlementError> {
        let idem = plan.capture_idem();
        let request = WalletTransferRequest {
            from: plan.escrow.clone(),
            to: plan.payee.clone(),
            asset: plan.asset.clone(),
            amount_minor: plan.capture_amount_minor.to_string(),
            nonce: 1,
            idempotency_key: None,
            memo: Some(short_memo("paid_storage_capture", plan)),
        };

        let receipt = self.post_transfer("/v1/capture", &idem, &request).await?;
        validate_settlement_receipt(
            &receipt,
            "capture",
            &plan.escrow,
            &plan.payee,
            &plan.asset,
            plan.capture_amount_minor,
        )?;

        Ok(receipt)
    }

    async fn release_remainder(
        &self,
        plan: &PaidStorageSettlementPlan,
    ) -> Result<WalletReceipt, PaidSettlementError> {
        let idem = plan.release_idem();
        let request = WalletTransferRequest {
            from: plan.escrow.clone(),
            to: plan.payer.clone(),
            asset: plan.asset.clone(),
            amount_minor: plan.release_amount_minor.to_string(),
            nonce: 2,
            idempotency_key: None,
            memo: Some(short_memo("paid_storage_release", plan)),
        };

        let receipt = self.post_transfer("/v1/release", &idem, &request).await?;
        validate_settlement_receipt(
            &receipt,
            "release",
            &plan.escrow,
            &plan.payer,
            &plan.asset,
            plan.release_amount_minor,
        )?;

        Ok(receipt)
    }

    async fn post_transfer(
        &self,
        path: &str,
        idem: &str,
        request: &WalletTransferRequest,
    ) -> Result<WalletReceipt, PaidSettlementError> {
        let url = format!("{}{}", self.base_url, path);
        let mut http = self
            .client
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header("Idempotency-Key", idem)
            .json(request);

        if let Some(bearer) = &self.bearer {
            http = http.header(AUTHORIZATION, format!("Bearer {bearer}"));
        }

        let response = http.send().await.map_err(|err| {
            PaidSettlementError::settlement_failed(format!(
                "wallet settlement request failed for {path}: {err}"
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PaidSettlementError::settlement_failed(format!(
                "wallet settlement request {path} failed with status {status}: {body}"
            )));
        }

        response.json::<WalletReceipt>().await.map_err(|err| {
            PaidSettlementError::settlement_failed(format!(
                "wallet settlement request {path} returned invalid JSON: {err}"
            ))
        })
    }
}

#[derive(Debug, Serialize)]
struct WalletTransferRequest {
    from: String,
    to: String,
    asset: String,
    amount_minor: String,
    nonce: u64,
    idempotency_key: Option<String>,
    memo: Option<String>,
}

/// Settlement failure taxonomy used by the paid route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaidSettlementError {
    /// The existing hold cannot cover the requested paid operation.
    PaymentRequired(String),
    /// The storage write happened or was about to happen, but wallet settlement failed.
    SettlementFailed(String),
}

impl PaidSettlementError {
    /// Build a payment-required settlement error.
    #[must_use]
    pub fn payment_required(reason: impl Into<String>) -> Self {
        Self::PaymentRequired(reason.into())
    }

    /// Build a settlement-failed error.
    #[must_use]
    pub fn settlement_failed(reason: impl Into<String>) -> Self {
        Self::SettlementFailed(reason.into())
    }

    /// Stable reason string.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::PaymentRequired(reason) | Self::SettlementFailed(reason) => reason,
        }
    }
}

impl fmt::Display for PaidSettlementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaymentRequired(reason) => write!(f, "payment required: {reason}"),
            Self::SettlementFailed(reason) => write!(f, "settlement failed: {reason}"),
        }
    }
}

impl Error for PaidSettlementError {}

fn settlement_idem(kind: &str, plan: &PaidStorageSettlementPlan, amount_minor: u128) -> String {
    let canonical = format!(
        "v=1\nkind={kind}\nhold_txid={}\ncid={}\npayer={}\nescrow={}\npayee={}\nasset={}\namount_minor={amount_minor}\n",
        plan.hold_txid, plan.cid, plan.payer, plan.escrow, plan.payee, plan.asset
    );
    let hex = blake3::hash(canonical.as_bytes()).to_hex().to_string();

    match kind {
        "capture" => format!("storage_cap:{}", &hex[..32]),
        "release" => format!("storage_rel:{}", &hex[..32]),
        _ => format!("storage_failrel:{}", &hex[..32]),
    }
}

fn short_memo(kind: &str, plan: &PaidStorageSettlementPlan) -> String {
    format!("{kind} cid={} hold={}", plan.cid, plan.hold_txid)
}

fn validate_settlement_receipt(
    receipt: &WalletReceipt,
    expected_op: &str,
    expected_from: &str,
    expected_to: &str,
    expected_asset: &str,
    expected_amount_minor: u128,
) -> Result<(), PaidSettlementError> {
    ensure_eq("op", expected_op, receipt.op.as_str())?;
    ensure_eq("asset", expected_asset, receipt.asset.as_str())?;

    let from = receipt.from.as_deref().ok_or_else(|| {
        PaidSettlementError::settlement_failed("wallet settlement receipt is missing from account")
    })?;
    let to = receipt.to.as_deref().ok_or_else(|| {
        PaidSettlementError::settlement_failed("wallet settlement receipt is missing to account")
    })?;

    ensure_eq("from", expected_from, from)?;
    ensure_eq("to", expected_to, to)?;

    let amount = receipt.amount_minor.parse::<u128>().map_err(|_| {
        PaidSettlementError::settlement_failed("wallet settlement receipt amount is not an integer")
    })?;
    if amount != expected_amount_minor {
        return Err(PaidSettlementError::settlement_failed(format!(
            "wallet settlement amount mismatch: expected {expected_amount_minor}, got {amount}"
        )));
    }

    if !is_b3_cid(&receipt.receipt_hash) {
        return Err(PaidSettlementError::settlement_failed(
            "wallet settlement receipt hash must be b3:<64 lowercase hex>",
        ));
    }

    Ok(())
}

fn ensure_eq(label: &'static str, expected: &str, actual: &str) -> Result<(), PaidSettlementError> {
    if expected == actual {
        return Ok(());
    }

    Err(PaidSettlementError::settlement_failed(format!(
        "wallet settlement {label} mismatch: expected {expected}, got {actual}"
    )))
}

fn required_text(value: String, label: &'static str) -> Result<String, PaidSettlementError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PaidSettlementError::payment_required(format!(
            "{label} cannot be empty"
        )));
    }

    Ok(trimmed.to_string())
}

fn normalize_base_url(value: String) -> Result<String, PaidSettlementError> {
    let trimmed = value.trim().trim_end_matches('/');

    if trimmed.is_empty() {
        return Err(PaidSettlementError::settlement_failed(
            "wallet settlement base URL cannot be empty",
        ));
    }

    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(PaidSettlementError::settlement_failed(
            "wallet settlement base URL must start with http:// or https://",
        ));
    }

    Ok(trimmed.to_string())
}

fn is_b3_cid(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("b3:") else {
        return false;
    };

    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}
