//! RO:WHAT — Criterion microbench for canonicalize + BLAKE3 hashing of AuditRecord.
//! RO:WHY  — PERF: baseline BLAKE3 throughput on canonical audit payloads.
//! RO:INTERACTS — ron_audit::hash::b3_no_self; dto::AuditRecord; serde_json.
//! RO:INVARIANTS — pure; no I/O; stable record shape; no global state.
//! RO:METRICS — bench-only; no runtime counters.
//! RO:CONFIG — in-code record sizes; no env knobs yet.
//! RO:SECURITY — synthetic records only; no real keys/PII.
//! RO:TEST — perf: Criterion group `hash_b3_small` / `hash_b3_large`.

use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::AuditRecord;
use serde_json::json;

fn mk_record(attr_bytes: usize) -> AuditRecord {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let payload = "x".repeat(attr_bytes.max(1));

    let mut rec = AuditRecord {
        v: 1,
        ts_ms,
        writer_id: "bench@ron-audit".to_string(),
        seq: 0,
        stream: "hash_b3".to_string(),
        kind: AuditKind::IndexWrite,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("bench-hash-b3".to_string()),
        attrs: json!({ "payload": payload }),
        prev: "b3:0".to_string(),
        self_hash: String::new(),
    };

    // Fill self_hash once, even though `b3_no_self` ignores it, to keep the
    // record closer to real-world usage.
    rec.self_hash = b3_no_self(&rec).expect("hash");
    rec
}

fn hash_b3_small(c: &mut Criterion) {
    let rec = mk_record(64);

    c.bench_function("hash_b3_small_64B_attrs", |b| {
        b.iter(|| {
            let h = b3_no_self(black_box(&rec)).expect("hash");
            black_box(h);
        });
    });
}

fn hash_b3_large(c: &mut Criterion) {
    // ~1 KiB attrs to stay within DEFAULT_MAX_ATTRS_BYTES.
    let rec = mk_record(1024);

    c.bench_function("hash_b3_large_1KiB_attrs", |b| {
        b.iter(|| {
            let h = b3_no_self(black_box(&rec)).expect("hash");
            black_box(h);
        });
    });
}

criterion_group!(benches, hash_b3_small, hash_b3_large);
criterion_main!(benches);
