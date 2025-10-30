//! /metrics

use crate::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn metrics(State(state): State<Arc<AppState>>) -> Response {
    match state.metrics.render() {
        Ok(s) => s.into_response(),
        Err(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
