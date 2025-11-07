#![cfg(feature = "fast")]
//! RO:WHAT  A/B bench for fast (ring) sign on 128B message.
//! RO:WHY   Quantify single-op latency vs default dalek path.

use criterion::{criterion_group, criterion_main, Criterion};
use ron_kms::backends::ed25519;

fn bench_sign_fast(c: &mut Criterion) {
    let (sk_blob, _pk) = ed25519::ed25519_generate();
    let msg = vec![0u8; 128];

    c.bench_function("ed25519_sign_fast_128B", |b| {
        b.iter(|| {
            let _sig = ed25519::ed25519_sign(&sk_blob, &msg);
        })
    });
}

criterion_group!(benches, bench_sign_fast);
criterion_main!(benches);
