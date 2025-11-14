/*!
RO:WHAT — Privacy policy hook smoke tests.
RO:WHY — SEC/GOV: ensure the hook is callable and side-effect free for now.
RO:INTERACTS — ron_audit::privacy; dto::AuditRecord.
RO:INVARIANTS — validate() must not mutate; default is allow-all.
RO:TEST HOOKS — Unit tests here; future policy logic can extend coverage.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::privacy;
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
        kind: AuditKind::Unknown,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("privacy-test".to_string()),
        attrs: json!({ "field": "value" }),
        prev: "b3:0".to_string(),
        self_hash: "b3:placeholder".to_string(),
    }
}

#[test]
fn privacy_validate_is_noop_for_now() {
    let rec = base_record();
    privacy::validate(&rec).expect("default privacy hook should pass");
}
