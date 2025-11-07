use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rand::RngCore;
use ron_kms::{memory_keystore, Alg, Keystore, KmsError, Signer, Verifier};

// Tunables chosen to be stable on laptops/CI while still exercising contention.
const ROTATIONS: usize = 4;
const TEST_WINDOW_MS: u64 = 400; // window for verifies during rotation
const ROTATE_SLEEP_MS: u64 = 6; // small pacing between rotations
const PRE_SIGNED: usize = 16; // messages pre-signed at v1

// Retry helper: treat Busy as transient; back off a touch; give up after attempts.
fn retry_busy<F, T>(mut f: F, attempts: usize, backoff_us: u64) -> Result<T, KmsError>
where
    F: FnMut() -> Result<T, KmsError>,
{
    let mut i = 0usize;
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(KmsError::Busy) if i + 1 < attempts => {
                std::thread::sleep(Duration::from_micros(backoff_us));
                std::thread::yield_now();
                i += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

#[test]
fn rotation_during_verifies_is_consistent() {
    let kms = Arc::new(memory_keystore());
    let kid = kms.create_ed25519("tenant", "purpose").expect("create key");
    assert_eq!(kid.alg, Alg::Ed25519);

    // Pre-sign a pool at version 1 (before any rotations).
    let mut msgs: Vec<Vec<u8>> = Vec::with_capacity(PRE_SIGNED);
    let mut sigs: Vec<Vec<u8>> = Vec::with_capacity(PRE_SIGNED);
    for _ in 0..PRE_SIGNED {
        let mut msg = vec![0u8; 64];
        rand::thread_rng().fill_bytes(&mut msg);
        let sig = kms.sign(&kid, &msg).expect("pre-sign at v1");
        msgs.push(msg);
        sigs.push(sig);
    }

    // Writer rotates a few times with pacing.
    let kms_w = kms.clone();
    let kid_w = kid.clone();
    let writer = thread::spawn(move || {
        for _ in 0..ROTATIONS {
            let _ = retry_busy(
                || kms_w.rotate(&kid_w),
                /*attempts*/ 1024,
                /*backoff_us*/ 300,
            );
            thread::sleep(Duration::from_millis(ROTATE_SLEEP_MS));
        }
    });

    // Reader: repeatedly verify pre-signed messages while rotations occur.
    let kms_r = kms.clone();
    let kid_r = kid.clone();
    let reader = thread::spawn(move || {
        let start = Instant::now();
        let end = start + Duration::from_millis(TEST_WINDOW_MS);
        let mut ok = 0usize;
        let mut idx = 0usize;

        while Instant::now() < end {
            let i = idx % PRE_SIGNED;
            idx += 1;

            let verified = match retry_busy(
                || kms_r.verify(&kid_r, &msgs[i], &sigs[i]),
                /*attempts*/ 1024,
                /*backoff_us*/ 300,
            ) {
                Ok(v) => v,
                Err(KmsError::Busy) => {
                    // Couldn't acquire due to contention; try next iteration.
                    std::thread::yield_now();
                    continue;
                }
                Err(e) => panic!("verify failed with non-Busy error: {e:?}"),
            };

            assert!(verified, "verify returned false");
            ok += 1;

            // micro-snooze to avoid futile head-to-head spinning on some CPUs.
            std::thread::sleep(Duration::from_micros(150));
        }

        ok
    });

    writer.join().expect("writer thread joined");
    let ok = reader.join().expect("reader thread joined");

    // Success criterion: at least one verify succeeded during concurrent rotations.
    assert!(ok >= 1, "no successful verifies during rotation (ok={ok})");
}
