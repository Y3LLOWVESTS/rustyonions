/*!
RO:WHAT — Canonicalization invariants (drop self_hash, NFC, no floats).
RO:WHY — Integrity: stable hash surface and replay idempotency.
RO:INTERACTS — ron_audit::canon; dto::AuditRecord; serde_json.
RO:INVARIANTS — self_hash ignored; strings NFC-normalized; floats rejected.
RO:TEST HOOKS — Unit tests here; fuzz covers attr edge cases later.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::canon::{canonicalize_without_self_hash, CanonError};
use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::AuditRecord;
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
        kind: AuditKind::CapIssued,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("canon-test".to_string()),
        attrs: json!({ "label": "ok" }),
        prev: "b3:0".to_string(),
        self_hash: "b3:placeholder".to_string(),
    }
}

#[test]
fn canonicalization_drops_self_hash() {
    let mut r1 = base_record();
    r1.self_hash = "b3:aaaa".to_string();

    let mut r2 = r1.clone();
    r2.self_hash = "b3:bbbb".to_string();

    let c1 = canonicalize_without_self_hash(&r1).expect("canon r1");
    let c2 = canonicalize_without_self_hash(&r2).expect("canon r2");

    assert_eq!(c1, c2, "self_hash must not affect canonical bytes");
}

#[test]
fn canonicalization_rejects_floats_in_attrs() {
    let mut r = base_record();
    r.attrs = json!({ "price": 1.5 });

    let res = canonicalize_without_self_hash(&r);
    match res {
        Err(CanonError::FloatDisallowed) => {}
        other => panic!("expected FloatDisallowed, got {other:?}"),
    }
}

#[test]
fn canonicalization_nfc_normalizes_strings() {
    // "Café" with decomposed é
    let decomposed = "Cafe\u{0301}";

    let mut r = base_record();
    r.attrs = json!({ "label": decomposed });

    let bytes = canonicalize_without_self_hash(&r).expect("canon");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json from canon");

    let label = value["attrs"]["label"]
        .as_str()
        .expect("attrs.label must be string");

    assert_eq!(label, "Café", "label must be NFC-normalized");
}
