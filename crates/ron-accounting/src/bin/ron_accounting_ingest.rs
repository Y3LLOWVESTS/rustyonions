//! RO:WHAT — Small blocking HTTP ingest binary for live WEB3 storage→accounting smoke tests.
//! RO:WHY — Pillar 12; Concerns: ECON/DX/RES. Gives svc-storage a real accounting endpoint without new deps.
//! RO:INTERACTS — ron_accounting::http_ingest, Recorder, EventIngestPolicy.
//! RO:INVARIANTS — usage only; no wallet mutation; no ledger mutation; idempotency required on POST.
//! RO:METRICS — none yet; /v1/snapshot exposes row_count and rows for smoke verification.
//! RO:CONFIG — RON_ACCOUNTING_ADDR, RON_ACC_BEARER.
//! RO:SECURITY — optional bearer gate; dev default bearer is "dev".
//! RO:TEST — bash scripts/web3_paid_storage_live_smoke.sh.

#![forbid(unsafe_code)]

use ron_accounting::{
    http_ingest::{addr_from_env, bearer_from_env, serve_blocking, IngestState},
    EventIngestPolicy, Recorder,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("ron-accounting ingest failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> ron_accounting::Result<()> {
    let addr = addr_from_env()?;
    let state = IngestState::new(
        Recorder::default(),
        EventIngestPolicy::default(),
        bearer_from_env(),
    );

    eprintln!("ron-accounting ingest listening on {addr}");
    serve_blocking(addr, state)
}
