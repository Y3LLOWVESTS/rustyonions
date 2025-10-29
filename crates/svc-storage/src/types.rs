use serde::Serialize;

/// Response body for a successful PUT.
#[derive(Debug, Clone, Serialize)]
pub struct PutResponse {
    pub cid: String,
}
