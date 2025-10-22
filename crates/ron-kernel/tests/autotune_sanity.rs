use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn bench_autotune(c: &mut Criterion) {
    let mut g = c.benchmark_group("autotune");
    g.bench_function("expected_4_none", |b| {
        b.iter(|| {
            let mut sum = 0usize;
            for n in 0..1000 {
                // Call through a small trampoline to avoid LTO folding in release.
                sum ^= tramp(4 + (n & 1));
            }
            black_box(sum)
        })
    });
    g.finish();
}

#[inline(never)]
fn tramp(expected: usize) -> usize {
    ron_kernel::mog_autotune::autotune_capacity(expected, None)
}

criterion_group!(benches, bench_autotune);
criterion_main!(benches);
