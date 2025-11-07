//! Throughput bench for parallel multiscalar batch verification.
//! Measures sustained verifies/sec at several batch sizes while keeping setup outside the timing.
//!
//! Run (parallel multiscalar enabled):
//! RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=4 \
//!   cargo bench -p ron-kms --features "dalek-batch,parallel-batch" \
//!   --bench throughput_batch -- --measurement-time 14 --sample-size 90
//!
//! What it reports:
//! - Criterion shows time/iter, and because we set Throughput::Elements(n), it also shows
//!   "elements/s" = messages verified per second for each batch size.
//!
//! Notes:
//! - We generate valid (pk, msg, sig) tuples once per N and reuse them to isolate curve verify cost.
//! - No unsafe. No 'static lifetimes required.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use ron_kms::backends::ed25519;

/// Build a valid batch and return OWNED data so we can derive &\[u8] slices later.
fn prepare_valid_batch(n: usize) -> (Vec<[u8; 32]>, Vec<Box<[u8]>>, Vec<[u8; 64]>) {
    let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF_F00D_1234);
    let mut msg_bufs: Vec<Box<[u8]>> = Vec::with_capacity(n);
    let mut pks: Vec<[u8; 32]> = Vec::with_capacity(n);
    let mut sigs: Vec<[u8; 64]> = Vec::with_capacity(n);

    for _ in 0..n {
        // Random message length in [64, 256] to add mild variance
        let mlen = 64 + (rng.next_u32() as usize % 193);
        let mut m = vec![0u8; mlen].into_boxed_slice();
        rng.fill_bytes(&mut m);

        let (pk, sk) = ed25519::generate();
        let sig = ed25519::sign(&sk, &m);

        msg_bufs.push(m);
        pks.push(pk);
        sigs.push(sig);
    }

    (pks, msg_bufs, sigs)
}

fn bench_throughput(c: &mut Criterion) {
    for &n in &[64usize, 128, 256, 512] {
        // Keep owned buffers alive for the entire group scope
        let (pks, owned_msgs, sigs) = prepare_valid_batch(n);

        // Build borrowed slice view from owned buffers
        let msgs: Vec<&[u8]> = owned_msgs.iter().map(|b| b.as_ref()).collect();

        let mut group = c.benchmark_group(format!("batch_throughput/{n}"));
        group.throughput(Throughput::Elements(n as u64));

        group.bench_function("verify_parallel_multiscalar", |b| {
            b.iter(|| {
                let ok = ed25519::verify_batch(&pks, &msgs, &sigs);
                // Keep the assert so the optimizer can't DCE the call.
                assert!(ok);
            });
        });

        group.finish();
    }
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
