//! RO:WHAT — Typed configuration schema for ryker.
//! RO:WHY  — Deterministic defaults + strict validation (docs-aligned).
//! RO:INTERACTS — loader merges sources; runtime/mailbox read snapshots.
//! RO:INVARIANTS — capacity>0; 0 < deadline ≤ 60s; max_msg_bytes ≤ 1MiB; yield_every_n ≥ batch.

use crate::errors::{ConfigError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RykerConfig {
    pub defaults: Defaults,
    pub fairness: FairnessCfg,
    pub supervisor: SupervisionCfg,
    pub amnesia: bool,
    pub observe: ObserveCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    pub mailbox_capacity: usize,
    pub max_msg_bytes: usize,
    #[serde(with = "humantime_serde")]
    pub deadline: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FairnessCfg {
    pub batch_messages: usize,
    pub yield_every_n_msgs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionCfg {
    pub backoff_base_ms: u64,
    pub backoff_cap_ms: u64,
    pub decorrelated_jitter: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObserveCfg {
    /// If true, ryker may sample queue depths via observer hooks.
    pub queue_depth_sampling: bool,
}

impl Default for RykerConfig {
    fn default() -> Self {
        Self {
            defaults: Defaults {
                mailbox_capacity: 256,
                max_msg_bytes: 64 * 1024,
                deadline: Duration::from_secs(1),
            },
            fairness: FairnessCfg {
                batch_messages: 32,
                yield_every_n_msgs: 64,
            },
            supervisor: SupervisionCfg {
                backoff_base_ms: 100,
                backoff_cap_ms: 5_000,
                decorrelated_jitter: true,
            },
            amnesia: false,
            observe: ObserveCfg {
                queue_depth_sampling: true,
            },
        }
    }
}

impl RykerConfig {
    pub fn validate(&self) -> Result<()> {
        // mailbox must have capacity
        if self.defaults.mailbox_capacity == 0 {
            return Err(ConfigError::Invalid("mailbox_capacity=0".into()).into());
        }

        // Upper bound aligns with crate docs (≤ 1 MiB)
        const MIB: usize = 1024 * 1024;
        if self.defaults.max_msg_bytes > MIB {
            return Err(ConfigError::Invalid("max_msg_bytes > 1MiB".into()).into());
        }

        // 1 ms ..= 60 s recommended bounds
        if self.defaults.deadline < Duration::from_millis(1)
            || self.defaults.deadline > Duration::from_secs(60)
        {
            return Err(ConfigError::Invalid("deadline out of [1ms, 60s]".into()).into());
        }

        // yield_every_n must be ≥ batch size
        if self.fairness.yield_every_n_msgs < self.fairness.batch_messages {
            return Err(ConfigError::Invalid("yield_every_n_msgs < batch_messages".into()).into());
        }

        // backoff base/cap relationship must be coherent and non-zero
        if self.supervisor.backoff_base_ms == 0
            || self.supervisor.backoff_cap_ms == 0
            || self.supervisor.backoff_base_ms > self.supervisor.backoff_cap_ms
        {
            return Err(
                ConfigError::Invalid("invalid backoff base/cap relationship".into()).into(),
            );
        }
        Ok(())
    }

    /// Apply in-process overrides (builder-like) then revalidate.
    pub fn with_overrides<F: FnOnce(&mut RykerConfig)>(mut self, f: F) -> Result<RykerConfig> {
        f(&mut self);
        self.validate()?;
        Ok(self)
    }
}
