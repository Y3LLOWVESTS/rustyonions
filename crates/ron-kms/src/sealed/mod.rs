//! RO:WHAT   Soft-seal (RAM-only) AEAD for short-lived secrets (feature `soft-seal`).
//! RO:INV    No persistent storage; ChaCha20-Poly1305; 12-byte random nonce; strict header.
//! RO:FORMAT [MAGIC:4][VER:1][TS_MS:8][NONCE:12][CIPHERTEXT+TAG:..]
//!           MAGIC="RKMS"; VER=1
//! RO:DO     Intended for transient sealing (e.g., in-flight, IPC). Not a DB at rest.

#![cfg(feature = "soft-seal")]
#![forbid(unsafe_code)]

mod aead;
mod anti_rollback;
mod header;

pub use aead::{seal, unseal};
pub use header::{Header, MAGIC, VERSION};
