use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use ron_kernel::bus::bounded::Bus;
use tokio::sync::broadcast;

// Drain helper (same shape as other benches).
fn drain_now<T: Clone + Send + 'static>(rx: &mut broadcast::Receiver<T>) -> usize {
    use tokio::sync::broadcast::error::TryRecvError::*;
    let mut n = 0usize;
    loop {
        match rx.try_recv() {
            Ok(_) => n += 1,
            Err(Empty) => break,
            Err(Lagged(_)) => continue,
            Err(Closed) => break,
        }
    }
    n
}

fn bench_a2_batch_vs_single(c: &mut Criterion) {
    // single-threaded RT for stability
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("a2_publish_many");
    let subs_set = [1usize, 4];        // focus where notify matters
    let cap = 64usize;                 // tuned sweet spot from MOG
    let bursts = [1usize, 1_000, 5_000, 10_000];

    for &subs in &subs_set {
        for &batch_len in &bursts {
            group.throughput(Throughput::Elements(batch_len as u64));

            // Baseline: single publishes in a loop (no bus_batch feature needed)
            group.bench_with_input(
                BenchmarkId::new("single_loop", format!("subs={subs},cap={cap},n={batch_len}")),
                &(),
                |b, _| {
                    rt.block_on(async {
                        let bus: Bus<u64> = Bus::with_capacity(cap);
                        let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();

                        b.iter(|| {
                            for i in 0..batch_len as u64 {
                                let _ = bus.publish(i);
                            }
                            for rx in &mut rxs {
                                let _ = drain_now(rx);
                            }
                        });
                    });
                },
            );

            // A2: publish_many (feature-gated); when feature off, this target won’t exist
            #[cfg(feature = "bus_batch")]
            group.bench_with_input(
                BenchmarkId::new("publish_many", format!("subs={subs},cap={cap},n={batch_len}")),
                &(),
                |b, _| {
                    rt.block_on(async {
                        let bus: Bus<u64> = Bus::with_capacity(cap);
                        let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();

                        // preallocate batch to avoid alloc noise in iter
                        let batch: Vec<u64> = (0..batch_len as u64).collect();

                        b.iter(|| {
                            let _ = bus.publish_many(&batch);
                            for rx in &mut rxs {
                                let _ = drain_now(rx);
                            }
                        });
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = {
        Criterion::default()
            .measurement_time(Duration::from_secs(8))
            .warm_up_time(Duration::from_secs(3))
            .sample_size(20)
    };
    targets = bench_a2_batch_vs_single
}
criterion_main!(benches);
