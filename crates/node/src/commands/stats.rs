use anyhow::{Context, Result};
use common::Config;
use overlay::Store;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Try to open the DB and print stats; if it's locked, query the running node's metrics endpoint.
pub fn stats_json(config_path: &str, data_dir_override: Option<&str>) -> Result<()> {
    let mut cfg = Config::load(config_path).context("loading config")?;
    if let Some(dd) = data_dir_override {
        cfg.data_dir = dd.into(); // PathBuf
    }

    match Store::open(&cfg.data_dir, cfg.chunk_size) {
        Ok(store) => {
            let stats = store.stats()?;
            let json = serde_json::to_string_pretty(&stats)?;
            println!("{json}");
            Ok(())
        }
        Err(open_err) => {
            // Likely "Resource temporarily unavailable" (sled lock). Fall back to metrics endpoint.
            eprintln!("store busy, trying metrics endpoint at {}", cfg.dev_inbox_addr);
            let addr: SocketAddr = cfg.dev_inbox_addr;
            let mut s = TcpStream::connect(addr).with_context(|| "connect metrics endpoint")?;
            s.set_read_timeout(Some(Duration::from_secs(2)))?;
            s.write_all(b"GET /metrics.json HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
            let mut buf = Vec::new();
            s.read_to_end(&mut buf)?;
            let resp = String::from_utf8_lossy(&buf);

            // split headers/body on CRLFCRLF
            if let Some((_headers, body)) = resp.split_once("\r\n\r\n") {
                // pretty-print body if possible
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(body.trim()) {
                    println!("{}", serde_json::to_string_pretty(&v)?);
                    return Ok(());
                }
                // else, just print raw body
                println!("{}", body.trim());
                return Ok(());
            }
            // If we got here, we didn't parse an HTTP response; show the original open error too.
            Err(open_err.context("failed to open store; bad metrics HTTP response"))?
        }
    }
}

