use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use ron_kms::backends::ed25519;

#[cfg(not(feature = "fast"))]
mod native {
    pub use ed25519_dalek::{Signer as _, SigningKey};
}
#[cfg(feature = "fast")]
mod native {
    pub use ring::signature::Ed25519KeyPair;
}

fn rand_bytes(len: usize, rng: &mut StdRng) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);
    buf
}

pub fn bench_sign(c: &mut Criterion) {
    let mut g = c.benchmark_group("ed25519_sign_128B");
    let mut rng = StdRng::seed_from_u64(42);

    // Adapter path (seed -> sign) to reflect API
    {
        let (_pk, sk_seed) = ed25519::generate();
        let msg = rand_bytes(128, &mut rng);
        g.bench_function(BenchmarkId::new("adapter", "seed_→_sign"), |b| {
            b.iter(|| {
                let _sig = ed25519::sign(&sk_seed, &msg);
            });
        });
    }

    // Steady-state: prebuilt key, measure only .sign()
    #[cfg(not(feature = "fast"))]
    {
        use native::*;
        let sk_seed = {
            let (_, s) = ed25519::generate();
            s
        };
        let sk = SigningKey::from_bytes(&sk_seed);
        let msg = rand_bytes(128, &mut rng);

        g.bench_function(BenchmarkId::new("steady", "dalek_signingkey.sign"), |b| {
            b.iter(|| {
                let _ = sk.sign(&msg);
            })
        });
    }

    #[cfg(feature = "fast")]
    {
        use native::*;
        let sk_seed = {
            let (_, s) = ed25519::generate();
            s
        };
        let kp = Ed25519KeyPair::from_seed_unchecked(&sk_seed).expect("ring seed");
        let msg = rand_bytes(128, &mut rng);

        g.bench_function(BenchmarkId::new("steady", "ring_keypair.sign"), |b| {
            b.iter(|| {
                let _ = kp.sign(&msg);
            })
        });
    }

    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(120)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = bench_sign
}
criterion_main!(benches);
