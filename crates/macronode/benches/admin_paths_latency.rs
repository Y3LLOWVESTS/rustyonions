//! RO:WHAT — Ad-hoc latency probe for macronode admin endpoints.
//! RO:WHY  — Quick, zero-setup way to see per-path latency from a single client.
//!
//! HOW TO USE
//! ----------
//! 1) Make sure macronode is running, e.g.:
//!      RUST_LOG=info cargo run -p macronode -- run --config macronode.toml
//! 2) In another terminal, run:
//!      cargo bench -p macronode --bench admin_paths_latency
//!
//! By default this targets http://127.0.0.1:8080. Override with:
//!      RON_HTTP_ADDR=127.0.0.1:9090 cargo bench -p macronode --bench admin_paths_latency
//!
//! This is a plain binary bench; we’re not using the unstable `#[bench]`
//! harness or Criterion here — just a small async client.

use std::time::{Duration, Instant};

use reqwest::Client;

const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::var("RON_HTTP_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let base = format!("http://{addr}");

    println!();
    println!("macronode admin latency probe");
    println!("  base = {base}");
    println!("  (override with RON_HTTP_ADDR=host:port)");
    println!();

    let client = Client::builder()
        .pool_idle_timeout(Some(Duration::from_secs(5)))
        .timeout(Duration::from_secs(2))
        .build()?;

    let paths = [
        "/healthz",
        "/readyz",
        "/version",
        "/metrics",
        "/api/v1/status",
    ];

    for path in paths {
        let url = format!("{base}{path}");
        let start = Instant::now();
        let resp = client.get(&url).send().await;
        let elapsed = start.elapsed();

        match resp {
            Ok(r) => {
                println!("{:<18} {:>3}  {:>8.3?}", path, r.status().as_u16(), elapsed);
            }
            Err(e) => {
                println!("{:<18} ERR  {:>8.3?}  ({e})", path, elapsed);
            }
        }
    }

    println!();
    Ok(())
}
