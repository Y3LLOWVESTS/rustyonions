//! RO:WHAT — Simple correlation id generator/injector for responses.

use axum::{http::HeaderValue, response::Response};
use ulid::Ulid;

pub fn add_corr_id(mut r: Response) -> Response {
    let id = Ulid::new().to_string();
    r.headers_mut()
        .insert("x-corr-id", HeaderValue::from_str(&id).unwrap());
    r
}
