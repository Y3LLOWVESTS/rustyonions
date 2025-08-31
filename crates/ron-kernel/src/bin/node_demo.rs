#![forbid(unsafe_code)]

use ron_kernel::{Bus, Config, HealthState, KernelEvent, Metrics};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, Level};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct ServiceSpec {
    name: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    let bus: Bus = Bus::new(1024);

    // IMPORTANT: build Metrics by value so `serve(self, ..)` can take ownership.
    let metrics = Metrics::new();

    let health = Arc::new(HealthState::new());

    // Start admin HTTP for metrics/health using the owned Metrics value.
    // FIX: parse String -> SocketAddr for Metrics::serve(addr: SocketAddr)
    let admin_addr: SocketAddr = Config::default().admin_addr.parse()?;
    let (_http_handle, bound) = metrics.clone().serve(admin_addr).await?;
    info!(%bound, "node_demo admin started");

    // After calling serve, wrap Metrics in Arc for sharing across tasks.
    let metrics = Arc::new(metrics);

    // Initialize health flags individually (no set_all on HealthState)
    for s in ["transport", "overlay", "index"] {
        health.set(s, false);
    }

    // Spawn a few mock services
    let specs = vec![
        ServiceSpec { name: "transport" },
        ServiceSpec { name: "overlay" },
        ServiceSpec { name: "index" },
    ];

    for spec in specs.clone() {
        let h = health.clone();
        let m = metrics.clone();
        let b = bus.clone();
        tokio::spawn(run_service(spec, h, m, b));
    }

    // Supervise loop
    tokio::spawn(supervise(
        specs,
        health.clone(),
        metrics.clone(),
        bus.clone(),
    ));

    let _ = bus.publish(KernelEvent::Health {
        service: "node_demo".into(),
        ok: true,
    });

    ron_kernel::wait_for_ctrl_c().await?;
    Ok(())
}

async fn run_service(spec: ServiceSpec, health: Arc<HealthState>, metrics: Arc<Metrics>, bus: Bus) {
    // pretend to do work and become healthy
    tokio::time::sleep(Duration::from_millis(200)).await;
    health.set(spec.name, true);
    metrics
        .service_restarts_total
        .with_label_values(&[spec.name])
        .inc();
    let _ = bus.publish(KernelEvent::Health {
        service: spec.name.into(),
        ok: true,
    });
}

async fn supervise(
    specs: Vec<ServiceSpec>,
    _health: Arc<HealthState>,
    metrics: Arc<Metrics>,
    _bus: Bus,
) {
    loop {
        for s in &specs {
            metrics.request_latency_seconds.observe(0.001);
            tracing::debug!("supervising {}", s.name);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
