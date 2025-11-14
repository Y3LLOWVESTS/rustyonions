//! RO:WHAT — Criterion microbench for per-record vs batched append into an AuditSink.
//! RO:WHY  — PERF/ECON: rough guidance for WAL/batch tuning in hosts (fsync cadence, batch size).
//! RO:INTERACTS — ron_audit::sink::{ram::RamSink, AuditSink}; stream::BufferedSink; hash::b3_no_self.
//! RO:INVARIANTS — append-only; prev == last.self_hash; bounded record sizes.
//! RO:METRICS — bench-only; host metrics will live in svc-* crates.
//! RO:CONFIG — batch size configured in code; env toggles can be added later.
//! RO:SECURITY — synthetic records; no real keys/PII; in-RAM only.
//! RO:TEST — perf: Criterion group `wal_single_append` / `wal_buffered_append`.

use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::sink::{ram::RamSink, AuditSink};
use ron_audit::stream::BufferedSink;
use ron_audit::AuditRecord;
use serde_json::json;

fn mk_records(count: usize) -> Vec<AuditRecord> {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    let mut out = Vec::with_capacity(count);

    // genesis
    let mut prev_hash = "b3:0".to_string();
    for seq in 0..count as u64 {
        let mut rec = AuditRecord {
            v: 1,
            ts_ms,
            writer_id: "bench@ron-audit".to_string(),
            seq,
            stream: "wal_batching".to_string(),
            kind: AuditKind::IndexWrite,
            actor: ActorRef::default(),
            subject: SubjectRef::default(),
            reason: ReasonCode("bench-wal-batching".to_string()),
            attrs: json!({ "seq": seq }),
            prev: prev_hash.clone(),
            self_hash: String::new(),
        };
        rec.self_hash = b3_no_self(&rec).expect("hash");
        prev_hash = rec.self_hash.clone();
        out.push(rec);
    }

    out
}

fn wal_single_append(c: &mut Criterion) {
    // A moderately large batch to make per-record append overhead visible.
    let records = mk_records(512);

    c.bench_function("wal_single_append_512", |b| {
        b.iter(|| {
            let sink = RamSink::new();
            for rec in black_box(&records) {
                sink.append(rec).expect("append");
            }
        });
    });
}

fn wal_buffered_append(c: &mut Criterion) {
    let records = mk_records(512);

    c.bench_function("wal_buffered_append_512", |b| {
        b.iter(|| {
            let sink = RamSink::new();
            let buffered = BufferedSink::new(sink);
            buffered
                .append_all(black_box(&records))
                .expect("buffered append_all");
        });
    });
}

criterion_group!(benches, wal_single_append, wal_buffered_append);
criterion_main!(benches);
