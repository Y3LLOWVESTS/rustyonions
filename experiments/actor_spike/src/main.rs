//! Milestone 0a: restart semantics + clean shutdown + tracing.
//! Run with: cargo run -p actor_spike

use anyhow::Result;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Duration;
use tokio::{
    select,
    task::JoinSet,
    time::{sleep, Instant},
};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone)]
struct ServiceConfig {
    name: &'static str,
    // Simulate occasional panics every N..2N iterations.
    mean_iterations_before_panic: u32,
    // Simulated unit of work duration.
    work_ms: u64,
}

async fn service_run(cfg: ServiceConfig, mut rng: StdRng) -> Result<()> {
    let mut i: u64 = 0;
    let next_panic_after: u32 =
        rng.gen_range(cfg.mean_iterations_before_panic..(cfg.mean_iterations_before_panic * 2));

    loop {
        // Do some "work"
        sleep(Duration::from_millis(cfg.work_ms)).await;
        i += 1;

        if i % 50 == 0 {
            info!(service = cfg.name, iter = i, "heartbeat");
        }

        // Occasionally panic to test supervisor.
        if i as u32 >= next_panic_after {
            error!(service = cfg.name, iter = i, "simulated panic");
            panic!("simulated panic in {}", cfg.name);
        }
    }
}

/// Exponential backoff with jitter (bounded).
fn backoff(attempt: u32) -> Duration {
    let base_ms = 200u64;
    let max_ms = 5_000u64;
    let factor = 1u64 << attempt.min(5); // cap growth
    let raw = base_ms.saturating_mul(factor);
    let bounded = raw.min(max_ms);

    // tiny jitter: +/-10%
    let jitter = (bounded as f64 * 0.1) as u64;
    let now = Instant::now();
    let seed = now.elapsed().as_nanos() as u64;
    let jitter_val = (seed % (2 * jitter + 1)) as i64 - jitter as i64;
    let with_jitter = if jitter_val.is_negative() {
        bounded.saturating_sub(jitter_val.unsigned_abs())
    } else {
        bounded.saturating_add(jitter_val as u64)
    };
    Duration::from_millis(with_jitter.max(50))
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // Tracing: RUST_LOG=info,actor_spike=debug
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,actor_spike=debug"));
    fmt().with_env_filter(filter).compact().init();

    info!("actor_spike starting…  (Ctrl-C to stop)");

    // Our "kernel" for this spike: supervise one service.
    let cfg = ServiceConfig {
        name: "hello_service",
        mean_iterations_before_panic: 140,
        work_ms: 20,
    };

    // Spawn supervisor task(s) into a JoinSet so we can cancel/drain on shutdown.
    let mut tasks = JoinSet::new();
    tasks.spawn(supervise(cfg.clone()));

    // Wait for Ctrl-C
    select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("Ctrl-C received — initiating shutdown");
        }
    }

    // Drain: cancel tasks (JoinSet abort is fine for the spike).
    tasks.abort_all();
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            warn!(error = ?e, "task aborted (expected during shutdown)");
        }
    }

    info!("actor_spike stopped cleanly.");
    Ok(())
}

async fn supervise(cfg: ServiceConfig) {
    let mut attempt: u32 = 0;

    loop {
        let rng = StdRng::from_entropy();
        info!(service = cfg.name, "starting service run");
        let res = tokio::spawn(service_run(cfg.clone(), rng)).await;

        match res {
            Ok(Ok(())) => {
                // Unlikely in this spike; included for completeness.
                info!(service = cfg.name, "service completed");
                break;
            }
            Ok(Err(err)) => {
                error!(service = cfg.name, ?err, "service error; will restart");
            }
            Err(join_err) if join_err.is_panic() => {
                error!(service = cfg.name, ?join_err, "service panicked; will restart");
            }
            Err(join_err) => {
                error!(service = cfg.name, ?join_err, "service join error; will restart");
            }
        }

        // Backoff and restart
        let delay = backoff(attempt);
        warn!(service = cfg.name, attempt, ?delay, "backing off before restart");
        sleep(delay).await;
        attempt = attempt.saturating_add(1);
    }
}
