// DTOs (stub)
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ComputeRequest {
    pub inputs_cid: String,
    pub policy_id: String,
    pub policy_hash: String,
}
