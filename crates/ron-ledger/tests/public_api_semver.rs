//! RO:WHAT — Smoke test for the intended public API surface.
//! RO:WHY  — Pillar 12; Concerns: GOV/DX. Catch accidental drift in the crate's public imports.
//! RO:INTERACTS — ron_ledger public reexports.
//! RO:INVARIANTS — expected types remain reachable from the crate root.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — none.
//! RO:TEST — integration test.

use ron_ledger::{
    AccumulatorKind, EngineMode, LedgerConfig, LedgerError, PqMode, RejectReason, Root, Seq,
};

#[test]
fn public_surface_smoke() {
    let cfg = LedgerConfig::default();
    assert!(matches!(
        cfg.engine_mode,
        EngineMode::Amnesia | EngineMode::Persistent
    ));
    assert!(matches!(
        cfg.accumulator_kind,
        AccumulatorKind::Merkle | AccumulatorKind::Verkle
    ));
    assert!(matches!(cfg.pq_mode, PqMode::Off | PqMode::Hybrid));
    let reason = RejectReason::Invalid;
    assert_eq!(reason.as_str(), "invalid");
    let root = Root::zero();
    assert_eq!(root.to_hex().len(), 64);
    let seq = Seq(1);
    assert_eq!(seq.get(), 1);
    let err = LedgerError::reject(RejectReason::Conflict, "boom");
    assert_eq!(err.reject_reason(), Some(RejectReason::Conflict));
}
