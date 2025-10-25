//! RO:WHAT — Lightweight observability hooks (metrics/tracing integration points).
//! RO:WHY  — Keep ryker decoupled from metrics exporters; host wires Prometheus/OTEL.
//! RO:INTERACTS — mailbox builder/queue calls observer methods; tracing spans are optional.
//! RO:INVARIANTS — no heavy work in hooks; sampling controlled by config.

pub mod metrics;
pub mod trace;

pub use metrics::{MailboxObserver, NoopObserver};
