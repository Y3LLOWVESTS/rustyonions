//! RO:WHAT — Contract test for the `/metrics` surface.
//! RO:WHY  — Ensure Macronode always exposes a Prometheus text endpoint,
//!           even before we add richer metric series.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18081;
const GATEWAY_PORT: u16 = 18091;

async fn spawn_macronode() -> Result<(Child, Client, String)> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("failed to spawn macronode binary")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{ADMIN_PORT}");

    // Wait for `/healthz` to be OK.
    for _ in 0..50 {
        match client.get(format!("{base}/healthz")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((child, client, base)),
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not become healthy in time"))
}

async fn shutdown_macronode(mut child: Child) {
    // Best-effort kill; this test is only concerned that /metrics is present.
    for _ in 0..10 {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
    let _ = child.kill();
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_endpoint_exists_and_is_text() -> Result<()> {
    let (child, client, base) = spawn_macronode().await?;

    let resp = client
        .get(format!("{base}/metrics"))
        .send()
        .await
        .context("GET /metrics failed")?;
    assert!(resp.status().is_success(), "/metrics must return 200 OK");

    let headers = resp.headers().clone();
    let body = resp.text().await.context("decode /metrics body")?;

    // Content-type should be text/plain; charset=utf-8 (Axum default for String).
    if let Some(ct) = headers.get(reqwest::header::CONTENT_TYPE) {
        let ct = ct.to_str().unwrap_or_default();
        assert!(
            ct.starts_with("text/plain"),
            "expected text/plain content-type for /metrics, got {ct}"
        );
    }

    // Even if we have no custom metrics yet, the body should not be enormous
    // and should be valid UTF-8 text.
    assert!(
        body.len() < 1024 * 1024,
        "metrics body should not exceed 1 MiB in tests"
    );

    shutdown_macronode(child).await;
    Ok(())
}
