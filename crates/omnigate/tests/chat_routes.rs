//! chat_routes.rs — integration tests for `/v1/chat/*`.
//!
//! RO:WHAT — Proves b3 descriptor create/resolve, free send, paid quote, paid-send guards, and moderation fail-closed contracts.
//! RO:WHY — CrabLink Chat needs canonical room URLs without pretending live messages are durable yet.
//! RO:INTERACTS — omnigate::routes::v1::chat, mock svc-storage.
//! RO:INVARIANTS — descriptor gets b3 URL; messages remain in-memory; paid send without quote fails closed; no fake receipt.
//! RO:CONFIG — test sets OMNIGATE_STORAGE_BASE_URL to an in-process mock storage server.
//! RO:TEST — cargo test -p omnigate --test chat_routes.

use axum::{
    body::{self, Body, Bytes},
    extract::{Path, State},
    http::{Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use omnigate::routes::v1;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::net::TcpListener;
use tower::ServiceExt;

const MOCK_CHAT_DESCRIPTOR_CID: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test]
async fn chat_create_resolve_quote_and_paid_send_requires_quote() {
    let _guard = test_env_lock().lock().await;
    let storage = spawn_mock_storage().await;
    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", storage.base_url.clone());

    let app = v1::router::<()>();

    let create_body = json!({
        "schema": "crablink.chat-room-create.v1",
        "descriptor": {
            "schema": "crablink.chat-room.v1",
            "kind": "chat",
            "title": "Paid Crab Chat Test",
            "description": "A paid chat proof room.",
            "ownerPassport": "passport:main:creator",
            "ownerAccount": "acct_creator",
            "ownerDisplay": "@creator",
            "access": {
                "sendMode": "paid_per_message",
                "messagePriceRoc": 2
            },
            "payout": {
                "creatorShareBps": 9000,
                "platformShareBps": 1000,
                "moderatorPoolBps": 0
            },
            "moderation": {
                "mods": ["@modname"],
                "blockedUsernames": [],
                "blockedTerms": [],
                "maxMessageChars": 500,
                "allowEmoji": true,
                "allowReactions": true
            },
            "pinnedNote": "Welcome 🦀"
        },
        "ownerPassport": "passport:main:creator",
        "walletAccount": "acct_creator",
        "clientIdempotencyKey": "paid-chat-create-test"
    });

    let create = request_json(app.clone(), Method::POST, "/chat", create_body).await;
    assert_eq!(create.status, StatusCode::CREATED);
    assert_eq!(create.body["schema"], "omnigate.chat-room-create-result.v1");
    assert_eq!(create.body["walletMutation"], false);
    assert_eq!(create.body["receipt"], Value::Null);
    assert_eq!(create.body["durable"], true);
    assert_eq!(create.body["b3Cid"], MOCK_CHAT_DESCRIPTOR_CID);
    assert_eq!(
        create.body["roomUrl"],
        "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.chat"
    );
    assert_eq!(create.body["room"]["backend"]["durable"], true);

    let room_id = create.body["roomId"].as_str().expect("room id").to_owned();
    assert_eq!(
        room_id,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );

    let room_url = create.body["roomUrl"]
        .as_str()
        .expect("room url")
        .to_owned();

    let resolve_path = format!("/chat/resolve?url={}", urlencoding_like(&room_url));
    let resolve = request_json(app.clone(), Method::GET, &resolve_path, json!({})).await;
    assert_eq!(resolve.status, StatusCode::OK);
    assert_eq!(resolve.body["schema"], "omnigate.chat-room-page.v1");
    assert_eq!(resolve.body["room"]["roomId"], room_id);
    assert_eq!(
        resolve.body["room"]["descriptorCid"],
        MOCK_CHAT_DESCRIPTOR_CID
    );

    let quote = request_json(
        app.clone(),
        Method::POST,
        &format!("/chat/{room_id}/messages/quote"),
        json!({
            "senderPassport": "passport:main:visitor-b",
            "walletAccount": "acct_visitor_b",
            "body": "Hello paid room 🦀",
            "clientNonce": "paid-msg-one"
        }),
    )
    .await;
    assert_eq!(quote.status, StatusCode::OK);
    assert_eq!(quote.body["schema"], "omnigate.chat-message-quote.v1");
    assert_eq!(quote.body["amountRoc"], 2);
    assert_eq!(quote.body["recipientAccount"], "acct_creator");
    assert_eq!(quote.body["walletMutation"], false);
    assert_eq!(quote.body["receiptCreated"], false);

    let paid_send_without_quote = request_json(
        app,
        Method::POST,
        &format!("/chat/{room_id}/messages/send"),
        json!({
            "senderPassport": "passport:main:visitor-b",
            "senderDisplay": "@visitor-b",
            "walletAccount": "acct_visitor_b",
            "body": "This should not be accepted without an explicit quote."
        }),
    )
    .await;
    assert_eq!(paid_send_without_quote.status, StatusCode::PAYMENT_REQUIRED);
    assert_eq!(
        paid_send_without_quote.body["reason"],
        "paid_chat_quote_required"
    );
    assert_eq!(
        paid_send_without_quote.body["truthBoundary"]["cacheCanUnlockPaidChat"],
        false
    );
}

#[tokio::test]
async fn free_chat_send_list_latest_and_mod_fails_closed() {
    let _guard = test_env_lock().lock().await;
    let storage = spawn_mock_storage().await;
    std::env::set_var("OMNIGATE_STORAGE_BASE_URL", storage.base_url.clone());

    let app = v1::router::<()>();

    let create = request_json(
        app.clone(),
        Method::POST,
        "/chat",
        json!({
            "descriptor": {
                "title": "Free Crab Chat Test",
                "description": "A free chat proof room.",
                "ownerPassport": "passport:main:creator",
                "ownerAccount": "acct_creator",
                "access": {
                    "sendMode": "free",
                    "messagePriceRoc": 0
                },
                "moderation": {
                    "mods": ["@modname"],
                    "blockedUsernames": ["@blocked"],
                    "blockedTerms": ["badterm"],
                    "maxMessageChars": 500
                }
            }
        }),
    )
    .await;

    assert_eq!(create.status, StatusCode::CREATED);
    assert_eq!(create.body["durable"], true);
    assert_eq!(create.body["b3Cid"], MOCK_CHAT_DESCRIPTOR_CID);

    let room_id = create.body["roomId"].as_str().expect("room id").to_owned();

    let send = request_json(
        app.clone(),
        Method::POST,
        &format!("/chat/{room_id}/messages/send"),
        json!({
            "senderPassport": "passport:main:visitor-b",
            "senderDisplay": "@visitor-b",
            "walletAccount": "acct_visitor_b",
            "body": "Free hello 🦀",
            "clientIdempotencyKey": "free-message-one"
        }),
    )
    .await;

    assert_eq!(send.status, StatusCode::CREATED);
    assert_eq!(send.body["schema"], "omnigate.chat-message-send-result.v1");
    assert_eq!(send.body["receipt"], Value::Null);
    assert_eq!(send.body["walletMutation"], false);
    assert_eq!(send.body["message"]["backendConfirmed"], true);

    let list = request_json(
        app.clone(),
        Method::GET,
        &format!("/chat/{room_id}/messages?limit=10"),
        json!({}),
    )
    .await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.body["messages"].as_array().expect("messages").len(), 1);

    let latest = request_json(
        app.clone(),
        Method::GET,
        &format!("/chat/{room_id}/messages/latest"),
        json!({}),
    )
    .await;
    assert_eq!(latest.status, StatusCode::OK);
    assert_eq!(latest.body["messages"][0]["body"], "Free hello 🦀");

    let blocked = request_json(
        app.clone(),
        Method::POST,
        &format!("/chat/{room_id}/messages/send"),
        json!({
            "senderPassport": "passport:main:blocked",
            "senderDisplay": "@blocked",
            "body": "I should be blocked."
        }),
    )
    .await;
    assert_eq!(blocked.status, StatusCode::FORBIDDEN);
    assert_eq!(blocked.body["reason"], "chat_sender_blocked");

    let moderation = request_json(
        app,
        Method::POST,
        &format!("/chat/{room_id}/mod/delete"),
        json!({
            "moderatorPassport": "passport:main:mod",
            "messageId": "free-message-one",
            "reason": "test"
        }),
    )
    .await;
    assert_eq!(moderation.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(moderation.body["reason"], "chat_moderation_not_wired");
}

#[derive(Debug)]
struct TestResponse {
    status: StatusCode,
    body: Value,
}

async fn request_json(app: axum::Router, method: Method, path: &str, body: Value) -> TestResponse {
    let request = Request::builder()
        .method(method.clone())
        .uri(path)
        .header("content-type", "application/json")
        .body(if method == Method::GET {
            Body::empty()
        } else {
            Body::from(body.to_string())
        })
        .expect("request");

    let response = app.oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("body");

    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));

    TestResponse { status, body }
}

#[derive(Clone)]
struct MockStorageState {
    stored: Arc<Mutex<HashMap<String, Bytes>>>,
}

struct MockStorage {
    base_url: String,
}

async fn spawn_mock_storage() -> MockStorage {
    let state = MockStorageState {
        stored: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/o", post(mock_storage_put))
        .route("/o/:cid", get(mock_storage_get))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock storage");
    let addr: SocketAddr = listener.local_addr().expect("mock storage addr");

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock storage serve");
    });

    MockStorage {
        base_url: format!("http://{addr}"),
    }
}

async fn mock_storage_put(State(state): State<MockStorageState>, body: Bytes) -> Response {
    state
        .stored
        .lock()
        .expect("mock storage lock")
        .insert(MOCK_CHAT_DESCRIPTOR_CID.to_owned(), body);

    (
        StatusCode::OK,
        Json(json!({
            "cid": MOCK_CHAT_DESCRIPTOR_CID
        })),
    )
        .into_response()
}

async fn mock_storage_get(
    State(state): State<MockStorageState>,
    Path(cid): Path<String>,
) -> Response {
    let cid = cid.trim().to_owned();

    let Some(body) = state
        .stored
        .lock()
        .expect("mock storage lock")
        .get(&cid)
        .cloned()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "reason": "not_found"
            })),
        )
            .into_response();
    };

    (StatusCode::OK, [("content-type", "application/json")], body).into_response()
}

fn test_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn urlencoding_like(value: &str) -> String {
    value
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('@', "%40")
}
