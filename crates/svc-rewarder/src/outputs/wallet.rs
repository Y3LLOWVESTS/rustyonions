//! RO:WHAT — Wallet issue clients for turning reward settlements into svc-wallet issue requests.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Rewarder targets wallet as the mutation boundary.
//! RO:INTERACTS — outputs::intents, http handlers, svc-wallet /v1/issue.
//! RO:INVARIANTS — rewarder never mutates ledger directly; dry-run emits nothing; wallet idempotency keys are preserved.
//! RO:METRICS — handlers count wallet/ledger intent outcomes.
//! RO:CONFIG — wallet base URL and issue path come from Config.ingress.
//! RO:SECURITY — Authorization is explicit; no bearer/cap values are logged.
//! RO:TEST — tests/unit/wallet_client.rs and scripts/web3_accounting_rewarder_wallet_smoke.sh.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::outputs::intents::{
    IntentResult, IntentStore, SettlementBatch, WalletIssueBatch, WalletIssueRequest,
    WALLET_ISSUE_PATH,
};
use crate::{Result, RewarderError};

/// Result returned by the local/dev wallet issue client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletIssueOutcome {
    /// Egress result.
    pub result: IntentResult,
    /// Wallet-compatible issue batch.
    pub batch: WalletIssueBatch,
}

/// Result returned by the HTTP wallet issue client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletHttpIssueOutcome {
    /// Egress result.
    pub result: IntentResult,
    /// Wallet-compatible issue batch.
    pub batch: WalletIssueBatch,
    /// Wallet receipts returned by `svc-wallet`.
    pub receipts: Vec<Value>,
}

/// Trait seam for local/dev wallet issue clients.
pub trait WalletIssueClient: Send + Sync {
    /// Preview wallet issue requests without emitting anything.
    fn preview_issue_batch(&self, settlement: &SettlementBatch) -> Result<WalletIssueBatch>;

    /// Emit a wallet issue batch through the client.
    fn emit_issue_batch(
        &self,
        settlement: &SettlementBatch,
        dry_run: bool,
    ) -> Result<WalletIssueOutcome>;
}

/// Development wallet issue client.
#[derive(Debug, Clone)]
pub struct DevWalletIssueClient {
    store: Arc<IntentStore>,
    issue_path: String,
}

impl DevWalletIssueClient {
    /// Build a dev client from the shared intent store and configured wallet path.
    #[must_use]
    pub fn new(store: Arc<IntentStore>, issue_path: impl Into<String>) -> Self {
        Self {
            store,
            issue_path: normalize_issue_path(issue_path.into()),
        }
    }
}

impl WalletIssueClient for DevWalletIssueClient {
    fn preview_issue_batch(&self, settlement: &SettlementBatch) -> Result<WalletIssueBatch> {
        let mut batch = settlement.to_wallet_issue_batch();
        batch.wallet_path = self.issue_path.clone();
        Ok(batch)
    }

    fn emit_issue_batch(
        &self,
        settlement: &SettlementBatch,
        dry_run: bool,
    ) -> Result<WalletIssueOutcome> {
        let result = self.store.emit_batch_once(settlement, dry_run);
        let batch = self.preview_issue_batch(settlement)?;
        Ok(WalletIssueOutcome { result, batch })
    }
}

/// Minimal HTTP/1.1 wallet issue client.
///
/// This deliberately uses Tokio TCP directly for now so we avoid dependency churn in the WEB3
/// integration path. It is strict enough for local service-to-service HTTP and can later be
/// replaced by a shared transport/client adapter.
#[derive(Debug, Clone)]
pub struct HttpWalletIssueClient {
    base: ParsedHttpBaseUrl,
    issue_path: String,
    bearer_token: String,
    timeout: Duration,
}

impl HttpWalletIssueClient {
    /// Build an HTTP wallet issue client.
    pub fn try_new(
        wallet_base_url: impl Into<String>,
        issue_path: impl Into<String>,
        bearer_token: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self> {
        let base = ParsedHttpBaseUrl::parse(&wallet_base_url.into())?;
        let issue_path = normalize_issue_path(issue_path.into());
        let bearer_token = checked_header_value("bearer token", &bearer_token.into())?;
        if timeout.is_zero() {
            return Err(RewarderError::Config(
                "wallet HTTP client timeout must be > 0".into(),
            ));
        }

        Ok(Self {
            base,
            issue_path,
            bearer_token,
            timeout,
        })
    }

    /// Preview wallet issue requests without network side effects.
    pub fn preview_issue_batch(&self, settlement: &SettlementBatch) -> Result<WalletIssueBatch> {
        let mut batch = settlement.to_wallet_issue_batch();
        batch.wallet_path = self.base.endpoint_path(&self.issue_path);
        Ok(batch)
    }

    /// Emit the wallet issue batch by POSTing each request to `svc-wallet`.
    pub async fn emit_issue_batch(
        &self,
        settlement: &SettlementBatch,
        dry_run: bool,
    ) -> Result<WalletHttpIssueOutcome> {
        let batch = self.preview_issue_batch(settlement)?;
        if dry_run {
            return Ok(WalletHttpIssueOutcome {
                result: IntentResult::DryRun,
                batch,
                receipts: Vec::new(),
            });
        }

        let mut receipts = Vec::with_capacity(batch.requests.len());
        for request in &batch.requests {
            receipts.push(self.post_issue(request).await?);
        }

        Ok(WalletHttpIssueOutcome {
            result: IntentResult::Accepted,
            batch,
            receipts,
        })
    }

    async fn post_issue(&self, request: &WalletIssueRequest) -> Result<Value> {
        let idempotency_key = request
            .idempotency_key
            .as_deref()
            .ok_or_else(|| {
                RewarderError::Internal("wallet issue request missing idempotency_key".into())
            })
            .and_then(|value| checked_header_value("Idempotency-Key", value))?;

        // svc-wallet accepts the idempotency key from the header or the body.
        // Keep it in both places so the request fingerprint remains stable and the public
        // preview DTO matches the emitted DTO shape documented in the rewarder notes.
        let wallet_body = json!({
            "to": request.to,
            "asset": request.asset,
            "amount_minor": request.amount_minor,
            "idempotency_key": request.idempotency_key,
            "memo": request.memo,
        });

        let body = serde_json::to_vec(&wallet_body)
            .map_err(|err| RewarderError::Internal(format!("wallet issue encode failed: {err}")))?;

        let path = self.base.endpoint_path(&self.issue_path);
        let header = format!(
            "POST {path} HTTP/1.1\r\n\
             Host: {host}\r\n\
             Authorization: Bearer {bearer}\r\n\
             Idempotency-Key: {idempotency_key}\r\n\
             Content-Type: application/json\r\n\
             Accept: application/json\r\n\
             Content-Length: {len}\r\n\
             Connection: close\r\n\r\n",
            host = self.base.host_header,
            bearer = self.bearer_token,
            len = body.len()
        );

        tokio::time::timeout(self.timeout, async {
            let mut stream = TcpStream::connect(&self.base.connect_addr)
                .await
                .map_err(|err| {
                    RewarderError::DependencyUnavailable(format!(
                        "wallet connect failed at {}: {err}",
                        self.base.connect_addr
                    ))
                })?;

            stream.write_all(header.as_bytes()).await.map_err(|err| {
                RewarderError::DependencyUnavailable(format!(
                    "wallet request header write failed: {err}"
                ))
            })?;
            stream.write_all(&body).await.map_err(|err| {
                RewarderError::DependencyUnavailable(format!(
                    "wallet request body write failed: {err}"
                ))
            })?;

            let mut response = Vec::new();
            stream.read_to_end(&mut response).await.map_err(|err| {
                RewarderError::DependencyUnavailable(format!("wallet response read failed: {err}"))
            })?;

            parse_wallet_response(&response)
        })
        .await
        .map_err(|_| RewarderError::Timeout("wallet issue request timed out".into()))?
    }
}

#[derive(Debug, Clone)]
struct ParsedHttpBaseUrl {
    host_header: String,
    connect_addr: String,
    base_path: String,
}

impl ParsedHttpBaseUrl {
    fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim().trim_end_matches('/');
        let without_scheme = trimmed.strip_prefix("http://").ok_or_else(|| {
            RewarderError::Config(
                "ingress.wallet_base_url must use http:// for the current dependency-free client"
                    .into(),
            )
        })?;

        if without_scheme.is_empty() {
            return Err(RewarderError::Config(
                "ingress.wallet_base_url missing host".into(),
            ));
        }

        let (authority, path) = without_scheme
            .split_once('/')
            .map_or((without_scheme, ""), |(authority, path)| (authority, path));

        let authority = authority.trim();
        if authority.is_empty() {
            return Err(RewarderError::Config(
                "ingress.wallet_base_url missing host".into(),
            ));
        }
        if authority.contains('@') || authority.chars().any(char::is_whitespace) {
            return Err(RewarderError::Config(
                "ingress.wallet_base_url authority is invalid".into(),
            ));
        }

        let connect_addr = if authority.contains(':') {
            authority.to_string()
        } else {
            format!("{authority}:80")
        };

        let base_path = if path.trim().is_empty() {
            String::new()
        } else {
            format!("/{}", path.trim_matches('/'))
        };

        Ok(Self {
            host_header: authority.to_string(),
            connect_addr,
            base_path,
        })
    }

    fn endpoint_path(&self, issue_path: &str) -> String {
        let issue_path = normalize_issue_path(issue_path.to_string());
        if self.base_path.is_empty() {
            issue_path
        } else {
            format!("{}{}", self.base_path, issue_path)
        }
    }
}

fn parse_wallet_response(response: &[u8]) -> Result<Value> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| {
            RewarderError::DependencyUnavailable("wallet response missing header terminator".into())
        })?;

    let header_bytes = &response[..header_end];
    let body = &response[header_end + 4..];

    let header = std::str::from_utf8(header_bytes).map_err(|err| {
        RewarderError::DependencyUnavailable(format!("wallet response header was not UTF-8: {err}"))
    })?;

    let mut lines = header.lines();
    let status_line = lines.next().ok_or_else(|| {
        RewarderError::DependencyUnavailable("wallet response missing status line".into())
    })?;

    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| {
            RewarderError::DependencyUnavailable("wallet response had invalid status line".into())
        })?;

    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect::<Vec<_>>();

    let decoded_body = if headers.iter().any(|(name, value)| {
        name == "transfer-encoding" && value.to_ascii_lowercase().contains("chunked")
    }) {
        decode_chunked_body(body)?
    } else {
        body.to_vec()
    };

    if !(200..300).contains(&status) {
        let body_text = String::from_utf8_lossy(&decoded_body);
        return Err(RewarderError::DependencyUnavailable(format!(
            "wallet issue rejected with HTTP {status}: {body_text}"
        )));
    }

    serde_json::from_slice::<Value>(&decoded_body).map_err(|err| {
        let body_text = String::from_utf8_lossy(&decoded_body);
        RewarderError::DependencyUnavailable(format!(
            "wallet response body was not JSON: {err}; body={body_text}"
        ))
    })
}

fn decode_chunked_body(body: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0usize;

    loop {
        let size_line_end = find_crlf(body, pos).ok_or_else(|| {
            RewarderError::DependencyUnavailable("chunked wallet response missing size line".into())
        })?;

        let size_line = std::str::from_utf8(&body[pos..size_line_end]).map_err(|err| {
            RewarderError::DependencyUnavailable(format!(
                "chunked wallet response size line was not UTF-8: {err}"
            ))
        })?;

        let size_hex = size_line.split(';').next().unwrap_or_default().trim();

        let size = usize::from_str_radix(size_hex, 16).map_err(|err| {
            RewarderError::DependencyUnavailable(format!(
                "chunked wallet response invalid chunk size {size_hex:?}: {err}"
            ))
        })?;

        pos = size_line_end + 2;

        if size == 0 {
            return Ok(out);
        }

        let chunk_end = pos.checked_add(size).ok_or_else(|| {
            RewarderError::DependencyUnavailable("chunked wallet response size overflow".into())
        })?;

        if chunk_end + 2 > body.len() {
            return Err(RewarderError::DependencyUnavailable(
                "chunked wallet response truncated".into(),
            ));
        }

        out.extend_from_slice(&body[pos..chunk_end]);

        if &body[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(RewarderError::DependencyUnavailable(
                "chunked wallet response missing chunk terminator".into(),
            ));
        }

        pos = chunk_end + 2;
    }
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|offset| start + offset)
}

fn checked_header_value(name: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RewarderError::Config(format!("{name} must not be empty")));
    }
    if trimmed.contains('\r') || trimmed.contains('\n') {
        return Err(RewarderError::Config(format!(
            "{name} must not contain line breaks"
        )));
    }
    Ok(trimmed.to_string())
}

fn normalize_issue_path(path: String) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        WALLET_ISSUE_PATH.into()
    } else if trimmed.starts_with('/') {
        trimmed.into()
    } else {
        format!("/{trimmed}")
    }
}
