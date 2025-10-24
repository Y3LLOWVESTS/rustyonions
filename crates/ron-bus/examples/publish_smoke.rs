//! RO:WHAT — Minimal publish/subscribe smoke example
//! RO:WHY  — Shows intended pattern: one receiver per task, bounded queue, graceful shutdown
//! RO:INTERACTS — Bus, BusConfig, Event
//! RO:INVARIANTS — no background tasks created by the library; host owns subscribers/metrics

use ron_bus::{Bus, BusConfig, Event};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let bus = Bus::new(BusConfig::new().with_capacity(1024)).expect("bus");
    let tx = bus.sender();

    let mut rx = bus.subscribe();
    let worker = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::Shutdown) => {
                    println!("worker: shutdown received");
                    break;
                }
                Ok(ev) => {
                    println!("worker: got {:?}", ev);
                }
                Err(e) => {
                    println!("worker: recv error = {:?}", e);
                    break;
                }
            }
        }
    });

    let _ = tx.send(Event::Health {
        service: "svc.a".into(),
        ok: true,
    });
    let _ = tx.send(Event::ConfigUpdated { version: 1 });
    let _ = tx.send(Event::Shutdown);

    let _ = worker.await;
}
