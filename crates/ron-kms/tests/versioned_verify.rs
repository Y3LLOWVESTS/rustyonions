use ron_kms::{memory_keystore, Keystore, Signer, Verifier};

#[test]
fn old_signatures_verify_after_rotate() {
    let kms = memory_keystore();

    // Create v1 and sign a message.
    let kid_v1 = kms.create_ed25519("auth", "signing").expect("create");
    let msg_v1 = b"old message";
    let sig_v1 = kms.sign(&kid_v1, msg_v1).expect("sign v1");
    assert!(kms.verify(&kid_v1, msg_v1, &sig_v1).expect("verify v1"));

    // Rotate to v2.
    let kid_v2 = kms.rotate(&kid_v1).expect("rotate");
    assert_eq!(kid_v2.version, kid_v1.version + 1);

    // Old v1 signature should still verify using the v1 KeyId.
    assert!(kms
        .verify(&kid_v1, msg_v1, &sig_v1)
        .expect("verify v1 after rotate"));

    // New v2 signing works; verifying with v2 KeyId works.
    let msg_v2 = b"new message";
    let sig_v2 = kms.sign(&kid_v2, msg_v2).expect("sign v2");
    assert!(kms.verify(&kid_v2, msg_v2, &sig_v2).expect("verify v2"));

    // Attempting to sign with stale v1 should fail (only latest may sign).
    let stale_sign = kms.sign(&kid_v1, b"should fail");
    assert!(stale_sign.is_err(), "stale KeyId should not sign");
}
