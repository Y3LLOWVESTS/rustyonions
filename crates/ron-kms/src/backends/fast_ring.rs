//! Fast Ed25519 path using `ring`.
//!
//! Exposes the same free functions as the dalek backend so callers can feature-switch:
//! - `ed25519_generate()` -> (`public_key_bytes`, `secret_key_bytes`)
//! - `ed25519_sign(seed, msg)` -> signature bytes
//! - `ed25519_verify(pk, msg, sig)` -> bool
//! - `ed25519_verify_batch(pks, msgs, sigs)` -> bool
//!
//! Invariants:
//! - Always return fixed-size arrays ([u8; 32] pubkey/seed, [u8; 64] signature)
//! - No allocations on hot sign/verify paths.
//! - Pure `ring` API; no unsafe.

#![allow(clippy::module_name_repetitions)]

use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::{Ed25519KeyPair, KeyPair, Signature, UnparsedPublicKey, ED25519};

/// Generate a new Ed25519 keypair, returning (`public_key_bytes`, `secret_key_bytes`).
/// - public key: 32 bytes
/// - secret key (seed): 32 bytes
#[must_use]
pub fn ed25519_generate() -> ([u8; 32], [u8; 32]) {
    let rng = SystemRandom::new();

    // ring’s Ed25519KeyPair can be constructed from a 32-byte seed.
    let mut seed = [0u8; 32];
    rng.fill(&mut seed).expect("ring RNG failed");

    let kp = Ed25519KeyPair::from_seed_unchecked(&seed).expect("ring from_seed_unchecked");
    let mut pk = [0u8; 32];
    pk.copy_from_slice(kp.public_key().as_ref()); // KeyPair::public_key() -> &[u8] (32)

    (pk, seed)
}

/// Sign `msg` using a 32-byte Ed25519 secret key (seed).
/// Returns the 64-byte signature.
#[must_use]
pub fn ed25519_sign(secret_seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    let kp = Ed25519KeyPair::from_seed_unchecked(secret_seed).expect("ring from_seed_unchecked");
    let sig: Signature = kp.sign(msg);
    let mut out = [0u8; 64];
    out.copy_from_slice(sig.as_ref()); // 64 bytes
    out
}

/// Verify a 64-byte signature against a 32-byte public key.
/// Returns true if valid.
#[must_use]
pub fn ed25519_verify(pk_bytes: &[u8; 32], msg: &[u8], sig_bytes: &[u8; 64]) -> bool {
    let verifier = UnparsedPublicKey::new(&ED25519, pk_bytes);
    verifier.verify(msg, sig_bytes).is_ok()
}

/// Batch verify N signatures. Expects `pks.len() == msgs.len() == sigs.len()`.
#[must_use]
pub fn ed25519_verify_batch(pks: &[[u8; 32]], msgs: &[&[u8]], sigs: &[[u8; 64]]) -> bool {
    if !(pks.len() == msgs.len() && msgs.len() == sigs.len()) {
        return false;
    }
    // Build once; ring's UnparsedPublicKey holds a reference, so no copies.
    let verifiers: Vec<UnparsedPublicKey<&[u8; 32]>> = pks
        .iter()
        .map(|pk| UnparsedPublicKey::new(&ED25519, pk))
        .collect();
    for i in 0..pks.len() {
        if verifiers[i].verify(msgs[i], &sigs[i]).is_err() {
            return false;
        }
    }
    true
}
