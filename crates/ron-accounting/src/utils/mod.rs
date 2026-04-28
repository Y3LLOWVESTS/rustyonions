//! RO:WHAT — Small deterministic utility helpers for time, hashing, and canonical encoding.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Keeps shared invariants out of hot modules.
//! RO:INTERACTS — accounting::slice, accounting::window, config validation, future vectors.
//! RO:INVARIANTS — BLAKE3 b3 digests; canonical bytes capped at 1MiB; UTC window math.
//! RO:METRICS — none directly.
//! RO:CONFIG — encoding cap mirrors OAP/1 max_frame=1MiB.
//! RO:SECURITY — hashing helpers never hash secrets intentionally; labels already redacted.
//! RO:TEST — unit and prop tests exercise stable bytes/digests.

pub mod encode;
pub mod hashing;
pub mod time;
