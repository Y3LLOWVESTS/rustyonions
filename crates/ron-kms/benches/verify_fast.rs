#![cfg(feature = "fast")]
//! RO:WHAT  A/B bench for fast (ring) verify on 128B message.
//! RO:WHY   Quantify single-op latency vs default dalek path.

use criterion::{criterion_group, criterion_main, Criterion};
use ron_kms::backends::ed25519;

fn bench_verify_fast(c: &mut Criterion) {
    let (sk_blob, pk) = ed25519::ed25519_generate();
    let msg = vec![0u8; 128];
    let sig = ed25519::ed25519_sign(&sk_blob, &msg);

    c.bench_function("ed25519_verify_fast_128B", |b| {
        b.iter(|| {
            let _ok = ed25519::ed25519_verify(&pk, &msg, &sig);
        })
    });
}

criterion_group!(benches, bench_verify_fast);
criterion_main!(benches);
