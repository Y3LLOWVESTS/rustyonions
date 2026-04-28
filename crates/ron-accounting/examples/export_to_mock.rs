//! RO:WHAT — Mock exporter example for a sealed accounting slice.
//! RO:WHY — Pillar 12; Concerns: DX/ECON. Shows downstream integration seam without a ledger.
//! RO:INTERACTS — Exporter trait, Ack, SealedSlice.
//! RO:INVARIANTS — exporter treats same slice ID/digest as idempotent.
//! RO:METRICS — none in this example.
//! RO:CONFIG — none.
//! RO:SECURITY — no capabilities in mock; real adapters enforce them.
//! RO:TEST — compiled by cargo test --examples.

use ron_accounting::{Ack, BoxExportFuture, Exporter, SealedSlice};

#[derive(Debug, Default)]
struct MockExporter;

impl Exporter for MockExporter {
    fn put<'a>(&'a self, _slice: &'a SealedSlice) -> BoxExportFuture<'a> {
        Box::pin(async move { Ok(Ack::Ok) })
    }
}

fn main() {
    let _exporter = MockExporter;
    println!("mock exporter is ready");
}
