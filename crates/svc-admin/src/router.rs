use crate::dto;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/ui-config", get(ui_config))
        .route("/api/me", get(me))
        .route("/api/nodes", get(nodes))
        .route("/api/nodes/:id/status", get(node_status))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ready": true }))
}

async fn ui_config(State(state): State<Arc<AppState>>) -> Json<dto::ui::UiConfigDto> {
    Json(dto::ui::UiConfigDto::from_cfg(&state.config))
}

async fn me() -> Json<dto::me::MeResponse> {
    Json(dto::me::MeResponse::dev_default())
}

async fn nodes(State(_state): State<Arc<AppState>>) -> Json<Vec<dto::node::NodeSummary>> {
    Json(vec![]) // TODO: list nodes from registry
}

async fn node_status(
    State(_state): State<Arc<AppState>>,
) -> Json<dto::node::AdminStatusView> {
    Json(dto::node::AdminStatusView::placeholder())
}
