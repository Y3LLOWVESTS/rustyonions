//! Backend selection and re-exports.
//! Default: dalek (pure Rust). Optional: ring via `fast` feature.

#![allow(clippy::module_name_repetitions)]

#[cfg(not(feature = "fast"))]
pub mod dalek;
#[cfg(not(feature = "fast"))]
pub use crate::backends::dalek::{
    ed25519_generate, ed25519_sign, ed25519_verify, ed25519_verify_batch,
};

#[cfg(feature = "fast")]
pub mod fast_ring;
#[cfg(feature = "fast")]
pub use crate::backends::fast_ring::{
    ed25519_generate, ed25519_sign, ed25519_verify, ed25519_verify_batch,
};

pub mod memory;
pub use memory::MemoryKeystore;

/// Stable adapter so benches/tests can `use ron_kms::backends::ed25519`.
/// It forwards to the currently selected backend (dalek or ring).
pub mod ed25519 {
    /// Generate a new Ed25519 keypair, returning (`public_key_bytes`, `secret_key_bytes`).
    #[must_use]
    pub fn generate() -> ([u8; 32], [u8; 32]) {
        super::ed25519_generate()
    }
    /// Sign `msg` using a 32-byte Ed25519 secret key (seed). Returns a 64-byte signature.
    #[must_use]
    pub fn sign(secret_seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
        super::ed25519_sign(secret_seed, msg)
    }
    /// Verify a 64-byte signature against a 32-byte public key.
    #[must_use]
    pub fn verify(pk_bytes: &[u8; 32], msg: &[u8], sig_bytes: &[u8; 64]) -> bool {
        super::ed25519_verify(pk_bytes, msg, sig_bytes)
    }
    /// Batch verify N signatures. `pks.len() == msgs.len() == sigs.len()`.
    #[must_use]
    pub fn verify_batch(pks: &[[u8; 32]], msgs: &[&[u8]], sigs: &[[u8; 64]]) -> bool {
        super::ed25519_verify_batch(pks, msgs, sigs)
    }
}
