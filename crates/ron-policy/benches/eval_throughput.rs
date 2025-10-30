use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

fn bench_eval(c: &mut Criterion) {
    let bundle = load_json(include_bytes!("../tests/vectors/deny_region.json")).unwrap();
    let ev = Evaluator::new(&bundle).unwrap();
    let clock = SystemClock::default();

    c.bench_function("eval:get/us", |b| {
        b.iter(|| {
            let ctx = Context::builder()
                .tenant("t")
                .method("GET")
                .region("US")
                .build(&clock);
            let d = ev.evaluate(&ctx).unwrap();
            black_box(d);
        })
    });
}

criterion_group!(benches, bench_eval);
criterion_main!(benches);
