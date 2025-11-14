/*!
RO:WHAT — Idempotency of dedupe_key and hash/verify pipeline.
RO:WHY — ECON/RES: safe replay via stable canonicalization.
RO:INTERACTS — ron_audit::hash; verify; dto::AuditRecord.
RO:INVARIANTS — Same canonical record → same dedupe key; tamper breaks verify.
RO:TEST HOOKS — Unit tests here; fuzz targets canonicalization later.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::{b3_no_self, dedupe_key};
use ron_audit::verify::verify_record;
use ron_audit::{AuditRecord, VerifyError};
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
        reason: ReasonCode("idempotency-test".to_string()),
        attrs: json!({ "k": "v" }),
        prev: "b3:0".to_string(),
        self_hash: "b3:placeholder".to_string(),
    }
}

#[test]
fn dedupe_key_is_stable_across_self_hash_changes() {
    let mut r1 = base_record();
    let mut r2 = base_record();

    r1.self_hash = "b3:aaaa".to_string();
    r2.self_hash = "b3:bbbb".to_string();

    let k1 = dedupe_key(&r1).expect("dedupe r1");
    let k2 = dedupe_key(&r2).expect("dedupe r2");

    assert_eq!(k1, k2, "dedupe key must ignore self_hash differences");
}

#[test]
fn verify_record_succeeds_for_matching_hash() {
    let mut r = base_record();
    r.self_hash = b3_no_self(&r).expect("hash");
    verify_record(&r).expect("verify must succeed");
}

#[test]
fn verify_record_fails_on_tamper() {
    let mut r = base_record();
    r.self_hash = b3_no_self(&r).expect("hash");
    verify_record(&r).expect("baseline verify");

    // Tamper: change attrs but keep old self_hash.
    r.attrs = json!({ "k": "tampered" });

    let err = verify_record(&r).expect_err("tamper must fail verify");
    match err {
        VerifyError::HashMismatch => {}
        other => panic!("expected HashMismatch, got {other:?}"),
    }
}
