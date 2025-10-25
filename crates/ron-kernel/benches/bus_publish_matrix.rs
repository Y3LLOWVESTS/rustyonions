/*!
RO: benches/bus_publish_matrix.rs
WHAT: Parameterized publish() cost across subscriber counts and capacities.
WHY : Locate sweet spots for default capacity vs fan-out cost.
NOTE: Subscribers actively drain to avoid unbounded lag skewing results.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use ron_kernel::{Bus, KernelEvent, Metrics};
use std::time::Duration;
use tokio::runtime::Builder;

const CAPS: &[usize] = &[32, 64, 128, 256, 4096];
const SUBS: &[usize] = &[0, 1, 4, 16];
const INNER_PUBLISHES: usize = 10_000;

fn bench_bus_publish_matrix(c: &mut Criterion) {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();

    let mut group = c.benchmark_group("bus_publish_matrix");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(60);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(8));

    for &subs in SUBS {
        for &cap in CAPS {
            let id = BenchmarkId::new("publish", format!("subs{}_cap{}", subs, cap));
            group.bench_with_input(id, &(), |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let metrics = Metrics::new(false);
                        let bus: Bus<KernelEvent> = metrics.make_bus(cap);

                        // spawn drains
                        let mut joins = Vec::new();
                        for _ in 0..subs {
                            let mut rx = bus.subscribe();
                            joins.push(tokio::spawn(async move {
                                while let Ok(_ev) = rx.recv().await {}
                            }));
                        }

                        for i in 0..black_box(INNER_PUBLISHES) {
                            let _ = bus.publish(KernelEvent::ConfigUpdated { version: i as u64 });
                        }

                        drop(bus);
                        for j in joins {
                            let _ = j.await;
                        }
                    });
                });
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_bus_publish_matrix);
criterion_main!(benches);
