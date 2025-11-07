use rand::RngCore;
use ron_kms::backends::ed25519;

/// Property: batch verify must agree with single verifies for the same tuples.
#[test]
fn batch_and_single_verify_agree() {
    // Use the public backend adapter (no keystore traits needed).
    // One keypair, many messages/signatures.
    let (pk, seed) = ed25519::generate();

    let n = 64usize;
    let mut msgs: Vec<Vec<u8>> = Vec::with_capacity(n);
    let mut sigs: Vec<[u8; 64]> = Vec::with_capacity(n);
    let mut pks: Vec<[u8; 32]> = Vec::with_capacity(n);

    for _ in 0..n {
        let mut msg = vec![0u8; 128];
        rand::thread_rng().fill_bytes(&mut msg);
        let sig = ed25519::sign(&seed, &msg);
        // keep parallel arrays
        pks.push(pk);
        sigs.push(sig);
        msgs.push(msg);
    }

    // 1) Single strict verifies
    for i in 0..n {
        let ok = ed25519::verify(&pks[i], &msgs[i], &sigs[i]);
        assert!(ok, "single verify {} failed", i);
    }

    // 2) Batch verify
    let msgs_ref: Vec<&[u8]> = msgs.iter().map(|m| m.as_slice()).collect();
    let batch_ok = ed25519::verify_batch(&pks, &msgs_ref, &sigs);
    assert!(batch_ok, "batch verify failed");
}
