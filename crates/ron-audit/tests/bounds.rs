/*!
RO:WHAT — Bounds checking for attrs and full record sizes.
RO:WHY — PERF/RES: protect sinks from unbounded payloads.
RO:INTERACTS — ron_audit::bounds; dto::AuditRecord; serde_json.
RO:INVARIANTS — attrs ≤ configured max; record ≤ configured max.
RO:TEST HOOKS — Unit tests here; fuzz will hit more attr shapes later.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::bounds;
use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::{AuditRecord, BoundsError};
use serde_json::json;

fn base_record() -> AuditRecord {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    AuditRecord {
        v: 1,
        ts_ms,
        writer_id: "writer@test".to_string(),
        seq: 0,
        stream: "stream@test".to_string(),
        kind: AuditKind::Unknown,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("bounds-test".to_string()),
        attrs: json!({ "ok": true }),
        prev: "b3:0".to_string(),
        self_hash: "b3:placeholder".to_string(),
    }
}

#[test]
fn small_record_respects_default_bounds() {
    let rec = base_record();
    bounds::check(
        &rec,
        bounds::DEFAULT_MAX_ATTRS_BYTES,
        bounds::DEFAULT_MAX_RECORD_BYTES,
    )
    .expect("small record should pass bounds");
}

#[test]
fn oversized_attrs_are_rejected() {
    let mut rec = base_record();
    let big = "x".repeat(bounds::DEFAULT_MAX_ATTRS_BYTES + 1);
    rec.attrs = json!(big);

    let err = bounds::check(
        &rec,
        bounds::DEFAULT_MAX_ATTRS_BYTES,
        bounds::DEFAULT_MAX_RECORD_BYTES,
    )
    .expect_err("attrs beyond limit must be rejected");

    match err {
        BoundsError::AttrsTooLarge { .. } => {}
        other => panic!("expected AttrsTooLarge, got {other:?}"),
    }
}

#[test]
fn oversized_record_is_rejected() {
    let mut rec = base_record();
    // Make the record body large via a long reason string.
    rec.reason = ReasonCode("x".repeat(5_000));

    let err = bounds::check(&rec, bounds::DEFAULT_MAX_ATTRS_BYTES, 512)
        .expect_err("record beyond limit must be rejected");

    match err {
        BoundsError::RecordTooLarge { .. } => {}
        other => panic!("expected RecordTooLarge, got {other:?}"),
    }
}
