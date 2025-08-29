#![forbid(unsafe_code)]

use ron_kernel::{wait_for_ctrl_c, Bus, HealthState, KernelEvent, Metrics};
use std::{net::SocketAddr, time::Duration};
use tokio::task::JoinHandle;
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[derive(Clone)]
struct ServiceSpec {
    name: &'static str,
    // how many loops before we simulate a crash
    crash_every: u64,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_logging();
    println!("Starting node_demo (metrics + bus + mini-supervisor)…");

    // Metrics + admin HTTP on fixed port
    let metrics = Metrics::new();
    let addr: SocketAddr = "127.0.0.1:9095".parse().expect("parse addr");
    let (http_handle, bound) = metrics.clone().serve(addr).await;
    println!(
        "Admin endpoints: /metrics /healthz /readyz at http://{}/",
        bound
    );

    // IMPORTANT: use the SAME HealthState that /readyz reads.
    let health: HealthState = metrics.health().clone();
    health.set_all(&["transport", "overlay", "index"], false);

    // Bus for kernel events
    let bus: Bus<KernelEvent> = Bus::new(64);
    let mut sub = bus.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = sub.recv().await {
            println!("[node_demo] bus event: {:?}", ev);
            tracing::info!(?ev, "bus event");
        }
    });

    // Mini supervisor managing three fake services
    let services = vec![
        ServiceSpec {
            name: "transport",
            crash_every: 37,
        },
        ServiceSpec {
            name: "overlay",
            crash_every: 53,
        },
        ServiceSpec {
            name: "index",
            crash_every: 0, // never crash
        },
    ];

    // Spawn each service with supervision
    let mut handles: Vec<(ServiceSpec, JoinHandle<()>)> = Vec::new();
    for spec in services.clone() {
        let h = health.clone();
        let m = metrics.clone();
        let b = bus.clone();
        let handle = tokio::spawn(run_service(spec.clone(), h, m, b));
        handles.push((spec, handle));
    }

    println!("Try:");
    println!("  curl http://127.0.0.1:9095/metrics");
    println!("  curl http://127.0.0.1:9095/readyz");
    println!("Press Ctrl-C to shutdown…");

    // Supervisor loop: watch for exits and restart
    let supervisor = tokio::spawn(supervise(
        handles,
        health.clone(),
        metrics.clone(),
        bus.clone(),
    ));

    // Keep metrics in sync with health readiness, publish node heartbeat
    let bus_for_ready = bus.clone(); // keep original bus for shutdown
    let readiness_publisher = tokio::spawn(async move {
        loop {
            let _ = bus_for_ready.publish(KernelEvent::Health {
                service: "node".into(),
                ok: true,
            });
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    wait_for_ctrl_c().await;
    let _ = bus.publish(KernelEvent::Shutdown);

    supervisor.abort();
    readiness_publisher.abort();
    http_handle.abort();
    println!("node_demo exiting");
}

async fn run_service(
    spec: ServiceSpec,
    health: HealthState,
    metrics: std::sync::Arc<Metrics>,
    bus: Bus<KernelEvent>,
) {
    let mut n: u64 = 0;

    loop {
        // mark healthy while running
        health.set(spec.name, true);

        // update some metrics to show activity
        metrics.bytes_in.inc_by(256);
        metrics.bytes_out.inc_by(128);
        metrics.conns_gauge.set((n % 5) as i64);
        metrics
            .req_latency
            .observe(0.004 + (n as f64 % 5.0) * 0.0007);

        // occasional bus heartbeat
        if n % 10 == 0 {
            let _ = bus.publish(KernelEvent::Health {
                service: spec.name.to_string(),
                ok: true,
            });
        }

        // simulate work
        tokio::time::sleep(Duration::from_millis(150)).await;
        n += 1;

        // simulate crash if configured
        if spec.crash_every != 0 && n % spec.crash_every == 0 {
            break;
        }
    }

    // going down -> mark unhealthy
    health.set(spec.name, false);
}

async fn supervise(
    mut handles: Vec<(ServiceSpec, JoinHandle<()>)>,
    health: HealthState,
    metrics: std::sync::Arc<Metrics>,
    bus: Bus<KernelEvent>,
) {
    // Simple supervisor: if a task ends, emit crash event, increment restart metric, and restart it
    loop {
        for i in 0..handles.len() {
            if handles[i].1.is_finished() {
                let (spec, old_handle) = handles.remove(i);
                let _ = old_handle.await; // clear JoinError if any

                let reason = format!("{} loop ended (simulated crash)", spec.name);
                let _ = bus.publish(KernelEvent::ServiceCrashed {
                    service: spec.name.to_string(),
                    reason: reason.clone(),
                });
                metrics.restarts.with_label_values(&[spec.name]).inc();

                // restart with tiny backoff
                tokio::time::sleep(Duration::from_millis(200)).await;

                let h = health.clone();
                let m = metrics.clone();
                let b = bus.clone();
                let new_handle = tokio::spawn(run_service(spec.clone(), h, m, b));
                handles.insert(i, (spec, new_handle));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
