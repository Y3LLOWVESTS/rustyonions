// crates/micronode/src/layers/decode_guard.rs
//! RO:WHAT — Simple decode policy guard: reject any Content-Encoding and stacked encodings.
//! RO:WHY  — We don't transparently decompress; callers must send identity bodies.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Stateless decode guard: 415 on any Content-Encoding; 415 on stacked encodings ("," in header).
pub async fn guard(req: Request<Body>, next: Next) -> Response {
    match req.headers().get(header::CONTENT_ENCODING) {
        None => next.run(req).await,
        Some(hv) => {
            let enc = match hv.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "invalid Content-Encoding header")
                        .into_response();
                }
            };

            if enc.contains(',') {
                return (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "stacked content encodings are not supported",
                )
                    .into_response();
            }

            (StatusCode::UNSUPPORTED_MEDIA_TYPE, "compressed request bodies are not supported")
                .into_response()
        }
    }
}
