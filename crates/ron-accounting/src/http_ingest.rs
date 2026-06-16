//! RO:WHAT — Tiny std-only HTTP ingest adapter for ron-accounting usage events.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Provides a live storage→accounting proof without new deps.
//! RO:INTERACTS — UsageEvent, EventIngestPolicy, Recorder, record_usage_events, svc-storage export.
//! RO:INVARIANTS — accounting records usage only; no wallet/ledger mutation; idempotency key required.
//! RO:METRICS — no Prometheus here yet; response reports inspected/recorded/skipped/duplicate.
//! RO:CONFIG — RON_ACCOUNTING_ADDR, RON_ACC_BEARER.
//! RO:SECURITY — optional bearer gate for ingest; no object bytes or wallet secrets stored.
//! RO:TEST — live smoke: scripts/web3_paid_storage_live_smoke.sh; quickchain_preflight_ingest_poisoning.

use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    record_usage_events, EventIngestPolicy, EventIngestReport, Recorder, Result, UsageEvent,
};

/// Expected schema from svc-storage accounting export.
pub const STORAGE_USAGE_EVENTS_SCHEMA: &str = "svc-storage.usage-events.v1";

/// Default bind address for the lightweight ingest adapter.
pub const DEFAULT_ACCOUNTING_ADDR: &str = "127.0.0.1:19600";

/// Environment variable for the lightweight ingest adapter bind address.
pub const ENV_ACCOUNTING_ADDR: &str = "RON_ACCOUNTING_ADDR";

/// Environment variable for the optional bearer token accepted by ingest.
///
/// Default is `dev` so the local WEB3 smoke can run without another config file.
pub const ENV_ACCOUNTING_BEARER: &str = "RON_ACC_BEARER";

/// Maximum usage events accepted in one lightweight ingest request.
pub const MAX_USAGE_EVENTS_PER_REQUEST: usize = 1024;

/// Request body accepted at `POST /v1/usage-events`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageEventsIngestRequest {
    /// Stable schema name.
    pub schema: String,
    /// Object CID associated with this paid write.
    pub cid: String,
    /// Wallet transaction ID associated with this paid write.
    pub wallet_txid: String,
    /// Source service label.
    pub source_service: String,
    /// Usage events.
    pub events: Vec<UsageEvent>,
}

impl UsageEventsIngestRequest {
    /// Validate the public ingest request before idempotency state is consumed.
    ///
    /// This is intentionally public so boundary tests can prove poisoned request
    /// bodies fail before they can be treated as accounting authority.
    pub fn validate_for_ingest(&self) -> Result<()> {
        if self.schema != STORAGE_USAGE_EVENTS_SCHEMA {
            return Err(crate::Error::schema("unexpected usage event schema"));
        }

        if !is_b3_cid(&self.cid) {
            return Err(crate::Error::schema("cid must be b3:<64 lowercase hex>"));
        }

        validate_ingest_string("wallet_txid", &self.wallet_txid)?;
        validate_ingest_string("source_service", &self.source_service)?;

        if self.events.len() > MAX_USAGE_EVENTS_PER_REQUEST {
            return Err(crate::Error::schema(format!(
                "usage event batch exceeds {MAX_USAGE_EVENTS_PER_REQUEST} events"
            )));
        }

        for event in &self.events {
            event.validate()?;
        }

        Ok(())
    }
}

/// Response body returned from accepted ingest requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageEventsIngestResponse {
    /// Whether this ingest call was accepted.
    pub ok: bool,
    /// Whether this Idempotency-Key was already seen.
    pub duplicate: bool,
    /// Idempotency key used for this ingest request.
    pub idempotency_key: String,
    /// Events inspected.
    pub inspected: usize,
    /// Events recorded.
    pub recorded: usize,
    /// Zero-value events skipped.
    pub skipped_zero: usize,
    /// Current recorder row count after the request.
    pub row_count: usize,
}

/// Shared state for the lightweight ingest adapter.
#[derive(Debug, Clone)]
pub struct IngestState {
    recorder: Recorder,
    policy: EventIngestPolicy,
    seen_idempotency_keys: Arc<Mutex<HashSet<String>>>,
    bearer: Option<String>,
}

impl IngestState {
    /// Build a new ingest state.
    #[must_use]
    pub fn new(recorder: Recorder, policy: EventIngestPolicy, bearer: Option<String>) -> Self {
        Self {
            recorder,
            policy,
            seen_idempotency_keys: Arc::new(Mutex::new(HashSet::new())),
            bearer,
        }
    }

    /// Access the recorder used by this adapter.
    #[must_use]
    pub fn recorder(&self) -> &Recorder {
        &self.recorder
    }

    fn ingest(&self, headers: &[(String, String)], body: &[u8]) -> HttpResponse {
        if let Err(response) = self.authorize(headers) {
            return response;
        }

        let Some(idempotency_key) = header_value(headers, "idempotency-key") else {
            return problem(400, "missing Idempotency-Key header");
        };

        if idempotency_key.trim().is_empty() || idempotency_key.len() > 160 {
            return problem(400, "invalid Idempotency-Key header");
        }

        let request = match serde_json::from_slice::<UsageEventsIngestRequest>(body) {
            Ok(request) => request,
            Err(err) => return problem(400, format!("invalid usage event JSON: {err}")),
        };

        if let Err(err) = request.validate_for_ingest() {
            return problem(400, format!("usage event request rejected: {err}"));
        }

        let duplicate = {
            let mut seen = self
                .seen_idempotency_keys
                .lock()
                .expect("idempotency key set lock should not be poisoned");

            !seen.insert(idempotency_key.clone())
        };

        let report = if duplicate {
            EventIngestReport {
                inspected: request.events.len(),
                recorded: 0,
                skipped_zero: 0,
            }
        } else {
            match record_usage_events(&self.recorder, &request.events, &self.policy) {
                Ok(report) => report,
                Err(err) => {
                    let mut seen = self
                        .seen_idempotency_keys
                        .lock()
                        .expect("idempotency key set lock should not be poisoned");
                    seen.remove(&idempotency_key);

                    return problem(400, format!("usage event ingest rejected: {err}"));
                }
            }
        };

        json_response(
            if duplicate { 200 } else { 202 },
            &UsageEventsIngestResponse {
                ok: true,
                duplicate,
                idempotency_key,
                inspected: report.inspected,
                recorded: report.recorded,
                skipped_zero: report.skipped_zero,
                row_count: self.recorder.row_count(),
            },
        )
    }

    fn authorize(&self, headers: &[(String, String)]) -> std::result::Result<(), HttpResponse> {
        let Some(expected) = &self.bearer else {
            return Ok(());
        };

        let expected = expected.trim();
        if expected.is_empty() {
            return Ok(());
        }

        let authorization = header_value(headers, "authorization").unwrap_or_default();
        if authorization == format!("Bearer {expected}") {
            Ok(())
        } else {
            Err(problem(401, "missing or invalid Authorization bearer"))
        }
    }
}

/// Run the lightweight blocking HTTP server forever.
pub fn serve_blocking(addr: SocketAddr, state: IngestState) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .map_err(|err| crate::Error::other(format!("bind {addr}: {err}")))?;
    listener
        .set_nonblocking(false)
        .map_err(|err| crate::Error::other(format!("set listener blocking mode: {err}")))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = state.clone();
                thread::spawn(move || {
                    let _ = handle_stream(stream, &state);
                });
            }
            Err(err) => return Err(crate::Error::other(format!("accept failed: {err}"))),
        }
    }

    Ok(())
}

/// Parse bind address from env.
pub fn addr_from_env() -> Result<SocketAddr> {
    let value =
        std::env::var(ENV_ACCOUNTING_ADDR).unwrap_or_else(|_| DEFAULT_ACCOUNTING_ADDR.to_string());

    value.parse::<SocketAddr>().map_err(|err| {
        crate::Error::schema(format!("invalid {ENV_ACCOUNTING_ADDR}={value}: {err}"))
    })
}

/// Parse optional bearer from env.
#[must_use]
pub fn bearer_from_env() -> Option<String> {
    std::env::var(ENV_ACCOUNTING_BEARER)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| Some("dev".to_string()))
}

fn handle_stream(mut stream: TcpStream, state: &IngestState) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let request = match read_http_request(&mut stream) {
        Ok(request) => request,
        Err(err) => {
            let response = problem(400, format!("bad request: {err}"));
            write_response(&mut stream, &response)?;
            return Ok(());
        }
    };

    let response = route_request(request, state);
    write_response(&mut stream, &response)
}

fn route_request(request: HttpRequest, state: &IngestState) -> HttpResponse {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/healthz") => json_response(
            200,
            &json!({
                "service": "ron-accounting",
                "ok": true
            }),
        ),
        ("GET", "/readyz") => json_response(
            200,
            &json!({
                "service": "ron-accounting",
                "ready": true,
                "rows": state.recorder.row_count()
            }),
        ),
        ("GET", "/v1/snapshot") => json_response(
            200,
            &json!({
                "schema": "ron-accounting.snapshot.v1",
                "row_count": state.recorder.row_count(),
                "rows": state.recorder.snapshot()
            }),
        ),
        ("POST", "/v1/usage-events") => state.ingest(&request.headers, &request.body),
        _ => problem(404, "not found"),
    }
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut buffer = Vec::with_capacity(8192);
    let mut temp = [0_u8; 1024];

    let header_end = loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed before headers",
            ));
        }

        buffer.extend_from_slice(&temp[..n]);

        if buffer.len() > 64 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "headers too large",
            ));
        }

        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }
    };

    let header_bytes = &buffer[..header_end];
    let header_text = std::str::from_utf8(header_bytes)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;

    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut request_parts = request_line.split_whitespace();

    let method = request_parts
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let path = request_parts
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .to_string();

    if method.is_empty() || path.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid request line",
        ));
    }

    let headers: Vec<(String, String)> = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();

    let content_length = header_value(&headers, "content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    if content_length > 1_048_576 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "body too large",
        ));
    }

    let body_start = header_end + 4;
    let mut body = buffer.get(body_start..).unwrap_or_default().to_vec();

    while body.len() < content_length {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed before body",
            ));
        }
        body.extend_from_slice(&temp[..n]);
    }

    body.truncate(content_length);

    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: &HttpResponse) -> std::io::Result<()> {
    let reason = reason_phrase(response.status);
    let headers = format!(
        "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );

    stream.write_all(headers.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn json_response<T>(status: u16, value: &T) -> HttpResponse
where
    T: Serialize,
{
    match serde_json::to_vec(value) {
        Ok(body) => HttpResponse {
            status,
            content_type: "application/json",
            body,
        },
        Err(err) => problem(500, format!("failed to encode JSON response: {err}")),
    }
}

fn problem(reason_status: u16, reason: impl Into<String>) -> HttpResponse {
    let reason = reason.into();
    let body = serde_json::to_vec(&json!({
        "ok": false,
        "error": reason
    }))
    .unwrap_or_else(|_| b"{\"ok\":false,\"error\":\"response encode failed\"}".to_vec());

    HttpResponse {
        status: reason_status,
        content_type: "application/json",
        body,
    }
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.clone())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

/// Validate canonical RustyOnions BLAKE3 CID shape.
#[must_use]
pub fn is_b3_cid(value: &str) -> bool {
    value.len() == 67
        && value.starts_with("b3:")
        && value.as_bytes()[3..]
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn validate_ingest_string(name: &str, value: &str) -> Result<()> {
    let value = value.trim();

    if value.is_empty() {
        return Err(crate::Error::schema(format!("{name} must not be empty")));
    }

    if value.len() > 160 {
        return Err(crate::Error::schema(format!("{name} exceeds 160 bytes")));
    }

    Ok(())
}
