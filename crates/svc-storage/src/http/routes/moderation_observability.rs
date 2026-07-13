//! RO:WHAT — Bounded moderation-refusal observability for object-read routes.
//! RO:WHY — Phase 10 requires truthful refusal evidence without high-cardinality leakage.
//! RO:INTERACTS — legacy GET/HEAD handlers, OAP OBJ_GET handler, svc-storage metrics.
//! RO:INVARIANTS — closed route enum; reason labels come from ron-policy; no CID/path/IP labels.
//! RO:METRICS — storage_moderation_refusals_total{route,reason}.
//! RO:CONFIG — metrics behavior remains gated by the existing `metrics` feature.
//! RO:SECURITY — no identity, address, policy path, content ID, wallet, or ledger data.
//! RO:TEST — moderation_observability.

use ron_policy::ModerationReasonCode;

/// Closed set of object-read surfaces that can refuse on moderation grounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModerationReadRoute {
    OapObjGet,
    LegacyGet,
    LegacyHead,
}

#[cfg(feature = "metrics")]
impl ModerationReadRoute {
    const fn as_str(self) -> &'static str {
        match self {
            Self::OapObjGet => "oap_obj_get",
            Self::LegacyGet => "legacy_get",
            Self::LegacyHead => "legacy_head",
        }
    }
}

/// Record one refusal when metrics are enabled; otherwise remain a no-op.
pub(super) fn observe_refusal(route: ModerationReadRoute, reason: ModerationReasonCode) {
    #[cfg(feature = "metrics")]
    crate::metrics::observe_moderation_refusal(route.as_str(), reason);

    #[cfg(not(feature = "metrics"))]
    let _ = (route, reason);
}
