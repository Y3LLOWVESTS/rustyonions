use criterion::{criterion_group, criterion_main, Criterion};
fn bench_reward(c: &mut Criterion) { c.bench_function("reward_calc_stub", |b| b.iter(|| 1+1)); }
criterion_group!(benches, bench_reward);
criterion_main!(benches);
