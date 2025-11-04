//! GET /o/{addr}

use crate::headers::etag::etag_from_b3;
use axum::{extract::Path, response::IntoResponse};

pub async fn get_object(Path(addr): Path<String>) -> impl IntoResponse {
    // MVP: echo stub (no overlay forwarding yet).
    (
        [(http::header::ETAG, etag_from_b3(&addr))],
        axum::body::Body::from(format!("object stub for {}", addr)),
    )
}
