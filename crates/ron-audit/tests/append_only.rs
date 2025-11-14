/*!
RO:WHAT — Append-only behavior and basic chain head semantics for RamSink.
RO:WHY — Integrity: enforce append-only and detect tamper on prev/self linkage.
RO:INTERACTS — ron_audit::sink::ram::RamSink; hash::b3_no_self; AppendError.
RO:INVARIANTS — append is append-only; prev must equal last.self_hash; per-stream heads tracked.
RO:TEST HOOKS — Unit tests in this file; fuzz/loom reserved for host q.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::sink::{ram::RamSink, AuditSink, AuditStream};
use ron_audit::{AppendError, AuditRecord};
use serde_json::json;

fn mk_record(stream: &str, seq: u64, prev: &str) -> AuditRecord {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let mut rec = AuditRecord {
        v: 1,
        ts_ms,
        writer_id: "writer@test".to_string(),
        seq,
        stream: stream.to_string(),
        kind: AuditKind::Unknown,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("test-append-only".to_string()),
        attrs: json!({ "case": "append_only" }),
        prev: prev.to_string(),
        self_hash: String::new(),
    };

    rec.self_hash = b3_no_self(&rec).expect("hash");
    rec
}

#[test]
fn append_updates_head_and_seq_per_stream() {
    let sink = RamSink::new();

    let genesis = mk_record("s1", 0, "b3:0");
    let head1 = sink.append(&genesis).expect("append genesis");
    let state1 = sink.state("s1");
    assert_eq!(state1.seq, 0);
    assert_eq!(state1.head, head1);

    let rec2 = mk_record("s1", 1, &head1);
    let head2 = sink.append(&rec2).expect("append second");
    let state2 = sink.state("s1");
    assert_eq!(state2.seq, 1);
    assert_eq!(state2.head, head2);
    assert_ne!(state2.head, head1);
}

#[test]
fn append_rejects_prev_mismatch_tamper() {
    let sink = RamSink::new();

    let genesis = mk_record("s1", 0, "b3:0");
    let head1 = sink.append(&genesis).expect("append genesis");

    // Tampered record: prev does NOT match last.self_hash.
    let mut bad = mk_record("s1", 1, "b3:not-the-head");
    // keep self_hash consistent for the bad record itself
    bad.self_hash = b3_no_self(&bad).expect("hash bad");

    assert_ne!(bad.prev, head1);

    let err = sink.append(&bad).expect_err("tamper should be rejected");
    match err {
        AppendError::Tamper => {}
        other => panic!("expected AppendError::Tamper, got {other:?}"),
    }
}
