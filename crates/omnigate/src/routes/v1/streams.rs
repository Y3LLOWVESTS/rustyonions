//! RO:WHAT — Stream-lite backend session and latest-segment routes.
//! RO:WHY — Lets CrabLink prove paid stream viewing after content_view payment without pretending full live streaming exists.
//! RO:INTERACTS — CrabLink Tauri stream page, svc-gateway proxy, svc-wallet receipt lookup, content_view receipts.
//! RO:INVARIANTS — in-memory stream session only; latest bounded segment only; viewer media requires wallet receipt lookup; no wallet mutation here.
//! RO:METRICS — route middleware captures HTTP metrics/correlation; route body includes safe status and warnings.
//! RO:CONFIG — OMNIGATE_WALLET_BASE_URL, OMNIGATE_WALLET_BEARER.
//! RO:SECURITY — fail-closed receipt validation; no stream keys; no ingest secrets; bounded body; no arbitrary execution.
//! RO:TEST — cargo test -p omnigate --test streams.

use axum::{
    body::Bytes,
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    env,
    sync::RwLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";
const MAX_STREAM_ID_BYTES: usize = 96;
const MAX_ACCOUNT_BYTES: usize = 160;
const MAX_TITLE_BYTES: usize = 240;
const MAX_SEGMENT_BODY_BYTES: usize = 768 * 1024;
const MAX_DATA_URL_BYTES: usize = 640 * 1024;
const MAX_TEXT_SEGMENT_BYTES: usize = 64 * 1024;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate streams route reqwest client should build")
});

static STREAMS: Lazy<RwLock<HashMap<String, StreamSession>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Router for `/v1/streams/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/:stream_id/start", post(stream_start))
        .route("/:stream_id/stop", post(stream_stop))
        .route("/:stream_id/status", get(stream_status))
        .route("/:stream_id/segments", post(stream_segment_put))
        .route("/:stream_id/segments/latest", post(stream_segment_latest))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct StreamSession {
    schema: &'static str,
    stream_id: String,
    status: String,
    asset_crab_url: String,
    asset_cid: String,
    manifest_cid: Option<String>,
    title: String,
    creator_account: Option<String>,
    creator_passport: Option<String>,
    started_at_ms: u64,
    stopped_at_ms: Option<u64>,
    latest_seq: u64,
    latest_segment: Option<StreamSegment>,
    delivery: StreamDelivery,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct StreamDelivery {
    mode: &'static str,
    backend_live: bool,
    transport: &'static str,
    note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct StreamSegment {
    schema: &'static str,
    stream_id: String,
    seq: u64,
    media_type: String,
    data_url: Option<String>,
    text: Option<String>,
    created_at_ms: u64,
    producer_account: Option<String>,
    source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamStartRequest {
    #[serde(default, alias = "streamId")]
    stream_id: Option<String>,
    #[serde(default, alias = "assetCrabUrl", alias = "crab_url")]
    asset_crab_url: Option<String>,
    #[serde(default, alias = "assetCid", alias = "asset_cid")]
    asset_cid: Option<String>,
    #[serde(default, alias = "manifestCid", alias = "manifest_cid")]
    manifest_cid: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, alias = "creatorAccount", alias = "creator_wallet_account")]
    creator_account: Option<String>,
    #[serde(default, alias = "creatorPassport", alias = "creator_passport_subject")]
    creator_passport: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamStopRequest {
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamSegmentRequest {
    #[serde(default, alias = "assetCrabUrl", alias = "crab_url")]
    asset_crab_url: Option<String>,
    #[serde(default, alias = "mediaType", alias = "content_type")]
    media_type: Option<String>,
    #[serde(default, alias = "dataUrl")]
    data_url: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LatestSegmentRequest {
    #[serde(default, alias = "assetCrabUrl", alias = "crab_url")]
    asset_crab_url: Option<String>,
    #[serde(default, alias = "payerAccount", alias = "viewerWalletAccount")]
    payer_account: Option<String>,
    #[serde(default, alias = "recipientAccount")]
    recipient_account: Option<String>,
    #[serde(default)]
    txid: Option<String>,
    #[serde(default, alias = "receiptHash", alias = "wallet_receipt_hash")]
    receipt_hash: Option<String>,
    #[serde(default, alias = "amountMinor")]
    amount_minor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WalletReceipt {
    txid: String,
    op: String,
    from: Option<String>,
    to: Option<String>,
    asset: String,
    amount_minor: String,
    nonce: Option<u64>,
    idem: Option<String>,
    ledger_root: Option<String>,
    receipt_hash: String,
}

#[derive(Debug, Serialize)]
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}

pub async fn stream_start(
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if body.len() > MAX_SEGMENT_BODY_BYTES {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "stream_start_too_large",
            "stream start request is too large",
            false,
            "body_too_large",
        );
    }

    let request = match serde_json::from_slice::<StreamStartRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_start_request",
                "stream start request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let stream_id = match normalize_stream_id(request.stream_id.as_deref().unwrap_or(&stream_id)) {
        Some(value) => value,
        None => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_id",
                "stream id must be a safe CrabLink stream identifier",
                false,
                "invalid_stream_id",
            );
        }
    };

    let asset_crab_url = match clean_required(request.asset_crab_url.as_deref(), "asset_crab_url") {
        Ok(value) if is_stream_crab_url(&value) => value,
        _ => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_asset",
                "stream start requires crab://<64hex>.stream",
                false,
                "invalid_asset_crab_url",
            );
        }
    };

    let asset_cid = request
        .asset_cid
        .as_deref()
        .and_then(normalize_b3_cid)
        .or_else(|| asset_hash_from_crab_url(&asset_crab_url).map(|hash| format!("b3:{hash}")))
        .unwrap_or_default();

    let creator_account = clean_optional(request.creator_account.as_deref())
        .or_else(|| grab(&headers, "x-ron-wallet-account"));
    let creator_passport = clean_optional(request.creator_passport.as_deref())
        .or_else(|| grab(&headers, "x-ron-passport"));

    let title = clean_optional(request.title.as_deref())
        .unwrap_or_else(|| format!("Stream {}", short_id(&stream_id)));

    if title.len() > MAX_TITLE_BYTES {
        return problem(
            StatusCode::BAD_REQUEST,
            "stream_title_too_long",
            "stream title is too long",
            false,
            "title_too_long",
        );
    }

    let now = now_ms();
    let session = StreamSession {
        schema: "omnigate.stream-session.v1",
        stream_id: stream_id.clone(),
        status: "live".to_owned(),
        asset_crab_url,
        asset_cid,
        manifest_cid: request.manifest_cid.as_deref().and_then(normalize_b3_cid),
        title,
        creator_account,
        creator_passport,
        started_at_ms: now,
        stopped_at_ms: None,
        latest_seq: 0,
        latest_segment: None,
        delivery: StreamDelivery {
            mode: "stream_lite_latest_segment",
            backend_live: true,
            transport: "bounded_latest_data_url",
            note: "stream-lite v1 stores only the latest bounded segment in Omnigate memory",
        },
    };

    {
        let Ok(mut guard) = STREAMS.write() else {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_state_unavailable",
                "stream session state is unavailable",
                true,
                "state_lock",
            );
        };

        guard.insert(stream_id.clone(), session.clone());
    }

    (
        StatusCode::OK,
        Json(json!({
            "schema": "omnigate.stream-start.v1",
            "ok": true,
            "session": session,
            "truth_boundary": "This is a bounded in-memory stream-lite session. It does not replace durable stream manifests, storage, wallet, or ledger truth."
        })),
    )
        .into_response()
}

pub async fn stream_stop(Path(stream_id): Path<String>, body: Bytes) -> Response {
    let request = if body.is_empty() {
        StreamStopRequest { reason: None }
    } else {
        match serde_json::from_slice::<StreamStopRequest>(&body) {
            Ok(request) => request,
            Err(_) => {
                return problem(
                    StatusCode::BAD_REQUEST,
                    "invalid_stream_stop_request",
                    "stream stop request must be strict JSON",
                    false,
                    "bad_json",
                );
            }
        }
    };

    let Some(stream_id) = normalize_stream_id(&stream_id) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_id",
            "stream id must be a safe CrabLink stream identifier",
            false,
            "invalid_stream_id",
        );
    };

    let mut stopped = None;

    {
        let Ok(mut guard) = STREAMS.write() else {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_state_unavailable",
                "stream session state is unavailable",
                true,
                "state_lock",
            );
        };

        if let Some(session) = guard.get_mut(&stream_id) {
            session.status = "stopped".to_owned();
            session.stopped_at_ms = Some(now_ms());
            stopped = Some(session.clone());
        }
    }

    match stopped {
        Some(session) => (
            StatusCode::OK,
            Json(json!({
                "schema": "omnigate.stream-stop.v1",
                "ok": true,
                "reason": clean_optional(request.reason.as_deref()).unwrap_or_else(|| "stopped".to_owned()),
                "session": session
            })),
        )
            .into_response(),
        None => problem(
            StatusCode::NOT_FOUND,
            "stream_session_not_found",
            "stream session was not found",
            false,
            "stream_not_found",
        ),
    }
}

pub async fn stream_status(Path(stream_id): Path<String>) -> Response {
    let Some(stream_id) = normalize_stream_id(&stream_id) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_id",
            "stream id must be a safe CrabLink stream identifier",
            false,
            "invalid_stream_id",
        );
    };

    let session = {
        let Ok(guard) = STREAMS.read() else {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_state_unavailable",
                "stream session state is unavailable",
                true,
                "state_lock",
            );
        };

        guard.get(&stream_id).cloned()
    };

    match session {
        Some(session) => (
            StatusCode::OK,
            Json(json!({
                "schema": "omnigate.stream-status.v1",
                "ok": true,
                "session": session
            })),
        )
            .into_response(),
        None => problem(
            StatusCode::NOT_FOUND,
            "stream_session_not_found",
            "stream session was not found",
            false,
            "stream_not_found",
        ),
    }
}

pub async fn stream_segment_put(
    Path(stream_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if body.len() > MAX_SEGMENT_BODY_BYTES {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "stream_segment_too_large",
            "stream segment request is too large",
            false,
            "body_too_large",
        );
    }

    let request = match serde_json::from_slice::<StreamSegmentRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_segment_request",
                "stream segment request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let Some(stream_id) = normalize_stream_id(&stream_id) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_id",
            "stream id must be a safe CrabLink stream identifier",
            false,
            "invalid_stream_id",
        );
    };

    let media_type = clean_optional(request.media_type.as_deref())
        .unwrap_or_else(|| "image/jpeg".to_owned())
        .to_ascii_lowercase();

    if !matches!(
        media_type.as_str(),
        "image/jpeg" | "image/png" | "image/webp" | "text/plain" | "application/json"
    ) {
        return problem(
            StatusCode::BAD_REQUEST,
            "stream_segment_media_type_rejected",
            "stream segment media type is not supported by stream-lite v1",
            false,
            "unsupported_media_type",
        );
    }

    let data_url = clean_optional(request.data_url.as_deref());
    let text = clean_optional(request.text.as_deref());

    if data_url.is_none() && text.is_none() {
        return problem(
            StatusCode::BAD_REQUEST,
            "stream_segment_empty",
            "stream segment requires data_url or text",
            false,
            "empty_segment",
        );
    }

    if let Some(data_url) = &data_url {
        if data_url.len() > MAX_DATA_URL_BYTES || !data_url.starts_with("data:") {
            return problem(
                StatusCode::BAD_REQUEST,
                "stream_segment_data_url_rejected",
                "stream segment data_url is too large or invalid",
                false,
                "invalid_data_url",
            );
        }
    }

    if let Some(text) = &text {
        if text.len() > MAX_TEXT_SEGMENT_BYTES {
            return problem(
                StatusCode::BAD_REQUEST,
                "stream_segment_text_too_large",
                "stream text segment is too large",
                false,
                "text_too_large",
            );
        }
    }

    let producer_account = grab(&headers, "x-ron-wallet-account");

    let updated = {
        let Ok(mut guard) = STREAMS.write() else {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_state_unavailable",
                "stream session state is unavailable",
                true,
                "state_lock",
            );
        };

        let Some(session) = guard.get_mut(&stream_id) else {
            return problem(
                StatusCode::NOT_FOUND,
                "stream_session_not_found",
                "stream session was not found",
                false,
                "stream_not_found",
            );
        };

        if session.status != "live" {
            return problem(
                StatusCode::CONFLICT,
                "stream_session_not_live",
                "stream session is not live",
                false,
                "stream_not_live",
            );
        }

        if let (Some(expected), Some(actual)) = (&session.creator_account, &producer_account) {
            if expected != actual {
                return problem(
                    StatusCode::FORBIDDEN,
                    "stream_segment_creator_mismatch",
                    "stream segment producer does not match stream creator account",
                    false,
                    "creator_mismatch",
                );
            }
        }

        if let Some(request_url) = clean_optional(request.asset_crab_url.as_deref()) {
            if request_url != session.asset_crab_url {
                return problem(
                    StatusCode::BAD_REQUEST,
                    "stream_segment_asset_mismatch",
                    "stream segment asset URL does not match the live stream session",
                    false,
                    "asset_mismatch",
                );
            }
        }

        let seq = session.latest_seq.saturating_add(1);
        let segment = StreamSegment {
            schema: "omnigate.stream-segment.v1",
            stream_id: stream_id.clone(),
            seq,
            media_type,
            data_url,
            text,
            created_at_ms: now_ms(),
            producer_account,
            source: clean_optional(request.source.as_deref())
                .unwrap_or_else(|| "crablink_tauri_creator_snapshot".to_owned()),
        };

        session.latest_seq = seq;
        session.latest_segment = Some(segment.clone());

        (session.clone(), segment)
    };

    (
        StatusCode::OK,
        Json(json!({
            "schema": "omnigate.stream-segment-put.v1",
            "ok": true,
            "session": updated.0,
            "segment": updated.1,
            "truth_boundary": "Segment is a bounded stream-lite latest frame held in Omnigate memory; it is not durable b3 media truth."
        })),
    )
        .into_response()
}

pub async fn stream_segment_latest(Path(stream_id): Path<String>, body: Bytes) -> Response {
    if body.len() > 64 * 1024 {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "stream_access_request_too_large",
            "stream access request is too large",
            false,
            "body_too_large",
        );
    }

    let request = match serde_json::from_slice::<LatestSegmentRequest>(&body) {
        Ok(request) => request,
        Err(_) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_stream_access_request",
                "latest stream segment request must be strict JSON",
                false,
                "bad_json",
            );
        }
    };

    let Some(stream_id) = normalize_stream_id(&stream_id) else {
        return problem(
            StatusCode::BAD_REQUEST,
            "invalid_stream_id",
            "stream id must be a safe CrabLink stream identifier",
            false,
            "invalid_stream_id",
        );
    };

    let (session, latest) = {
        let Ok(guard) = STREAMS.read() else {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "stream_state_unavailable",
                "stream session state is unavailable",
                true,
                "state_lock",
            );
        };

        let Some(session) = guard.get(&stream_id).cloned() else {
            return problem(
                StatusCode::NOT_FOUND,
                "stream_session_not_found",
                "stream session was not found",
                false,
                "stream_not_found",
            );
        };

        let Some(latest) = session.latest_segment.clone() else {
            return problem(
                StatusCode::NOT_FOUND,
                "stream_segment_not_found",
                "no backend stream segment has been published yet",
                true,
                "segment_not_found",
            );
        };

        (session, latest)
    };

    if session.status != "live" {
        return problem(
            StatusCode::CONFLICT,
            "stream_session_not_live",
            "stream session is not live",
            false,
            "stream_not_live",
        );
    }

    if let Some(request_url) = clean_optional(request.asset_crab_url.as_deref()) {
        if request_url != session.asset_crab_url {
            return problem(
                StatusCode::BAD_REQUEST,
                "stream_access_asset_mismatch",
                "stream access asset URL does not match the live stream session",
                false,
                "asset_mismatch",
            );
        }
    }

    let payer_account = match clean_optional(request.payer_account.as_deref()) {
        Some(value) => value,
        None => {
            return problem(
                StatusCode::PAYMENT_REQUIRED,
                "stream_access_missing_payer",
                "stream access requires payer account",
                false,
                "missing_payer",
            );
        }
    };

    let receipt_hash = match clean_optional(request.receipt_hash.as_deref()) {
        Some(value) if is_b3_hash(&value) || is_canonical_b3_cid(&value) => value,
        _ => {
            return problem(
                StatusCode::PAYMENT_REQUIRED,
                "stream_access_missing_receipt_hash",
                "stream access requires wallet receipt hash",
                false,
                "missing_receipt_hash",
            );
        }
    };

    let txid = match clean_optional(request.txid.as_deref()) {
        Some(value) if is_safe_txid(&value) => value,
        _ => {
            return problem(
                StatusCode::PAYMENT_REQUIRED,
                "stream_access_missing_txid",
                "stream access requires wallet txid",
                false,
                "missing_txid",
            );
        }
    };

    let expected_recipient = request
        .recipient_account
        .as_deref()
        .and_then(|value| clean_optional(Some(value)))
        .or_else(|| session.creator_account.clone());

    let Some(expected_recipient) = expected_recipient else {
        return problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_missing_recipient",
            "stream access cannot verify receipt without a creator recipient account",
            false,
            "missing_recipient",
        );
    };

    let receipt = match fetch_wallet_receipt(&txid).await {
        Ok(receipt) => receipt,
        Err(response) => return response,
    };

    if let Err(response) = validate_content_view_receipt(
        &receipt,
        &session,
        &payer_account,
        &expected_recipient,
        &receipt_hash,
        request.amount_minor.as_deref(),
    ) {
        return response;
    }

    (
        StatusCode::OK,
        Json(json!({
            "schema": "omnigate.stream-latest-segment.v1",
            "ok": true,
            "stream_id": &stream_id,
            "asset_crab_url": &session.asset_crab_url,
            "session": session,
            "segment": latest,
            "access": {
                "schema": "omnigate.stream-access-proof.v1",
                "status": "receipt_verified",
                "payer_account": payer_account,
                "recipient_account": expected_recipient,
                "txid": receipt.txid,
                "receipt_hash": receipt.receipt_hash,
                "ledger_root": receipt.ledger_root,
                "nonce": receipt.nonce,
                "idempotency_key": receipt.idem,
                "wallet_front_door": "svc-wallet /v1/tx/{txid}"
            },
            "truth_boundary": "Viewer media is returned only after Omnigate looked up the wallet transfer receipt and checked the stream-bound content_view idempotency key."
        })),
    )
        .into_response()
}

async fn fetch_wallet_receipt(txid: &str) -> Result<WalletReceipt, Response> {
    let url = format!("{}/v1/tx/{txid}", wallet_base_url());

    let upstream_res = match HTTP_CLIENT
        .get(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_receipt_unavailable",
                "svc-wallet receipt lookup is unavailable",
                true,
                "wallet_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "wallet_receipt_read_failed",
                "failed to read svc-wallet receipt response",
                true,
                "wallet_read",
            ));
        }
    };

    if !status.is_success() {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_receipt_lookup_rejected",
            "svc-wallet did not return the stream access receipt",
            status.as_u16() >= 500,
            "receipt_lookup_rejected",
        ));
    }

    serde_json::from_slice::<WalletReceipt>(&body).map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "wallet_receipt_bad_json",
            "svc-wallet receipt response was not valid JSON",
            true,
            "wallet_bad_json",
        )
    })
}

fn validate_content_view_receipt(
    receipt: &WalletReceipt,
    session: &StreamSession,
    payer_account: &str,
    expected_recipient: &str,
    expected_receipt_hash: &str,
    expected_amount_minor: Option<&str>,
) -> Result<(), Response> {
    if receipt.op != "transfer" {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_wrong_receipt_op",
            "stream access requires a wallet transfer receipt",
            false,
            "wrong_receipt_op",
        ));
    }

    if receipt.asset != "roc" {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_wrong_receipt_asset",
            "stream access receipt asset must be roc",
            false,
            "wrong_receipt_asset",
        ));
    }

    if receipt.from.as_deref() != Some(payer_account) {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_payer_mismatch",
            "stream access receipt payer mismatch",
            false,
            "payer_mismatch",
        ));
    }

    if receipt.to.as_deref() != Some(expected_recipient) {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_recipient_mismatch",
            "stream access receipt recipient mismatch",
            false,
            "recipient_mismatch",
        ));
    }

    if normalize_receipt_hash(&receipt.receipt_hash)
        != normalize_receipt_hash(expected_receipt_hash)
    {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_receipt_hash_mismatch",
            "stream access receipt hash mismatch",
            false,
            "receipt_hash_mismatch",
        ));
    }

    if let Some(expected_amount) = clean_optional(expected_amount_minor) {
        if receipt.amount_minor != expected_amount {
            return Err(problem(
                StatusCode::PAYMENT_REQUIRED,
                "stream_access_amount_mismatch",
                "stream access receipt amount mismatch",
                false,
                "amount_mismatch",
            ));
        }
    }

    let expected_idem =
        expected_content_view_idempotency_key(&session.asset_crab_url, payer_account);

    if receipt.idem.as_deref() != Some(expected_idem.as_str()) {
        return Err(problem(
            StatusCode::PAYMENT_REQUIRED,
            "stream_access_receipt_not_bound",
            "stream access receipt is not bound to this stream asset",
            false,
            "receipt_not_bound_to_stream",
        ));
    }

    Ok(())
}

fn expected_content_view_idempotency_key(asset_crab_url: &str, payer_account: &str) -> String {
    let hash = asset_hash_from_crab_url(asset_crab_url).unwrap_or_default();
    let short_hash = hash.get(0..16).unwrap_or("unknownstream");
    format!("cl-view-pay:{short_hash}:{}", fnv1a_hex(payer_account))
}

fn fnv1a_hex(value: &str) -> String {
    let mut hash = 0x811c9dc5u32;

    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }

    format!("{hash:08x}")
}

fn problem(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
) -> Response {
    (
        status,
        Json(Problem {
            code,
            message,
            retryable,
            reason,
        }),
    )
        .into_response()
}

fn wallet_base_url() -> String {
    env::var("OMNIGATE_WALLET_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_WALLET_BASE_URL"))
        .unwrap_or_else(|_| DEFAULT_WALLET_BASE_URL.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn wallet_bearer() -> String {
    env::var("OMNIGATE_WALLET_BEARER").unwrap_or_else(|_| DEFAULT_WALLET_BEARER.to_owned())
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| clean_optional(Some(value)))
}

fn clean_required(value: Option<&str>, field: &'static str) -> Result<String, Response> {
    clean_optional(value).ok_or_else(|| {
        problem(
            StatusCode::BAD_REQUEST,
            "missing_required_stream_field",
            match field {
                "asset_crab_url" => "asset_crab_url is required",
                _ => "required stream field is missing",
            },
            false,
            "missing_required_field",
        )
    })
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_stream_id(value: &str) -> Option<String> {
    let clean = value.trim();

    if clean.is_empty() || clean.len() > MAX_STREAM_ID_BYTES {
        return None;
    }

    if clean
        .bytes()
        .all(|byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-'))
    {
        Some(clean.to_owned())
    } else {
        None
    }
}

fn normalize_b3_cid(value: &str) -> Option<String> {
    let clean = value.trim().to_ascii_lowercase();

    if is_canonical_b3_cid(&clean) {
        Some(clean)
    } else if is_b3_hash(&clean) {
        Some(format!("b3:{clean}"))
    } else {
        None
    }
}

fn normalize_receipt_hash(value: &str) -> String {
    let clean = value.trim().to_ascii_lowercase();

    if is_b3_hash(&clean) {
        format!("b3:{clean}")
    } else {
        clean
    }
}

fn is_canonical_b3_cid(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("b3:") else {
        return false;
    };

    is_b3_hash(hash)
}

fn is_b3_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_stream_crab_url(value: &str) -> bool {
    asset_hash_from_crab_url(value).is_some() && value.ends_with(".stream")
}

fn asset_hash_from_crab_url(value: &str) -> Option<String> {
    let clean = value.trim().to_ascii_lowercase();
    let rest = clean.strip_prefix("crab://")?;
    let hash = rest.strip_suffix(".stream")?;

    if is_b3_hash(hash) {
        Some(hash.to_owned())
    } else {
        None
    }
}

fn is_safe_txid(value: &str) -> bool {
    let clean = value.trim();

    !clean.is_empty()
        && clean.len() <= 96
        && clean
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-'))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn short_id(value: &str) -> String {
    value.chars().take(12).collect()
}
