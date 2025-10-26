//! RO:WHAT — Readiness/health gate
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct HealthGate(Arc<RwLock<State>>);

#[derive(Default)]
struct State {
    pub listeners_bound: bool,
    pub metrics_bound: bool,
    pub cfg_loaded: bool,
    pub queues_ok: bool,
    pub shed_rate_ok: bool,
    pub fd_headroom: bool,
}

impl HealthGate {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(State::default())))
    }

    pub fn set_listeners_bound(&self, v: bool) {
        self.0.write().listeners_bound = v;
    }
    pub fn set_metrics_bound(&self, v: bool) {
        self.0.write().metrics_bound = v;
    }
    pub fn set_cfg_loaded(&self, v: bool) {
        self.0.write().cfg_loaded = v;
    }
    pub fn set_queues_ok(&self, v: bool) {
        self.0.write().queues_ok = v;
    }
    pub fn set_shed_rate_ok(&self, v: bool) {
        self.0.write().shed_rate_ok = v;
    }
    pub fn set_fd_headroom(&self, v: bool) {
        self.0.write().fd_headroom = v;
    }

    pub fn readyz_state(&self) -> (u16, serde_json::Value) {
        let s = self.0.read();
        let ok = s.listeners_bound
            && s.metrics_bound
            && s.cfg_loaded
            && s.queues_ok
            && s.shed_rate_ok
            && s.fd_headroom;

        if ok {
            (200, serde_json::json!({"ready": true}))
        } else {
            let mut missing = vec![];
            if !s.listeners_bound {
                missing.push("listeners_bound");
            }
            if !s.metrics_bound {
                missing.push("metrics_bound");
            }
            if !s.cfg_loaded {
                missing.push("cfg_loaded");
            }
            if !s.queues_ok {
                missing.push("queues_ok");
            }
            if !s.shed_rate_ok {
                missing.push("shed_rate_ok");
            }
            if !s.fd_headroom {
                missing.push("fd_headroom");
            }

            (
                503,
                serde_json::json!({
                    "ready": false,
                    "degraded": true,
                    "missing": missing,
                    "retry_after": 5
                }),
            )
        }
    }

    pub fn healthz(&self) -> (u16, serde_json::Value) {
        (200, serde_json::json!({"ok": true}))
    }
}

// ✅ Satisfy clippy: `new_without_default`
impl Default for HealthGate {
    fn default() -> Self {
        Self::new()
    }
}
