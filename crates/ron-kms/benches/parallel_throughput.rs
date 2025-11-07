use criterion::{criterion_group, criterion_main, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use std::sync::Arc;
use std::thread;

use ron_kms::backends::ed25519;

fn rand_bytes(len: usize, rng: &mut StdRng) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);
    buf
}

const OPS: usize = 4_000; // per run
const THREADS: usize = 4;

pub fn bench_parallel(c: &mut Criterion) {
    let mut g = c.benchmark_group("parallel");

    // --- Sign 4× ---
    g.bench_function("parallel_sign_4x", |b| {
        let mut rng = StdRng::seed_from_u64(777);
        let (_pk, sk) = ed25519::generate();
        let msg = Arc::new(rand_bytes(128, &mut rng));

        b.iter(|| {
            let mut handles = Vec::with_capacity(THREADS);
            for _ in 0..THREADS {
                let sk = sk; // copy seed
                let msg = Arc::clone(&msg);
                handles.push(thread::spawn(move || {
                    for _ in 0..(OPS / THREADS) {
                        let _ = ed25519::sign(&sk, &msg);
                    }
                }));
            }
            for h in handles {
                let _ = h.join();
            }
        });
    });

    // --- Verify 4× ---
    g.bench_function("parallel_verify_4x", |b| {
        let mut rng = StdRng::seed_from_u64(778);
        let (pk, sk) = ed25519::generate();
        let msg = Arc::new(rand_bytes(128, &mut rng));
        let sig = ed25519::sign(&sk, &msg);

        b.iter(|| {
            let mut handles = Vec::with_capacity(THREADS);
            for _ in 0..THREADS {
                let pk = pk;
                let msg = Arc::clone(&msg);
                let sig = sig;
                handles.push(thread::spawn(move || {
                    for _ in 0..(OPS / THREADS) {
                        let _ = ed25519::verify(&pk, &msg, &sig);
                    }
                }));
            }
            for h in handles {
                let _ = h.join();
            }
        });
    });

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(60)
        .measurement_time(std::time::Duration::from_secs(8))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = bench_parallel
}
criterion_main!(benches);
