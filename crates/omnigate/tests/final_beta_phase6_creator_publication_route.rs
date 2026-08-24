//! RO:WHAT — Focused Omnigate creator-publication read-route tests.
//! RO:WHY — Proves bounded forwarding from Omnigate to svc-index without adding economic or relationship authority.

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode, Uri},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Serialize)]
struct Problem {
    code: &'static str,
    message: &'static str,
    retryable: bool,
    reason: &'static str,
}

async fn start_dummy_index() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn list_handler(
        Path(username): Path<String>,
        headers: HeaderMap,
        uri: Uri,
    ) -> (StatusCode, Json<Value>) {
        let query = parse_query(&uri);

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "schema": "crablink.publication-page.v1",
                "items": [
                    {
                        "schema": "crablink.publication-summary.v1",
                        "publicationId": "publication-001",
                        "kind": "post",
                        "crabUrl": "crab://publication-001.post",
                        "title": "Publication 001",
                        "summary": "Canonical publication projection",
                        "creator": {
                            "username": username,
                            "displayName": "Rusty Crab",
                            "profileUrl": "crab://@rusty_crab"
                        },
                        "publishedAt": "2026-08-05T20:00:00.000Z",
                        "updatedAt": "2026-08-05T20:00:00.000Z",
                        "visibility": "public",
                        "access": "free",
                        "thumbnail": null,
                        "references": null,
                        "pinned": false
                    }
                ],
                "nextCursor": query.get("cursor"),
                "hasMore": false,
                "receivedLimit": query.get("limit"),
                "authorization": grab(
                    &headers,
                    "authorization",
                ),
                "xCorrelationId": grab(
                    &headers,
                    "x-correlation-id",
                ),
                "xRequestId": grab(
                    &headers,
                    "x-request-id",
                ),
                "host": grab(
                    &headers,
                    "host",
                ),
                "connection": grab(
                    &headers,
                    "connection",
                )
            })),
        )
    }

    async fn detail_handler(
        Path((username, publication_id)): Path<(String, String)>,
        headers: HeaderMap,
    ) -> (StatusCode, Json<Value>) {
        if publication_id == "missing" {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::to_value(Problem {
                        code: "not_found",
                        message: "publication was not found",
                        retryable: false,
                        reason: "publication_not_found",
                    })
                    .expect("serialize not-found problem"),
                ),
            );
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "schema": "crablink.publication-summary.v1",
                "publicationId": publication_id,
                "kind": "article",
                "crabUrl": "crab://publication-detail.article",
                "title": "Publication Detail",
                "summary": "Canonical publication detail",
                "creator": {
                    "username": username,
                    "displayName": "Rusty Crab",
                    "profileUrl": "crab://@rusty_crab"
                },
                "publishedAt": "2026-08-05T20:00:00.000Z",
                "updatedAt": "2026-08-05T20:00:00.000Z",
                "visibility": "public",
                "access": "free",
                "thumbnail": null,
                "references": null,
                "pinned": false,
                "authorization": grab(
                    &headers,
                    "authorization",
                )
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route(
            "/v1/index/creators/:username/publications",
            get(list_handler),
        )
        .route(
            "/v1/index/creators/:username/publications/:publication_id",
            get(detail_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy index");

    let address = listener.local_addr().expect("dummy index address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("dummy index server");
    });

    wait_for_health(format!("http://{address}/healthz",)).await;

    address
}

async fn start_omnigate(index_address: SocketAddr) -> SocketAddr {
    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        format!("http://{index_address}",),
    );

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate");

    let address = listener.local_addr().expect("omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate server");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    address
}

#[tokio::test]
async fn phase6b3_list_route_proxies_bounded_publication_page() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_address = start_dummy_index().await;

    let omnigate_address = start_omnigate(index_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{omnigate_address}/v1/creators/rusty_crab/publications",
        ))
        .query(&[("cursor", "p_00000002"), ("limit", "20")])
        .header("authorization", "Bearer read-only")
        .header("x-correlation-id", "corr-phase6b3")
        .header("x-request-id", "req-phase6b3")
        .header("connection", "close")
        .send()
        .await
        .expect("publication list response");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("publication list JSON");

    assert_eq!(body["schema"], "crablink.publication-page.v1",);

    assert_eq!(body["items"][0]["creator"]["username"], "rusty_crab",);

    assert_eq!(body["nextCursor"], "p_00000002",);

    assert_eq!(body["receivedLimit"], "20",);

    assert_eq!(body["authorization"], "Bearer read-only",);

    assert_eq!(body["xCorrelationId"], "corr-phase6b3",);

    assert_eq!(body["xRequestId"], "req-phase6b3",);

    assert!(body["connection"].is_null(),);

    clear_env();
}

#[tokio::test]
async fn phase6b3_detail_route_proxies_publication_summary() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_address = start_dummy_index().await;

    let omnigate_address = start_omnigate(index_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{omnigate_address}/v1/creators/rusty_crab/publications/publication-001",
        ))
        .header("authorization", "Bearer read-only")
        .send()
        .await
        .expect("publication detail response");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("publication detail JSON");

    assert_eq!(body["schema"], "crablink.publication-summary.v1",);

    assert_eq!(body["publicationId"], "publication-001",);

    assert_eq!(body["creator"]["username"], "rusty_crab",);

    assert_eq!(body["authorization"], "Bearer read-only",);

    clear_env();
}

#[tokio::test]
async fn phase6b3_upstream_not_found_passes_through() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_address = start_dummy_index().await;

    let omnigate_address = start_omnigate(index_address).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{omnigate_address}/v1/creators/rusty_crab/publications/missing",
        ))
        .send()
        .await
        .expect("publication not-found response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND,);

    let body: Value = response.json().await.expect("not-found JSON");

    assert_eq!(body["code"], "not_found",);

    clear_env();
}

#[tokio::test]
async fn phase6b3_invalid_path_segments_fail_before_upstream() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate");

    let address = listener.local_addr().expect("omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate server");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();

    let invalid_username = client
        .get(format!("http://{address}/v1/creators/BadUser/publications",))
        .send()
        .await
        .expect("invalid username response");

    assert_eq!(invalid_username.status(), StatusCode::BAD_REQUEST,);

    let invalid_id = client
        .get(format!(
            "http://{address}/v1/creators/rusty_crab/publications/bad%25id",
        ))
        .send()
        .await
        .expect("invalid publication response");

    assert_eq!(invalid_id.status(), StatusCode::BAD_REQUEST,);

    clear_env();
}

#[tokio::test]
async fn phase6b3_unknown_query_fields_are_rejected() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_address = start_dummy_index().await;

    let omnigate_address = start_omnigate(index_address).await;

    let response =
        reqwest::Client::new()
            .get(
                format!(
                    "http://{omnigate_address}/v1/creators/rusty_crab/publications?limit=20&walletBalance=100",
                ),
            )
            .send()
            .await
            .expect(
                "unknown query response",
            );

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);

    clear_env();
}

#[tokio::test]
async fn phase6b3_transport_failure_returns_stable_bad_gateway() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind omnigate");

    let address = listener.local_addr().expect("omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("omnigate server");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let response = reqwest::Client::new()
        .get(format!(
            "http://{address}/v1/creators/rusty_crab/publications",
        ))
        .send()
        .await
        .expect("transport failure response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY,);

    let body: Value = response.json().await.expect("bad gateway JSON");

    assert_eq!(body["code"], "index_upstream",);

    assert_eq!(body["retryable"], true,);

    clear_env();
}

#[test]
fn phase6b3_route_surface_adds_no_economic_or_relationship_authority() {
    let route_source = include_str!("../src/routes/v1/creator_publications.rs",);

    let router_source = include_str!("../src/routes/v1/mod.rs",);

    let compact_router: String = router_source.split_whitespace().collect();

    for required in [
        "/creators/:username/publications",
        "/creators/:username/publications/:publication_id",
        "creator_publications::list_creator_publications",
        "creator_publications::get_creator_publication",
    ] {
        assert!(
            compact_router.contains(required,),
            "missing route fragment: {required}",
        );
    }

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "follow_mutation",
        "settlement_authority",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
    ] {
        assert!(
            route_source.contains(forbidden,) == false,
            "forbidden authority: {forbidden}",
        );
    }

    assert!(route_source.contains("proxy only",),);
}

fn parse_query(uri: &Uri) -> HashMap<String, String> {
    let mut values = HashMap::new();

    let Some(query) = uri.query() else {
        return values;
    };

    for pair in query.split('&').filter(|pair| pair.is_empty() == false) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));

        values.insert(key.to_owned(), value.to_owned());
    }

    values
}

fn grab(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| value.is_empty() == false)
        .map(ToOwned::to_owned)
}

async fn wait_for_health(url: String) {
    let client = reqwest::Client::new();

    for _ in 0..40 {
        if let Ok(response) = client.get(&url).send().await {
            if response.status().is_success() {
                return;
            }
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    panic!("dummy index did not become healthy at {url}",);
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");

    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
}
