use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssueRequest {
    pub sub: String,
    pub aud: Vec<String>,
    pub scopes: Vec<String>,
    #[serde(default)]
    pub ctx: Option<Value>,
    #[serde(default)]
    pub ttl_s: Option<u64>,
    #[serde(default)]
    pub nbf: Option<i64>,
    #[serde(default)]
    pub nonce: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IssueResponse {
    pub envelope: super::verify::Envelope,
}
