/*!
RO:WHAT — API surface smoke test for prelude and core modules.
RO:WHY — GOV/DX: catch obvious API breakage before semver checks.
RO:INTERACTS — ron_audit::prelude; dto::AuditRecord.
RO:INVARIANTS — prelude compiles; basic hash/verify/bounds round-trip works.
RO:TEST HOOKS — Unit tests here; CI later wires cargo-public-api.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::prelude::*;
use serde_json::json;

fn mk_record() -> AuditRecord {
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
        reason: ReasonCode("api-compat-test".to_string()),
        attrs: json!({ "ok": true }),
        prev: "b3:0".to_string(),
        self_hash: String::new(),
    }
}

#[test]
fn prelude_smoke_round_trip() {
    let mut rec = mk_record();

    // hash without self, then set self_hash and verify
    rec.self_hash = b3_no_self(&rec).expect("hash");
    verify_record(&rec).expect("verify");

    // bounds check via prelude re-exports
    check_bounds(&rec, DEFAULT_MAX_ATTRS_BYTES, DEFAULT_MAX_RECORD_BYTES).expect("bounds");
}
