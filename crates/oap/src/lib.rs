// Public surface re-exports (no implementation in scaffold).
// Keep this file tiny and audit-friendly.

pub mod consts;
pub mod prelude;

pub mod envelope;
pub mod frame;
pub mod error;
pub mod metrics;
pub mod seq;

pub mod parser;
pub mod writer;

// NOTE: Add doc(cfg) and rustdoc examples once implementation arrives.
