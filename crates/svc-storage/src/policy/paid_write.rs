//! RO:WHAT — Paid-write admission policy and wallet receipt verification seams.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must fail closed unless a ROC wallet hold is proven.
//! RO:INTERACTS — svc-storage /paid/o route, svc-wallet GET /v1/tx/{txid}, wallet receipt DTOs.
//! RO:INVARIANTS — op=hold; asset=roc; positive integer amount; b3 receipt hash; payer and escrow required.
//! RO:METRICS — route layer maps verifier outcomes into storage_paid_write_total status labels.
//! RO:CONFIG — wallet base URL, bearer token, timeout, and verifier mode are read by route/config.
//! RO:SECURITY — dev header mode is explicit; wallet mode requires remote receipt lookup and fail-closed validation.
//! RO:TEST — paid_write_verifier.rs and paid_write_http_client.rs cover contract, mock, and HTTP lookup paths.

use std::{error::Error, fmt, time::Duration};

use axum::http::{HeaderMap, HeaderValue};
use reqwest::header::{ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};

/// Paid proof operation header.
pub const H_PAID_OP: &str = "x-ron-paid-op";

/// Paid proof asset header.
pub const H_PAID_ASSET: &str = "x-ron-paid-asset";

/// Paid proof estimated amount header.
pub const H_PAID_ESTIMATE_MINOR: &str = "x-ron-paid-estimate-minor";

/// Wallet receipt transaction ID header.
pub const H_WALLET_TXID: &str = "x-ron-wallet-txid";

/// Wallet receipt hash header.
pub const H_WALLET_RECEIPT_HASH: &str = "x-ron-wallet-receipt-hash";

/// Wallet payer/source account header.
pub const H_WALLET_FROM: &str = "x-ron-wallet-from";

/// Wallet escrow/destination account header.
pub const H_WALLET_TO: &str = "x-ron-wallet-to";

/// Wallet receipt DTO subset accepted by svc-storage for paid-write verification.
///
/// The full svc-wallet receipt may include additional fields. This DTO keeps the
/// fields storage needs for admission while remaining forward-compatible with
/// extra wallet metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletReceipt {
    /// Stable wallet transaction ID.
    pub txid: String,

    /// Wallet operation label. Paid storage requires `hold`.
    pub op: String,

    /// Source account. For holds, this is the payer.
    pub from: Option<String>,

    /// Destination account. For holds, this is the escrow account.
    pub to: Option<String>,

    /// Asset symbol. Paid storage currently requires `roc`.
    pub asset: String,

    /// Amount in minor units, encoded as a string for JSON safety.
    pub amount_minor: String,

    /// Optional nonce from wallet mutation routes.
    pub nonce: Option<u64>,

    /// Optional idempotency key or idempotency reference.
    pub idem: Option<String>,

    /// Optional wallet receipt timestamp.
    pub ts: Option<u64>,

    /// Optional ledger sequence start.
    pub ledger_seq_start: Option<u64>,

    /// Optional ledger sequence end.
    pub ledger_seq_end: Option<u64>,

    /// Optional ledger root.
    pub ledger_root: Option<String>,

    /// Stable receipt commitment.
    pub receipt_hash: String,
}

impl WalletReceipt {
    /// Validate this wallet receipt as a paid-storage hold proof.
    pub fn validate_as_paid_write_hold(
        &self,
    ) -> Result<PaidWriteProof, PaidWriteVerificationError> {
        let op = required_text(&self.op, "wallet op")?;
        if op != "hold" {
            return Err(payment_required(
                "paid proof must reference a wallet hold receipt",
            ));
        }

        let asset = required_text(&self.asset, "wallet asset")?;
        if asset != "roc" {
            return Err(payment_required("paid proof asset must be roc"));
        }

        let estimate_minor = required_text(&self.amount_minor, "wallet amount_minor")?
            .parse::<u128>()
            .map_err(|_| payment_required("paid proof estimate must be an integer"))?;

        if estimate_minor == 0 {
            return Err(payment_required(
                "paid proof estimate must be greater than zero",
            ));
        }

        let txid = required_text(&self.txid, "wallet txid")?;
        let receipt_hash = required_text(&self.receipt_hash, "wallet receipt_hash")?;

        if !is_b3_cid(&receipt_hash) {
            return Err(payment_required(
                "wallet receipt hash must be b3:<64 lowercase hex>",
            ));
        }

        let payer = required_optional_text(self.from.as_deref(), "wallet payer")?;
        let escrow = required_optional_text(self.to.as_deref(), "wallet escrow account")?;

        Ok(PaidWriteProof {
            txid,
            receipt_hash,
            payer,
            escrow,
            asset,
            estimate_minor,
        })
    }
}

/// Verified paid-write proof extracted from a wallet hold receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaidWriteProof {
    /// Stable wallet transaction ID.
    pub txid: String,

    /// Stable wallet receipt commitment.
    pub receipt_hash: String,

    /// Paying account.
    pub payer: String,

    /// Escrow account that received the hold.
    pub escrow: String,

    /// Asset symbol. Current paid storage requires `roc`.
    pub asset: String,

    /// Maximum held estimate in minor units.
    pub estimate_minor: u128,
}

/// Result of a paid-write verification pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPaidWrite {
    /// Validated proof.
    pub proof: PaidWriteProof,

    /// Verifier implementation label.
    pub verifier: &'static str,
}

impl VerifiedPaidWrite {
    /// Build a verified paid-write result.
    #[must_use]
    pub fn new(proof: PaidWriteProof, verifier: &'static str) -> Self {
        Self { proof, verifier }
    }
}

/// Paid-write verification failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaidWriteVerificationError {
    /// Payment proof is missing, malformed, untrusted, mismatched, or rejected.
    PaymentRequired(String),
}

impl PaidWriteVerificationError {
    /// Stable human-readable failure reason for tests, responses, and logs.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::PaymentRequired(reason) => reason,
        }
    }
}

impl fmt::Display for PaidWriteVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaymentRequired(reason) => write!(f, "payment required: {reason}"),
        }
    }
}

impl Error for PaidWriteVerificationError {}

/// Verifier interface for sync paid storage admission paths.
pub trait PaidWriteVerifier {
    /// Verify inbound request headers and return a trusted paid-write proof.
    fn verify(&self, headers: &HeaderMap) -> Result<VerifiedPaidWrite, PaidWriteVerificationError>;
}

/// Receipt lookup seam used by the mockable verifier.
pub trait WalletReceiptLookup {
    /// Return the wallet receipt for a transaction ID.
    fn lookup_receipt(&self, txid: &str) -> Result<WalletReceipt, PaidWriteVerificationError>;
}

/// Development verifier that trusts explicit wallet receipt metadata headers.
#[derive(Debug, Clone, Copy, Default)]
pub struct DevHeaderVerifier;

impl PaidWriteVerifier for DevHeaderVerifier {
    fn verify(&self, headers: &HeaderMap) -> Result<VerifiedPaidWrite, PaidWriteVerificationError> {
        let receipt = WalletReceipt {
            txid: required_header(headers, H_WALLET_TXID)?,
            op: required_header(headers, H_PAID_OP)?,
            from: Some(required_header(headers, H_WALLET_FROM)?),
            to: Some(required_header(headers, H_WALLET_TO)?),
            asset: required_header(headers, H_PAID_ASSET)?,
            amount_minor: required_header(headers, H_PAID_ESTIMATE_MINOR)?,
            nonce: None,
            idem: None,
            ts: None,
            ledger_seq_start: None,
            ledger_seq_end: None,
            ledger_root: None,
            receipt_hash: required_header(headers, H_WALLET_RECEIPT_HASH)?,
        };

        let proof = receipt.validate_as_paid_write_hold()?;
        Ok(VerifiedPaidWrite::new(proof, "dev-header"))
    }
}

/// Fail-closed lookup used by `WalletReceiptVerifier::new`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FailClosedWalletReceiptLookup;

impl WalletReceiptLookup for FailClosedWalletReceiptLookup {
    fn lookup_receipt(&self, _txid: &str) -> Result<WalletReceipt, PaidWriteVerificationError> {
        Err(payment_required(
            "wallet receipt verifier is not wired yet; refusing paid write",
        ))
    }
}

/// Production-shaped verifier that validates paid-write headers against a wallet receipt lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletReceiptVerifier<L = FailClosedWalletReceiptLookup> {
    base_url: String,
    lookup: L,
}

impl WalletReceiptVerifier<FailClosedWalletReceiptLookup> {
    /// Build a fail-closed verifier placeholder.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            lookup: FailClosedWalletReceiptLookup,
        }
    }
}

impl<L> WalletReceiptVerifier<L> {
    /// Build a wallet-backed verifier with an explicit receipt lookup backend.
    #[must_use]
    pub fn with_lookup(base_url: impl Into<String>, lookup: L) -> Self {
        Self {
            base_url: base_url.into(),
            lookup,
        }
    }

    /// Configured wallet base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl<L> PaidWriteVerifier for WalletReceiptVerifier<L>
where
    L: WalletReceiptLookup,
{
    fn verify(&self, headers: &HeaderMap) -> Result<VerifiedPaidWrite, PaidWriteVerificationError> {
        let expected = DevHeaderVerifier.verify(headers)?.proof;
        let receipt = self.lookup.lookup_receipt(&expected.txid)?;
        verify_expected_against_receipt(expected, receipt, "wallet-receipt")
    }
}

/// Async HTTP client for `svc-wallet` receipt lookup.
#[derive(Debug, Clone)]
pub struct WalletReceiptHttpClient {
    client: reqwest::Client,
    base_url: String,
    bearer: Option<String>,
}

impl WalletReceiptHttpClient {
    /// Build a new HTTP wallet receipt lookup client.
    pub fn new(
        base_url: impl Into<String>,
        timeout: Duration,
        bearer: Option<String>,
    ) -> Result<Self, PaidWriteVerificationError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| {
                payment_required(format!("failed to build wallet receipt HTTP client: {err}"))
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

    /// Lookup a wallet receipt by transaction ID.
    pub async fn lookup_receipt(
        &self,
        txid: &str,
    ) -> Result<WalletReceipt, PaidWriteVerificationError> {
        let txid = required_text(txid, "wallet txid")?;

        if !is_safe_txid_path_segment(&txid) {
            return Err(payment_required(
                "wallet txid contains characters unsafe for receipt lookup path",
            ));
        }

        let url = format!("{}/v1/tx/{txid}", self.base_url);
        let mut request = self.client.get(url).header(ACCEPT, "application/json");

        if let Some(bearer) = &self.bearer {
            request = request.header(AUTHORIZATION, format!("Bearer {bearer}"));
        }

        let response = request.send().await.map_err(|err| {
            payment_required(format!("wallet receipt lookup request failed: {err}"))
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(payment_required(format!(
                "wallet receipt lookup rejected txid {txid} with status {status}"
            )));
        }

        response.json::<WalletReceipt>().await.map_err(|err| {
            payment_required(format!(
                "wallet receipt lookup returned invalid JSON: {err}"
            ))
        })
    }

    /// Verify paid-write headers by fetching and validating the wallet receipt over HTTP.
    pub async fn verify_headers(
        &self,
        headers: &HeaderMap,
    ) -> Result<VerifiedPaidWrite, PaidWriteVerificationError> {
        let expected = DevHeaderVerifier.verify(headers)?.proof;
        let receipt = self.lookup_receipt(&expected.txid).await?;
        verify_expected_against_receipt(expected, receipt, "wallet-http")
    }
}

fn verify_expected_against_receipt(
    expected: PaidWriteProof,
    receipt: WalletReceipt,
    verifier: &'static str,
) -> Result<VerifiedPaidWrite, PaidWriteVerificationError> {
    let actual = receipt.validate_as_paid_write_hold()?;

    ensure_same("txid", &expected.txid, &actual.txid)?;
    ensure_same("receipt_hash", &expected.receipt_hash, &actual.receipt_hash)?;
    ensure_same("payer", &expected.payer, &actual.payer)?;
    ensure_same("escrow", &expected.escrow, &actual.escrow)?;
    ensure_same("asset", &expected.asset, &actual.asset)?;
    ensure_same_amount(expected.estimate_minor, actual.estimate_minor)?;

    Ok(VerifiedPaidWrite::new(actual, verifier))
}

fn required_header(
    headers: &HeaderMap,
    name: &'static str,
) -> Result<String, PaidWriteVerificationError> {
    let value = headers
        .get(name)
        .ok_or_else(|| payment_required(format!("missing required paid proof header: {name}")))?;

    visible_header_value(value, name)
}

fn visible_header_value(
    value: &HeaderValue,
    name: &'static str,
) -> Result<String, PaidWriteVerificationError> {
    let value = value
        .to_str()
        .map_err(|_| {
            payment_required(format!(
                "paid proof header is not visible ASCII/UTF-8: {name}"
            ))
        })?
        .trim();

    if value.is_empty() {
        return Err(payment_required(format!(
            "paid proof header cannot be empty: {name}"
        )));
    }

    Ok(value.to_string())
}

fn required_text(value: &str, label: &'static str) -> Result<String, PaidWriteVerificationError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(payment_required(format!("{label} cannot be empty")));
    }

    Ok(trimmed.to_string())
}

fn required_optional_text(
    value: Option<&str>,
    label: &'static str,
) -> Result<String, PaidWriteVerificationError> {
    let value = value.ok_or_else(|| payment_required(format!("{label} is required")))?;
    required_text(value, label)
}

fn is_b3_cid(value: &str) -> bool {
    value.len() == 67
        && value.starts_with("b3:")
        && value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_safe_txid_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value.bytes().all(|byte| {
            matches!(
                byte,
                b'a'..=b'z'
                    | b'A'..=b'Z'
                    | b'0'..=b'9'
                    | b'_'
                    | b'-'
                    | b'.'
                    | b':'
            )
        })
}

fn ensure_same(
    label: &'static str,
    expected: &str,
    actual: &str,
) -> Result<(), PaidWriteVerificationError> {
    if expected == actual {
        return Ok(());
    }

    Err(payment_required(format!(
        "wallet receipt {label} mismatch: expected {expected}, got {actual}"
    )))
}

fn ensure_same_amount(expected: u128, actual: u128) -> Result<(), PaidWriteVerificationError> {
    if expected == actual {
        return Ok(());
    }

    Err(payment_required(format!(
        "wallet receipt amount mismatch: expected {expected}, got {actual}"
    )))
}

fn normalize_base_url(value: String) -> Result<String, PaidWriteVerificationError> {
    let trimmed = value.trim().trim_end_matches('/');

    if trimmed.is_empty() {
        return Err(payment_required("wallet base URL cannot be empty"));
    }

    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(payment_required(
            "wallet base URL must start with http:// or https://",
        ));
    }

    Ok(trimmed.to_string())
}

fn payment_required(reason: impl Into<String>) -> PaidWriteVerificationError {
    PaidWriteVerificationError::PaymentRequired(reason.into())
}
