//! RO:WHAT — Simulate activity against Metrics so /metrics moves live.
//! Run: RON_METRICS_METRICS_ADDR=127.0.0.1:0 cargo run -p ron-metrics --example pump

use ron_metrics::{build_info::build_version, BaseLabels, HealthState, Metrics};
use rand::{rng, Rng}; // rand 0.9 API
use std::{env, net::SocketAddr, time::Duration};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let service = env::var("RON_SERVICE").unwrap_or_else(|_| "demo".into());
    let instance = env::var("RON_INSTANCE").unwrap_or_else(|_| "local-1".into());
    let amnesia = match env::var("RON_AMNESIA").ok().as_deref() {
        Some("on") | Some("1") | Some("true") => "on".to_string(),
        _ => "off".to_string(),
    };

    let base = BaseLabels {
        service,
        instance,
        build_version: build_version(),
        amnesia,
    };

    let health = HealthState::new();
    health.set("config_loaded".to_string(), true);
    health.set("db".to_string(), false);
    health.set("cache".to_string(), false);

    let metrics = Metrics::new(base, health)?;
    let bind: SocketAddr = env::var("RON_METRICS_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:0".into())
        .parse()?;

    let (_jh, addr) = metrics.clone().serve(bind).await?;
    println!("metrics:  http://{addr}/metrics");
    println!("healthz:  http://{addr}/healthz");
    println!("readyz :  http://{addr}/readyz");

    // Simulate activity forever
    let mut tick: u64 = 0;
    loop {
        let mut r = rng();

        // Simulate a request with 0.3–20 ms latency
        let lat_ms: f64 = r.random_range(0.3..20.0);
        metrics.observe_request(lat_ms / 1000.0);

        // Pretend a service restarted every ~15s
        if tick % 15 == 0 {
            metrics.inc_service_restart("worker-A");
        }

        // Pretend the bus overwrote some lagged messages sporadically
        if tick % 7 == 0 {
            let overwrites: u64 = r.random_range(1..5);
            metrics.add_bus_lag("kernel", overwrites);
        }

        // Flip readiness of db/cache occasionally to show /readyz truth
        if tick % 9 == 0 {
            let ok = r.random::<bool>();
            metrics.set_ready("db", ok);
        }
        if tick % 11 == 0 {
            let ok = r.random::<bool>();
            metrics.set_ready("cache", ok);
        }

        tick += 1;
        sleep(Duration::from_millis(1000)).await;
    }
}
