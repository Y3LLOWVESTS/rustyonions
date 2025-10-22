use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tokio::sync::broadcast;

// Measure publish throughput across (subs × capacity) matrix using only public API.
// This bench compiles with/without features (bus_edge_notify, bus_batch).

use ron_kernel::bus::bounded::Bus;

// Drain helper: non-blocking drain, like a simple “now_or_never” for broadcast.
fn drain_now<T: Clone + Send + 'static>(rx: &mut broadcast::Receiver<T>) -> usize {
    use tokio::sync::broadcast::error::TryRecvError::*;
    let mut n = 0usize;
    loop {
        match rx.try_recv() {
            Ok(_) => n += 1,
            Err(Empty) => break,
            Err(Lagged(_)) => continue, // skip lagged; keep draining to head
            Err(Closed) => break,
        }
    }
    n
}

#[cfg(feature = "bus_edge_notify")]
mod edge_metrics {
    use prometheus::proto::MetricType;
    use prometheus::Registry;

    pub fn snapshot(reg: &Registry, name: &str) -> u64 {
        let mut sum = 0f64;
        for mf in reg.gather() {
            if mf.name() == name && mf.get_field_type() == MetricType::COUNTER {
                for m in mf.get_metric() {
                    if let Some(c) = m.get_counter().as_ref() {
                        // rust-protobuf 3.x style: scalar getter is `value()`
                        sum += c.value();
                    }
                }
            }
        }
        sum as u64
    }

    pub fn default_registry() -> Registry {
        prometheus::default_registry().clone()
    }
}

fn bench_publish_edge_matrix(c: &mut Criterion) {
    // Single-threaded runtime for more stable results
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("publish_edge_matrix");
    // Tunable matrix; keep small for CI time but representative for perf
    let subs_set = [0usize, 1, 4, 16];
    let caps_set = [32usize, 64, 128, 256];

    // Amount of work per iteration (elements pushed)
    const ELEMS: u64 = 50_000;

    #[cfg(feature = "bus_edge_notify")]
    let reg = edge_metrics::default_registry();

    for &subs in &subs_set {
        for &cap in &caps_set {
            group.throughput(Throughput::Elements(ELEMS));
            group.bench_with_input(
                BenchmarkId::new("publish_single", format!("subs={subs},cap={cap}")),
                &(),
                |b, _| {
                    rt.block_on(async {
                        let bus: Bus<u64> = Bus::with_capacity(cap);

                        // Spawn subscribers
                        let mut rxs: Vec<_> = (0..subs).map(|_| bus.subscribe()).collect();

                        #[cfg(feature = "bus_edge_notify")]
                        let sends_before = edge_metrics::snapshot(&reg, "bus_notify_sends_total");
                        #[cfg(feature = "bus_edge_notify")]
                        let suppressed_before =
                            edge_metrics::snapshot(&reg, "bus_notify_suppressed_total");

                        b.iter(|| {
                            // Producer: push ELEMS events
                            for i in 0..ELEMS {
                                let _ = bus.publish(i);
                            }
                            // Drain for fairness (so next iter starts empty)
                            for rx in &mut rxs {
                                let _ = drain_now(rx);
                            }
                            black_box(())
                        });

                        #[cfg(feature = "bus_edge_notify")]
                        {
                            let sends_after =
                                edge_metrics::snapshot(&reg, "bus_notify_sends_total");
                            let suppressed_after =
                                edge_metrics::snapshot(&reg, "bus_notify_suppressed_total");
                            let sends = sends_after.saturating_sub(sends_before);
                            let suppressed =
                                suppressed_after.saturating_sub(suppressed_before);
                            let total = sends + suppressed;
                            if total > 0 {
                                let pct = (suppressed as f64) * 100.0 / (total as f64);
                                eprintln!(
                                    "[edge] subs={subs}, cap={cap}: sends={sends}, suppressed={suppressed} ({pct:.1}% suppressed)"
                                );
                            }
                        }
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
    targets = bench_publish_edge_matrix
}
criterion_main!(benches);
