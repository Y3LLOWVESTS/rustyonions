//! Registry-backed governance review surfaces.
//!
//! These modules extend canonical `ron-proto` validation with registry-owned
//! identity, lifecycle, and policy checks. They do not verify cryptographic
//! signatures or execute wallet, ledger, reward, mint, or finality mutations.

pub mod quorum;
