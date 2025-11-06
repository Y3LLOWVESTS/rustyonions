// Minimal metrics demo: perform a few ops, then print Prometheus text.
use ron_kms::memory_keystore;
use ron_kms::ops::{attest::attest, create, rotate, sign, verify};

fn main() -> anyhow::Result<()> {
    // Do a few ops so counters/histogram tick.
    let kms = memory_keystore();
    let kid = create::ed25519(&kms, "auth", "signing")?;
    let msg = b"hello-metrics";
    let sig = sign::sign(&kms, &kid, msg)?;
    assert!(verify::verify(&kms, &kid, msg, &sig)?);
    let kid2 = rotate::rotate(&kms, &kid)?;
    let _ = attest(&kms, &kid2)?;

    // Print Prometheus exposition to stdout (feature `with-metrics` must be enabled).
    #[cfg(feature = "with-metrics")]
    {
        use prometheus::{gather, Encoder, TextEncoder};
        let encoder = TextEncoder::new();
        let mf = gather();
        let mut buf = Vec::new();
        encoder.encode(&mf, &mut buf).expect("encode");
        println!("{}", String::from_utf8_lossy(&buf));
    }

    Ok(())
}
