use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use parking_lot::RwLock;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounterVec, Opts, TextEncoder};
use ron_ledger::{AccountId, InMemoryLedger, TokenError, TokenLedger};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct AppState {
    started: SystemTime,
    ready: Arc<AtomicBool>,
    ledger: Arc<RwLock<InMemoryLedger>>,
    metrics: Arc<Metrics>,
    service_name: &'static str,
    version: &'static str,
}

struct Metrics {
    tx_total: IntCounterVec,
    tx_failed_total: IntCounterVec,
    request_latency_seconds: Histogram,
}

impl Metrics {
    fn new() -> Self {
        let tx_total = IntCounterVec::new(
            Opts::new("tx_total", "Total successful token operations"),
            &["op"],
        )
        .expect("tx_total");
        let tx_failed_total = IntCounterVec::new(
            Opts::new("tx_failed_total", "Total failed token operations"),
            &["op", "reason"],
        )
        .expect("tx_failed_total");
        let request_latency_seconds =
            Histogram::with_opts(HistogramOpts::new("request_latency_seconds", "Request latency (seconds)"))
                .expect("request_latency_seconds");

        prometheus::register(Box::new(tx_total.clone())).ok();
        prometheus::register(Box::new(tx_failed_total.clone())).ok();
        prometheus::register(Box::new(request_latency_seconds.clone())).ok();

        Self {
            tx_total,
            tx_failed_total,
            request_latency_seconds,
        }
    }
}

#[derive(Serialize)]
struct StatusPayload<'a> {
    service: &'a str,
    version: &'a str,
    ok: bool,
    uptime_secs: u64,
}

#[derive(Deserialize)]
struct MintReq {
    account: String,
    amount: u128,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct BurnReq {
    account: String,
    amount: u128,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct TransferReq {
    from: String,
    to: String,
    amount: u128,
    reason: Option<String>,
}

#[derive(Serialize)]
struct BalanceResp {
    account: String,
    balance: u128,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let bind: SocketAddr = std::env::var("ECONOMY_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3003".to_string())
        .parse()
        .expect("ECONOMY_ADDR must be host:port");

    let state = AppState {
        started: SystemTime::now(),
        ready: Arc::new(AtomicBool::new(false)),
        ledger: Arc::new(RwLock::new(InMemoryLedger::new())),
        metrics: Arc::new(Metrics::new()),
        service_name: "svc-economy",
        version: env!("CARGO_PKG_VERSION"),
    };

    let app = Router::new()
        // Public API
        .route("/mint", post(post_mint))
        .route("/burn", post(post_burn))
        .route("/transfer", post(post_transfer))
        .route("/balance/:account", get(get_balance))
        .route("/supply", get(get_supply))
        // Ops
        .route("/", get(root))
        .route("/version", get(version))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(metrics))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!("svc-economy listening on http://{bind}");

    state.ready.store(true, Ordering::SeqCst);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("svc-economy shutdown complete");
    Ok(())
}

fn init_tracing() {
    // Respect RUST_LOG if provided, default to info
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_target(false)
        .with_timer(fmt::time::uptime())
        .with_max_level(Level::INFO)
        .with_env_filter(env_filter)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received Ctrl-C, shutting down…");
}

async fn root(State(st): State<AppState>) -> impl IntoResponse {
    let up = st.started.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    let payload = StatusPayload {
        service: st.service_name,
        version: st.version,
        ok: true,
        uptime_secs: up,
    };
    (StatusCode::OK, Json(payload))
}

async fn version(State(st): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "service": st.service_name,
        "version": st.version
    })))
}

async fn healthz() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn readyz(State(st): State<AppState>) -> impl IntoResponse {
    let ok = st.ready.load(Ordering::SeqCst);
    let code = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (code, Json(serde_json::json!({ "ready": ok })))
}

async fn metrics() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let mut buf = Vec::new();
    let encoder = TextEncoder::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buf) {
        let body = format!("encode error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, body).into_response();
    }
    (StatusCode::OK, String::from_utf8_lossy(&buf).to_string()).into_response()
}

// ---------- API handlers ----------

async fn post_mint(State(st): State<AppState>, Json(req): Json<MintReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "mint";
    let result = {
        let mut l = st.ledger.write();
        l.mint(AccountId(req.account.clone()), req.amount, req.reason)
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn post_burn(State(st): State<AppState>, Json(req): Json<BurnReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "burn";
    let result = {
        let mut l = st.ledger.write();
        l.burn(AccountId(req.account.clone()), req.amount, req.reason)
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn post_transfer(State(st): State<AppState>, Json(req): Json<TransferReq>) -> impl IntoResponse {
    let timer = st.metrics.request_latency_seconds.start_timer();
    let op = "transfer";
    let result = {
        let mut l = st.ledger.write();
        l.transfer(
            AccountId(req.from.clone()),
            AccountId(req.to.clone()),
            req.amount,
            req.reason,
        )
    };
    let resp = match result {
        Ok(r) => {
            st.metrics.tx_total.with_label_values(&[op]).inc();
            (StatusCode::OK, Json(serde_json::json!({ "receipt": r }))).into_response()
        }
        Err(e) => fail(st.metrics.as_ref(), op, &e),
    };
    timer.observe_duration();
    resp
}

async fn get_balance(State(st): State<AppState>, Path(account): Path<String>) -> impl IntoResponse {
    let l = st.ledger.read();
    let bal = l.balance(&AccountId(account.clone()));
    (StatusCode::OK, Json(BalanceResp { account, balance: bal }))
}

async fn get_supply(State(st): State<AppState>) -> impl IntoResponse {
    let l = st.ledger.read();
    (StatusCode::OK, Json(serde_json::json!({ "total_supply": l.total_supply() })))
}

// ---------- helpers ----------

fn fail(metrics: &Metrics, op: &str, e: &dyn std::error::Error) -> axum::response::Response {
    let (code, reason) = classify(e);
    metrics.tx_failed_total.with_label_values(&[op, reason]).inc();
    (code, Json(serde_json::json!({ "error": reason }))).into_response()
}

fn classify(e: &dyn std::error::Error) -> (StatusCode, &'static str) {
    if let Some(te) = e.downcast_ref::<TokenError>() {
        match te {
            TokenError::ZeroAmount => (StatusCode::BAD_REQUEST, "zero_amount"),
            TokenError::InsufficientFunds { .. } => (StatusCode::BAD_REQUEST, "insufficient_funds"),
            TokenError::Overflow => (StatusCode::INTERNAL_SERVER_ERROR, "overflow"),
        }
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal")
    }
}
