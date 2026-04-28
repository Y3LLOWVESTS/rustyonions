//! RO:WHAT — Accounting integration module tree.
//! RO:WHY  — Pillar 12; Concerns: ECON/PERF. Wallet may emit usage/accounting side-effects, but truth stays in ron-ledger.
//! RO:INTERACTS — accounting::client and future ron-accounting exporter.
//! RO:INVARIANTS — accounting is derivative counters only; never authoritative balance truth.
//! RO:METRICS — future accounting export counters.
//! RO:CONFIG — future accounting endpoint config.
//! RO:SECURITY — identifiers only.
//! RO:TEST — client noop smoke.

pub mod client;
