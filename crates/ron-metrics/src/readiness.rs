//! RO:WHAT — Ready JSON schema + policy helpers.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

#[derive(Clone, Copy)]
pub struct ReadyPolicy {
    pub retry_after_secs: u64,
}
impl Default for ReadyPolicy {
    fn default() -> Self {
        Self {
            retry_after_secs: 5,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadyJson {
    pub degraded: bool,
    #[serde(default)]
    pub missing: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
}

pub fn make_ready_json(
    all_ready: bool,
    missing: Vec<String>,
    policy: ReadyPolicy,
    since: SystemTime,
) -> ReadyJson {
    let since_secs = since
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();

    if all_ready {
        ReadyJson {
            degraded: false,
            missing: Vec::new(),
            retry_after: None,
            since: Some(since_secs),
        }
    } else {
        ReadyJson {
            degraded: true,
            missing,
            retry_after: Some(policy.retry_after_secs),
            since: Some(since_secs),
        }
    }
}
