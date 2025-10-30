//! RO:WHAT — DTOs for HTTP responses/requests.
//! RO:WHY  — Interop hygiene; `#[serde(deny_unknown_fields)]`.
//! RO:INVARIANTS — b3: hex shape; stable error taxonomy.
//! RO:SECURITY — no secrets in payloads.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolveResponse {
    pub key: String,              // name:* or b3:<hex>
    pub manifest: Option<String>, // usually a CID of a manifest
    pub providers: Vec<ProviderEntry>,
    pub etag: Option<String>, // "b3:<hex>" when applicable
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderEntry {
    pub id: String, // provider id (e.g., node addr)
    pub region: Option<String>,
    pub score: f32, // ranking hint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidersResponse {
    pub cid: String, // b3:<hex>
    pub providers: Vec<ProviderEntry>,
    pub truncated: bool,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub code: String, // "not_found" | "over_capacity" | ...
    pub message: String,
}
