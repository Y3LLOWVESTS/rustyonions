use axum::{extract::State, response::IntoResponse, Json};
use blake3;
use serde::Serialize;

use crate::http::extractors::AppState;

#[derive(Serialize)]
struct PutResp {
    cid: String,
}

pub async fn handler(State(app): State<AppState>, body: bytes::Bytes) -> impl IntoResponse {
    // Compute b3 content id from the body (exactly what your script expects)
    let digest = blake3::hash(&body).to_hex().to_string();
    let cid = format!("b3:{digest}");

    // Store full body
    if let Err(e) = app.store.put(&cid, body).await {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("put failed: {e}"),
        )
            .into_response();
    }

    (axum::http::StatusCode::OK, Json(PutResp { cid })).into_response()
}
