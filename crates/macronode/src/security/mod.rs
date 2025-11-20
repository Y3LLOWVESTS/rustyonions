//! RO:WHAT — Security utilities for Macronode.
//! RO:WHY  — Central home for amnesia posture, TLS options, and capability
//!           token helpers. This keeps security-related logic coherent and
//!           discoverable without bloating `main` or HTTP modules.
//! RO:INVARIANTS —
//!   - This module is pure helper surface; it does not perform I/O by itself.
//!   - Higher layers remain responsible for actually enforcing policies.

#![allow(dead_code)]

pub(crate) mod amnesia;
pub(crate) mod macaroon;
pub(crate) mod tls;

pub(crate) use amnesia::{classify_amnesia, AmnesiaMode};
pub(crate) use tls::{TlsConfig, TlsMode};
