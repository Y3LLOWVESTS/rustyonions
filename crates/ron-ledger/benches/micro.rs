//! RO:WHAT — Criterion microbench for small batch commits against the in-memory ledger backend.
//! RO:WHY  — Pillar 12; Concerns: PERF/RES. Keeps commit-path latency visible during local iteration.
//! RO:INTERACTS — ron_ledger::engine::Ledger, MemoryStorage.
//! RO:INVARIANTS — benchmark does not alter correctness; uses deterministic single-entry mint batches.
//! RO:METRICS — Criterion only.
//! RO:CONFIG — default LedgerConfig.
//! RO:SECURITY — no secrets.
//! RO:TEST — perf bench.

use criterion::{criterion_group, criterion_main, Criterion};
use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{Ledger, MemoryStorage},
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};

fn bench_ingest(c: &mut Criterion) {
    c.bench_function("ledger_ingest_one_mint", |b| {
        b.iter(|| {
            let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default()).unwrap();
            let req = IngestRequest {
                batch: vec![Entry::new(
                    "bench-1",
                    1,
                    EntryKind::Mint,
                    AccountId::new("acct_bench").unwrap(),
                    1,
                    Nonce::from_base64("AAAAAAAAAAAAAAAAAAAAAA==").unwrap(),
                    Kid::new("kid-bench").unwrap(),
                    CapabilityRef::new("cap-bench").unwrap(),
                    1,
                )
                .unwrap()],
                idem_id: None,
            };
            let _ = ledger.ingest(req).unwrap();
        });
    });
}

criterion_group!(benches, bench_ingest);
criterion_main!(benches);
