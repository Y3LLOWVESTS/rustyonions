use ron_kms::{memory_keystore, Alg, KeyId, Keystore, Signer, Verifier};
use std::str::FromStr;

#[test]
fn keyid_format_parse_roundtrip() {
    let kid = KeyId::new("tenantA", "purposeX", Alg::Ed25519);
    let s = kid.to_string();
    let parsed = KeyId::from_str(&s).expect("parse");
    assert_eq!(kid.tenant, parsed.tenant);
    assert_eq!(kid.purpose, parsed.purpose);
    assert_eq!(kid.alg, parsed.alg);
    assert_eq!(kid.uuid, parsed.uuid);
    assert_eq!(kid.version, parsed.version);
}

#[test]
fn sign_verify_roundtrip() {
    let kms = memory_keystore();
    let kid = kms.create_ed25519("auth", "signing").expect("create");
    let msg = b"roundtrip";
    let sig = kms.sign(&kid, msg).expect("sign");
    let ok = kms.verify(&kid, msg, &sig).expect("verify");
    assert!(ok);
}

#[test]
fn rotate_bumps_version_and_still_works() {
    let kms = memory_keystore();
    let kid_v1 = kms.create_ed25519("auth", "signing").expect("create");

    // v1 sign/verify
    let msg1 = b"before rotate";
    let sig1 = kms.sign(&kid_v1, msg1).expect("sign v1");
    assert!(kms.verify(&kid_v1, msg1, &sig1).expect("verify v1"));

    // rotate → v2
    let kid_v2 = kms.rotate(&kid_v1).expect("rotate");
    assert_eq!(kid_v2.version, kid_v1.version + 1);

    // v2 sign/verify
    let msg2 = b"after rotate";
    let sig2 = kms.sign(&kid_v2, msg2).expect("sign v2");
    assert!(kms.verify(&kid_v2, msg2, &sig2).expect("verify v2"));

    // NOTE: In the current dev backend, rotating replaces the stored public key.
    // Old signatures are not guaranteed to verify after rotation (by design here).
}
