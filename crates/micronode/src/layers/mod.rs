//! RO:WHAT — Ingress guard layers (body cap, decode guard, concurrency).
//! RO:WHY  — Enforce limits *before* heavy work; deterministic early rejects.

pub mod body_cap;
pub mod concurrency;
pub mod decode_guard;
pub mod security;
