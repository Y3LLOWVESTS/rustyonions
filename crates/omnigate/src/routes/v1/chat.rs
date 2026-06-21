//! RO:QUICKCHAIN-PREFLIGHT — paid send via svc-wallet; paid send uses svc-wallet only; paid message appears only after backend wallet success; cache never unlocks paid chat; no direct ledger mutation.
//! RO:WHAT — CrabLink chat route proof with durable b3 room descriptors, in-memory live messages, and paid send via svc-wallet.
//! RO:WHY — Gives CrabLink Tauri canonical `crab://<b3hash>.chat` room links while staying honest that live chat fanout is not durable yet.
//! RO:INTERACTS — svc-gateway `/chat/*` proxy routes, svc-storage `/o`, svc-wallet `/v1/transfer`, CrabLink ChatPage, future svc-mailbox.
//! RO:INVARIANTS — descriptor storage is b3-addressed; live messages remain in-memory; paid send uses svc-wallet only; no direct ledger mutation.
//! RO:METRICS — inherits omnigate HTTP middleware/correlation; downstream storage/wallet emit their own service metrics.
//! RO:CONFIG — OMNIGATE_STORAGE_BASE_URL, OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL, OMNIGATE_WALLET_BASE_URL, OMNIGATE_WALLET_BEARER, OMNIGATE_CHAT_MESSAGE_NONCE.
//! RO:SECURITY — text-only messages; bounded bodies; paid message appears only after backend wallet success; cache never unlocks paid chat.
//! RO:TEST — cargo test -p omnigate --test chat_routes.

use axum::{
    body::Bytes,
    extract::{Path, Query},
    http::{header, HeaderMap, HeaderName, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CHAT_SCHEMA_ROOM_PAGE: &str = "omnigate.chat-room-page.v1";
const CHAT_SCHEMA_PREPARE: &str = "omnigate.chat-room-prepare.v1";
const CHAT_SCHEMA_CREATE: &str = "omnigate.chat-room-create-result.v1";
const CHAT_SCHEMA_QUOTE: &str = "omnigate.chat-message-quote.v1";
const CHAT_SCHEMA_SEND: &str = "omnigate.chat-message-send-result.v1";
const CHAT_SCHEMA_LIST: &str = "omnigate.chat-message-list.v1";
const CHAT_SCHEMA_MOD: &str = "omnigate.chat-moderation-result.v1";

const CHAT_DESCRIPTOR_SCHEMA: &str = "crablink.chat-room-descriptor.v1";
const CHAT_ROOM_SCHEMA: &str = "crablink.chat-room.v1";
const CHAT_MESSAGE_SCHEMA: &str = "crablink.chat-message.v1";

const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_ROOM_TITLE_CHARS: usize = 96;
const MAX_DESCRIPTION_CHARS: usize = 420;
const MAX_MESSAGES_PER_ROOM: usize = 500;
const MAX_DESCRIPTOR_BYTES: usize = 64 * 1024;

const DEFAULT_STORAGE_BASE_URL: &str = "http://127.0.0.1:5303";
const DEFAULT_WALLET_BASE_URL: &str = "http://127.0.0.1:8088";
const DEFAULT_WALLET_BEARER: &str = "dev";
const DEFAULT_ASSET: &str = "roc";
const DEFAULT_WALLET_NONCE: u64 = 1;

static CHAT_STORE: OnceLock<Mutex<ChatStore>> = OnceLock::new();

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .use_rustls_tls()
        .build()
        .expect("omnigate chat route reqwest client should build")
});

/// Router for `/v1/chat/*`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/resolve", get(resolve_room))
        .route("/prepare", post(prepare_room))
        .route("/", post(create_room))
        .route("/:room_id/messages", get(list_messages))
        .route("/:room_id/messages/latest", get(latest_messages))
        .route("/:room_id/messages/quote", post(quote_message))
        .route("/:room_id/messages/send", post(send_message))
        .route("/:room_id/mod/delete", post(mod_delete_message))
        .route("/:room_id/mod/block", post(mod_block_username))
        .route("/:room_id/mod/pin", post(mod_pin_message))
}

#[derive(Debug, Default)]
struct ChatStore {
    rooms: HashMap<String, ChatRoom>,
}

#[derive(Debug)]
struct UpstreamBody {
    status: StatusCode,
    body: Bytes,
}

#[derive(Debug)]
struct StoredDescriptor {
    cid: String,
    storage_path: String,
    storage_response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoom {
    pub room_id: String,
    pub room_url: String,
    pub schema: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub owner_passport: String,
    pub owner_account: String,
    pub owner_display: String,
    pub attached_to: Vec<String>,
    pub access: ChatAccess,
    pub payout: ChatPayout,
    pub expiry: ChatExpiry,
    pub moderation: ChatModeration,
    pub pinned_note: String,
    pub created_at: String,
    pub backend: ChatBackendTruth,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatAccess {
    pub read_mode: String,
    pub send_mode: String,
    pub message_price_roc: u64,
    pub currency: String,
    pub explicit_confirm_required: bool,
}

impl Default for ChatAccess {
    fn default() -> Self {
        Self {
            read_mode: "public".to_owned(),
            send_mode: "paid_per_message".to_owned(),
            message_price_roc: 1,
            currency: "ROC".to_owned(),
            explicit_confirm_required: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPayout {
    pub creator_share_bps: u16,
    pub platform_share_bps: u16,
    pub moderator_pool_bps: u16,
    pub total_bps: u16,
    pub valid_total: bool,
    pub backend_confirmed: bool,
}

impl Default for ChatPayout {
    fn default() -> Self {
        Self {
            creator_share_bps: 9000,
            platform_share_bps: 1000,
            moderator_pool_bps: 0,
            total_bps: 10000,
            valid_total: true,
            backend_confirmed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatExpiry {
    pub mode: String,
    pub expires_at: Option<String>,
    pub archive_mode: String,
    pub backend_confirmed: bool,
}

impl Default for ChatExpiry {
    fn default() -> Self {
        Self {
            mode: "never_expires".to_owned(),
            expires_at: None,
            archive_mode: "read_only_after_expiry".to_owned(),
            backend_confirmed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModeration {
    pub mode: String,
    pub mods: Vec<String>,
    pub blocked_usernames: Vec<String>,
    pub blocked_terms: Vec<String>,
    pub allow_emoji: bool,
    pub allow_reactions: bool,
    pub max_message_chars: usize,
    pub slow_mode_seconds: u64,
    pub backend_confirmed: bool,
}

impl Default for ChatModeration {
    fn default() -> Self {
        Self {
            mode: "moderated".to_owned(),
            mods: Vec::new(),
            blocked_usernames: Vec::new(),
            blocked_terms: Vec::new(),
            allow_emoji: true,
            allow_reactions: true,
            max_message_chars: 500,
            slow_mode_seconds: 0,
            backend_confirmed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatBackendTruth {
    pub storage: String,
    pub durable: bool,
    pub assigns_b3_cid: bool,
    pub writes_index_pointer: bool,
    pub uses_wallet: bool,
    pub creates_receipts: bool,
    pub fanout: String,
    pub warning: String,
    pub descriptor_cid: Option<String>,
    pub canonical_room_url: Option<String>,
    pub storage_path: Option<String>,
}

impl Default for ChatBackendTruth {
    fn default() -> Self {
        Self {
            storage: "omnigate_in_memory_dev_only".to_owned(),
            durable: false,
            assigns_b3_cid: false,
            writes_index_pointer: false,
            uses_wallet: true,
            creates_receipts: true,
            fanout: "poll_latest_only".to_owned(),
            warning: "Chat live messages are in-memory dev proof. Descriptor may be durable only after create returns a b3 CID.".to_owned(),
            descriptor_cid: None,
            canonical_room_url: None,
            storage_path: None,
        }
    }
}

impl ChatBackendTruth {
    fn descriptor_backed(cid: &str) -> Self {
        let raw = cid.trim_start_matches("b3:");
        Self {
            storage: "svc_storage_chat_descriptor".to_owned(),
            durable: true,
            assigns_b3_cid: true,
            writes_index_pointer: false,
            uses_wallet: true,
            creates_receipts: true,
            fanout: "poll_latest_only".to_owned(),
            warning: "Chat room descriptor is b3-addressed. Live messages remain in omnigate memory until svc-mailbox/event-log fanout is wired.".to_owned(),
            descriptor_cid: Some(cid.to_owned()),
            canonical_room_url: Some(format!("crab://{raw}.chat")),
            storage_path: Some(format!("/o/{cid}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub schema: String,
    pub message_id: String,
    pub room: String,
    pub room_id: String,
    pub sender_passport: String,
    pub sender_display: String,
    pub body: String,
    pub emoji_only: bool,
    pub created_at: String,
    pub backend_confirmed: bool,
    pub paid: ChatPaidProof,
    pub moderation: ChatMessageModeration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPaidProof {
    pub required: bool,
    pub amount_roc: u64,
    pub receipt_hash: Option<String>,
    pub wallet_txid: Option<String>,
    pub ledger_root: Option<String>,
    pub wallet_receipt: Option<Value>,
    pub backend_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageModeration {
    pub state: String,
    pub deleted_by: Option<String>,
    pub deleted_at: Option<String>,
    pub reason: Option<String>,
    pub backend_confirmed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveQuery {
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomEnvelope {
    #[serde(default)]
    pub descriptor: Value,
    #[serde(default)]
    pub room: Value,
    #[serde(default)]
    pub owner_passport: String,
    #[serde(default)]
    pub wallet_account: String,
    #[serde(default)]
    pub client_idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatQuoteRequest {
    #[serde(default)]
    pub sender_passport: String,
    #[serde(default)]
    pub wallet_account: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub client_nonce: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSendRequest {
    #[serde(default)]
    pub sender_passport: String,
    #[serde(default)]
    pub sender_display: String,
    #[serde(default)]
    pub wallet_account: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub quote: Value,
    #[serde(default)]
    pub paid_proof: Value,
    #[serde(default)]
    pub client_idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageListQuery {
    pub after: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestQuery {
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModerationRequest {
    #[serde(default)]
    pub moderator_passport: String,
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub client_idempotency_key: String,
}

#[derive(Debug, Serialize)]
struct Problem {
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
}

/// Resolve a chat room page by crab URL.
///
/// `crab://chat` returns the built-in preview page.
/// `crab://<64hex>.chat` resolves from the in-memory store first, then from
/// durable svc-storage descriptor bytes.
pub async fn resolve_room(headers: HeaderMap, Query(query): Query<ResolveQuery>) -> Response {
    let requested = query.url.unwrap_or_else(|| "crab://chat".to_owned());
    let room_id = room_id_from_url(&requested);

    if let Some(room) = load_room(&room_id) {
        return room_page(room).into_response();
    }

    if let Some(cid) = canonical_chat_cid_from_url(&requested) {
        let descriptor = match fetch_descriptor_from_storage(&cid, &headers).await {
            Ok(Some(descriptor)) => descriptor,
            Ok(None) => {
                return not_found("chat_descriptor_not_found", &room_id);
            }
            Err(response) => return response,
        };

        let payload = ChatRoomEnvelope {
            descriptor: descriptor.clone(),
            room: Value::Null,
            owner_passport: String::new(),
            wallet_account: String::new(),
            client_idempotency_key: String::new(),
        };
        let mut room = room_from_descriptor(&descriptor, &payload);
        apply_canonical_descriptor_identity(&mut room, &cid);
        remember_room(room.clone());

        return room_page(room).into_response();
    }

    room_page(builtin_room(&requested)).into_response()
}

/// Prepare a room descriptor.
///
/// Prepare is side-effect-safe. It normalizes the descriptor and estimates the
/// bytes that will be stored later, but it does not store, mutate wallet state,
/// or create receipts.
pub async fn prepare_room(Json(payload): Json<ChatRoomEnvelope>) -> Response {
    let descriptor = descriptor_value(&payload);
    let canonical_descriptor = canonical_descriptor_value(&descriptor, &payload);
    let room_preview = room_from_descriptor(&canonical_descriptor, &payload);
    let estimated_bytes = match serde_json::to_vec(&canonical_descriptor) {
        Ok(bytes) => bytes.len(),
        Err(_) => 0,
    };

    (
        StatusCode::OK,
        Json(json!({
            "schema": CHAT_SCHEMA_PREPARE,
            "ok": true,
            "roomIdPreview": room_preview.room_id,
            "roomUrlPreview": room_preview.room_url,
            "canonicalDescriptor": canonical_descriptor,
            "estimatedDescriptorBytes": estimated_bytes,
            "pricing": {
                "asset": "ROC",
                "amountRoc": 0,
                "mode": "chat_room_descriptor_store_dev_free",
                "walletMutation": false,
                "receiptCreated": false
            },
            "storage": {
                "willStoreDescriptorOnCreate": true,
                "path": "/o",
                "contentAddressedBy": "svc-storage"
            },
            "idempotencyKey": fallback_idempotency(&payload.client_idempotency_key, "chat-prepare", &room_preview.room_id),
            "truthBoundary": truth_boundary_json(),
            "warning": "Prepare is read-only. The canonical b3 chat URL is assigned when /chat stores the descriptor in svc-storage."
        })),
    )
        .into_response()
}

/// Create a room by storing its immutable descriptor in svc-storage.
///
/// This creates a canonical `crab://<64hex>.chat` URL for the descriptor. Live
/// messages are still in-memory in this batch.
pub async fn create_room(headers: HeaderMap, Json(payload): Json<ChatRoomEnvelope>) -> Response {
    let descriptor = descriptor_value(&payload);
    let canonical_descriptor = canonical_descriptor_value(&descriptor, &payload);
    let descriptor_bytes = match serde_json::to_vec(&canonical_descriptor) {
        Ok(bytes) => bytes,
        Err(_) => {
            return problem(
                StatusCode::INTERNAL_SERVER_ERROR,
                "chat_descriptor_encode_failed",
                "failed to encode chat descriptor JSON",
                false,
                "descriptor_encode_failed",
            );
        }
    };

    if descriptor_bytes.len() > MAX_DESCRIPTOR_BYTES {
        return problem(
            StatusCode::PAYLOAD_TOO_LARGE,
            "chat_descriptor_too_large",
            "chat descriptor is too large",
            false,
            "descriptor_too_large",
        );
    }

    let stored = match store_descriptor_object(&headers, Bytes::from(descriptor_bytes)).await {
        Ok(stored) => stored,
        Err(response) => return response,
    };

    let mut room = room_from_descriptor(&canonical_descriptor, &payload);
    room.created_at = now_isoish();
    apply_canonical_descriptor_identity(&mut room, &stored.cid);
    remember_room(room.clone());

    (
        StatusCode::CREATED,
        Json(json!({
            "schema": CHAT_SCHEMA_CREATE,
            "ok": true,
            "room": room_public(&room),
            "roomId": room.room_id,
            "roomUrl": room.room_url,
            "canonicalRoomUrl": room.room_url,
            "created": true,
            "durable": true,
            "backendLevel": "b3_descriptor_plus_in_memory_messages",
            "receipt": null,
            "walletMutation": false,
            "indexPointerWritten": false,
            "b3Cid": stored.cid,
            "b3Hash": room.room_id,
            "descriptor": {
                "schema": CHAT_DESCRIPTOR_SCHEMA,
                "cid": stored.cid,
                "storagePath": stored.storage_path,
                "canonicalCrabUrl": room.room_url,
                "storageResponse": stored.storage_response
            },
            "truthBoundary": truth_boundary_json(),
            "warning": "Chat descriptor is stored as b3. Live messages still live in omnigate memory until svc-mailbox/event-log is wired."
        })),
    )
        .into_response()
}

/// List backend-confirmed messages for an in-memory room.
pub async fn list_messages(
    Path(room_id): Path<String>,
    Query(query): Query<MessageListQuery>,
) -> Response {
    let normalized_room_id = normalize_room_id(&room_id);
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let after = query.after.unwrap_or_default();

    let Some(room) = load_room(&normalized_room_id) else {
        return not_found("chat_room_not_found", &normalized_room_id);
    };

    let messages = room
        .messages
        .iter()
        .filter(|message| after.is_empty() || message.message_id > after)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(json!({
            "schema": CHAT_SCHEMA_LIST,
            "room": room.room_url,
            "roomId": room.room_id,
            "cursor": messages.last().map(|message| message.message_id.clone()),
            "messages": messages,
            "truthBoundary": {
                "backendConfirmedMessagesOnly": true,
                "durableDescriptor": room.backend.durable,
                "liveMessagesDurable": false,
                "fanout": "poll_latest_only"
            }
        })),
    )
        .into_response()
}

/// Return messages newer than a cursor for polling.
pub async fn latest_messages(
    Path(room_id): Path<String>,
    Query(query): Query<LatestQuery>,
) -> Response {
    let normalized_room_id = normalize_room_id(&room_id);
    let since = query.since.unwrap_or_default();

    let Some(room) = load_room(&normalized_room_id) else {
        return not_found("chat_room_not_found", &normalized_room_id);
    };

    let messages = room
        .messages
        .iter()
        .filter(|message| since.is_empty() || message.message_id > since)
        .take(100)
        .cloned()
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(json!({
            "schema": CHAT_SCHEMA_LIST,
            "room": room.room_url,
            "roomId": room.room_id,
            "cursor": messages.last().map(|message| message.message_id.clone()),
            "messages": messages,
            "truthBoundary": {
                "backendConfirmedMessagesOnly": true,
                "durableDescriptor": room.backend.durable,
                "liveMessagesDurable": false,
                "fanout": "poll_latest_only"
            }
        })),
    )
        .into_response()
}

/// Quote a message.
///
/// The quote is read-only. It gives the frontend an explicit amount, recipient,
/// and idempotency seed before any paid send can be confirmed.
pub async fn quote_message(
    Path(room_id): Path<String>,
    Json(payload): Json<ChatQuoteRequest>,
) -> Response {
    let normalized_room_id = normalize_room_id(&room_id);

    let Some(room) = load_room(&normalized_room_id) else {
        return not_found("chat_room_not_found", &normalized_room_id);
    };

    let inspected = bounded_message_body(&payload.body, room.moderation.max_message_chars);
    if inspected.is_empty() {
        return bad_request("empty_chat_message", "Chat message body is required.");
    }

    if room.access.send_mode == "disabled" {
        return fail_closed(
            StatusCode::FORBIDDEN,
            "chat_send_disabled",
            "This chat room does not accept messages.",
        );
    }

    let amount = if room.access.send_mode == "paid_per_message" {
        room.access.message_price_roc
    } else {
        0
    };
    let idempotency_key = fallback_idempotency(
        &payload.client_nonce,
        "chat-message",
        &format!(
            "{}:{}:{}:{}",
            room.room_id, payload.sender_passport, payload.wallet_account, inspected
        ),
    );

    (
        StatusCode::OK,
        Json(json!({
            "schema": CHAT_SCHEMA_QUOTE,
            "room": room.room_url,
            "roomId": room.room_id,
            "amountRoc": amount,
            "amount_roc": amount,
            "amountMinor": amount.to_string(),
            "amount_minor": amount.to_string(),
            "asset": "ROC",
            "recipient": room.owner_account,
            "recipientAccount": room.owner_account,
            "recipient_account": room.owner_account,
            "senderPassport": fallback_string(&payload.sender_passport, "passport:anonymous"),
            "walletAccount": fallback_string(&payload.wallet_account, "unknown"),
            "expiresAt": unix_now_plus_seconds(60),
            "idempotencyKey": idempotency_key,
            "walletMutation": false,
            "receiptCreated": false,
            "explicitConfirmationRequired": amount > 0,
            "truthBoundary": {
                "quoteOnly": true,
                "sendNotPerformed": true,
                "walletNotMutated": true,
                "receiptNotCreated": true
            }
        })),
    )
        .into_response()
}

/// Send a message.
///
/// Free rooms append directly to in-memory state. Paid rooms call svc-wallet
/// first, then append the message only after wallet success. The route does not
/// mutate ron-ledger directly; svc-wallet remains the economic front door.
pub async fn send_message(
    Path(room_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<ChatSendRequest>,
) -> Response {
    let normalized_room_id = normalize_room_id(&room_id);

    let Some(room) = load_room(&normalized_room_id) else {
        return not_found("chat_room_not_found", &normalized_room_id);
    };

    if room.access.send_mode == "disabled" {
        return fail_closed(
            StatusCode::FORBIDDEN,
            "chat_send_disabled",
            "This chat room does not accept messages.",
        );
    }

    let body = bounded_message_body(&payload.body, room.moderation.max_message_chars);
    if body.is_empty() {
        return bad_request("empty_chat_message", "Chat message body is required.");
    }

    if sender_blocked(&room, &payload.sender_display)
        || sender_blocked(&room, &payload.sender_passport)
    {
        return fail_closed(
            StatusCode::FORBIDDEN,
            "chat_sender_blocked",
            "This sender is blocked by the room policy.",
        );
    }

    if contains_blocked_term(&room, &body) {
        return fail_closed(
            StatusCode::FORBIDDEN,
            "chat_message_blocked_term",
            "This message contains a blocked term.",
        );
    }

    let idempotency_key = fallback_idempotency(
        &payload.client_idempotency_key,
        "msg",
        &format!("{}:{}:{}", room.room_id, payload.sender_passport, body),
    );

    if let Some(existing) = existing_message(&room.room_id, &idempotency_key) {
        return chat_message_response(
            StatusCode::OK,
            existing,
            None,
            false,
            false,
            true,
            "Message already accepted for this idempotency key.",
        );
    }

    if room.access.send_mode == "paid_per_message" {
        return send_paid_message(headers, room, payload, body, idempotency_key).await;
    }

    let message = build_chat_message(
        &room,
        &payload,
        body,
        idempotency_key,
        ChatPaidProof {
            required: false,
            amount_roc: 0,
            receipt_hash: None,
            wallet_txid: None,
            ledger_root: None,
            wallet_receipt: None,
            backend_confirmed: true,
        },
    );

    append_message_and_respond(
        room.room_id.clone(),
        message,
        None,
        false,
        false,
        false,
        "Free backend message accepted. No ROC was spent and no receipt was created.",
    )
}

async fn send_paid_message(
    headers: HeaderMap,
    room: ChatRoom,
    payload: ChatSendRequest,
    body: String,
    idempotency_key: String,
) -> Response {
    let amount_roc = room.access.message_price_roc;

    if amount_roc == 0 {
        return bad_request(
            "paid_chat_zero_amount",
            "Paid chat room price must be greater than zero.",
        );
    }

    let payer_account = clean_optional(payload.wallet_account.as_str())
        .or_else(|| grab(&headers, "x-ron-wallet-account"));
    let Some(payer_account) = payer_account else {
        return fail_closed(
            StatusCode::PAYMENT_REQUIRED,
            "paid_chat_wallet_account_required",
            "Paid chat send requires a payer wallet account.",
        );
    };

    if payer_account == room.owner_account {
        return fail_closed(
            StatusCode::CONFLICT,
            "paid_chat_self_payment_rejected",
            "Creator self-payment for paid chat is rejected in the dev proof.",
        );
    }

    if payload.quote.is_null() || !payload.quote.is_object() {
        return fail_closed(
            StatusCode::PAYMENT_REQUIRED,
            "paid_chat_quote_required",
            "Paid chat send requires a quote object from /messages/quote before confirmation.",
        );
    }

    if quote_amount_roc(&payload.quote) != Some(amount_roc) {
        return fail_closed(
            StatusCode::CONFLICT,
            "paid_chat_quote_amount_mismatch",
            "Paid chat quote amount does not match current room price.",
        );
    }

    if let Some(recipient) = quote_recipient_account(&payload.quote) {
        if recipient != room.owner_account {
            return fail_closed(
                StatusCode::CONFLICT,
                "paid_chat_quote_recipient_mismatch",
                "Paid chat quote recipient does not match current room owner account.",
            );
        }
    }

    let amount_minor = amount_roc.to_string();
    let nonce = default_chat_message_nonce();
    let wallet_response = match send_wallet_chat_transfer(ChatTransferRequest {
        headers: &headers,
        payer_account: &payer_account,
        recipient_account: &room.owner_account,
        amount_minor: &amount_minor,
        nonce,
        idempotency_key: &idempotency_key,
        room: &room,
        body: &body,
    })
    .await
    {
        Ok(response) => response,
        Err(response) => return response,
    };

    let wallet_status = wallet_response.status;
    let wallet_body = wallet_response.body;
    let wallet_json = serde_json::from_slice::<Value>(&wallet_body).ok();

    let (wallet_json, paid_nonce) = if wallet_status.is_success() {
        (wallet_json, nonce)
    } else if wallet_status == StatusCode::CONFLICT {
        let expected_nonce = wallet_json
            .as_ref()
            .and_then(|value| value_string(value, "message"))
            .and_then(|message| parse_expected_nonce(&message));

        if let Some(expected_nonce) = expected_nonce.filter(|expected| *expected != nonce) {
            match send_wallet_chat_transfer(ChatTransferRequest {
                headers: &headers,
                payer_account: &payer_account,
                recipient_account: &room.owner_account,
                amount_minor: &amount_minor,
                nonce: expected_nonce,
                idempotency_key: &idempotency_key,
                room: &room,
                body: &body,
            })
            .await
            {
                Ok(retry_response) if retry_response.status.is_success() => (
                    serde_json::from_slice::<Value>(&retry_response.body).ok(),
                    expected_nonce,
                ),
                Ok(retry_response) => {
                    return wallet_transfer_problem(retry_response.status, retry_response.body)
                }
                Err(response) => return response,
            }
        } else {
            return wallet_transfer_problem(wallet_status, wallet_body);
        }
    } else {
        return wallet_transfer_problem(wallet_status, wallet_body);
    };

    if let Some(existing) = existing_message(&room.room_id, &idempotency_key) {
        return chat_message_response(
            StatusCode::OK,
            existing,
            wallet_json,
            true,
            true,
            true,
            "Paid backend message was already accepted for this idempotency key.",
        );
    }

    let txid = wallet_json
        .as_ref()
        .and_then(|value| value_string(value, "txid"));
    let receipt_hash = wallet_json
        .as_ref()
        .and_then(|value| value_string(value, "receipt_hash"));
    let ledger_root = wallet_json
        .as_ref()
        .and_then(|value| value_string(value, "ledger_root"));

    let paid = ChatPaidProof {
        required: true,
        amount_roc,
        receipt_hash: receipt_hash.clone(),
        wallet_txid: txid.clone(),
        ledger_root: ledger_root.clone(),
        wallet_receipt: wallet_json.clone(),
        backend_confirmed: true,
    };

    let message = build_chat_message(&room, &payload, body, idempotency_key.clone(), paid);
    let receipt = json!({
        "kind": "chat_message",
        "room": room.room_url,
        "roomId": room.room_id,
        "messageId": message.message_id,
        "walletTxid": txid,
        "walletReceiptHash": receipt_hash,
        "ledgerRoot": ledger_root,
        "idempotencyKey": idempotency_key,
        "payerAccount": payer_account,
        "recipientAccount": room.owner_account,
        "amountRoc": amount_roc,
        "amountMinor": amount_minor,
        "nonce": paid_nonce,
        "paidAtMs": now_ms()
    });

    append_message_and_respond(
        room.room_id,
        message,
        Some(receipt),
        true,
        true,
        false,
        "Paid backend message accepted after svc-wallet receipt. Refresh balances to see ledger-backed changes.",
    )
}

/// Moderation route placeholder: delete requires future backend authority.
pub async fn mod_delete_message(
    Path(room_id): Path<String>,
    Json(payload): Json<ChatModerationRequest>,
) -> Response {
    moderation_not_implemented("delete_message", &room_id, payload)
}

/// Moderation route placeholder: block requires future backend authority.
pub async fn mod_block_username(
    Path(room_id): Path<String>,
    Json(payload): Json<ChatModerationRequest>,
) -> Response {
    moderation_not_implemented("block_username", &room_id, payload)
}

/// Moderation route placeholder: pin requires future backend authority.
pub async fn mod_pin_message(
    Path(room_id): Path<String>,
    Json(payload): Json<ChatModerationRequest>,
) -> Response {
    moderation_not_implemented("pin_message", &room_id, payload)
}

fn build_chat_message(
    room: &ChatRoom,
    payload: &ChatSendRequest,
    body: String,
    message_id: String,
    paid: ChatPaidProof,
) -> ChatMessage {
    ChatMessage {
        schema: CHAT_MESSAGE_SCHEMA.to_owned(),
        message_id,
        room: room.room_url.clone(),
        room_id: room.room_id.clone(),
        sender_passport: fallback_string(&payload.sender_passport, "passport:anonymous"),
        sender_display: fallback_string(&payload.sender_display, "@anonymous"),
        body: body.clone(),
        emoji_only: is_emoji_only(&body),
        created_at: now_isoish(),
        backend_confirmed: true,
        paid,
        moderation: ChatMessageModeration {
            state: "visible".to_owned(),
            deleted_by: None,
            deleted_at: None,
            reason: None,
            backend_confirmed: true,
        },
    }
}

fn append_message_and_respond(
    room_id: String,
    message: ChatMessage,
    receipt: Option<Value>,
    wallet_mutation: bool,
    balance_refresh_recommended: bool,
    duplicate: bool,
    response_message: &str,
) -> Response {
    let mut store = store().lock().expect("chat store poisoned");
    let Some(room) = store.rooms.get_mut(&room_id) else {
        return not_found("chat_room_not_found", &room_id);
    };

    if let Some(existing) = room
        .messages
        .iter()
        .find(|item| item.message_id == message.message_id)
        .cloned()
    {
        return chat_message_response(
            StatusCode::OK,
            existing,
            receipt,
            wallet_mutation,
            balance_refresh_recommended,
            true,
            "Message already accepted for this idempotency key.",
        );
    }

    room.messages.push(message.clone());
    if room.messages.len() > MAX_MESSAGES_PER_ROOM {
        let excess = room.messages.len() - MAX_MESSAGES_PER_ROOM;
        room.messages.drain(0..excess);
    }

    chat_message_response(
        StatusCode::CREATED,
        message,
        receipt,
        wallet_mutation,
        balance_refresh_recommended,
        duplicate,
        response_message,
    )
}

fn chat_message_response(
    status: StatusCode,
    message: ChatMessage,
    receipt: Option<Value>,
    wallet_mutation: bool,
    balance_refresh_recommended: bool,
    duplicate: bool,
    response_message: &str,
) -> Response {
    let paid_receipt_created = message.paid.required && message.paid.backend_confirmed;

    (
        status,
        Json(json!({
            "schema": CHAT_SCHEMA_SEND,
            "ok": true,
            "message": message,
            "receipt": receipt,
            "walletMutation": wallet_mutation,
            "balanceRefreshRecommended": balance_refresh_recommended,
            "duplicate": duplicate,
            "notice": response_message,
            "truthBoundary": {
                "messageBackendConfirmed": true,
                "durableDescriptor": true,
                "liveMessagesDurable": false,
                "paidReceiptCreated": paid_receipt_created,
                "ledgerMutated": wallet_mutation,
                "ledgerMutationSource": if wallet_mutation { "svc-wallet" } else { "none" },
                "cacheCanUnlockPaidChat": false
            }
        })),
    )
        .into_response()
}

fn existing_message(room_id: &str, message_id: &str) -> Option<ChatMessage> {
    store()
        .lock()
        .expect("chat store poisoned")
        .rooms
        .get(room_id)
        .and_then(|room| {
            room.messages
                .iter()
                .find(|message| message.message_id == message_id)
                .cloned()
        })
}

struct ChatTransferRequest<'a> {
    headers: &'a HeaderMap,
    payer_account: &'a str,
    recipient_account: &'a str,
    amount_minor: &'a str,
    nonce: u64,
    idempotency_key: &'a str,
    room: &'a ChatRoom,
    body: &'a str,
}

async fn send_wallet_chat_transfer(req: ChatTransferRequest<'_>) -> Result<UpstreamBody, Response> {
    let url = format!("{}/v1/transfer", wallet_base_url());
    let body = json!({
        "from": req.payer_account,
        "to": req.recipient_account,
        "asset": DEFAULT_ASSET,
        "amount_minor": req.amount_minor,
        "nonce": req.nonce,
        "idempotency_key": req.idempotency_key,
        "memo": format!("crablink chat message {} {}", req.room.room_url, fnv1a_hex(req.body)),
    });

    let mut builder = HTTP_CLIENT
        .post(url)
        .bearer_auth(wallet_bearer())
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header("idempotency-key", req.idempotency_key)
        .json(&body);

    if let Some(correlation_id) = grab(req.headers, "x-correlation-id") {
        builder = builder.header("x-correlation-id", correlation_id);
    }

    if let Some(request_id) = grab(req.headers, "x-request-id") {
        builder = builder.header("x-request-id", request_id);
    }

    let upstream_res = match builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(fail_closed(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_unavailable",
                "svc-wallet transfer route is unavailable. No chat message was accepted.",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(fail_closed(
                StatusCode::BAD_GATEWAY,
                "wallet_transfer_read_failed",
                "failed to read svc-wallet transfer response. No chat message was accepted.",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

fn wallet_transfer_problem(status: StatusCode, body: Bytes) -> Response {
    let retryable = status.as_u16() >= 500 || status == StatusCode::TOO_MANY_REQUESTS;
    let wallet_error = serde_json::from_slice::<Value>(&body).unwrap_or_else(|_| {
        json!({
            "message": String::from_utf8_lossy(&body).to_string()
        })
    });

    (
        status,
        Json(json!({
            "ok": false,
            "code": "wallet_chat_message_transfer_rejected",
            "message": "svc-wallet rejected paid chat message transfer. No chat message was accepted.",
            "retryable": retryable,
            "reason": "wallet_transfer_rejected",
            "wallet_status": status.as_u16(),
            "wallet_error": wallet_error,
            "truthBoundary": {
                "messageBackendConfirmed": false,
                "walletMutatedByChat": false,
                "receiptCreated": false,
                "failClosed": true
            }
        })),
    )
        .into_response()
}

async fn store_descriptor_object(
    headers: &HeaderMap,
    body: Bytes,
) -> Result<StoredDescriptor, Response> {
    let upstream = send_to_storage(
        Method::POST,
        "/o",
        headers,
        body,
        "chat descriptor storage upstream unavailable",
    )
    .await?;

    let parsed = serde_json::from_slice::<Value>(&upstream.body).ok();

    if !upstream.status.is_success() {
        return Err((
            upstream.status,
            Json(json!({
                "ok": false,
                "code": "chat_descriptor_storage_rejected",
                "message": "svc-storage rejected chat descriptor storage. No chat room was created.",
                "retryable": upstream.status.as_u16() >= 500,
                "reason": "storage_rejected",
                "storage_status": upstream.status.as_u16(),
                "storage_error": parsed
            })),
        )
            .into_response());
    }

    let Some(parsed) = parsed else {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "chat_descriptor_storage_bad_json",
            "svc-storage returned invalid JSON while storing chat descriptor",
            true,
            "storage_bad_json",
        ));
    };

    let Some(cid) = value_string(&parsed, "cid").filter(|cid| is_canonical_b3_cid(cid)) else {
        return Err(problem(
            StatusCode::BAD_GATEWAY,
            "chat_descriptor_storage_bad_cid",
            "svc-storage did not return a canonical b3 CID for the chat descriptor",
            true,
            "storage_bad_cid",
        ));
    };

    Ok(StoredDescriptor {
        storage_path: format!("/o/{cid}"),
        cid,
        storage_response: parsed,
    })
}

async fn fetch_descriptor_from_storage(
    cid: &str,
    headers: &HeaderMap,
) -> Result<Option<Value>, Response> {
    if !is_canonical_b3_cid(cid) {
        return Err(problem(
            StatusCode::BAD_REQUEST,
            "invalid_chat_descriptor_cid",
            "chat descriptor CID must be canonical b3",
            false,
            "bad_cid",
        ));
    }

    let upstream_path = format!("/o/{cid}");
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let mut builder = HTTP_CLIENT
        .get(upstream_url)
        .header(header::ACCEPT, "application/json");

    for (name, value) in headers {
        if should_forward_header(name) {
            builder = builder.header(name, value);
        }
    }

    let upstream_res = match builder.send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "chat_descriptor_storage_unavailable",
                "chat descriptor storage upstream unavailable",
                true,
                "storage_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "chat_descriptor_storage_read_failed",
                "failed to read chat descriptor from storage",
                true,
                "storage_read",
            ));
        }
    };

    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        return Err((
            status,
            Json(json!({
                "ok": false,
                "code": "chat_descriptor_fetch_rejected",
                "message": "svc-storage rejected chat descriptor fetch",
                "retryable": status.as_u16() >= 500,
                "reason": "storage_rejected",
                "storage_status": status.as_u16(),
                "storage_error": serde_json::from_slice::<Value>(&body).ok()
            })),
        )
            .into_response());
    }

    let descriptor = serde_json::from_slice::<Value>(&body).map_err(|_| {
        problem(
            StatusCode::BAD_GATEWAY,
            "chat_descriptor_bad_json",
            "stored chat descriptor is not valid JSON",
            true,
            "descriptor_bad_json",
        )
    })?;

    Ok(Some(descriptor))
}

async fn send_to_storage(
    method: Method,
    upstream_path: &str,
    headers: &HeaderMap,
    body: Bytes,
    unavailable_message: &'static str,
) -> Result<UpstreamBody, Response> {
    let storage_base = storage_base_url();
    let upstream_url = format!("{}{}", storage_base.trim_end_matches('/'), upstream_path);

    let reqwest_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(method) => method,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "storage_bad_method",
                unavailable_message,
                true,
                "bad_method",
            ));
        }
    };

    let mut builder = HTTP_CLIENT
        .request(reqwest_method, upstream_url)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json");

    for (name, value) in headers {
        if should_forward_header(name) {
            builder = builder.header(name, value);
        }
    }

    let upstream_res = match builder.body(body).send().await {
        Ok(upstream_res) => upstream_res,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "storage_unavailable",
                unavailable_message,
                true,
                "storage_connect",
            ));
        }
    };

    let status = upstream_res.status();
    let body = match upstream_res.bytes().await {
        Ok(body) => body,
        Err(_) => {
            return Err(problem(
                StatusCode::BAD_GATEWAY,
                "storage_read_failed",
                unavailable_message,
                true,
                "storage_read",
            ));
        }
    };

    Ok(UpstreamBody { status, body })
}

fn moderation_not_implemented(
    action: &str,
    room_id: &str,
    payload: ChatModerationRequest,
) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "schema": CHAT_SCHEMA_MOD,
            "ok": false,
            "reason": "chat_moderation_not_wired",
            "action": action,
            "roomId": normalize_room_id(room_id),
            "moderatorPassport": payload.moderator_passport,
            "messageId": payload.message_id,
            "username": payload.username,
            "clientIdempotencyKey": payload.client_idempotency_key,
            "message": "Chat moderation authority is not wired in this batch. No message was deleted, blocked, or pinned.",
            "truthBoundary": {
                "moderationActionRecorded": false,
                "messageMutated": false,
                "blocklistMutated": false,
                "pinMutated": false
            }
        })),
    )
        .into_response()
}

fn room_page(room: ChatRoom) -> (StatusCode, Json<Value>) {
    let latest = room
        .messages
        .iter()
        .rev()
        .take(50)
        .cloned()
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(json!({
            "schema": CHAT_SCHEMA_ROOM_PAGE,
            "room": room_public(&room),
            "policy": {
                "readMode": room.access.read_mode,
                "sendMode": room.access.send_mode,
                "messagePriceRoc": room.access.message_price_roc,
                "currency": room.access.currency,
                "maxMessageChars": room.moderation.max_message_chars,
                "slowModeSeconds": room.moderation.slow_mode_seconds,
                "allowEmoji": room.moderation.allow_emoji,
                "allowReactions": room.moderation.allow_reactions,
                "expiryMode": room.expiry.mode,
                "archiveMode": room.expiry.archive_mode
            },
            "viewer": {
                "canRead": true,
                "canSend": room.access.send_mode != "disabled",
                "canModerate": false,
                "paidSendRequiresWalletPath": room.access.send_mode == "paid_per_message"
            },
            "latest": latest.into_iter().rev().collect::<Vec<_>>(),
            "truthBoundary": truth_boundary_json()
        })),
    )
}

fn room_public(room: &ChatRoom) -> Value {
    json!({
        "schema": room.schema,
        "kind": room.kind,
        "roomId": room.room_id,
        "roomUrl": room.room_url,
        "canonicalRoomUrl": room.backend.canonical_room_url,
        "descriptorCid": room.backend.descriptor_cid,
        "title": room.title,
        "description": room.description,
        "ownerPassport": room.owner_passport,
        "ownerAccount": room.owner_account,
        "ownerDisplay": room.owner_display,
        "attachedTo": room.attached_to,
        "access": room.access,
        "payout": room.payout,
        "expiry": room.expiry,
        "moderation": room.moderation,
        "pinnedNote": room.pinned_note,
        "createdAt": room.created_at,
        "backend": room.backend
    })
}

fn canonical_descriptor_value(descriptor: &Value, payload: &ChatRoomEnvelope) -> Value {
    let room = room_from_descriptor(descriptor, payload);

    json!({
        "schema": CHAT_DESCRIPTOR_SCHEMA,
        "kind": "chat",
        "title": room.title,
        "description": room.description,
        "ownerPassport": room.owner_passport,
        "ownerAccount": room.owner_account,
        "ownerDisplay": room.owner_display,
        "attachedTo": room.attached_to,
        "access": room.access,
        "payout": room.payout,
        "expiry": room.expiry,
        "moderation": room.moderation,
        "pinnedNote": room.pinned_note,
        "live": {
            "messageState": "backend_event_state",
            "currentBackend": "omnigate_in_memory_dev_only",
            "futureFanout": "svc-mailbox",
            "futureArchive": "b3 chatlog snapshots",
            "messagesIncludedInDescriptorHash": false
        },
        "truthBoundary": {
            "descriptorIsImmutable": true,
            "liveMessagesAreNotDescriptorTruth": true,
            "walletTruthOwner": "svc-wallet",
            "ledgerTruthOwner": "ron-ledger"
        }
    })
}

fn room_from_descriptor(descriptor: &Value, payload: &ChatRoomEnvelope) -> ChatRoom {
    let title = clamp_text(
        descriptor
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("CrabLink Creator Chat"),
        MAX_ROOM_TITLE_CHARS,
    );
    let description = clamp_text(
        descriptor
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Portable CrabLink chat room."),
        MAX_DESCRIPTION_CHARS,
    );

    let owner_passport = fallback_string(
        payload.owner_passport.as_str(),
        descriptor
            .get("ownerPassport")
            .and_then(Value::as_str)
            .unwrap_or("passport:main:dev"),
    );
    let owner_account = fallback_string(
        payload.wallet_account.as_str(),
        descriptor
            .get("ownerAccount")
            .and_then(Value::as_str)
            .unwrap_or("acct_dev"),
    );
    let owner_display = descriptor
        .get("ownerDisplay")
        .and_then(Value::as_str)
        .unwrap_or("@creator")
        .to_owned();

    let room_id = normalize_room_id(&format!(
        "chat-{}-{}",
        slug(&title),
        fnv1a_hex(&format!("{title}:{owner_passport}:{owner_account}"))
    ));
    let room_url = format!("crab://{room_id}.chat");

    let access_value = descriptor.get("access").cloned().unwrap_or(Value::Null);
    let payout_value = descriptor.get("payout").cloned().unwrap_or(Value::Null);
    let expiry_value = descriptor.get("expiry").cloned().unwrap_or(Value::Null);
    let moderation_value = descriptor.get("moderation").cloned().unwrap_or(Value::Null);

    ChatRoom {
        room_id,
        room_url,
        schema: CHAT_ROOM_SCHEMA.to_owned(),
        kind: "chat".to_owned(),
        title,
        description,
        owner_passport,
        owner_account,
        owner_display,
        attached_to: string_array(descriptor.get("attachedTo"))
            .into_iter()
            .take(32)
            .collect(),
        access: parse_access(&access_value),
        payout: parse_payout(&payout_value),
        expiry: parse_expiry(&expiry_value),
        moderation: parse_moderation(&moderation_value),
        pinned_note: clamp_text(
            descriptor
                .get("pinnedNote")
                .and_then(Value::as_str)
                .unwrap_or("Welcome to the room. Keep it fun, honest, and crabby. 🦀"),
            280,
        ),
        created_at: now_isoish(),
        backend: ChatBackendTruth::default(),
        messages: Vec::new(),
    }
}

fn apply_canonical_descriptor_identity(room: &mut ChatRoom, cid: &str) {
    let raw = cid.trim_start_matches("b3:").to_owned();
    room.room_id = raw.clone();
    room.room_url = format!("crab://{raw}.chat");
    room.backend = ChatBackendTruth::descriptor_backed(cid);
}

fn remember_room(room: ChatRoom) {
    let mut store = store().lock().expect("chat store poisoned");
    store.rooms.insert(room.room_id.clone(), room);
}

fn builtin_room(requested: &str) -> ChatRoom {
    let title = if requested == "crab://chat" {
        "CrabLink Chat".to_owned()
    } else {
        "Uncreated chat room".to_owned()
    };
    let room_id = room_id_from_url(requested);

    ChatRoom {
        room_id: room_id.clone(),
        room_url: requested.to_owned(),
        schema: CHAT_ROOM_SCHEMA.to_owned(),
        kind: "chat".to_owned(),
        title,
        description:
            "Built-in chat route preview. Create a room to get a canonical b3 chat descriptor."
                .to_owned(),
        owner_passport: "passport:main:dev".to_owned(),
        owner_account: "acct_dev".to_owned(),
        owner_display: "@creator".to_owned(),
        attached_to: Vec::new(),
        access: ChatAccess::default(),
        payout: ChatPayout::default(),
        expiry: ChatExpiry::default(),
        moderation: ChatModeration {
            mods: vec!["@modname".to_owned()],
            ..ChatModeration::default()
        },
        pinned_note: "Welcome to CrabLink Chat. Create a room to mint a b3-addressed descriptor."
            .to_owned(),
        created_at: now_isoish(),
        backend: ChatBackendTruth::default(),
        messages: Vec::new(),
    }
}

fn descriptor_value(payload: &ChatRoomEnvelope) -> Value {
    if payload.descriptor.is_object() {
        payload.descriptor.clone()
    } else if payload.room.is_object() {
        payload.room.clone()
    } else {
        json!({})
    }
}

fn parse_access(value: &Value) -> ChatAccess {
    let send_mode = match value.get("sendMode").and_then(Value::as_str) {
        Some("free") => "free",
        Some("disabled") => "disabled",
        _ => "paid_per_message",
    }
    .to_owned();

    let amount = value
        .get("messagePriceRoc")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .min(1_000_000);

    ChatAccess {
        read_mode: "public".to_owned(),
        send_mode: send_mode.clone(),
        message_price_roc: if send_mode == "free" { 0 } else { amount },
        currency: "ROC".to_owned(),
        explicit_confirm_required: send_mode == "paid_per_message",
    }
}

fn parse_payout(value: &Value) -> ChatPayout {
    let creator = as_u16_bps(value.get("creatorShareBps"), 9000);
    let platform = as_u16_bps(value.get("platformShareBps"), 1000);
    let moderator = as_u16_bps(value.get("moderatorPoolBps"), 0);
    let total = creator.saturating_add(platform).saturating_add(moderator);

    ChatPayout {
        creator_share_bps: creator,
        platform_share_bps: platform,
        moderator_pool_bps: moderator,
        total_bps: total,
        valid_total: total == 10000,
        backend_confirmed: false,
    }
}

fn parse_expiry(value: &Value) -> ChatExpiry {
    let mode = match value.get("mode").and_then(Value::as_str) {
        Some("expires_at") => "expires_at",
        Some("manual_close") => "manual_close",
        _ => "never_expires",
    }
    .to_owned();

    let archive_mode = match value.get("archiveMode").and_then(Value::as_str) {
        Some("hide_after_expiry") => "hide_after_expiry",
        _ => "read_only_after_expiry",
    }
    .to_owned();

    ChatExpiry {
        mode,
        expires_at: value
            .get("expiresAt")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned),
        archive_mode,
        backend_confirmed: false,
    }
}

fn parse_moderation(value: &Value) -> ChatModeration {
    ChatModeration {
        mode: "moderated".to_owned(),
        mods: string_array(value.get("mods")),
        blocked_usernames: string_array(value.get("blockedUsernames")),
        blocked_terms: string_array(value.get("blockedTerms")),
        allow_emoji: value
            .get("allowEmoji")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        allow_reactions: value
            .get("allowReactions")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_message_chars: value
            .get("maxMessageChars")
            .and_then(Value::as_u64)
            .unwrap_or(500)
            .clamp(1, MAX_MESSAGE_CHARS as u64) as usize,
        slow_mode_seconds: value
            .get("slowModeSeconds")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(86_400),
        backend_confirmed: false,
    }
}

fn store() -> &'static Mutex<ChatStore> {
    CHAT_STORE.get_or_init(|| Mutex::new(ChatStore::default()))
}

fn load_room(room_id: &str) -> Option<ChatRoom> {
    store()
        .lock()
        .expect("chat store poisoned")
        .rooms
        .get(room_id)
        .cloned()
}

fn truth_boundary_json() -> Value {
    json!({
        "descriptorCanBeB3Addressed": true,
        "durableDescriptor": true,
        "liveMessagesDurable": false,
        "assignsB3Cid": true,
        "writesIndexPointer": false,
        "walletMutatedByRoomCreate": false,
        "paidWalletPathAvailable": true,
        "receiptCreatedByRoomCreate": false,
        "cacheCanUnlockPaidChat": false,
        "moderatorAuthorityGranted": false,
        "messageFanout": "poll_latest_only"
    })
}

fn not_found(reason: &str, room_id: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "ok": false,
            "reason": reason,
            "roomId": room_id,
            "message": "Chat room was not found in memory or durable descriptor storage.",
            "truthBoundary": truth_boundary_json()
        })),
    )
        .into_response()
}

fn bad_request(reason: &str, message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "ok": false,
            "reason": reason,
            "message": message,
            "truthBoundary": truth_boundary_json()
        })),
    )
        .into_response()
}

fn fail_closed(status: StatusCode, reason: &str, message: &str) -> Response {
    (
        status,
        Json(json!({
            "ok": false,
            "reason": reason,
            "message": message,
            "failClosed": true,
            "truthBoundary": truth_boundary_json()
        })),
    )
        .into_response()
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

fn bounded_message_body(value: &str, max_chars: usize) -> String {
    value
        .replace('\0', "")
        .trim()
        .chars()
        .take(max_chars.min(MAX_MESSAGE_CHARS))
        .collect()
}

fn clamp_text(value: &str, max_chars: usize) -> String {
    value.trim().chars().take(max_chars).collect()
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.chars().take(128).collect::<String>())
            .take(128)
            .collect(),
        Some(Value::String(items)) => items
            .split([',', '\n'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.chars().take(128).collect::<String>())
            .take(128)
            .collect(),
        _ => Vec::new(),
    }
}

fn as_u16_bps(value: Option<&Value>, default: u16) -> u16 {
    value
        .and_then(Value::as_u64)
        .map(|n| n.min(10000) as u16)
        .unwrap_or(default)
}

fn room_id_from_url(value: &str) -> String {
    let clean = value
        .trim()
        .strip_prefix("crab://")
        .unwrap_or(value.trim())
        .split(['?', '#'])
        .next()
        .unwrap_or("chat");

    let clean = clean
        .strip_prefix("b3:")
        .unwrap_or(clean)
        .trim_end_matches(".chat");

    normalize_room_id(clean)
}

fn canonical_chat_cid_from_url(value: &str) -> Option<String> {
    let clean = value
        .trim()
        .strip_prefix("crab://")
        .unwrap_or(value.trim())
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(".chat");

    let hash = clean.strip_prefix("b3:").unwrap_or(clean);

    if hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Some(format!("b3:{hash}"))
    } else {
        None
    }
}

fn normalize_room_id(value: &str) -> String {
    let mut out = String::new();

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '@') {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }

    let out = out.trim_matches('-').chars().take(140).collect::<String>();
    if out.is_empty() {
        "local-chat-preview".to_owned()
    } else {
        out
    }
}

fn slug(value: &str) -> String {
    normalize_room_id(value)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .take(48)
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn fallback_string(value: &str, fallback: &str) -> String {
    let clean = value.trim();
    if clean.is_empty() {
        fallback.to_owned()
    } else {
        clean.chars().take(128).collect()
    }
}

fn fallback_idempotency(value: &str, prefix: &str, seed: &str) -> String {
    let clean = normalize_room_id(value);
    if !clean.is_empty() && clean != "local-chat-preview" {
        clean.chars().take(64).collect()
    } else {
        format!("{prefix}:{}", fnv1a_hex(seed))
    }
}

fn sender_blocked(room: &ChatRoom, sender: &str) -> bool {
    let normalized = sender.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }

    room.moderation
        .blocked_usernames
        .iter()
        .any(|blocked| blocked.trim().eq_ignore_ascii_case(&normalized))
}

fn contains_blocked_term(room: &ChatRoom, body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    room.moderation
        .blocked_terms
        .iter()
        .any(|term| !term.trim().is_empty() && lower.contains(&term.to_ascii_lowercase()))
}

fn is_emoji_only(value: &str) -> bool {
    let stripped = value.replace(char::is_whitespace, "");
    !stripped.is_empty()
        && stripped
            .chars()
            .all(|ch| !ch.is_ascii_alphanumeric() && !ch.is_ascii_punctuation() && !ch.is_control())
}

fn unix_now_plus_seconds(seconds: u64) -> String {
    let now = SystemTime::now()
        .checked_add(Duration::from_secs(seconds))
        .unwrap_or(SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    format!("unix:{now}")
}

fn quote_amount_roc(value: &Value) -> Option<u64> {
    value
        .get("amountRoc")
        .and_then(Value::as_u64)
        .or_else(|| value.get("amount_roc").and_then(Value::as_u64))
}

fn quote_recipient_account(value: &Value) -> Option<String> {
    value
        .get("recipientAccount")
        .and_then(Value::as_str)
        .or_else(|| value.get("recipient_account").and_then(Value::as_str))
        .or_else(|| value.get("recipient").and_then(Value::as_str))
        .and_then(clean_optional)
}

fn clean_optional(value: &str) -> Option<String> {
    let clean = value.trim();

    if clean.is_empty() {
        None
    } else {
        Some(clean.chars().take(128).collect())
    }
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(value) => Some(value.trim().to_owned()).filter(|s| !s.is_empty()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn parse_expected_nonce(message: &str) -> Option<u64> {
    let lower = message.to_ascii_lowercase();
    let marker = "expected ";
    let index = lower.find(marker)?;
    let after = &lower[index + marker.len()..];
    let digits = after
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();

    digits.parse::<u64>().ok()
}

fn default_chat_message_nonce() -> u64 {
    env::var("OMNIGATE_CHAT_MESSAGE_NONCE")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|nonce| *nonce >= DEFAULT_WALLET_NONCE)
        .unwrap_or(DEFAULT_WALLET_NONCE)
}

fn wallet_base_url() -> String {
    env::var("OMNIGATE_WALLET_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BASE_URL.to_owned())
}

fn wallet_bearer() -> String {
    env::var("OMNIGATE_WALLET_BEARER")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_WALLET_BEARER.to_owned())
}

fn storage_base_url() -> String {
    env::var("OMNIGATE_STORAGE_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL"))
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_STORAGE_BASE_URL.to_owned())
}

fn is_canonical_b3_cid(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("b3:") else {
        return false;
    };

    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn should_forward_header(name: &HeaderName) -> bool {
    if is_hop_by_hop_or_host(name) || name == header::CONTENT_LENGTH {
        return false;
    }

    name == header::AUTHORIZATION
        || name == header::ACCEPT
        || name == header::CONTENT_TYPE
        || super::header_policy::is_allowed_ron_context_header(name)
        || name.as_str() == "x-correlation-id"
        || name.as_str() == "x-request-id"
        || name.as_str() == "idempotency-key"
}

fn is_hop_by_hop_or_host(name: &HeaderName) -> bool {
    name == header::HOST
        || name == header::CONNECTION
        || name == header::PROXY_AUTHORIZATION
        || name == header::TE
        || name == header::TRAILER
        || name == header::TRANSFER_ENCODING
        || name == header::UPGRADE
}

fn now_ms() -> u64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 1;
    };

    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn now_isoish() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    format!("unix:{now}")
}

fn fnv1a_hex(value: &str) -> String {
    let mut hash = 0x811c9dc5u32;

    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x01000193);
    }

    format!("{hash:08x}")
}
