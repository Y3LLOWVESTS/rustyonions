use criterion::{criterion_group, criterion_main, Criterion};
fn bench_stub(_c: &mut Criterion) {}
criterion_group!(benches, bench_stub);
criterion_main!(benches);
