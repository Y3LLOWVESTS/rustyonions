//! RO:WHAT — Local error taxonomy for Bus construction/usage
//! RO:WHY  — Keep external semantics explicit & stable (SemVer)
//! RO:INTERACTS — Returned by Bus::new(); complements tokio::broadcast RecvError at call sites
//! RO:INVARIANTS — small, non_exhaustive; no std::error::Error to avoid error stack bloat
//! RO:TEST — Unit: config errors; Integration: negative patterns

/// Errors constructing or using the Bus surface.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusError {
    /// Invalid configuration: contains a human-readable reason.
    Config(String),
    /// Channel was closed (no subscribers left).
    Closed,
}
