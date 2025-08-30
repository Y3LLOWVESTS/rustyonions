// crates/ron-kernel/src/bin/node_demo.rs
#![forbid(unsafe_code)]

use ron_kernel::{Bus, KernelEvent, Metrics};
use ron_kernel::metrics::HealthState;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[derive(Clone, Debug)]
struct ServiceSpec {
    name: &'static str,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting node_demo…");

    // --- Bus + Metrics -------------------------------------------------------
    let bus: Bus<KernelEvent> = Bus::new(1024);
    let metrics = Metrics::new();
    let health = metrics.health().clone();

    // Serve admin endpoints on a fixed port for the demo.
    let admin_addr: SocketAddr = "127.0.0.1:9097".parse().expect("valid socket addr");
    let (_http_handle, bound) = metrics.clone().serve(admin_addr).await;
    println!("Admin endpoints: /metrics /healthz /readyz at http://{bound}/");

    // --- Demo service specs ---------------------------------------------------
    let specs = vec![
        ServiceSpec { name: "transport" },
        ServiceSpec { name: "overlay" },
        ServiceSpec { name: "index" },
    ];

    // IMPORTANT: pass the slice by value, not by reference, so we yield &str (not &&str).
    health.set_all(["transport", "overlay", "index"], false);

    // Use Arc<Metrics> where tasks need shared ownership.
    let m_arc: Arc<Metrics> = Arc::new(metrics.clone());

    // Spawn each service runner.
    let mut handles = Vec::new();
    for spec in specs.clone() {
        let h = health.clone();
        let m = m_arc.clone();
        let b = bus.clone();
        let handle = tokio::spawn(run_service(spec.clone(), h, m, b));
        handles.push(handle);
    }

    // Spawn a simple supervisor that listens for crash events and bumps a restart counter.
    let sup = tokio::spawn(supervise(specs, health.clone(), m_arc.clone(), bus.clone()));

    // Also subscribe in main just to print all bus events.
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            info!(?ev, "bus event");
        }
    });

    // Demo runtime: let services run for a bit, then signal shutdown.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let _ = bus.publish(KernelEvent::Shutdown);

    // Wait for tasks to wind down (best-effort).
    for h in handles {
        if let Err(e) = h.await {
            warn!("service task join error: {e}");
        }
    }
    if let Err(e) = sup.await {
        warn!("supervisor join error: {e}");
    }

    println!("node_demo exiting");
}

/// Simulated service task: marks health OK, emits a health event, then idles.
/// In a real service, this would run the actual logic, using `metrics` as needed.
async fn run_service(
    spec: ServiceSpec,
    health: HealthState,
    metrics: Arc<Metrics>,
    bus: Bus<KernelEvent>,
) {
    // Mark service healthy and emit a health event.
    health.set(spec.name, true);
    let _ = bus.publish(KernelEvent::Health {
        service: spec.name.to_string(),
        ok: true,
    });

    // Simulate some activity.
    metrics.req_latency.observe(0.005);
    tokio::time::sleep(Duration::from_millis(750)).await;

    // Stay alive a bit to show liveness; in a real service this would be a run loop.
    tokio::time::sleep(Duration::from_secs(3)).await;

    // For the demo, flip to not-ok just before exit so /healthz could reflect it.
    health.set(spec.name, false);
}

/// Supervisor listens for crash events and increments a restart counter by service.
/// Here we also emit a synthetic crash to demonstrate the metric path.
async fn supervise(
    specs: Vec<ServiceSpec>,
    _health: HealthState,
    metrics: Arc<Metrics>,
    bus: Bus<KernelEvent>,
) {
    // Emit one synthetic crash event per service for the demo.
    for s in &specs {
        let _ = bus.publish(KernelEvent::ServiceCrashed {
            service: s.name.to_string(),
            reason: "demo-synthetic".to_string(),
        });
    }

    // Consume events; bump restart counters for crashes.
    let mut rx = bus.subscribe();
    while let Ok(ev) = rx.recv().await {
        match ev {
            KernelEvent::ServiceCrashed { service, .. } => {
                metrics.restarts.with_label_values(&[&service]).inc();
                info!(service, "supervisor recorded restart");
            }
            KernelEvent::Shutdown => {
                info!("supervisor received shutdown");
                break;
            }
            _ => {}
        }
    }
}
