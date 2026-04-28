//! RO:WHAT — No-op accounting client seam for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Keeps ron-accounting integration optional while preserving the boundary.
//! RO:INTERACTS — ledger commit path and future ron-accounting exporter.
//! RO:INVARIANTS — derivative counters only; never replaces ron-ledger truth.
//! RO:METRICS — future export success/failure counters.
//! RO:CONFIG — none yet.
//! RO:SECURITY — no secrets.
//! RO:TEST — noop_client_accepts_event.

/// Accounting event emitted after wallet operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountingEvent {
    /// Operation label.
    pub op: &'static str,
    /// Asset.
    pub asset: String,
    /// Amount in minor units.
    pub amount_minor: u128,
}

/// No-op accounting client.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAccountingClient;

impl NoopAccountingClient {
    /// Accept an accounting event without side effects.
    pub fn record(&self, _event: AccountingEvent) {}
}
