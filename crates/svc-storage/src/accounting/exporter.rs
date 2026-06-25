//! RO:WHAT — Optional HTTP exporter for svc-storage usage events.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Bridges storage metering to accounting without coupling crates.
//! RO:INTERACTS — UsageEventDto, reqwest, future ron-accounting HTTP ingest endpoint.
//! RO:INVARIANTS — export is retry/idempotency shaped; export failure never mutates ledger or wallet state.
//! RO:METRICS — emits storage_accounting_export_total status observations through route path.
//! RO:CONFIG — RON_STORAGE_ACCOUNTING_EXPORT_* envs.
//! RO:SECURITY — optional bearer only sent to accounting endpoint; no wallet receipt/body bytes exported.
//! RO:TEST — tests/paid_write_accounting_export.rs.

use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use crate::{
    accounting::UsageEventDto,
    config::{
        accounting_export_base_url_from_env, accounting_export_bearer_from_env,
        accounting_export_timeout_from_env, AccountingExportMode,
    },
};

/// Export request schema sent to the accounting HTTP adapter.
pub const ACCOUNTING_EXPORT_SCHEMA: &str = "svc-storage.usage-events.v1";

/// Default accounting ingest path appended to the configured base URL.
pub const ACCOUNTING_USAGE_EVENTS_PATH: &str = "/v1/usage-events";

/// Report returned in `/paid/o` responses for observability/debugging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingExportReport {
    /// Export mode used.
    pub mode: &'static str,
    /// Machine status: skipped, exported, or failed.
    pub status: &'static str,
    /// Number of events considered for export.
    pub event_count: usize,
    /// Deterministic idempotency key for the batch.
    pub idempotency_key: Option<String>,
    /// HTTP status returned by accounting, if a request was made.
    pub http_status: Option<u16>,
    /// Failure reason, if any.
    pub reason: Option<String>,
}

impl AccountingExportReport {
    /// Build a skipped report.
    #[must_use]
    pub fn skipped(event_count: usize) -> Self {
        Self {
            mode: "disabled",
            status: "skipped",
            event_count,
            idempotency_key: None,
            http_status: None,
            reason: None,
        }
    }

    /// Build an exported report.
    #[must_use]
    pub fn exported(event_count: usize, idempotency_key: String, http_status: u16) -> Self {
        Self {
            mode: "http",
            status: "exported",
            event_count,
            idempotency_key: Some(idempotency_key),
            http_status: Some(http_status),
            reason: None,
        }
    }

    /// Build a failed report.
    #[must_use]
    pub fn failed(
        mode: &'static str,
        event_count: usize,
        idempotency_key: Option<String>,
        http_status: Option<u16>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            mode,
            status: "failed",
            event_count,
            idempotency_key,
            http_status,
            reason: Some(reason.into()),
        }
    }
}

/// HTTP payload for usage event export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountingExportRequest {
    /// Stable schema name.
    pub schema: &'static str,
    /// Object CID associated with this paid write.
    pub cid: String,
    /// Wallet hold transaction ID associated with this paid write.
    pub wallet_txid: String,
    /// Source service label.
    pub source_service: &'static str,
    /// Usage events.
    pub events: Vec<UsageEventDto>,
}

impl AccountingExportRequest {
    /// Build a request.
    #[must_use]
    pub fn new(cid: &str, wallet_txid: &str, events: &[UsageEventDto]) -> Self {
        Self {
            schema: ACCOUNTING_EXPORT_SCHEMA,
            cid: cid.to_string(),
            wallet_txid: wallet_txid.to_string(),
            source_service: "svc-storage",
            events: events.to_vec(),
        }
    }
}

/// HTTP accounting usage exporter.
#[derive(Debug, Clone)]
pub struct AccountingHttpExporter {
    client: reqwest::Client,
    base_url: String,
    bearer: Option<String>,
}

impl AccountingHttpExporter {
    /// Build a new exporter.
    pub fn new(
        base_url: impl Into<String>,
        timeout: Duration,
        bearer: Option<String>,
    ) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|err| format!("failed to build accounting HTTP client: {err}"))?;

        Ok(Self {
            client,
            base_url: normalize_base_url(base_url.into())?,
            bearer,
        })
    }

    /// Configured base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Export usage events to accounting.
    pub async fn export_usage_events(
        &self,
        cid: &str,
        wallet_txid: &str,
        events: &[UsageEventDto],
    ) -> AccountingExportReport {
        let event_count = events.len();
        let idempotency_key = accounting_export_idem(cid, wallet_txid, events);
        let request = AccountingExportRequest::new(cid, wallet_txid, events);
        let url = format!("{}{}", self.base_url, ACCOUNTING_USAGE_EVENTS_PATH);

        let mut http = self
            .client
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header("Idempotency-Key", idempotency_key.as_str())
            .json(&request);

        if let Some(bearer) = &self.bearer {
            http = http.header(AUTHORIZATION, format!("Bearer {bearer}"));
        }

        let response = match http.send().await {
            Ok(response) => response,
            Err(err) => {
                return AccountingExportReport::failed(
                    "http",
                    event_count,
                    Some(idempotency_key),
                    None,
                    format!("accounting export request failed: {err}"),
                );
            }
        };

        let status = response.status();
        if !status.is_success() {
            return AccountingExportReport::failed(
                "http",
                event_count,
                Some(idempotency_key),
                Some(status.as_u16()),
                format!("accounting export rejected with status {status}"),
            );
        }

        AccountingExportReport::exported(event_count, idempotency_key, status.as_u16())
    }
}

/// Export usage events according to current environment configuration.
///
/// Export failure is deliberately reported, not raised. Accounting is transient
/// metering and must not undo a successful wallet-settled storage write.
pub async fn export_usage_events_from_env(
    cid: &str,
    wallet_txid: &str,
    events: &[UsageEventDto],
) -> AccountingExportReport {
    let mode = match AccountingExportMode::from_env() {
        Ok(mode) => mode,
        Err(err) => {
            return AccountingExportReport::failed(
                "config",
                events.len(),
                None,
                None,
                err.to_string(),
            );
        }
    };

    match mode {
        AccountingExportMode::Disabled => AccountingExportReport::skipped(events.len()),
        AccountingExportMode::Http => {
            let exporter = match AccountingHttpExporter::new(
                accounting_export_base_url_from_env(),
                accounting_export_timeout_from_env(),
                accounting_export_bearer_from_env(),
            ) {
                Ok(exporter) => exporter,
                Err(reason) => {
                    return AccountingExportReport::failed(
                        "http",
                        events.len(),
                        None,
                        None,
                        reason,
                    );
                }
            };

            exporter.export_usage_events(cid, wallet_txid, events).await
        }
    }
}

/// Deterministic idempotency key for one accounting export batch.
#[must_use]
pub fn accounting_export_idem(cid: &str, wallet_txid: &str, events: &[UsageEventDto]) -> String {
    let mut canonical = format!(
        "schema={ACCOUNTING_EXPORT_SCHEMA}\ncid={cid}\nwallet_txid={wallet_txid}\ncount={}\n",
        events.len()
    );

    for event in events {
        canonical.push_str(&format!(
            "event|{}|{}|{}|{}|{}|{}|{}|{}\n",
            event.timestamp_ms,
            event.tenant,
            event.subject,
            event.metric_kind,
            event.value,
            event.source_service,
            event.region,
            event.route
        ));
    }

    let hex = blake3::hash(canonical.as_bytes()).to_hex().to_string();
    format!("storage_acct:{}", &hex[..32])
}

fn normalize_base_url(value: String) -> Result<String, String> {
    let trimmed = value.trim().trim_end_matches('/');

    if trimmed.is_empty() {
        return Err("accounting base URL cannot be empty".to_string());
    }

    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("accounting base URL must start with http:// or https://".to_string());
    }

    Ok(trimmed.to_string())
}
