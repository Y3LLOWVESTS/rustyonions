use criterion::{criterion_group, criterion_main, Criterion};
fn bench_balance(c: &mut Criterion) { c.bench_function("balance_read", |b| b.iter(|| 1)); }
criterion_group!(benches, bench_balance);
criterion_main!(benches);
