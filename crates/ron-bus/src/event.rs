//! RO:WHAT — Canonical Event enum carried on the bus
//! RO:WHY  — Aligns with kernel public surface; additive-safe growth (#[non_exhaustive])
//! RO:INTERACTS — Consumed by hosts/services; produced by kernel/supervision
//! RO:INVARIANTS — DTO hygiene; keep variants small; no secrets/PII in payloads
//! RO:TEST — Unit: variant roundtrips; Integration: fanout_ok

/// Kernel-aligned, additive-safe event set.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Liveness of a named service.
    Health { service: String, ok: bool },
    /// Host config hot-reload emitted version.
    ConfigUpdated { version: u64 },
    /// Supervisor noticed a crash; reason is informational.
    ServiceCrashed { service: String, reason: String },
    /// Coordinated shutdown signal.
    Shutdown,
}
