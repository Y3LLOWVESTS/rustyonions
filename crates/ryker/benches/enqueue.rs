//! RO:WHAT — Criterion bench: enqueue throughput (producer side).
//! RO:INVAR — try_send burst, ignore Busy (we measure producer cost, not success rate).

use criterion::{criterion_group, criterion_main, Criterion};
use ryker::prelude::*;
use std::time::Duration;
use tokio::runtime::{Builder as TokioBuilder, Runtime as TokioRt};

const CAP: usize = 2048;
const N_TRIES: usize = CAP * 2; // intentionally over capacity

fn tokio_rt() -> TokioRt {
    TokioBuilder::new_current_thread()
        .enable_time()
        .build()
        .expect("tokio rt")
}

fn bench_enqueue(c: &mut Criterion) {
    let rt = tokio_rt();
    c.bench_function("ryker_enqueue_try_send", |b| {
        b.to_async(&rt).iter(|| async {
            let cfg = ryker::config::from_env_validated().unwrap();
            let ry = Runtime::new(cfg);

            let mb = ry
                .mailbox::<u64>("bench.enqueue")
                .capacity(CAP)
                .deadline(Duration::from_millis(5))
                .build();

            let (tx, _rx) = mb.split();

            for i in 0..N_TRIES as u64 {
                let _ = tx.try_send(i);
            }
        });
    });
}

criterion_group!(benches, bench_enqueue);
criterion_main!(benches);
