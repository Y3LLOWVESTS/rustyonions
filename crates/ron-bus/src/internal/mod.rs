//! RO:WHAT — Small internal helpers to keep public files tiny
//! RO:WHY  — Encapsulate channel creation and document invariants once
//! RO:INTERACTS — channel (Tokio broadcast wrapper)
//! RO:INVARIANTS — no locks across .await; bounded channels only
//! RO:TEST — Covered indirectly via integration tests

pub mod channel;
