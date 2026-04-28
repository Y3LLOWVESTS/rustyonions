//! RO:WHAT — HTTP DTOs for svc-rewarder v1 routes.
//! RO:WHY — Pillar 12; Concerns: DX/ECON/GOV. Stable JSON contract for compute and inspect routes.
//! RO:INTERACTS — inputs DTOs, outputs manifests, http handlers.
//! RO:INVARIANTS — serde deny_unknown_fields; amounts are decimal strings; no floats.
//! RO:METRICS — schema rejects counted by handler/error layer.
//! RO:CONFIG — defaults policy_id from Config when needed.
//! RO:SECURITY — DTOs contain no secrets; Authorization is header-only.
//! RO:TEST — integration/http_compute.rs.

use serde::{Deserialize, Serialize};

use crate::inputs::{AccountingSnapshot, RewardPolicy};
use crate::outputs::RewardManifest;

/// POST /rewarder/epochs/{epoch_id}/compute request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputeEpochRequest {
    /// Sealed input snapshot CID.
    pub inputs_cid: String,
    /// Policy id expected by caller.
    pub policy_id: String,
    /// Policy hash expected by caller.
    pub policy_hash: String,
    /// True to compute without settlement egress.
    #[serde(default)]
    pub dry_run: bool,
    /// Optional operator notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Inline snapshot for batch-1 local/dev compute.
    #[serde(default)]
    pub snapshot: Option<AccountingSnapshot>,
    /// Inline policy for batch-1 local/dev compute.
    #[serde(default)]
    pub policy: Option<RewardPolicy>,
}

/// Compute response.
pub type ComputeEpochResponse = RewardManifest;

/// GET /rewarder/epochs/{epoch_id} response.
pub type GetEpochResponse = RewardManifest;

/// Build/version response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VersionResponse {
    /// Package name.
    pub name: &'static str,
    /// Package version.
    pub version: &'static str,
    /// Build features.
    pub features: Vec<&'static str>,
}
