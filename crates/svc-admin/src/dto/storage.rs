// crates/svc-admin/src/dto/storage.rs
//
// RO:WHAT — Curated storage/DB inventory DTOs for svc-admin.
// RO:WHY  — SPA expects stable, operator-friendly shapes regardless of node internals.
// RO:SOURCE — proxied from node admin-plane storage endpoints when available.
// RO:INVARIANTS —
//   - camelCase JSON for SPA.
//   - Sizes are bytes (u64).
//   - Optional enrichment fields are allowed (UI can ignore).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePermissionDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>, // e.g. "0755"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>, // e.g. "ron" or "uid:501"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>, // e.g. "ron"

    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_readable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_writable: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBandwidthDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_bytes_per_sec: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_bytes_per_sec: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub iops: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummaryDto {
    // ---- SPA contract (v1) ----
    pub fs_type: String,
    pub mount: String,

    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_read_bps: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_write_bps: Option<u64>,

    // ---- Optional enrichment (safe to add; UI can ignore) ----
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<FilePermissionDto>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<StorageBandwidthDto>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseEntryDto {
    // ---- SPA contract (v1) ----
    pub name: String,
    pub engine: String,
    pub size_bytes: u64,

    pub mode: String,
    pub owner: String,

    /// "ok" | "degraded" | "error"
    pub health: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_readable: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_writable: Option<bool>,

    // ---- Optional enrichment ----
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseDetailDto {
    // ---- SPA contract (v1) ----
    pub name: String,
    pub engine: String,
    pub size_bytes: u64,

    pub mode: String,
    pub owner: String,

    /// "ok" | "degraded" | "error"
    pub health: String,

    pub path_alias: String,

    pub file_count: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_compaction: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub approx_keys: Option<u64>,

    pub warnings: Vec<String>,

    // ---- Optional enrichment ----
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<FilePermissionDto>,
}
