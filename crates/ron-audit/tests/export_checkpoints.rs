use std::collections::HashMap;

use ron_audit::dto::{ActorRef, AuditKind, AuditRecord, ReasonCode, SubjectRef};
use ron_audit::sink::ram::RamSink;
use ron_audit::AuditSink;
use serde_json::json;

/// Helper to build a minimal, self-consistent `AuditRecord` for tests.
///
/// NOTE: This does *not* compute a real BLAKE3 self_hash; tests here only care
/// about append-only semantics and head export, not cryptographic integrity.
fn make_record(stream: &str, seq: u64, prev: &str, self_hash: &str) -> AuditRecord {
    AuditRecord {
        v: 1,
        ts_ms: 0,
        writer_id: "svc-test@inst-1".to_string(),
        seq,
        stream: stream.to_string(),
        kind: AuditKind::Unknown,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("test".to_string()),
        attrs: json!({}),
        prev: prev.to_string(),
        self_hash: self_hash.to_string(),
    }
}

#[test]
fn export_heads_returns_latest_head_per_stream() {
    let sink = RamSink::new();

    // Build a small chain on two logical streams: "ingress" and "policy".
    // We use simple fake hashes here; we only care about linkage and export.
    let r1_ing = make_record("ingress", 1, "b3:0", "b3:ing-1");
    let r2_ing = make_record("ingress", 2, "b3:ing-1", "b3:ing-2");

    let r1_pol = make_record("policy", 1, "b3:0", "b3:pol-1");
    let r2_pol = make_record("policy", 2, "b3:pol-1", "b3:pol-2");
    let r3_pol = make_record("policy", 3, "b3:pol-2", "b3:pol-3");

    // Append in interleaved order to ensure ordering logic is per-stream.
    sink.append(&r1_ing).expect("append r1_ing");
    sink.append(&r1_pol).expect("append r1_pol");
    sink.append(&r2_ing).expect("append r2_ing");
    sink.append(&r2_pol).expect("append r2_pol");
    sink.append(&r3_pol).expect("append r3_pol");

    let heads = sink.heads();
    assert_eq!(heads.len(), 2, "expected one head per stream");

    let mut by_stream: HashMap<String, (u64, String)> = HashMap::new();
    for head in heads {
        by_stream.insert(head.stream.clone(), (head.seq, head.head.clone()));
    }

    // ingress: last ing record was seq=2, self_hash="b3:ing-2"
    let ingress = by_stream.get("ingress").expect("ingress head missing");
    assert_eq!(ingress.0, 2, "Ingress seq should be 2");
    assert_eq!(ingress.1, "b3:ing-2", "Ingress head hash mismatch");

    // policy: last pol record was seq=3, self_hash="b3:pol-3"
    let policy = by_stream.get("policy").expect("policy head missing");
    assert_eq!(policy.0, 3, "Policy seq should be 3");
    assert_eq!(policy.1, "b3:pol-3", "Policy head hash mismatch");
}

#[test]
fn export_heads_skips_empty_streams() {
    let sink = RamSink::new();

    // Only write to "ingress", leave "policy" empty.
    let r1_ing = make_record("ingress", 1, "b3:0", "b3:ing-1");
    sink.append(&r1_ing).expect("append r1_ing");

    let heads = sink.heads();
    assert_eq!(heads.len(), 1, "only one non-empty stream expected");

    let head = &heads[0];
    assert_eq!(head.stream, "ingress");
    assert_eq!(head.seq, 1);
    assert_eq!(head.head, "b3:ing-1");
}
