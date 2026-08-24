//! RO:WHAT — Dependency and capability projection for CrabNode `/readyz`.
//! RO:WHY — CN-3 readiness must describe usable product capabilities, not merely listeners.
//! RO:INTERACTS — ReadySnapshot, admin `/readyz`, CrabNode operator diagnostics.
//! RO:INVARIANTS — required planes never report ready from scheduling alone; unfinished optional planes do not overclaim capability.
//! RO:METRICS — projection only; no new collectors.
//! RO:CONFIG — no independent configuration authority.
//! RO:SECURITY — status labels only; no secrets or private authority.
//! RO:TEST — readiness unit test and crabnode_cn3_ingress.rs.

use serde::Serialize;

use super::probes::ReadySnapshot;

#[derive(Serialize)]
pub(super) struct ReadyDeps<'a> {
    pub(super) config: &'a str,
    pub(super) network: &'a str,
    pub(super) gateway: &'a str,
    pub(super) omnigate: &'a str,
    pub(super) passport: &'a str,
    pub(super) storage: &'a str,
    pub(super) index: &'a str,
    pub(super) overlay: &'a str,
    pub(super) mailbox: &'a str,
    pub(super) dht: &'a str,
}

#[derive(Serialize)]
pub(super) struct ReadyCapabilities<'a> {
    pub(super) operator_core: &'a str,
    pub(super) product_ingress: &'a str,
    pub(super) content_store: &'a str,
    pub(super) identity: &'a str,
    pub(super) discovery: &'a str,
    pub(super) peer_transport: &'a str,
    pub(super) messaging: &'a str,
    pub(super) economic_participation: &'a str,
}

#[derive(Serialize)]
pub(super) struct ReadyBody<'a> {
    pub(super) ready: bool,
    pub(super) deps: ReadyDeps<'a>,
    pub(super) capabilities: ReadyCapabilities<'a>,
    pub(super) mode: &'a str,
}

impl<'a> ReadyDeps<'a> {
    #[must_use]
    pub(super) fn from_snapshot(snap: &'a ReadySnapshot) -> Self {
        Self {
            config: if snap.cfg_loaded { "loaded" } else { "pending" },

            network: if snap.listeners_bound {
                "ok"
            } else {
                "pending"
            },

            gateway: if snap.gateway_bound { "ok" } else { "pending" },

            omnigate: if snap.omnigate_bound { "ok" } else { "pending" },

            passport: if snap.passport_bound { "ok" } else { "pending" },

            storage: if snap.storage_bound { "ok" } else { "pending" },

            index: if snap.index_bound { "ok" } else { "pending" },

            overlay: if snap.overlay_bound { "ok" } else { "pending" },

            mailbox: if snap.mailbox_bound { "ok" } else { "pending" },

            dht: if snap.dht_bound { "ok" } else { "pending" },
        }
    }
}

impl<'a> ReadyCapabilities<'a> {
    #[must_use]
    pub(super) fn from_snapshot(snap: &'a ReadySnapshot) -> Self {
        let operator_core = if snap.listeners_bound && snap.cfg_loaded && snap.deps_ok {
            "ready"
        } else {
            "degraded"
        };

        let product_ingress = if snap.gateway_bound && snap.omnigate_bound {
            "ready"
        } else {
            "degraded"
        };

        let content_store = if snap.storage_bound && snap.index_bound {
            "ready"
        } else {
            "degraded"
        };

        let identity = if snap.passport_bound {
            "ready"
        } else {
            "degraded"
        };

        let discovery = if snap.dht_bound {
            "degraded"
        } else {
            "disabled"
        };

        Self {
            operator_core,
            product_ingress,
            content_store,
            identity,
            discovery,
            peer_transport: "disabled",
            messaging: "disabled",
            economic_participation: "disabled",
        }
    }
}

impl<'a> ReadyBody<'a> {
    #[must_use]
    pub(super) fn new(
        ready: bool,
        deps: ReadyDeps<'a>,
        capabilities: ReadyCapabilities<'a>,
        mode: &'a str,
    ) -> Self {
        Self {
            ready,
            deps,
            capabilities,
            mode,
        }
    }
}
