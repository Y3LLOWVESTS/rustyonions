use crate::http::extractors::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};

pub async fn handler(State(app): State<AppState>, body: bytes::Bytes) -> Response {
    super::put_object::handler(State(app), body)
        .await
        .into_response()
}
