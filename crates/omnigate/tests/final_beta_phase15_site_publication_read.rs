//! RO:WHAT — Focused Omnigate tests for Site-keyed root-publication reads.
//! RO:WHY — Proves Phase 15 exposes durable Site roots through a strict read-only façade without creating Forum-specific backend authority.
//! RO:INTERACTS — Omnigate site_publications façade and dummy svc-index GET `/v1/index/site-publications`.
//! RO:INVARIANTS — Site is required; unknown query fields reject; cursor/limit remain transparent; upstream failures stay truthful.
//! RO:SECURITY — read proxy only; no identity proof, publication mutation, wallet, ledger, moderation, QuickChain, ROX, or Solana authority.
//! RO:TEST — this file.

use std::{collections::HashMap, net::SocketAddr};

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

const SITE_URL: &str = "crab://rusty-forum";

const POST_URL: &str =
    "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.post";

async fn start_dummy_index() -> SocketAddr {
    async fn healthz() -> &'static str {
        "ok"
    }

    async fn site_handler(
        Query(query): Query<HashMap<String, String>>,
        headers: HeaderMap,
    ) -> (StatusCode, Json<Value>) {
        let Some(site) = query.get("siteCrabUrl") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "code":
                        "invalid_site_publication_site",
                    "message":
                        "Site is required",
                    "retryable":
                        false,
                    "reason":
                        "missing_site"
                })),
            );
        };

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "schema":
                    "crablink.site-publication-page.v1",

                "items": [
                    {
                        "schema":
                            "crablink.site-publication.v1",

                        "publicationId":
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",

                        "kind":
                            "post",

                        "crabUrl":
                            POST_URL,

                        "title":
                            "Welcome to Rusty Forum",

                        "summary":
                            "Durable Forum thread root",

                        "creatorDisplay":
                            "@alice",

                        "createdAtMs":
                            1001,

                        "visibility":
                            "public",

                        "references": {
                            "manifestCid":
                                "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",

                            "contentCid":
                                "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",

                            "siteUrl":
                                site
                        },

                        "tags": [
                            "forum",
                            "forum-category:general"
                        ],

                        "siteCrabUrl":
                            site
                    }
                ],

                "nextCursor":
                    query.get(
                        "cursor",
                    ),

                "hasMore":
                    false,

                "receivedLimit":
                    query.get(
                        "limit",
                    ),

                "authorization":
                    header_value(
                        &headers,
                        "authorization",
                    ),

                "xCorrelationId":
                    header_value(
                        &headers,
                        "x-correlation-id",
                    ),

                "xRequestId":
                    header_value(
                        &headers,
                        "x-request-id",
                    ),

                "connection":
                    header_value(
                        &headers,
                        "connection",
                    )
            })),
        )
    }

    let router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/index/site-publications", get(site_handler));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind dummy Site-publication index");

    let address = listener.local_addr().expect("dummy index address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve dummy Site-publication index");
    });

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
        .expect("bind Omnigate");

    let address = listener.local_addr().expect("Omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve Omnigate");
    });

    address
}

#[tokio::test]
async fn phase15a4a2c1_site_publication_page_proxies_index_truth() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_address = start_dummy_index().await;

    let address = start_omnigate(index_address).await;

    let response = reqwest::Client::new()
        .get(format!("http://{address}/v1/site-publications",))
        .query(&[
            ("siteCrabUrl", SITE_URL),
            ("cursor", "s_00000001"),
            ("limit", "25"),
        ])
        .header("authorization", "Bearer site-read")
        .header("x-correlation-id", "corr-phase15a4a2c1")
        .header("x-request-id", "req-phase15a4a2c1")
        .header("connection", "close")
        .send()
        .await
        .expect("Omnigate Site-publication read");

    assert_eq!(response.status(), StatusCode::OK,);

    let body: Value = response.json().await.expect("Site-publication page JSON");

    assert_eq!(body["schema"], "crablink.site-publication-page.v1",);

    assert_eq!(body["items"][0]["kind"], "post",);

    assert_eq!(body["items"][0]["crabUrl"], POST_URL,);

    assert_eq!(body["items"][0]["siteCrabUrl"], SITE_URL,);

    assert_eq!(body["items"][0]["creatorDisplay"], "@alice",);

    assert_eq!(body["items"][0]["tags"][0], "forum",);

    assert_eq!(body["items"][0]["tags"][1], "forum-category:general",);

    assert_eq!(body["nextCursor"], "s_00000001",);

    assert_eq!(body["receivedLimit"], "25",);

    assert_eq!(body["authorization"], "Bearer site-read",);

    assert_eq!(body["xCorrelationId"], "corr-phase15a4a2c1",);

    assert_eq!(body["xRequestId"], "req-phase15a4a2c1",);

    assert!(body["connection"].is_null(),);

    clear_env();
}

#[tokio::test]
async fn phase15a4a2c1_unknown_query_fields_fail_before_index() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let address = start_omnigate_with_current_env().await;

    let response = reqwest::Client::new()
        .get(format!("http://{address}/v1/site-publications",))
        .query(&[("siteCrabUrl", SITE_URL), ("verifiedCreator", "alice")])
        .send()
        .await
        .expect("unknown Site-publication query response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);

    clear_env();
}

#[tokio::test]
async fn phase15a4a2c1_site_query_is_required() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let address = start_omnigate_with_current_env().await;

    let response = reqwest::Client::new()
        .get(format!("http://{address}/v1/site-publications?limit=20",))
        .send()
        .await
        .expect("missing Site-publication Site response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST,);

    clear_env();
}

#[tokio::test]
async fn phase15a4a2c1_index_transport_failure_is_structured_bad_gateway() {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let address = start_omnigate_with_current_env().await;

    let response = reqwest::Client::new()
        .get(format!("http://{address}/v1/site-publications",))
        .query(&[("siteCrabUrl", SITE_URL)])
        .send()
        .await
        .expect("Site-publication transport response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY,);

    let body: Value = response
        .json()
        .await
        .expect("Site-publication bad gateway JSON");

    assert_eq!(body["code"], "index_upstream",);

    assert_eq!(body["retryable"], true,);

    clear_env();
}

#[test]
fn phase15a4a2c1_route_surface_is_generic_read_only_proxy() {
    let route_source = include_str!("../src/routes/v1/site_publications.rs",);

    let router_source = include_str!("../src/routes/v1/mod.rs",);

    let compact_router: String = router_source.split_whitespace().collect();

    for required in [
        "pubmodsite_publications;",
        "/site-publications",
        "site_publications::list_site_publications",
    ] {
        assert!(
            compact_router.contains(required,),
            "missing Omnigate Site-publication route fragment: {required}",
        );
    }

    for required in [
        "/v1/index/site-publications",
        "siteCrabUrl",
        "cursor",
        "limit",
        "proxy only",
    ] {
        assert!(
            route_source.contains(required,),
            "missing Site-publication proxy fragment: {required}",
        );
    }

    for forbidden in [
        "verified_creator_username",
        "profile_url",
        "passport_ownership_claim",
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "moderation_mutation",
        "settlement_authority",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
        "quickchain_finality",
        "rox_settlement",
        "solana_mutation",
    ] {
        assert_eq!(
            route_source.contains(forbidden,),
            false,
            "forbidden Site-publication read authority: {forbidden}",
        );
    }
}

async fn start_omnigate_with_current_env() -> SocketAddr {
    let router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind Omnigate");

    let address = listener.local_addr().expect("Omnigate address");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve Omnigate");
    });

    address
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| value.is_empty() == false)
        .map(ToOwned::to_owned)
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");

    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
}
