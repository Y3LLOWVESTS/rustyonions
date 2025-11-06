// crates/ron-kms/examples/rotate.rs
use ron_kms::{memory_keystore, Keystore, Signer, Verifier};

fn main() -> anyhow::Result<()> {
    let kms = memory_keystore();

    // v1
    let kid_v1 = kms.create_ed25519("auth", "signing")?;
    let msg_v1 = b"first signature before rotation";
    let sig_v1 = kms.sign(&kid_v1, msg_v1)?;
    assert!(kms.verify(&kid_v1, msg_v1, &sig_v1)?);

    // rotate → v2
    let kid_v2 = kms.rotate(&kid_v1)?;
    assert_eq!(kid_v2.version, kid_v1.version + 1);

    // v2 signs and verifies
    let msg_v2 = b"second signature after rotation";
    let sig_v2 = kms.sign(&kid_v2, msg_v2)?;
    assert!(kms.verify(&kid_v2, msg_v2, &sig_v2)?);

    // NOTE: In this dev backend, the latest public key replaces the old one.
    // Old signatures are not guaranteed to verify after rotation (by design here).

    Ok(())
}
