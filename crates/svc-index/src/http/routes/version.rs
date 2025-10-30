//! /version

pub async fn version() -> String {
    format!("svc-index/{}", env!("CARGO_PKG_VERSION"))
}
