//! RO:WHAT — Incremental OAP frame parser facade.
//! RO:WHY  — Provide a clean API to feed bytes and pull parsed `Frame`s.
//! RO:INTERACTS — Wraps `OapDecoder`; used by gateways/SDKs.
//! RO:INVARIANTS — No blocking; bounded by decoder invariants; zero `unsafe`.

pub mod config;
pub mod state;

pub use config::ParserConfig;
pub use state::ParserState;
