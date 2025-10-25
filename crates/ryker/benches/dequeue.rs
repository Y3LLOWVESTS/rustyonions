//! RO:WHAT — Criterion bench: dequeue throughput.
//! RO:INVAR — Prefill with send(); drop TX; drain RX until N or Closed.

use criterion::{criterion_group, criterion_main, Criterion};
use ryker::prelude::*;
use std::time::Duration;
use tokio::runtime::{Builder as TokioBuilder, Runtime as TokioRt};

const CAP: usize = 1024;
const N_MSGS: usize = CAP;

fn tokio_rt() -> TokioRt {
    TokioBuilder::new_current_thread()
        .enable_time()
        .build()
        .expect("tokio rt")
}

fn bench_dequeue(c: &mut Criterion) {
    let rt = tokio_rt();
    c.bench_function("ryker_dequeue_pull", |b| {
        b.to_async(&rt).iter(|| async {
            let cfg = ryker::config::from_env_validated().unwrap();
            let ry = Runtime::new(cfg);

            let mb = ry
                .mailbox::<u64>("bench.dequeue")
                .capacity(CAP)
                .deadline(Duration::from_millis(10))
                .build();

            let (tx, mut rx) = mb.split();

            for i in 0..N_MSGS as u64 {
                tx.send(i).await.expect("prefill");
            }
            drop(tx); // important: close the channel

            let mut n = 0usize;
            loop {
                match rx.pull().await {
                    Ok(_m) => {
                        n += 1;
                        if n >= N_MSGS {
                            break;
                        }
                    }
                    Err(ryker::mailbox::MailboxError::Closed) => break,
                    Err(ryker::mailbox::MailboxError::Timeout) => break,
                    Err(e) => panic!("unexpected: {e}"),
                }
                tokio::task::yield_now().await;
            }
        });
    });
}

criterion_group!(benches, bench_dequeue);
criterion_main!(benches);
