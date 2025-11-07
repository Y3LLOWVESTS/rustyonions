//! RO:WHAT  Ed25519 adapter façade for ron-kms (dalek default).
//! RO:INV   Public, stable functions: generate, sign, verify, verify_batch.
//!          No behavior changes vs pre-instrumentation; metrics are feature-gated.

#![forbid(unsafe_code)]

use ed25519_dalek::{Signer as _, Verifier as _};

#[cfg(feature = "dalek-batch")]
use ed25519_dalek::Verifier as _;

#[cfg(feature = "dalek-batch")]
use ed25519_dalek::VerifierMut as _;

use ed25519_dalek::{SigningKey, VerifyingKey, Signature};

use rand_core::OsRng;

#[inline]
pub fn generate() -> ([u8; 32], [u8; 32]) {
    let sk = SigningKey::generate(&mut OsRng);
    let pk: [u8; 32] = sk.verifying_key().to_bytes();
    let seed: [u8; 32] = sk.to_bytes();
    #[cfg(feature = "with-metrics")]
    {
        use crate::telemetry;
        use std::time::Duration;
        // treat generate as a fast op; record as 0 duration (we only count occurrences)
        telemetry::metrics().observe("create", "ed25519", None, Duration::from_secs(0));
    }
    (pk, seed)
}

#[inline]
pub fn sign(seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    let sk = SigningKey::from_bytes(seed);
    #[cfg(feature = "with-metrics")]
    let start = std::time::Instant::now();

    let sig: Signature = sk.sign(msg);

    #[cfg(feature = "with-metrics")]
    {
        use crate::telemetry;
        telemetry::metrics().observe("sign", "ed25519", None, start.elapsed());
    }

    sig.to_bytes()
}

#[inline]
pub fn verify(pk: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    let vk = VerifyingKey::from_bytes(pk).ok();
    if vk.is_none() {
        #[cfg(feature = "with-metrics")]
        {
            use crate::telemetry;
            telemetry::metrics().fail("verify", "BadKey");
        }
        return false;
    }
    let vk = vk.unwrap();

    #[cfg(feature = "with-metrics")]
    let start = std::time::Instant::now();

    // Strict verification (reject malleable encodings)
    let sig = Signature::from_bytes(sig);
    let ok = vk.verify_strict(msg, &sig).is_ok();

    #[cfg(feature = "with-metrics")]
    {
        use crate::telemetry;
        if ok {
            telemetry::metrics().observe("verify", "ed25519", None, start.elapsed());
        } else {
            telemetry::metrics().fail("verify", "VerifyFailed");
        }
    }

    ok
}

/// Batch verify using dalek multiscalar when enabled; otherwise strict loop.
/// - `pks.len() == sigs.len() == msgs.len()`; `msgs` provided as `&[&[u8]]` to avoid copies.
/// - Returns `true` iff all tuples verify strictly.
pub fn verify_batch(pks: &[[u8; 32]], msgs: &[&[u8]], sigs: &[[u8; 64]]) -> bool {
    debug_assert_eq!(pks.len(), msgs.len());
    debug_assert_eq!(sigs.len(), msgs.len());

    #[cfg(feature = "with-metrics")]
    let start = std::time::Instant::now();

    let ok = {
        #[cfg(feature = "dalek-batch")]
        {
            // Build dalek types
            let vks: Vec<VerifyingKey> = pks
                .iter()
                .map(|pk| VerifyingKey::from_bytes(pk))
                .collect::<Result<_, _>>()
                .unwrap_or_default();

            if vks.len() != pks.len() {
                // at least one key was invalid encoding
                #[cfg(feature = "with-metrics")]
                {
                    use crate::telemetry;
                    telemetry::metrics().fail("verify_batch", "BadKey");
                }
                return false;
            }

            // Convert sigs
            let sigs_vec: Vec<Signature> =
                sigs.iter().map(|s| Signature::from_bytes(s)).collect();

            // dalek true multiscalar batch
            // Safety: dalek handles empty vecs as Ok (trivial truth)
            ed25519_dalek::Verifier::verify_batch_strict(&vks, msgs, &sigs_vec).is_ok()
        }

        #[cfg(not(feature = "dalek-batch"))]
        {
            // Strict loop fallback (no batching)
            // Early exit on first failure
            for i in 0..msgs.len() {
                let vk = match VerifyingKey::from_bytes(&pks[i]) {
                    Ok(v) => v,
                    Err(_) => {
                        #[cfg(feature = "with-metrics")]
                        {
                            use crate::telemetry;
                            telemetry::metrics().fail("verify_batch", "BadKey");
                        }
                        return false;
                    }
                };
                let sig = Signature::from_bytes(&sigs[i]);
                if vk.verify_strict(msgs[i], &sig).is_err() {
                    #[cfg(feature = "with-metrics")]
                    {
                        use crate::telemetry;
                        telemetry::metrics().fail("verify_batch", "VerifyFailed");
                    }
                    return false;
                }
            }
            true
        }
    };

    #[cfg(feature = "with-metrics")]
    {
        use crate::telemetry;
        if ok {
            telemetry::metrics().observe("verify_batch", "ed25519", Some(msgs.len()), start.elapsed());
        } else {
            telemetry::metrics().fail("verify_batch", "VerifyFailed");
        }
    }

    ok
}
