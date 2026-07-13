//! RO:WHAT  — Contract tests for `/readyz` truthfulness vs dev-forced mode.
//! RO:WHY   — Ensure orchestrators (K8s/systemd/CI) can trust readiness, and
//!            that the dev override flag behaves exactly as documented.
//!
//! RO:INVARIANTS —
//!   - Truthful mode: `MACRONODE_DEV_READY` is *not* set in the child env.
//!       * `/readyz` eventually returns HTTP 200 with `"mode":"truthful"` and
//!         `"ready":true` once the node has finished booting.
//!   - Dev-forced mode: `MACRONODE_DEV_READY=1` in the child env.
//!       * `/readyz` quickly returns HTTP 200 with `"ready":true` even if some
//!         deps are still pending; `mode` should be either `"dev-forced"` or
//!         `"truthful"` depending on how far the node has progressed.
//!
//! These tests never rely on a config file path. All config comes from
//! environment variables passed to the spawned child, just like the
//! `admin_smoke` and `metrics_contract` tests.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::sleep;

#[derive(Clone, Copy)]
struct TestPorts {
    admin: u16,
    gateway: u16,
    storage: u16,
    index: u16,
}

const TRUTHFUL_PORTS: TestPorts = TestPorts {
    admin: 18082,
    gateway: 18092,
    storage: 18102,
    index: 18112,
};

const DEV_FORCED_PORTS: TestPorts = TestPorts {
    admin: 18182,
    gateway: 18192,
    storage: 18202,
    index: 18212,
};

fn isolated_index_db(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "macronode-readiness-{label}-{}.sled",
        std::process::id(),
    ))
}

/// Spawn a macronode child process with a controlled environment.
///
/// If `dev_ready` is:
///   - `None`        => ensure `MACRONODE_DEV_READY` is *removed* from the child env.
///   - `Some(true)`  => set `MACRONODE_DEV_READY=1`.
///   - `Some(false)` => set `MACRONODE_DEV_READY=0` (does NOT trigger dev mode).
fn spawn_macronode(dev_ready: Option<bool>, ports: TestPorts, index_db: &Path) -> Child {
    let bin = env!("CARGO_BIN_EXE_macronode");

    let _ = fs::remove_dir_all(index_db);

    let mut cmd = Command::new(bin);
    cmd.arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{}", ports.admin))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", ports.gateway))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", ports.storage))
        .env("INDEX_BIND", format!("127.0.0.1:{}", ports.index))
        .env("RON_INDEX_DB", index_db)
        .env_remove("RON_SERVICE_NODE_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_SIGNED_MODERATION_POLICY_PATH")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_SIGNER_ID")
        .env_remove("RON_SERVICE_NODE_MODERATION_TRUSTED_PUBLIC_KEY_HEX")
        .env_remove("RON_SERVICE_NODE_MODERATION_ACCEPTED_STATE_PATH")
        .env_remove("RON_SERVICE_NODE_SEED_OBJECT")
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
        match client.get(format!("{admin_base}/readyz")).send().await {
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
                let ready = body.get("ready").and_then(Value::as_bool).unwrap_or(false);

                if mode == expected_mode && ready == expected_ready && status == StatusCode::OK {
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

/// POST `/api/v1/shutdown` and wait for the child process to exit.
///
/// For these tests we only require that the node terminates in a bounded
/// amount of time. We do *not* enforce that the exit code is zero, since
/// dev/test profiles may choose to exit with non-zero codes for various
/// reasons (e.g. simulated faults).
async fn shutdown_and_wait(client: &Client, admin_base: &str, child: &mut Child) {
    let resp = client
        .post(format!("{admin_base}/api/v1/shutdown"))
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
            eprintln!("[readiness_drain] macronode exited after /shutdown: {status}");
            // Do not assert on success; for readiness tests we only care that
            // the process actually terminates within the timeout.
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
    let index_db = isolated_index_db("truthful");
    let mut child = spawn_macronode(None, TRUTHFUL_PORTS, &index_db);
    let client = Client::new();
    let admin_base = format!("http://127.0.0.1:{}", TRUTHFUL_PORTS.admin,);

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
    let _ = fs::remove_dir_all(index_db);
}

#[tokio::test(flavor = "multi_thread")]
async fn readyz_dev_forced_mode() {
    // Spawn WITH dev override enabled only in the child env.
    let index_db = isolated_index_db("dev-forced");
    let mut child = spawn_macronode(Some(true), DEV_FORCED_PORTS, &index_db);
    let client = Client::new();
    let admin_base = format!("http://127.0.0.1:{}", DEV_FORCED_PORTS.admin,);

    // In dev-forced mode we care primarily that readiness flips to true quickly.
    // The mode string may be "dev-forced" early, then "truthful" once all deps
    // are genuinely ready; either is acceptable as long as ready=true.
    let overall_timeout = Duration::from_secs(10);
    let deadline = Instant::now() + overall_timeout;

    let mode = loop {
        match client.get(format!("{admin_base}/readyz")).send().await {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    let body: Value = resp
                        .json()
                        .await
                        .expect("failed to parse /readyz JSON body");

                    let ready = body.get("ready").and_then(Value::as_bool).unwrap_or(false);

                    if ready {
                        break body
                            .get("mode")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                    }
                }
            }
            Err(_e) => {
                // Listener not up yet; keep trying until deadline.
            }
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "/readyz never reached ready=true within {:?} (dev-forced test)",
                overall_timeout
            );
        }

        sleep(Duration::from_millis(100)).await;
    };

    // Sanity: mode should be one of the known variants.
    assert!(
        mode == "dev-forced" || mode == "truthful",
        "unexpected /readyz mode in dev-forced test: {mode}"
    );

    shutdown_and_wait(&client, &admin_base, &mut child).await;
    let _ = fs::remove_dir_all(index_db);
}
