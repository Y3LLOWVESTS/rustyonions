/*!
RO: benches/bus_overflow_drop.rs
WHAT: Stress bounded bus with a slow subscriber; ensure publish cost is bounded.
WHY : Validate overflow path keeps publisher fast (drops accounted on recv side).
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use ron_kernel::{Bus, KernelEvent, Metrics};
use std::time::Duration;
use tokio::runtime::Builder;

const INNER_PUBLISHES: usize = 50_000;

fn bench_overflow(c: &mut Criterion) {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("bus_overflow");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));

    group.bench_with_input(
        BenchmarkId::new("slow_single_subscriber", INNER_PUBLISHES),
        &(),
        |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let metrics = Metrics::new(false);
                    // Small capacity to induce overflow quickly
                    let bus: Bus<KernelEvent> = metrics.make_bus(32);

                    let mut rx = bus.subscribe();
                    let slow = tokio::spawn(async move {
                        loop {
                            match rx.recv().await {
                                Ok(_e) => {
                                    tokio::time::sleep(Duration::from_micros(black_box(50))).await
                                }
                                Err(_) => break,
                            }
                        }
                    });

                    for i in 0..black_box(INNER_PUBLISHES) {
                        let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                    }

                    drop(bus);
                    let _ = slow.await;
                });
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_overflow);
criterion_main!(benches);
