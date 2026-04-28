//! RO:WHAT — Paid CAS ingest handler for POST/PUT /paid/o.
//! RO:WHY — Pillar 12; Concerns: ECON/SEC/RES. Paid storage must require escrow proof before write.
//! RO:INTERACTS — AppState storage trait, paid_write verifier seam, svc-wallet receipt lookup, usage DTOs.
//! RO:INVARIANTS — no proof means no write; verifier mode explicit; CID is BLAKE3.
//! RO:METRICS — storage_paid_write_total{status}, storage_paid_write_bytes_total.
//! RO:CONFIG — RON_STORAGE_PAID_WRITE_VERIFIER_MODE, wallet URL/bearer/timeout, optional accounting headers.
//! RO:SECURITY — dev-header is explicit; wallet-receipt mode calls wallet and fails closed.
//! RO:TEST — paid_write_policy.rs, paid_write_verifier.rs, paid_write_http_client.rs, web3_paid_storage_loop.rs.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    config::{
        wallet_receipt_base_url_from_env, wallet_receipt_bearer_from_env,
        wallet_receipt_lookup_timeout_from_env, PaidWriteVerifierMode,
    },
    http::extractors::AppState,
    policy::paid_write::{
        DevHeaderVerifier, PaidWriteVerificationError, PaidWriteVerifier, VerifiedPaidWrite,
        WalletReceiptHttpClient,
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

#[derive(Debug, Clone)]
struct AccountingUsageContext {
    tenant: u128,
    subject: String,
    region: String,
    pin_seconds: u64,
}

#[derive(Debug, Serialize)]
struct AccountingUsageEventDto {
    timestamp_ms: u64,
    tenant: u128,
    subject: String,
    metric_kind: &'static str,
    value: u64,
    source_service: &'static str,
    region: String,
    route: &'static str,
}

#[derive(Debug, Serialize)]
struct PaidPutResp {
    cid: String,
    paid: bool,
    payer: String,
    escrow: String,
    wallet_txid: String,
    wallet_receipt_hash: String,
    estimate_minor: String,
    usage_events: Vec<AccountingUsageEventDto>,
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
}

impl PaidRouteReject {
    fn status(&self) -> StatusCode {
        match self {
            Self::PaymentRequired(_) => StatusCode::PAYMENT_REQUIRED,
            Self::Disabled(_) => StatusCode::FORBIDDEN,
            Self::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn metric_status(&self) -> &'static str {
        match self {
            Self::PaymentRequired(_) => "payment_required",
            Self::Disabled(_) => "disabled",
            Self::ConfigError(_) => "config_error",
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::PaymentRequired(_) => "payment_required",
            Self::Disabled(_) => "paid_write_disabled",
            Self::ConfigError(_) => "config_error",
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::PaymentRequired(reason) | Self::Disabled(reason) | Self::ConfigError(reason) => {
                reason
            }
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
/// The paid route is intentionally separate from `/o`. The free/dev CAS route
/// remains useful for local development and existing tests, while `/paid/o` is
/// the first enforcement seam for ROC-backed storage.
pub async fn handler(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    let verified = match verify_paid_write(&headers).await {
        Ok(verified) => verified,
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

    let bytes_stored = u64::try_from(body.len()).unwrap_or(u64::MAX);
    let digest = blake3::hash(&body).to_hex().to_string();
    let cid = format!("b3:{digest}");

    if let Err(err) = app.store.put(&cid, body).await {
        observe_paid_write_status("storage_error", 0);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("paid put failed: {err}"),
        )
            .into_response();
    }

    observe_paid_write_status("accepted", bytes_stored);

    let proof = verified.proof;
    let usage_events = usage.events_for_success(bytes_stored, now_millis());

    (
        StatusCode::OK,
        Json(PaidPutResp {
            cid,
            paid: true,
            payer: proof.payer,
            escrow: proof.escrow,
            wallet_txid: proof.txid,
            wallet_receipt_hash: proof.receipt_hash,
            estimate_minor: proof.estimate_minor.to_string(),
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

fn payment_required_from_verifier(err: PaidWriteVerificationError) -> PaidRouteReject {
    PaidRouteReject::PaymentRequired(err.reason().to_string())
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

    fn events_for_success(
        &self,
        bytes_stored: u64,
        timestamp_ms: u64,
    ) -> Vec<AccountingUsageEventDto> {
        let mut events = vec![
            self.event(timestamp_ms, "bytes_stored", bytes_stored),
            self.event(timestamp_ms, "request_ok", 1),
        ];

        if self.pin_seconds > 0 {
            events.push(self.event(timestamp_ms, "pin_seconds", self.pin_seconds));
        }

        events
    }

    fn event(
        &self,
        timestamp_ms: u64,
        metric_kind: &'static str,
        value: u64,
    ) -> AccountingUsageEventDto {
        AccountingUsageEventDto {
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

fn observe_paid_write_status(status: &'static str, bytes_stored: u64) {
    #[cfg(feature = "metrics")]
    crate::metrics::observe_paid_write(status, bytes_stored);

    #[cfg(not(feature = "metrics"))]
    {
        let _ = status;
        let _ = bytes_stored;
    }
}
