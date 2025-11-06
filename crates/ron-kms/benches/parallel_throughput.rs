//! RO:WHAT — Parallel throughput benches for sign/verify using 4 worker threads.
use criterion::{criterion_group, criterion_main, Criterion};
use ron_kms::{memory_keystore, Keystore, Signer, Verifier};
use std::sync::Arc;
use std::thread;

fn bench_parallel(c: &mut Criterion) {
    let kms = Arc::new(memory_keystore());
    let kid = kms.create_ed25519("bench", "parallel").expect("create");
    let msg = b"parallel-msg".to_vec();
    let sig = kms.sign(&kid, &msg).expect("sign");

    let workers = 4usize;
    let iters_per_worker = 1000usize;

    c.bench_function("parallel_sign_4x", |b| {
        b.iter(|| {
            thread::scope(|s| {
                let mut handles = Vec::new();
                for _ in 0..workers {
                    let kms = kms.clone();
                    let kid = kid.clone();
                    let msg = msg.clone();
                    handles.push(s.spawn(move || {
                        for _ in 0..iters_per_worker {
                            let _ = kms.sign(&kid, &msg).unwrap();
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    });

    c.bench_function("parallel_verify_4x", |b| {
        b.iter(|| {
            thread::scope(|s| {
                let mut handles = Vec::new();
                for _ in 0..workers {
                    let kms = kms.clone();
                    let kid = kid.clone();
                    let msg = msg.clone();
                    let sig = sig.clone();
                    handles.push(s.spawn(move || {
                        for _ in 0..iters_per_worker {
                            let _ = kms.verify(&kid, &msg, &sig).unwrap();
                        }
                    }));
                }
                for h in handles {
                    h.join().unwrap();
                }
            });
        });
    });
}

criterion_group!(benches, bench_parallel);
criterion_main!(benches);
