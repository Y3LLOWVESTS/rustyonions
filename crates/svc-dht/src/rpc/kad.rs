//! RO:WHAT — Kad request/response DTOs and handlers (placeholder)
//! RO:WHY — Wire surface; Concerns: DX/RES
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FindProviders {
    pub cid: String,
    pub limit: usize,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Providers {
    pub cid: String,
    pub nodes: Vec<String>,
}
