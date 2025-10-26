//! RO:WHAT — Connection primitives (backpressure, reader, writer, rate limits).
//! RO:INVARIANTS — single-writer discipline; bounded inflight.

pub mod backpressure;
pub mod rate_limit;
pub mod reader;
pub mod writer;
