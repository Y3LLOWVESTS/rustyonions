use criterion::{criterion_group, criterion_main, Criterion};
fn bench_transfer(c: &mut Criterion) { c.bench_function("transfer_commit", |b| b.iter(|| 1)); }
criterion_group!(benches, bench_transfer);
criterion_main!(benches);
