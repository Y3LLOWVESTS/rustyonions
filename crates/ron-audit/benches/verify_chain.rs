//! RO:WHAT — Criterion microbench for chain verification (scalar API vs SoA).
//! RO:WHY  — PERF/RES: compare verify_chain (reference) and verify_chain_soa (fast path)
//!           on realistic chain lengths.
//! RO:INTERACTS — ron_audit::hash::b3_no_self;
//!                ron_audit::verify::{verify_record, verify_link, verify_chain, verify_chain_soa}.
//! RO:INVARIANTS — no unsafe; same semantics for scalar and SoA; only performance differs.
//! RO:METRICS — bench-only; no runtime counters.
//! RO:CONFIG — chain length configured in-code; env knobs can be added later.
//! RO:SECURITY — synthetic records only; no real keys/PII.
//! RO:TEST — unit: tests/verify_soa.rs; perf: this bench.

use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_audit::dto::{ActorRef, AuditKind, ReasonCode, SubjectRef};
use ron_audit::hash::b3_no_self;
use ron_audit::verify::{verify_chain, verify_chain_soa};
use ron_audit::AuditRecord;
use serde_json::json;

fn mk_chain(len: usize) -> Vec<AuditRecord> {
    assert!(len > 0);

    let mut out = Vec::with_capacity(len);

    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64;

    // genesis
    let mut genesis = AuditRecord {
        v: 1,
        ts_ms,
        writer_id: "bench@ron-audit".to_string(),
        seq: 0,
        stream: "verify_chain".to_string(),
        kind: AuditKind::CapIssued,
        actor: ActorRef::default(),
        subject: SubjectRef::default(),
        reason: ReasonCode("bench-verify-chain".to_string()),
        attrs: json!({ "seq": 0u64 }),
        prev: "b3:0".to_string(),
        self_hash: String::new(),
    };
    genesis.self_hash = b3_no_self(&genesis).expect("hash");
    out.push(genesis);

    while out.len() < len {
        let prev = out.last().expect("non-empty");
        let mut rec = AuditRecord {
            v: 1,
            ts_ms,
            writer_id: prev.writer_id.clone(),
            seq: prev.seq + 1,
            stream: prev.stream.clone(),
            kind: AuditKind::IndexWrite,
            actor: ActorRef::default(),
            subject: SubjectRef::default(),
            reason: ReasonCode("bench-verify-chain".to_string()),
            attrs: json!({ "seq": prev.seq + 1 }),
            prev: prev.self_hash.clone(),
            self_hash: String::new(),
        };
        rec.self_hash = b3_no_self(&rec).expect("hash");
        out.push(rec);
    }

    out
}

/// For comparison: scalar reference API over an owned iterator.
///
/// NOTE: This includes the cost of cloning the chain when we call
/// `chain.clone().into_iter()`, which matches how callers would typically
/// use the public API.
fn bench_verify_chain_scalar_api(c: &mut Criterion) {
    let chain = mk_chain(512);

    // Sanity-check: scalar API must succeed once outside the hot loop.
    verify_chain(chain.clone().into_iter()).expect("verify_chain scalar API");

    c.bench_function("verify_chain_scalar_len_512", |b| {
        b.iter(|| {
            // Clone per-iteration to match real-world "owned iterator" usage.
            let owned = black_box(chain.clone());
            verify_chain(owned.into_iter()).expect("scalar verify");
        });
    });
}

/// SoA-style fast path over a contiguous slice.
///
/// This avoids per-iteration cloning and uses the SoA slice-based verifier.
/// It should be at least as fast as the scalar path, often faster for large chains.
fn bench_verify_chain_soa(c: &mut Criterion) {
    let chain = mk_chain(512);

    // Sanity-check: SoA API must succeed once outside the hot loop.
    verify_chain_soa(&chain).expect("verify_chain_soa");

    c.bench_function("verify_chain_soa_len_512", |b| {
        b.iter(|| {
            verify_chain_soa(black_box(&chain)).expect("soa verify");
        });
    });
}

criterion_group!(
    benches,
    bench_verify_chain_scalar_api,
    bench_verify_chain_soa
);
criterion_main!(benches);
