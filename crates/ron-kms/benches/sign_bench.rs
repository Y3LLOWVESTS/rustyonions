use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_kms::{memory_keystore, Keystore, Signer};

pub fn bench_sign(c: &mut Criterion) {
    let kms = memory_keystore();
    let kid = kms.create_ed25519("bench", "sign").expect("create");
    let msg = vec![0u8; 128];

    c.bench_function("ed25519_sign_128B", |b| {
        b.iter(|| {
            let sig = kms.sign(&kid, black_box(&msg)).expect("sign");
            black_box(sig);
        });
    });
}

criterion_group!(benches, bench_sign);
criterion_main!(benches);
