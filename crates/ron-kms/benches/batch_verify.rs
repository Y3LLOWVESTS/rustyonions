//! RO:WHAT — Criterion benches for batch verify (8/32/64).
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use ron_kms::ops::verify_batch::{verify_batch, VerifyItem};
use ron_kms::{memory_keystore, Keystore, Signer};

fn bench_batch_verify(c: &mut Criterion) {
    let kms = memory_keystore();
    let kid = kms.create_ed25519("bench", "batch").expect("create");

    let sizes = [8usize, 32, 64];
    for &n in &sizes {
        // Precompute messages and signatures
        let mut msgs = Vec::with_capacity(n);
        let mut sigs = Vec::with_capacity(n);
        for i in 0..n {
            let m = format!("msg-{i}").into_bytes();
            let s = kms.sign(&kid, &m).expect("sign");
            msgs.push(m);
            sigs.push(s);
        }

        c.bench_function(&format!("verify_batch_{n}"), |b| {
            b.iter_batched(
                || {
                    (0..n)
                        .map(|i| VerifyItem {
                            kid: &kid,
                            msg: &msgs[i],
                            sig: &sigs[i],
                        })
                        .collect::<Vec<_>>()
                },
                |items| {
                    let _ = verify_batch(&kms, &items).expect("batch");
                },
                BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(benches, bench_batch_verify);
criterion_main!(benches);
