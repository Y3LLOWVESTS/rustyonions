//! RO:WHAT — Demo: bridge ron-bus events into ron-metrics endpoints.
//! Run: cargo run -p ron-metrics --features bus --example watch_bus

use ron_metrics::{build_info::build_version, BaseLabels, HealthState, Metrics};
use std::{env, net::SocketAddr, time::Duration};

#[cfg(feature = "bus")]
use {
    ron_bus::{Bus, BusConfig, Event},
    ron_metrics::bus_watcher::start_bus_watcher,
    tokio::time::sleep,
};

/// When `bus` feature is OFF, provide a tiny placeholder main so examples still build under `cargo test`.
#[cfg(not(feature = "bus"))]
fn main() {
    eprintln!("watch_bus example requires `--features bus`");
}

/// Real example when `bus` feature is ON.
#[cfg(feature = "bus")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let base = BaseLabels {
        service: env::var("RON_SERVICE").unwrap_or_else(|_| "demo".into()),
        instance: env::var("RON_INSTANCE").unwrap_or_else(|_| "local-1".into()),
        build_version: build_version(),
        amnesia: env::var("RON_AMNESIA").unwrap_or_else(|_| "off".into()),
    };

    let cfg = BusConfig::default().with_capacity(256);
    let bus = Bus::new(cfg).expect("bus");

    let health = HealthState::new();
    health.set("config_loaded".into(), false);

    let metrics = Metrics::new(base, health)?;
    let bind: SocketAddr = env::var("RON_METRICS_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:0".into())
        .parse()?;

    let (_jh, addr) = metrics.clone().serve(bind).await?;
    println!("metrics:  http://{addr}/metrics");
    println!("healthz:  http://{addr}/healthz");
    println!("readyz :  http://{addr}/readyz");

    let _watcher = start_bus_watcher(metrics.clone(), &bus, "demo-watcher");

    let tx = bus.sender();
    tx.send(Event::Health {
        service: "config_loaded".into(),
        ok: false,
    })
    .expect("send");
    sleep(Duration::from_millis(250)).await;
    tx.send(Event::Health {
        service: "config_loaded".into(),
        ok: true,
    })
    .expect("send");

    println!("curl the endpoints now. shutting down in ~3s…");
    sleep(Duration::from_secs(3)).await;
    tx.send(Event::Shutdown).expect("send");

    Ok(())
}
