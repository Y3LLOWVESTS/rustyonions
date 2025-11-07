use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use ron_kms::backends::ed25519;

fn rand_bytes(len: usize, rng: &mut StdRng) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);
    buf
}

pub fn bench_batch_verify(c: &mut Criterion) {
    let mut g = c.benchmark_group("verify_batch");

    for &n in &[8usize, 32, 64] {
        g.bench_function(BenchmarkId::from_parameter(n), |b| {
            // Setup: N keypairs, messages, sigs
            let mut rng = StdRng::seed_from_u64(99 + n as u64);

            let mut pks = Vec::with_capacity(n);
            let mut msgs = Vec::with_capacity(n);
            let mut sigs = Vec::with_capacity(n);
            for _ in 0..n {
                let (pk, sk) = ed25519::generate();
                let m = rand_bytes(128, &mut rng);
                let s = ed25519::sign(&sk, &m);
                pks.push(pk);
                msgs.push(m);
                sigs.push(s);
            }
            let msg_refs: Vec<&[u8]> = msgs.iter().map(|m| m.as_slice()).collect();

            // Measure: single call into backend batch
            b.iter_batched(
                || (),
                |_| {
                    let _ = ed25519::verify_batch(&pks, &msg_refs, &sigs);
                },
                BatchSize::LargeInput,
            );
        });
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(90)
        .measurement_time(std::time::Duration::from_secs(12))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = bench_batch_verify
}
criterion_main!(benches);
