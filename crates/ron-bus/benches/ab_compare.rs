//! RO:WHAT  — A/B microbench: ron-bus vs tokio::broadcast vs flume/async-channel.
//! RO:WHY   — Credible, apples-to-apples comparison for README claims.
//! RO:INTERACTS — ron_bus::{Bus, BusConfig, Event}; tokio runtime controlled here.
//! RO:INVARIANTS — Same fanout/capacity/runtime; comparable small POD payloads.
//! RO:NOTES — Microbench only; real-world variance expected.

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Builder;

// SUT
use ron_bus::{Bus, BusConfig, Event};

// Baselines
use async_channel as ac;
use flume as fl;
use tokio::sync::broadcast as tbc;

fn bench_publish(c: &mut Criterion) {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("ab_publish_cap1024_subs8");
    group.sample_size(50);
    // NOTE: No fixed warm-up/measurement times here: allow CLI flags to control it.

    // Params (keep identical across contenders)
    let cap_usize: usize = 1024;
    let cap_u32: u32 = cap_usize.try_into().unwrap();
    let subs: usize = 8;

    // ----------------- ron-bus -----------------
    group.bench_function("ron_bus_publish", |b| {
        b.iter(|| {
            rt.block_on(async {
                let bus = Bus::new(BusConfig::new().with_capacity(cap_u32)).unwrap();
                let tx = bus.sender();

                // Create N independent receivers (one per task)
                let rxs = (0..subs).map(|_| bus.subscribe()).collect::<Vec<_>>();

                // Drain tasks: break on Shutdown OR on channel close
                let mut tasks = Vec::with_capacity(subs);
                for mut rx in rxs {
                    tasks.push(tokio::spawn(async move {
                        loop {
                            match rx.recv().await {
                                Ok(Event::Shutdown) => break,
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        }
                    }));
                }

                // Publish a small batch
                for i in 0u64..10_000 {
                    let _ = tx.send(Event::ConfigUpdated { version: i });
                }
                let _ = tx.send(Event::Shutdown);

                // IMPORTANT: drop the sender so receivers observe close and exit
                drop(tx);

                for t in tasks {
                    let _ = t.await;
                }
            })
        })
    });

    // ----------------- tokio::broadcast -----------------
    group.bench_function("tokio_broadcast_publish", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (tx, _) = tbc::channel::<u64>(cap_usize);

                // Each subscriber comes from subscribe(); Receiver is NOT clonable.
                let mut tasks = Vec::with_capacity(subs);
                for _ in 0..subs {
                    let mut rx = tx.subscribe();
                    tasks.push(tokio::spawn(
                        async move { while rx.recv().await.is_ok() {} },
                    ));
                }

                for i in 0u64..10_000 {
                    let _ = tx.send(i);
                }
                drop(tx); // receivers will get Err and exit

                for t in tasks {
                    let _ = t.await;
                }
            })
        })
    });

    // ----------------- flume (bounded MPMC) -----------------
    group.bench_function("flume_mpmc_publish", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (tx, rx) = fl::bounded::<u64>(cap_usize);

                let mut tasks = Vec::with_capacity(subs);
                for _ in 0..subs {
                    let rx = rx.clone();
                    tasks.push(tokio::spawn(async move {
                        while rx.recv_async().await.is_ok() {}
                    }));
                }

                for i in 0u64..10_000 {
                    let _ = tx.send(i);
                }
                drop(tx);

                for t in tasks {
                    let _ = t.await;
                }
            })
        })
    });

    // ----------------- async-channel (bounded MPMC) -----------------
    group.bench_function("async_channel_mpmc_publish", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (tx, rx) = ac::bounded::<u64>(cap_usize);

                let mut tasks = Vec::with_capacity(subs);
                for _ in 0..subs {
                    let rx = rx.clone();
                    tasks.push(tokio::spawn(
                        async move { while rx.recv().await.is_ok() {} },
                    ));
                }

                for i in 0u64..10_000 {
                    let _ = tx.send(i).await;
                }
                drop(tx);

                for t in tasks {
                    let _ = t.await;
                }
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_publish);
criterion_main!(benches);
