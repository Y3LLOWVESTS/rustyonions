use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use ron_kms::backends::ed25519;

#[cfg(not(feature = "fast"))]
mod native {
    pub use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
}
#[cfg(feature = "fast")]
mod native {
    pub use ring::signature::{UnparsedPublicKey, ED25519};
}

fn rand_bytes(len: usize, rng: &mut StdRng) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rng.fill_bytes(&mut buf);
    buf
}

pub fn bench_verify(c: &mut Criterion) {
    let mut g = c.benchmark_group("ed25519_verify_128B");
    let mut rng = StdRng::seed_from_u64(43);

    let (pk, sk_seed) = ed25519::generate();
    let msg = rand_bytes(128, &mut rng);
    let sig = ed25519::sign(&sk_seed, &msg);

    // Adapter path (bytes -> verify)
    g.bench_function(BenchmarkId::new("adapter", "bytes_verify"), |b| {
        b.iter(|| {
            let _ok = ed25519::verify(&pk, &msg, &sig);
        })
    });

    // Steady-state: preparse verifying key + signature; verify only
    #[cfg(not(feature = "fast"))]
    {
        use native::*;
        let vk = VerifyingKey::from_bytes(&pk).expect("vk");
        let sig = Signature::from_slice(&sig).expect("sig");

        g.bench_function(BenchmarkId::new("steady", "dalek_vk.verify"), |b| {
            b.iter(|| {
                let _ = vk.verify_strict(&msg, &sig);
            })
        });
    }

    #[cfg(feature = "fast")]
    {
        use native::*;
        let verifier = UnparsedPublicKey::new(&ED25519, &pk);

        g.bench_function(BenchmarkId::new("steady", "ring_unparsed.verify"), |b| {
            b.iter(|| {
                let _ = verifier.verify(&msg, &sig);
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
    targets = bench_verify
}
criterion_main!(benches);
