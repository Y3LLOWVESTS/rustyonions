// RO:WHAT — Focused integration proof for Omnigate Explore discovery proxy.
// RO:WHY — Locks strict query forwarding, upstream passthrough, and read-only authority before gateway exposure.
// RO:INTERACTS — real Omnigate v1 router plus dummy svc-index Explore endpoint.
// RO:INVARIANTS — default and explicit bounded limits reach svc-index exactly; upstream bodies/status pass through.
// RO:SECURITY — no social graph, ranking, wallet, ledger, receipt, entitlement, QuickChain, ROX, or Solana authority.

use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{net::TcpListener, sync::Mutex};

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct IndexQuery {
    publication_limit: usize,

    creator_limit: usize,

    site_limit: usize,
}

#[derive(Debug, Clone, Default)]
struct IndexState {
    last_query: Arc<Mutex<Option<IndexQuery>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryFixture {
    schema: &'static str,

    recent_publications: Vec<Value>,

    public_creators: Vec<Value>,

    template_sites: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct ErrorFixture {
    code: &'static str,

    message: &'static str,
}

fn ensure(condition: bool, message: &str) -> TestResult {
    if condition {
        return Ok(());
    }

    Err(std::io::Error::other(message).into())
}

async fn dummy_explore(
    State(state): State<IndexState>,
    Query(query): Query<IndexQuery>,
) -> Json<DiscoveryFixture> {
    *state.last_query.lock().await = Some(query);

    Json(DiscoveryFixture {
        schema: "crablink.explore-discovery.v1",

        recent_publications: Vec::new(),

        public_creators: Vec::new(),

        template_sites: Vec::new(),
    })
}

async fn dummy_not_found() -> (StatusCode, Json<ErrorFixture>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorFixture {
            code: "not_found",

            message: "discovery projection unavailable",
        }),
    )
}

async fn spawn_router(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test router");

    let address = listener.local_addr().expect("test router address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve test router");
    });

    address
}

async fn start_stack() -> (String, IndexState) {
    clear_env();

    let state = IndexState::default();

    let index_router = Router::new()
        .route("/v1/index/explore", get(dummy_explore))
        .with_state(state.clone());

    let index_address = spawn_router(index_router).await;

    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        ["http://", &index_address.to_string()].concat(),
    );

    let omnigate_router = Router::new().nest("/v1", omnigate::routes::v1::router());

    let omnigate_address = spawn_router(omnigate_router).await;

    tokio::time::sleep(Duration::from_millis(40)).await;

    (["http://", &omnigate_address.to_string()].concat(), state)
}

#[tokio::test]
async fn phase10a3c_defaults_are_forwarded_to_svc_index() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    let (base_url, state) = start_stack().await;

    let response = reqwest::Client::new()
        .get([&base_url, "/v1/explore"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::OK,
        "default Explore request must succeed",
    )?;

    let query = state.last_query.lock().await.clone();

    ensure(
        query
            == Some(IndexQuery {
                publication_limit: 12,

                creator_limit: 12,

                site_limit: 8,
            }),
        "default Explore limits did not reach svc-index",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_explicit_bounded_limits_are_forwarded_exactly() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    let (base_url, state) = start_stack().await;

    let response = reqwest::Client::new()
        .get(
            [
                &base_url,
                "/v1/explore",
                "?publicationLimit=24",
                "&creatorLimit=20",
                "&siteLimit=16",
            ]
            .concat(),
        )
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::OK,
        "bounded Explore request must succeed",
    )?;

    let query = state.last_query.lock().await.clone();

    ensure(
        query
            == Some(IndexQuery {
                publication_limit: 24,

                creator_limit: 20,

                site_limit: 16,
            }),
        "explicit Explore limits changed in transit",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_out_of_bound_limit_fails_before_upstream_activity() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    let (base_url, state) = start_stack().await;

    let response = reqwest::Client::new()
        .get([&base_url, "/v1/explore", "?publicationLimit=25"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::BAD_REQUEST,
        "out of bound publication limit must reject",
    )?;

    ensure(
        state.last_query.lock().await.is_none(),
        "rejected query must not reach svc-index",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_unknown_query_field_fails_closed() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    let (base_url, state) = start_stack().await;

    let response = reqwest::Client::new()
        .get([&base_url, "/v1/explore", "?ranking=popular"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::BAD_REQUEST,
        "unknown Explore query must reject",
    )?;

    ensure(
        state.last_query.lock().await.is_none(),
        "unknown query must not reach svc-index",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_success_body_matches_svc_index_wire_shape() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    let (base_url, _state) = start_stack().await;

    let response = reqwest::Client::new()
        .get([&base_url, "/v1/explore"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::OK,
        "Explore success status mismatch",
    )?;

    let body: Value = response.json().await?;

    ensure(
        body.get("schema").and_then(Value::as_str) == Some("crablink.explore-discovery.v1"),
        "Explore schema changed through Omnigate",
    )?;

    ensure(
        body.get("recentPublications")
            .and_then(Value::as_array)
            .is_some(),
        "recentPublications missing",
    )?;

    ensure(
        body.get("publicCreators")
            .and_then(Value::as_array)
            .is_some(),
        "publicCreators missing",
    )?;

    ensure(
        body.get("templateSites")
            .and_then(Value::as_array)
            .is_some(),
        "templateSites missing",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_upstream_status_and_body_pass_through() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    let index_router = Router::new().route("/v1/index/explore", get(dummy_not_found));

    let index_address = spawn_router(index_router).await;

    std::env::set_var(
        "OMNIGATE_INDEX_BASE_URL",
        ["http://", &index_address.to_string()].concat(),
    );

    let omnigate_address =
        spawn_router(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let response = reqwest::Client::new()
        .get(["http://", &omnigate_address.to_string(), "/v1/explore"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::NOT_FOUND,
        "upstream status must pass through",
    )?;

    let body: Value = response.json().await?;

    ensure(
        body.get("code").and_then(Value::as_str) == Some("not_found"),
        "upstream body must pass through",
    )?;

    clear_env();

    Ok(())
}

#[tokio::test]
async fn phase10a3c_transport_failure_returns_stable_bad_gateway() -> TestResult {
    let _guard = ENV_LOCK.lock().await;

    clear_env();

    std::env::set_var("OMNIGATE_INDEX_BASE_URL", "http://127.0.0.1:9");

    let omnigate_address =
        spawn_router(Router::new().nest("/v1", omnigate::routes::v1::router())).await;

    let response = reqwest::Client::new()
        .get(["http://", &omnigate_address.to_string(), "/v1/explore"].concat())
        .send()
        .await?;

    ensure(
        response.status() == StatusCode::BAD_GATEWAY,
        "transport failure must return bad gateway",
    )?;

    let body: Value = response.json().await?;

    ensure(
        body.get("code").and_then(Value::as_str) == Some("index_upstream"),
        "bad gateway code mismatch",
    )?;

    ensure(
        body.get("retryable").and_then(Value::as_bool) == Some(true),
        "bad gateway retryable posture mismatch",
    )?;

    clear_env();

    Ok(())
}

#[test]
fn phase10a3c_route_surface_is_read_only_and_authority_free() -> TestResult {
    let route_source = std::fs::read_to_string("src/routes/v1/explore_discovery.rs")?;

    let router_source = std::fs::read_to_string("src/routes/v1/mod.rs")?;

    let compact_router = router_source.split_whitespace().collect::<String>();

    ensure(
        compact_router.contains("/explore"),
        "Omnigate Explore route missing",
    )?;

    ensure(
        compact_router.contains("explore_discovery::get_explore_discovery"),
        "Omnigate Explore handler missing",
    )?;

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "follow_mutation",
        "social_graph",
        "engagement_score",
        "paid_ranking",
        "quickchain_mutation",
        "rox_interaction",
        "solana_interaction",
    ] {
        ensure(
            route_source.contains(forbidden) == false,
            &["forbidden authority token: ", forbidden].concat(),
        )?;
    }

    ensure(
        route_source.contains("proxy only"),
        "Explore route must remain proxy only",
    )
}

fn clear_env() {
    std::env::remove_var("OMNIGATE_INDEX_BASE_URL");

    std::env::remove_var("OMNIGATE_DOWNSTREAM_INDEX_BASE_URL");
}
