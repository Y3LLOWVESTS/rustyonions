//! RO:WHAT — Byte-size helpers for Macronode.
//! RO:WHY  — Make size-related config and limits more readable (MiB/GiB)
//!           and avoid repeating 1024 * 1024 everywhere.
//! RO:INVARIANTS —
//!   - Helpers are simple arithmetic and never panic on normal inputs.

#![allow(dead_code)]

/// 1 KiB in bytes.
pub const KIB: u64 = 1024;
/// 1 MiB in bytes.
pub const MIB: u64 = 1024 * 1024;
/// 1 GiB in bytes.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Return `n` kibibytes in bytes.
#[must_use]
pub const fn kib(n: u64) -> u64 {
    n * KIB
}

/// Return `n` mebibytes in bytes.
#[must_use]
pub const fn mib(n: u64) -> u64 {
    n * MIB
}

/// Return `n` gibibytes in bytes.
#[must_use]
pub const fn gib(n: u64) -> u64 {
    n * GIB
}
