#![cfg(feature = "soft-seal")]

use rand_core::RngCore;

#[test]
fn chacha20_soft_seal_roundtrip() {
    let mut key = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut key);

    let aad = b"context";
    let pt = b"super secret bytes";

    let sealed = ron_kms::sealed::seal(&key, pt, aad);
    let unsealed = ron_kms::sealed::unseal(&key, &sealed, aad).expect("unseal ok");
    assert_eq!(unsealed.as_slice(), pt);
}

#[test]
fn chacha20_soft_seal_tamper_detected() {
    let mut key = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut key);

    let aad = b"context";
    let pt = b"tamper me";

    let mut sealed = ron_kms::sealed::seal(&key, pt, aad);

    // Flip one byte safely (no aliasing of immutable + mutable borrows).
    if let Some(last) = sealed.last_mut() {
        *last ^= 0x01;
    }

    let res = ron_kms::sealed::unseal(&key, &sealed, aad);
    assert!(res.is_err(), "tamper must be detected");
}
