use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::http::extractors::AppState;

pub async fn handler(State(app): State<AppState>, Path(cid): Path<String>) -> impl IntoResponse {
    match app.store.head(&cid).await {
        Ok(meta) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::ETAG,
                HeaderValue::from_str(&meta.etag).unwrap_or(HeaderValue::from_static("")),
            );
            headers.insert(
                axum::http::header::CONTENT_LENGTH,
                HeaderValue::from_str(&meta.len.to_string()).unwrap(),
            );
            (StatusCode::OK, headers).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, ()).into_response(),
    }
}
