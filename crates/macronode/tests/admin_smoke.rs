//! RO:WHAT — End-to-end smoke test for the Macronode admin plane.
//! RO:WHY  — Prove that `/version`, `/healthz`, `/readyz`, `/metrics`,
//!           `/api/v1/status`, and `/api/v1/shutdown` all behave sanely.
//!
//! This test boots the real `macronode` binary via `CARGO_BIN_EXE_macronode`,
//! waits for it to come up, hits the core admin endpoints, and then shuts the
//! node down via the HTTP control surface.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

const ADMIN_PORT: u16 = 18080;
const GATEWAY_PORT: u16 = 18090;

/// Spawn the macronode binary and wait until the **full admin HTTP stack** is
/// available by polling `/version`, not just `/healthz`.
///
/// `/healthz` only proves that the event loop is alive; `/version` requires
/// the admin listener, router, and middleware stack to be bound and serving.
async fn spawn_macronode() -> Result<(Child, Client, String)> {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        // Per-test ports to avoid collisions when tests run in parallel.
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        // Keep test output quiet by default.
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("failed to spawn macronode binary")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build reqwest client")?;

    let base = format!("http://127.0.0.1:{ADMIN_PORT}");

    // Wait for `/version` to go green, which implies the full HTTP stack is up.
    for _ in 0..50 {
        match client.get(format!("{base}/version")).send().await {
            Ok(resp) if resp.status().is_success() => return Ok((child, client, base)),
            _ => sleep(Duration::from_millis(200)).await,
        }
    }

    Err(anyhow!("macronode did not expose /version in time"))
}

async fn shutdown_macronode(mut child: Child, client: &Client, base: &str) -> Result<()> {
    let resp = client
        .post(format!("{base}/api/v1/shutdown"))
        .send()
        .await
        .context("failed to call /api/v1/shutdown")?;

    // Log status/body when tests are run with --nocapture or RUST_LOG on.
    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    eprintln!("[admin_smoke] /shutdown status={status} body={body_text}");

    // Give the process a few seconds to exit cleanly.
    for _ in 0..50 {
        if let Ok(Some(_status)) = child.try_wait() {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }

    // If it is still running, kill it to avoid hanging tests.
    let _ = child.kill();
    Err(anyhow!("macronode did not exit cleanly after /shutdown"))
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_plane_smoke() -> Result<()> {
    let (child, client, base) = spawn_macronode().await?;

    // /version
    let resp = client
        .get(format!("{base}/version"))
        .send()
        .await
        .context("GET /version failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /version body")?;
    // /version contract: includes `service: "macronode"` plus build info.
    assert_eq!(body["service"], "macronode");
    assert!(body["version"].is_string());
    assert!(body["git_sha"].is_string());
    assert!(body["api"]["http"].is_string());

    // /healthz
    let resp = client
        .get(format!("{base}/healthz"))
        .send()
        .await
        .context("GET /healthz failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /healthz body")?;
    assert_eq!(body["ok"], true);

    // /readyz
    let resp = client
        .get(format!("{base}/readyz"))
        .send()
        .await
        .context("GET /readyz failed")?;
    assert!(
        resp.status().is_success(),
        "expected /readyz 200 when node is up"
    );
    let body: Value = resp.json().await.context("decode /readyz body")?;
    assert_eq!(body["ready"], true);
    // Basic sanity on deps.
    assert_eq!(body["deps"]["config"], "loaded");
    assert_eq!(body["deps"]["network"], "ok");
    assert_eq!(body["deps"]["gateway"], "ok");

    // /metrics
    let resp = client
        .get(format!("{base}/metrics"))
        .send()
        .await
        .context("GET /metrics failed")?;
    assert!(resp.status().is_success(), "/metrics must return 200 OK");

    let headers = resp.headers().clone();
    let text = resp.text().await.context("decode /metrics body")?;

    // Content-type should be text/plain; charset=utf-8 (Axum default for String).
    if let Some(ct) = headers.get(reqwest::header::CONTENT_TYPE) {
        let ct = ct.to_str().unwrap_or_default();
        assert!(
            ct.starts_with("text/plain"),
            "expected text/plain content-type for /metrics, got {ct}"
        );
    }

    // We don't yet enforce that the metrics body is non-empty, only that it is
    // reasonably small and successfully returned as text.
    assert!(
        text.len() < 1024 * 1024,
        "/metrics body should not exceed 1 MiB in tests"
    );

    // /api/v1/status
    let resp = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await
        .context("GET /api/v1/status failed")?;
    assert!(resp.status().is_success());
    let body: Value = resp.json().await.context("decode /api/v1/status body")?;
    // Status contract: uses `profile: "macronode"` (not `service`).
    assert_eq!(body["profile"], "macronode");
    assert!(body["uptime_seconds"].as_f64().unwrap_or(0.0) >= 0.0);
    // We expect a services map with at least gateway present.
    let services = body["services"].as_object().expect("services map present");
    assert!(
        services.contains_key("svc-gateway"),
        "status.services should contain svc-gateway"
    );

    // Drive shutdown through the HTTP surface.
    shutdown_macronode(child, &client, &base).await?;

    Ok(())
}
