use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use omnigate::{bootstrap::server, config::Config};

fn http_get_status(addr: &str, path: &str) -> Option<u16> {
    let mut stream = TcpStream::connect(addr).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_millis(800)))
        .ok()?;
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, addr
    );
    stream.write_all(req.as_bytes()).ok()?;

    let mut buf = Vec::with_capacity(4096);
    stream.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    if let Some(status_line) = text.lines().next() {
        let parts: Vec<_> = status_line.split_whitespace().collect();
        if parts.len() >= 2 {
            return parts[1].parse::<u16>().ok();
        }
    }
    None
}

#[tokio::test(flavor = "multi_thread")]
async fn ready_flips_with_config() {
    // Build an absolute path to configs/omnigate.toml relative to THIS crate.
    let cfg_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("configs")
        .join("omnigate.toml");

    let cfg = Config::from_toml_file(cfg_path.to_string_lossy().as_ref()).expect("load config");

    // Build the app (admin plane + readiness config gate flip happens in lib.rs).
    let app = omnigate::App::build(cfg.clone()).await.expect("build app");

    // Start the API server; keep the JoinHandle alive for the test lifetime.
    let server_cfg = cfg.server; // move once
    let api_addr = server_cfg.bind;
    let (_task, _bound) = server::serve(server_cfg, app.router).await.expect("serve");

    // Probe /healthz until it is up.
    let api = api_addr.to_string();
    let mut ok = false;
    for _ in 0..100 {
        if let Some(code) = http_get_status(&api, "/healthz") {
            if code == 200 {
                ok = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(ok, "healthz did not come up on {}", api);

    // Truthful readiness should be 200 after config gate flips.
    let mut ready_ok = false;
    for _ in 0..100 {
        if let Some(code) = http_get_status(&api, "/readyz") {
            if code == 200 {
                ready_ok = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(ready_ok, "readyz did not return 200");
}
