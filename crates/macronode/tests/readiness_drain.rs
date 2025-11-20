//! RO:WHAT  — Contract tests for `/readyz` truthfulness vs dev-forced mode.
//! RO:WHY   — Ensure orchestrators (K8s/systemd/CI) can trust readiness, and
//!            that the dev override flag behaves exactly as documented.
//!
//! RO:INVARIANTS —
//!   - Truthful mode: `MACRONODE_DEV_READY` is *not* set in the child env.
//!       * `/readyz` eventually returns HTTP 200 with `"mode":"truthful"` and
//!         `"ready":true` once the node has finished booting.
//!   - Dev-forced mode: `MACRONODE_DEV_READY=1` in the child env.
//!       * `/readyz` quickly returns HTTP 200 with `"mode":"dev-forced"`
//!         and `"ready":true`, even if some deps are still pending.
//!
//! These tests never rely on a config file path. All config comes from
//! environment variables passed to the spawned child, just like the
//! `admin_smoke` and `metrics_contract` tests.

use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::sleep;

// Use dedicated ports so we don't collide with other tests.
const ADMIN_PORT: u16 = 18082;
const GATEWAY_PORT: u16 = 18092;

/// Spawn a macronode child process with a controlled environment.
///
/// If `dev_ready` is:
///   - `None`        => ensure `MACRONODE_DEV_READY` is *removed* from the child env.
///   - `Some(true)`  => set `MACRONODE_DEV_READY=1`.
///   - `Some(false)` => set `MACRONODE_DEV_READY=0` (does NOT trigger dev mode).
fn spawn_macronode(dev_ready: Option<bool>) -> Child {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        // Keep logs visible enough for debugging without being spammy.
        .env("RUST_LOG", "info,macronode=debug")
        // Configure admin + gateway addresses via env (no config file).
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{ADMIN_PORT}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{GATEWAY_PORT}"))
        // Silence child stdout/stderr by default (tests can use --nocapture if desired).
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    match dev_ready {
        None => {
            cmd.env_remove("MACRONODE_DEV_READY");
        }
        Some(true) => {
            cmd.env("MACRONODE_DEV_READY", "1");
        }
        Some(false) => {
            cmd.env("MACRONODE_DEV_READY", "0");
        }
    }

    cmd.spawn().expect("failed to spawn macronode child")
}

/// Poll `/readyz` until it reports the expected mode + ready flag, or time out.
///
/// This function is tolerant of early connection failures (e.g. TCP
/// connection refused while the admin listener is still binding) and treats
/// them as "not ready yet".
async fn wait_for_readyz_mode(
    client: &Client,
    admin_base: &str,
    expected_mode: &str,
    expected_ready: bool,
    overall_timeout: Duration,
) {
    let deadline = Instant::now() + overall_timeout;

    loop {
        match client
            .get(&format!("{admin_base}/readyz"))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body: Value = resp
                    .json()
                    .await
                    .expect("failed to parse /readyz JSON body");

                let mode = body
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let ready = body
                    .get("ready")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if mode == expected_mode
                    && ready == expected_ready
                    && status == StatusCode::OK
                {
                    // Reached desired state.
                    return;
                }
            }
            Err(_e) => {
                // Connection refused / timeout: admin plane not up yet.
                // Treat as "not ready yet" and keep polling until deadline.
            }
        }

        if Instant::now() >= deadline {
            panic!(
                "/readyz never reached mode={expected_mode:?}, ready={expected_ready} \
                 within {:?}",
                overall_timeout
            );
        }

        sleep(Duration::from_millis(100)).await;
    }
}

/// POST `/api/v1/shutdown` and wait for the child process to exit cleanly.
async fn shutdown_and_wait(client: &Client, admin_base: &str, child: &mut Child) {
    let resp = client
        .post(&format!("{admin_base}/api/v1/shutdown"))
        .send()
        .await
        .expect("failed to send /shutdown");

    assert!(
        resp.status().is_success(),
        "/shutdown did not return success, got {}",
        resp.status()
    );

    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if let Some(status) = child.try_wait().expect("failed to poll child status") {
            assert!(
                status.success(),
                "macronode did not exit cleanly after /shutdown: {status}"
            );
            return;
        }

        if Instant::now() >= deadline {
            panic!("macronode did not exit within timeout after /shutdown");
        }

        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn readyz_truthful_mode_eventually_ready() {
    // Spawn WITHOUT dev override; explicitly remove MACRONODE_DEV_READY from
    // the child env so we are not affected by whatever the parent shell has.
    let mut child = spawn_macronode(None);
    let client = Client::new();
    let admin_base = format!("http://127.0.0.1:{ADMIN_PORT}");

    // In truthful mode we expect:
    //   { "mode": "truthful", "ready": true }
    // within a reasonable timeout.
    wait_for_readyz_mode(
        &client,
        &admin_base,
        "truthful",
        true,
        Duration::from_secs(20),
    )
    .await;

    shutdown_and_wait(&client, &admin_base, &mut child).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn readyz_dev_forced_mode() {
    // Spawn WITH dev override enabled only in the child env.
    let mut child = spawn_macronode(Some(true));
    let client = Client::new();
    let admin_base = format!("http://127.0.0.1:{ADMIN_PORT}");

    // In dev-forced mode we expect:
    //   { "mode": "dev-forced", "ready": true }
    // quickly.
    wait_for_readyz_mode(
        &client,
        &admin_base,
        "dev-forced",
        true,
        Duration::from_secs(10),
    )
    .await;

    shutdown_and_wait(&client, &admin_base, &mut child).await;
}
