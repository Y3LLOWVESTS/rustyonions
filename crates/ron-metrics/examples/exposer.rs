//! RO:WHAT — Tiny demo: start the exposer and print bound addr.

use ron_metrics::build_info::build_version;
use ron_metrics::{BaseLabels, HealthState, Metrics};
use std::env;
use std::net::SocketAddr;

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
    // Declare one dependency and mark ready
    health.set("config_loaded".to_string(), true);

    let metrics = Metrics::new(base, health)?;
    let bind: SocketAddr = env::var("RON_METRICS_METRICS_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:9100".into())
        .parse()?;

    let (_jh, addr) = metrics.serve(bind).await?;
    println!("metrics:  http://{addr}/metrics");
    println!("healthz:  http://{addr}/healthz");
    println!("readyz :  http://{addr}/readyz");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
