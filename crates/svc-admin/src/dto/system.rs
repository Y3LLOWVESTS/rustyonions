// crates/svc-admin/src/dto/system.rs
//
// RO:WHAT — System summary DTO (CPU/RAM + optional net rates) proxied from a node.
// RO:WHY  — SPA can render truthful utilization tiles without mocks/scraping.
// RO:INVARIANTS —
//   - camelCase JSON for SPA
//   - bytes are bytes (u64)
//   - optional rates may be None

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSummaryDto {
    pub updated_at: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,

    pub ram_total_bytes: u64,
    pub ram_used_bytes: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_rx_bps: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_tx_bps: Option<u64>,
}
