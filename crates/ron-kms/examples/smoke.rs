use ron_kms::{memory_keystore, Keystore, Signer, Verifier};

fn main() -> anyhow::Result<()> {
    let kms = memory_keystore();
    let kid = kms.create_ed25519("auth", "signing")?;
    let msg = b"rustyonions";
    let sig = kms.sign(&kid, msg)?;
    assert!(kms.verify(&kid, msg, &sig)?);
    Ok(())
}
