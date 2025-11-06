use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_kms::{memory_keystore, Keystore, Signer, Verifier};

pub fn bench_verify(c: &mut Criterion) {
    let kms = memory_keystore();
    let kid = kms.create_ed25519("bench", "verify").expect("create");
    let msg = vec![0u8; 128];
    let sig = kms.sign(&kid, &msg).expect("sign");

    c.bench_function("ed25519_verify_128B", |b| {
        b.iter(|| {
            let ok = kms
                .verify(&kid, black_box(&msg), black_box(&sig))
                .expect("verify");
            assert!(ok);
        });
    });
}

criterion_group!(benches, bench_verify);
criterion_main!(benches);
