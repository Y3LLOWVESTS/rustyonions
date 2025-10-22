#![cfg(feature = "bus_autotune_cap")]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, black_box};
use ron_kernel::autotune_capacity;



fn bench_autotune_mapping(c: &mut Criterion) {
    let mut g = c.benchmark_group("a3_autotune_mapping");

    for &n in &[1usize, 4, 16, 64, 128] {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut s = 0usize;
                for _ in 0..1024 {
                    s ^= autotune_capacity(n, None);
                }
                black_box(s)
            })
        });
    }

    g.bench_function("override_192", |b| {
        b.iter(|| black_box(autotune_capacity(16, Some(192))))
    });

    g.finish();
}

criterion_group!(benches, bench_autotune_mapping);
criterion_main!(benches);
