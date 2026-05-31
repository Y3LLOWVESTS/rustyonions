//! RO:WHAT — Paid CAS ingest handler for POST/PUT /paid/o.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage requires escrow proof and optional settlement/export.
//! RO:INTERACTS — AppState storage trait, paid_write verifier, settlement wallet client, accounting exporter.
//! RO:INVARIANTS — no proof means no write; wallet mode binds hold idem to body CID; accounting is usage only.
//! RO:METRICS — storage_paid_write_total, storage_paid_write_bytes_total, storage_accounting_export_total.
//! RO:CONFIG — paid verifier, wallet settlement, accounting exporter, economics policy, and accounting context headers.
//! RO:SECURITY — dev-header explicit; wallet-receipt/settlement/export modes fail closed or report failure.
//! RO:TEST — paid_write_policy.rs, paid_write_verifier.rs, paid_write_wallet_mode.rs, paid_write_economics.rs.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    accounting::{
        exporter::{export_usage_events_from_env, AccountingExportReport},
        UsageEventDto,
    },
    config::{
        paid_settlement_payee_from_env, wallet_receipt_base_url_from_env,
        wallet_receipt_bearer_from_env, wallet_receipt_lookup_timeout_from_env, PaidSettlementMode,
        PaidWriteVerifierMode,
    },
    http::extractors::AppState,
    policy::{
        economics::paid_storage_capture_amount_from_env,
        paid_write::{
            DevHeaderVerifier, PaidWriteVerificationError, PaidWriteVerifier, VerifiedPaidWrite,
            WalletReceiptHttpClient,
        },
        settlement::{
            PaidSettlementError, PaidStorageSettlement, PaidStorageSettlementPlan,
            WalletSettlementHttpClient,
        },
    },
};

const H_TENANT: &str = "x-ron-tenant";
const H_ACCOUNTING_SUBJECT: &str = "x-ron-accounting-subject";
const H_REGION: &str = "x-ron-region";
const H_PIN_SECONDS: &str = "x-ron-pin-seconds";

const DEFAULT_TENANT: u128 = 1;
const DEFAULT_ACCOUNTING_SUBJECT: &str = "svc_storage";
const DEFAULT_REGION: &str = "local";
const SOURCE_SERVICE: &str = "svc-storage";
const PAID_ROUTE: &str = "/paid/o";

const MIB: usize = 1024 * 1024;
const DEFAULT_PAID_OBJECT_BODY_LIMIT_BYTES: usize = 64 * MIB;
const MAX_PAID_OBJECT_BODY_LIMIT_BYTES: usize = 64 * MIB;
const ENV_RON_STORAGE_MAX_BODY: &str = "RON_STORAGE_MAX_BODY";

#[derive(Debug, Clone)]
struct AccountingUsageContext {
    tenant: u128,
    subject: String,
    region: String,
    pin_seconds: u64,
}

#[derive(Debug, Serialize)]
struct PaidPutResp {
    cid: String,
    paid: bool,
    payer: String,
    escrow: String,
    wallet_txid: String,
    wallet_receipt_hash: String,
    wallet_idem: Option<String>,
    paid_context_idem: Option<String>,
    verifier: &'static str,
    estimate_minor: String,
    settlement: Option<PaidStorageSettlement>,
    accounting_export: AccountingExportReport,
    usage_events: Vec<UsageEventDto>,
}

#[derive(Debug, Serialize)]
struct PaidErrorBody {
    error: &'static str,
    reason: String,
}

#[derive(Debug)]
enum PaidRouteReject {
    PaymentRequired(String),
    Disabled(String),
    ConfigError(String),
    SettlementFailed(String),
}

impl PaidRouteReject {
    fn status(&self) -> StatusCode {
        match self {
            Self::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            Self::Disabled(_) => StatusCode::FORBIDDEN,
            Self::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SettlementFailed(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn metric_status(&self) -> &'static str {
        match self {
            Self::PaymentRequired(_) => "payment_required",
            Self::Disabled(_) => "disabled",
            Self::ConfigError(_) => "config_error",
            Self::SettlementFailed(_) => "settlement_error",
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::PaymentRequired(_) => "payment_required",
            Self::Disabled(_) => "paid_write_disabled",
            Self::ConfigError(_) => "config_error",
            Self::SettlementFailed(_) => "settlement_failed",
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::PaymentRequired(reason)
            | Self::Disabled(reason)
            | Self::ConfigError(reason)
            | Self::SettlementFailed(reason) => reason,
        }
    }

    fn into_response(self) -> Response {
        let status = self.status();
        let error = self.error_code();
        let reason = self.reason().to_string();

        observe_paid_write_status(self.metric_status(), 0);

        (status, Json(PaidErrorBody { error, reason })).into_response()
    }
}

/// Handle paid object ingest.
///
/// Settlement/export are intentionally opt-in. A successful paid write may
/// report accounting export failure, but does not roll back the already-settled
/// wallet/storage path because accounting remains transient metering.
pub async fn handler(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> impl IntoResponse {
    let body = match to_bytes(body, paid_object_body_limit_bytes()).await {
        Ok(body) => body,
        Err(_) => {
            observe_paid_write_status("payload_too_large", 0);

            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(PaidErrorBody {
                    error: "payload_too_large",
                    reason: "paid object body exceeded configured storage body cap".to_string(),
                }),
            )
                .into_response();
        }
    };

    let bytes_stored = u64::try_from(body.len()).unwrap_or(u64::MAX);
    let digest = blake3::hash(&body).to_hex().to_string();
    let cid = format!("b3:{digest}");

    let verified = match verify_paid_write(&headers).await {
        Ok(verified) => verified,
        Err(reject) => return reject.into_response(),
    };

    let paid_context_idem = match validate_context_binding(&verified, &cid) {
        Ok(idem) => idem,
        Err(reject) => return reject.into_response(),
    };

    let settlement_mode = match PaidSettlementMode::from_env() {
        Ok(mode) => mode,
        Err(err) => return PaidRouteReject::ConfigError(err.to_string()).into_response(),
    };

    let settlement_client = match build_settlement_client(settlement_mode, &verified) {
        Ok(client) => client,
        Err(reject) => return reject.into_response(),
    };

    let settlement_plan =
        match build_settlement_plan(settlement_mode, &verified, &cid, bytes_stored) {
            Ok(plan) => plan,
            Err(reject) => return reject.into_response(),
        };

    let usage = match AccountingUsageContext::from_headers(&headers) {
        Ok(usage) => usage,
        Err(reason) => {
            observe_paid_write_status("bad_accounting_context", 0);
            return (
                StatusCode::BAD_REQUEST,
                Json(PaidErrorBody {
                    error: "bad_accounting_context",
                    reason,
                }),
            )
                .into_response();
        }
    };

    if let Err(err) = app.store.put(&cid, body).await {
        if let (Some(client), Some(plan)) = (&settlement_client, &settlement_plan) {
            let _ = client.release_failed_paid_storage(plan).await;
        }

        observe_paid_write_status("storage_error", 0);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("paid put failed: {err}"),
        )
            .into_response();
    }

    let settlement = match settle_after_success(&settlement_client, &settlement_plan).await {
        Ok(settlement) => settlement,
        Err(reject) => return reject.into_response(),
    };

    observe_paid_write_status("accepted", bytes_stored);

    let verifier = verified.verifier;
    let proof = verified.proof;
    let usage_events = usage.events_for_success(bytes_stored, now_millis());

    let accounting_export = export_usage_events_from_env(&cid, &proof.txid, &usage_events).await;
    observe_accounting_export_status(accounting_export.status, accounting_export.event_count);

    (
        StatusCode::OK,
        Json(PaidPutResp {
            cid,
            paid: true,
            payer: proof.payer,
            escrow: proof.escrow,
            wallet_txid: proof.txid,
            wallet_receipt_hash: proof.receipt_hash,
            wallet_idem: proof.idem,
            paid_context_idem,
            verifier,
            estimate_minor: proof.estimate_minor.to_string(),
            settlement,
            accounting_export,
            usage_events,
        }),
    )
        .into_response()
}

async fn verify_paid_write(headers: &HeaderMap) -> Result<VerifiedPaidWrite, PaidRouteReject> {
    let mode = PaidWriteVerifierMode::from_env()
        .map_err(|err| PaidRouteReject::ConfigError(err.to_string()))?;

    match mode {
        PaidWriteVerifierMode::DevHeader => DevHeaderVerifier
            .verify(headers)
            .map_err(payment_required_from_verifier),
        PaidWriteVerifierMode::WalletReceipt => {
            let client = WalletReceiptHttpClient::new(
                wallet_receipt_base_url_from_env(),
                wallet_receipt_lookup_timeout_from_env(),
                wallet_receipt_bearer_from_env(),
            )
            .map_err(payment_required_from_verifier)?;

            client
                .verify_headers(headers)
                .await
                .map_err(payment_required_from_verifier)
        }
        PaidWriteVerifierMode::Disabled => Err(PaidRouteReject::Disabled(
            "paid writes are disabled by RON_STORAGE_PAID_WRITE_VERIFIER_MODE".to_string(),
        )),
    }
}

fn validate_context_binding(
    verified: &VerifiedPaidWrite,
    cid: &str,
) -> Result<Option<String>, PaidRouteReject> {
    verified
        .validate_paid_storage_context(cid)
        .map_err(payment_required_from_verifier)
}

fn build_settlement_client(
    settlement_mode: PaidSettlementMode,
    verified: &VerifiedPaidWrite,
) -> Result<Option<WalletSettlementHttpClient>, PaidRouteReject> {
    match settlement_mode {
        PaidSettlementMode::Disabled => Ok(None),
        PaidSettlementMode::WalletCapture => {
            if !verified.requires_context_binding() {
                return Err(PaidRouteReject::ConfigError(
                    "wallet-capture settlement requires wallet-backed paid-write verifier mode"
                        .to_string(),
                ));
            }

            WalletSettlementHttpClient::new(
                wallet_receipt_base_url_from_env(),
                wallet_receipt_lookup_timeout_from_env(),
                wallet_receipt_bearer_from_env(),
            )
            .map(Some)
            .map_err(settlement_reject_from_error)
        }
    }
}

fn build_settlement_plan(
    settlement_mode: PaidSettlementMode,
    verified: &VerifiedPaidWrite,
    cid: &str,
    bytes_stored: u64,
) -> Result<Option<PaidStorageSettlementPlan>, PaidRouteReject> {
    match settlement_mode {
        PaidSettlementMode::Disabled => Ok(None),
        PaidSettlementMode::WalletCapture => {
            let capture_amount_minor = paid_storage_capture_amount_from_env(bytes_stored)
                .map_err(PaidRouteReject::ConfigError)?;

            PaidStorageSettlementPlan::from_paid_write_with_capture_amount(
                &verified.proof,
                cid,
                capture_amount_minor,
                paid_settlement_payee_from_env(),
            )
            .map(Some)
            .map_err(settlement_reject_from_error)
        }
    }
}

async fn settle_after_success(
    client: &Option<WalletSettlementHttpClient>,
    plan: &Option<PaidStorageSettlementPlan>,
) -> Result<Option<PaidStorageSettlement>, PaidRouteReject> {
    let (Some(client), Some(plan)) = (client, plan) else {
        return Ok(None);
    };

    client
        .settle_paid_storage(plan)
        .await
        .map(Some)
        .map_err(settlement_reject_from_error)
}

fn payment_required_from_verifier(err: PaidWriteVerificationError) -> PaidRouteReject {
    PaidRouteReject::PaymentRequired(err.reason().to_string())
}

fn settlement_reject_from_error(err: PaidSettlementError) -> PaidRouteReject {
    match err {
        PaidSettlementError::PaymentRequired(reason) => PaidRouteReject::PaymentRequired(reason),
        PaidSettlementError::SettlementFailed(reason) => PaidRouteReject::SettlementFailed(reason),
    }
}

impl AccountingUsageContext {
    fn from_headers(headers: &HeaderMap) -> Result<Self, String> {
        let tenant = optional_header(headers, H_TENANT)
            .map(|value| {
                value
                    .parse::<u128>()
                    .map_err(|_| "x-ron-tenant must be an integer".to_string())
            })
            .transpose()?
            .unwrap_or(DEFAULT_TENANT);

        if tenant == 0 {
            return Err("x-ron-tenant must be greater than zero".to_string());
        }

        let subject = optional_header(headers, H_ACCOUNTING_SUBJECT)
            .unwrap_or_else(|| DEFAULT_ACCOUNTING_SUBJECT.to_string());

        if subject.trim().is_empty() {
            return Err("x-ron-accounting-subject cannot be empty".to_string());
        }

        let region =
            optional_header(headers, H_REGION).unwrap_or_else(|| DEFAULT_REGION.to_string());

        if region.trim().is_empty() {
            return Err("x-ron-region cannot be empty".to_string());
        }

        let pin_seconds = optional_header(headers, H_PIN_SECONDS)
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|_| "x-ron-pin-seconds must be an integer".to_string())
            })
            .transpose()?
            .unwrap_or(0);

        Ok(Self {
            tenant,
            subject,
            region,
            pin_seconds,
        })
    }

    fn events_for_success(&self, bytes_stored: u64, timestamp_ms: u64) -> Vec<UsageEventDto> {
        let mut events = vec![
            self.event(timestamp_ms, "bytes_stored", bytes_stored),
            self.event(timestamp_ms, "request_ok", 1),
        ];

        if self.pin_seconds > 0 {
            events.push(self.event(timestamp_ms, "pin_seconds", self.pin_seconds));
        }

        events
    }

    fn event(&self, timestamp_ms: u64, metric_kind: &'static str, value: u64) -> UsageEventDto {
        UsageEventDto {
            timestamp_ms,
            tenant: self.tenant,
            subject: self.subject.clone(),
            metric_kind,
            value,
            source_service: SOURCE_SERVICE,
            region: self.region.clone(),
            route: PAID_ROUTE,
        }
    }
}

fn optional_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| visible_header_value(value, name).ok())
}

fn visible_header_value(
    value: &axum::http::HeaderValue,
    name: &'static str,
) -> Result<String, String> {
    let value = value
        .to_str()
        .map_err(|_| format!("header is not visible ASCII/UTF-8: {name}"))?
        .trim();

    if value.is_empty() {
        return Err(format!("header cannot be empty: {name}"));
    }

    Ok(value.to_string())
}

fn now_millis() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(1);

    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn paid_object_body_limit_bytes() -> usize {
    std::env::var(ENV_RON_STORAGE_MAX_BODY)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .map(|value| value.min(MAX_PAID_OBJECT_BODY_LIMIT_BYTES))
        .unwrap_or(DEFAULT_PAID_OBJECT_BODY_LIMIT_BYTES)
}

fn observe_paid_write_status(status: &'static str, bytes_stored: u64) {
    #[cfg(feature = "metrics")]
    crate::metrics::observe_paid_write(status, bytes_stored);

    #[cfg(not(feature = "metrics"))]
    {
        let _ = status;
        let _ = bytes_stored;
    }
}

fn observe_accounting_export_status(status: &'static str, event_count: usize) {
    #[cfg(feature = "metrics")]
    crate::metrics::observe_accounting_export(status, event_count as u64);

    #[cfg(not(feature = "metrics"))]
    {
        let _ = status;
        let _ = event_count;
    }
}
