/*!
RO:WHAT — Multi-stream / multi-writer state semantics for RamSink.
RO:WHY — GOV/RES: clarify that only per-stream heads are exposed; no global order.
RO:INTERACTS — ron_audit::sink::ram::RamSink; hash::b3_no_self.
RO:INVARIANTS — Single writer per stream head; streams are independent.
RO:TEST HOOKS — Unit tests here; loom model covers host queueing later.
*/

use std::time::{SystemTime, UNIX_EPOCH};

use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::sink::{ram::RamSink, AuditSink, AuditStream};
use ron_audit::AuditRecord;
use serde_json::json;

fn mk_record(writer_id: &str, stream: &str, seq: u64, prev: &str) -> AuditRecord {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let mut rec = AuditRecord {
        v: 1,
        ts_ms,
        writer_id: writer_id.to_string(),
        seq,
        stream: stream.to_string(),
        kind: AuditKind::IndexWrite,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("multi-writer-test".to_string()),
        attrs: json!({ "stream": stream, "seq": seq }),
        prev: prev.to_string(),
        self_hash: String::new(),
    };
    rec.self_hash = b3_no_self(&rec).expect("hash");
    rec
}

#[test]
fn per_stream_heads_are_independent() {
    let sink = RamSink::new();

    let a1 = mk_record("writer-a", "stream-a", 0, "b3:0");
    let b1 = mk_record("writer-b", "stream-b", 0, "b3:0");

    let head_a = sink.append(&a1).expect("append a1");
    let head_b = sink.append(&b1).expect("append b1");

    let state_a = sink.state("stream-a");
    let state_b = sink.state("stream-b");

    assert_eq!(state_a.seq, 0);
    assert_eq!(state_b.seq, 0);
    assert_eq!(state_a.head, head_a);
    assert_eq!(state_b.head, head_b);
    assert_ne!(
        state_a.head, state_b.head,
        "heads for distinct streams must be independent"
    );
}

#[test]
fn stream_state_is_snapshot_only() {
    let sink = RamSink::new();

    let a1 = mk_record("writer-a", "stream-a", 0, "b3:0");
    let head1 = sink.append(&a1).expect("append a1");
    let snapshot_before = sink.state("stream-a");
    assert_eq!(snapshot_before.seq, 0);
    assert_eq!(snapshot_before.head, head1);

    let a2 = mk_record("writer-a", "stream-a", 1, &head1);
    let head2 = sink.append(&a2).expect("append a2");
    let snapshot_after = sink.state("stream-a");

    assert_eq!(snapshot_after.seq, 1);
    assert_eq!(snapshot_after.head, head2);
    assert_ne!(snapshot_after.head, snapshot_before.head);
}
