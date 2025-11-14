/*!
RO:WHAT — Checkpoint/export surface smoke tests (feature-gated).
RO:WHY — INTEROP: stable representation for checkpoint spans.
RO:INTERACTS — ron_audit::sink::export; dto::AuditRecord.
RO:INVARIANTS — from_seq/to_seq/head align with record slice.
RO:TEST HOOKS — Unit tests, but only when `export` feature is enabled.
*/

#[cfg(feature = "export")]
mod export_tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
    use ron_audit::hash::b3_no_self;
    use ron_audit::sink::export::{checkpoint_from_slice, Checkpoint};
    use ron_audit::AuditRecord;
    use serde_json::json;

    fn mk_record(seq: u64, prev: &str) -> AuditRecord {
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_millis() as u64;

        let mut rec = AuditRecord {
            v: 1,
            ts_ms,
            writer_id: "writer@test".to_string(),
            seq,
            stream: "stream@test".to_string(),
            kind: AuditKind::IndexWrite,
            actor: ActorRef::default(),
            subject: SubjectRef::default(),
            reason: ReasonCode("export-test".to_string()),
            attrs: json!({ "seq": seq }),
            prev: prev.to_string(),
            self_hash: String::new(),
        };
        rec.self_hash = b3_no_self(&rec).expect("hash");
        rec
    }

    #[test]
    fn checkpoint_captures_span_bounds_and_head() {
        let r0 = mk_record(0, "b3:0");
        let r1 = mk_record(1, &r0.self_hash);
        let r2 = mk_record(2, &r1.self_hash);
        let records = vec![r0, r1, r2];

        let cp: Checkpoint = checkpoint_from_slice(&records).expect("checkpoint from slice");

        assert_eq!(cp.from_seq, 0);
        assert_eq!(cp.to_seq, 2);
        assert_eq!(cp.head, records.last().unwrap().self_hash);
    }
}
