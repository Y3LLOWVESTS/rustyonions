//! RO:WHAT — Generic receipt type for mutating operations.
//! RO:WHY  — Provide durable handle for async/queued mutations without picking a backend yet.
//! RO:INVARIANTS — Opaque ids; monotonic timestamps; status is conservative.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub type ReceiptId = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReceiptStatus {
    Accepted,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: ReceiptId,
    pub status: ReceiptStatus,
    pub created_ms: u128,
    pub last_update_ms: u128,
}

impl Receipt {
    pub fn new(id: ReceiptId) -> Self {
        let now = now_ms();
        Self {
            id,
            status: ReceiptStatus::Accepted,
            created_ms: now,
            last_update_ms: now,
        }
    }
    pub fn set_status(&mut self, s: ReceiptStatus) {
        self.status = s;
        self.last_update_ms = now_ms();
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
