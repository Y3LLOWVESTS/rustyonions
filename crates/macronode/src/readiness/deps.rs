// crates/macronode/src/readiness/deps.rs

//! RO:WHAT — JSON shapes and helpers for `/readyz` dependency reporting.
//! RO:WHY  — Keep HTTP response wiring separate from probe mechanics, while
//!           preserving a stable JSON contract for tests and operators.
//!
//! High-level JSON shape:
//!   {
//!     "ready": bool,
//!     "deps": {
//!       "config":  "loaded" | "pending",
//!       "network": "ok" | "pending",
//!       "gateway": "ok" | "pending",
//!       "storage": "ok" | "pending",
//!       "index":   "ok" | "pending",
//!       "overlay": "ok" | "pending",
//!       "mailbox": "ok" | "pending",
//!       "dht":     "ok" | "pending"
//!     },
//!     "mode": "truthful" | "dev-forced"
//!   }

use serde::Serialize;

use super::probes::ReadySnapshot;

/// Dependency state block for `/readyz`.
#[derive(Serialize)]
pub(super) struct ReadyDeps<'a> {
    pub(super) config: &'a str,
    pub(super) network: &'a str,
    pub(super) gateway: &'a str,
    pub(super) storage: &'a str,
    pub(super) index: &'a str,
    pub(super) overlay: &'a str,
    pub(super) mailbox: &'a str,
    pub(super) dht: &'a str,
}

/// Top-level `/readyz` response body.
#[derive(Serialize)]
pub(super) struct ReadyBody<'a> {
    pub(super) ready: bool,
    pub(super) deps: ReadyDeps<'a>,
    pub(super) mode: &'a str,
}

impl<'a> ReadyDeps<'a> {
    /// Construct dependency view from a snapshot.
    ///
    /// Mapping:
    ///   - config  ← cfg_loaded → "loaded"/"pending"
    ///   - network ← listeners_bound → "ok"/"pending"
    ///   - gateway ← gateway_bound → "ok"/"pending"
    ///   - storage ← deps_ok → "ok"/"pending"
    ///   - index   ← index_bound → "ok"/"pending"
    ///   - overlay ← overlay_bound → "ok"/"pending"
    ///   - mailbox ← mailbox_bound → "ok"/"pending"
    ///   - dht     ← dht_bound → "ok"/"pending"
    #[must_use]
    pub(super) fn from_snapshot(snap: &'a ReadySnapshot) -> Self {
        ReadyDeps {
            config: if snap.cfg_loaded { "loaded" } else { "pending" },
            network: if snap.listeners_bound {
                "ok"
            } else {
                "pending"
            },
            gateway: if snap.gateway_bound { "ok" } else { "pending" },
            storage: if snap.deps_ok { "ok" } else { "pending" },
            index: if snap.index_bound { "ok" } else { "pending" },
            overlay: if snap.overlay_bound { "ok" } else { "pending" },
            mailbox: if snap.mailbox_bound { "ok" } else { "pending" },
            dht: if snap.dht_bound { "ok" } else { "pending" },
        }
    }
}

impl<'a> ReadyBody<'a> {
    /// Helper to keep handler code small and readable.
    #[must_use]
    pub(super) fn new(ready: bool, deps: ReadyDeps<'a>, mode: &'a str) -> Self {
        ReadyBody { ready, deps, mode }
    }
}
