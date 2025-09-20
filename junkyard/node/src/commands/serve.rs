use anyhow::{anyhow, Context, Result};
use arti_transport::ArtiTransport;
use common::Config;
use overlay::{run_overlay_listener_with_transport, Store};
use serde_json::json;
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::thread;
use std::time::Duration;
use tracing::info;
use transport::TcpTransport;
use transport::tor_control::publish_v3;


pub fn serve(
    config_path: &str,
    data_dir_override: Option<&str>,
    transport: &str,
    hs_key_file: Option<&std::path::Path>,
) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }
    if transport == "tor" {
        if let Some(path) = hs_key_file {
            std::env::set_var("RO_HS_KEY_FILE", path.to_string_lossy().to_string());
        }
    }

    let store = Store::open(&cfg.data_dir, cfg.chunk_size)?;
    match transport {
        "tcp" => {
            let addr = cfg.overlay_addr;
            let tcp = TcpTransport::with_bind_addr(addr.to_string());
            let ctrs = tcp.counters();
            run_overlay_listener_with_transport(&tcp, store.clone())?;
            info!("serving TCP on {}; Ctrl+C to exit", addr);

            // periodic byte-counter logs
            {
                let ctrs = ctrs.clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_secs(60));
                    let s = ctrs.snapshot();
                    info!(
                        "stats/tcp total_in={} total_out={} last_min_in={} last_min_out={}",
                        s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                    );
                });
            }

            // start metrics endpoint on dev_inbox_addr
            start_metrics_server(
                store,
                move || {
                    let s = ctrs.snapshot();
                    (s.total_in, s.total_out)
                },
                cfg.dev_inbox_addr,
            )?;

            // park forever
            loop {
                std::thread::park();
            }
        }
        "tor" => {
            let arti = ArtiTransport::new(
                cfg.socks5_addr.clone(),
                cfg.tor_ctrl_addr.clone(),
                Duration::from_millis(cfg.connect_timeout_ms),
            );
            let ctrs = arti.counters();
            run_overlay_listener_with_transport(&arti, store.clone())?;
            info!("serving Tor HS via Arti (control={})", cfg.tor_ctrl_addr);

            {
                let ctrs = ctrs.clone();
                thread::spawn(move || loop {
                    thread::sleep(Duration::from_secs(60));
                    let s = ctrs.snapshot();
                    info!(
                        "stats/tor total_in={} total_out={} last_min_in={} last_min_out={}",
                        s.total_in, s.total_out, s.per_min_in[59], s.per_min_out[59]
                    );
                });
            }

            start_metrics_server(
                store,
                move || {
                    let s = ctrs.snapshot();
                    (s.total_in, s.total_out)
                },
                cfg.dev_inbox_addr,
            )?;

            loop {
                std::thread::park();
            }
        }
        other => Err(anyhow!("unknown transport {other}")),
    }
}

/// Tiny HTTP server that serves `/metrics.json` with store + transport counters.
/// `counters_fn` returns `(total_in, total_out)` snapshot when called.
fn start_metrics_server<F>(store: Store, counters_fn: F, listen_addr: SocketAddr) -> Result<()>
where
    F: Fn() -> (u64, u64) + Send + 'static,
{
    thread::spawn(move || {
        let listener = match TcpListener::bind(listen_addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("metrics bind error on {}: {e}", listen_addr);
                return;
            }
        };
        // Use tracing so it shows up alongside your other logs.
        info!("metrics listening on http://{}/metrics.json", listen_addr);

        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };

            // Try to read store stats; if unavailable, serve zeros but still respond 200.
            let store_json = match store.stats() {
                Ok(s) => json!({ "n_keys": s.n_keys, "total_bytes": s.total_bytes }),
                Err(_) => json!({ "n_keys": 0_u64, "total_bytes": 0_u64 }),
            };

            let (total_in, total_out) = counters_fn();

            let body = json!({
                "store": store_json,
                "transport": { "total_in": total_in, "total_out": total_out }
            })
            .to_string();

            // Write + flush response
            let _ = write_http_ok(&mut stream, &body);
            let _ = stream.flush();
        }
    });

    Ok(())
}

fn write_http_ok(stream: &mut dyn Write, body: &str) -> std::io::Result<()> {
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(resp.as_bytes())
}
