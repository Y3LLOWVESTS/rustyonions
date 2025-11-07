//! Dalek (pure Rust) Ed25519 helpers.
//! Uses `ed25519-dalek` v2 types explicitly to avoid collisions with the `ed25519` trait crate.

#![allow(clippy::module_name_repetitions)]

use ed25519_dalek::Signer; // trait for `sign`
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

#[cfg(not(all(feature = "dalek-batch", feature = "parallel-batch")))]
use std::cell::RefCell;

#[cfg(feature = "parallel-batch")]
use rayon::prelude::*;

/// Thread-local scratch to avoid per-call Vec allocs in batch verification (serial paths).
/// Only compiled when the parallel multiscalar path is NOT active.
#[cfg(not(all(feature = "dalek-batch", feature = "parallel-batch")))]
struct BatchScratch {
    vks: Vec<VerifyingKey>,
    sigs: Vec<Signature>,
}
#[cfg(not(all(feature = "dalek-batch", feature = "parallel-batch")))]
impl BatchScratch {
    fn with_capacity(n: usize) -> Self {
        Self {
            vks: Vec::with_capacity(n),
            sigs: Vec::with_capacity(n),
        }
    }
    fn reset(&mut self, n: usize) {
        if self.vks.capacity() < n {
            self.vks.reserve(n - self.vks.capacity());
        }
        if self.sigs.capacity() < n {
            self.sigs.reserve(n - self.sigs.capacity());
        }
        self.vks.clear();
        self.sigs.clear();
    }
}

#[cfg(not(all(feature = "dalek-batch", feature = "parallel-batch")))]
thread_local! {
    static SCRATCH: RefCell<BatchScratch> = RefCell::new(BatchScratch::with_capacity(64));
}

/// Generate a new Ed25519 keypair, returning (`public_key_bytes`, `secret_key_bytes`).
#[must_use]
pub fn ed25519_generate() -> ([u8; 32], [u8; 32]) {
    let mut csprng = OsRng;
    let sk: SigningKey = SigningKey::generate(&mut csprng);
    let vk: VerifyingKey = VerifyingKey::from(&sk);
    (vk.to_bytes(), sk.to_bytes())
}

/// Sign `msg` using a 32-byte Ed25519 secret key (seed). Returns the 64-byte signature.
#[must_use]
pub fn ed25519_sign(secret_seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    let sk: SigningKey = SigningKey::from_bytes(secret_seed);
    sk.sign(msg).to_bytes()
}

/// Verify a 64-byte signature against a 32-byte public key. Returns true if valid.
#[must_use]
pub fn ed25519_verify(pk_bytes: &[u8; 32], msg: &[u8], sig_bytes: &[u8; 64]) -> bool {
    let sig: Signature = Signature::from_bytes(sig_bytes);
    let Ok(vk) = VerifyingKey::from_bytes(pk_bytes) else {
        return false;
    };
    vk.verify_strict(msg, &sig).is_ok()
}

/// Batch verify N signatures. `pks.len() == msgs.len() == sigs.len()`.
#[must_use]
pub fn ed25519_verify_batch(pks: &[[u8; 32]], msgs: &[&[u8]], sigs: &[[u8; 64]]) -> bool {
    let n = pks.len();
    if !(n == msgs.len() && n == sigs.len()) {
        return false;
    }

    // ======== Parallel multiscalar batch ========
    #[cfg(all(feature = "dalek-batch", feature = "parallel-batch"))]
    {
        // Decode once into local vectors (no thread-local scratch in the parallel path).
        let mut vks = Vec::with_capacity(n);
        let mut sig_parsed = Vec::with_capacity(n);
        for pk in pks {
            let Ok(vk) = VerifyingKey::from_bytes(pk) else {
                return false;
            };
            vks.push(vk);
        }
        for s in sigs {
            sig_parsed.push(Signature::from_bytes(s));
        }

        // Pick a chunk count: up to 2× threads, but not more than n; ensure ≥8 items per chunk.
        let threads = rayon::current_num_threads().max(1);
        let mut chunks = (threads * 2).min(n.max(1));
        while chunks > 1 && (n / chunks) < 8 {
            chunks -= 1;
        }

        // Parallelize over 0..chunks; compute (start,end) inside the closure.
        let all_ok = (0..chunks).into_par_iter().all(|i| {
            let start = i * n / chunks;
            let end = ((i + 1) * n / chunks).min(n);
            ed25519_dalek::verify_batch(
                &msgs[start..end],
                &sig_parsed[start..end],
                &vks[start..end],
            )
            .is_ok()
        });

        return all_ok;
    }

    // ======== Serial multiscalar batch ========
    #[cfg(all(feature = "dalek-batch", not(feature = "parallel-batch")))]
    {
        return SCRATCH.with(|cell| {
            let mut scratch = cell.borrow_mut();
            scratch.reset(n);

            for pk in pks {
                let Ok(vk) = VerifyingKey::from_bytes(pk) else {
                    return false;
                };
                scratch.vks.push(vk);
            }
            for s in sigs {
                scratch.sigs.push(Signature::from_bytes(s));
            }
            ed25519_dalek::verify_batch(&msgs, &scratch.sigs, &scratch.vks).is_ok()
        });
    }

    // ======== Serial strict loop (no dalek batch available) ========
    #[cfg(not(feature = "dalek-batch"))]
    {
        SCRATCH.with(|cell| {
            let mut scratch = cell.borrow_mut();
            scratch.reset(n);

            for pk in pks {
                let Ok(vk) = VerifyingKey::from_bytes(pk) else {
                    return false;
                };
                scratch.vks.push(vk);
            }
            for s in sigs {
                scratch.sigs.push(Signature::from_bytes(s));
            }

            for ((vk, sig), msg) in scratch
                .vks
                .iter()
                .zip(scratch.sigs.iter())
                .zip(msgs.iter().copied())
            {
                if vk.verify_strict(msg, sig).is_err() {
                    return false;
                }
            }
            true
        })
    }
}
