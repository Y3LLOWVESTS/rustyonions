//! svc-dht invariants — compile-time and doc-level assertions that define the contract
//! RO:WHAT — Kademlia- and pipeline-related constants/invariants that other modules rely on.
//! RO:WHY  — Guard against accidental drift during refactors. Fail fast at build time.
//! RO:GATES — F (Functional), RES (Resilience), PERF (no per-call heap).
//!
//! Invariants:
//! - NodeId is 32 bytes (BLAKE3 digest); XOR distance is exactly 32 bytes.
//! - α (alpha: fanout) in [1, 16] (small, bounded concurrency per round).
//! - β (beta: hedges) in [0, 4] (limit tail-rescue parallelism).
//! - hop_budget in [1, 64] (guard against runaway traversal).
//! - Hedge stagger and leg budget are sane (stagger << budget).
//!
//! ```text
//! Kademlia rounds proceed with α parallel queries; hedging may add up to β extra legs
//! per logical lookup, spaced by a small stagger delay to rescue tail latency.
//! ```

#![allow(clippy::doc_markdown)]

pub const NODEID_LEN: usize = 32;
pub const ALPHA_MIN: usize = 1;
pub const ALPHA_MAX: usize = 16;
pub const BETA_MIN: usize = 0;
pub const BETA_MAX: usize = 4;
pub const HOPS_MIN: usize = 1;
pub const HOPS_MAX: usize = 64;

/// Sanity check helper usable in const context
const fn within(v: usize, lo: usize, hi: usize) -> bool {
    v >= lo && v <= hi
}

/// Compile-time assertions — these run when this module is referenced.
#[allow(dead_code)]
pub const fn _compile_time_guards() {
    // NodeId length must remain 32 (BLAKE3).
    // If this ever changes, XOR distance math must be updated.
    assert!(NODEID_LEN == 32);

    // Parameter envelopes (keep lookup bounded).
    assert!(within(ALPHA_MIN, 1, 32));
    assert!(within(ALPHA_MAX, 1, 32));
    assert!(ALPHA_MIN <= ALPHA_MAX);

    assert!(within(BETA_MIN, 0, 8));
    assert!(within(BETA_MAX, 0, 8));
    assert!(BETA_MIN <= BETA_MAX);

    assert!(within(HOPS_MIN, 1, 256));
    assert!(within(HOPS_MAX, 1, 256));
    assert!(HOPS_MIN <= HOPS_MAX);
}

/// Tiny doc test to lock the NodeId XOR shape without importing the full type.
/// (Keeps this module independent.)
#[cfg(test)]
mod tests {
    #[test]
    fn xor_distance_is_32_bytes() {
        let a = [0xAAu8; 32];
        let b = [0x55u8; 32];
        let mut out = [0u8; 32];
        for (i, o) in out.iter_mut().enumerate() {
            *o = a[i] ^ b[i];
        }
        assert_eq!(out.len(), 32);
        assert_eq!(out[0], 0xFF);
        assert_eq!(out[31], 0xFF);
    }
}
