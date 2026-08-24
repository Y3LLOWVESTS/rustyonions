//! RO:WHAT — Live integration proof for the CN-3 canonical CrabNode ingress.
//! RO:WHY — Readiness must require the real svc-gateway → Omnigate product path.
//! RO:INTERACTS — macronode, embedded svc-gateway, Omnigate, profile-only svc-passport, admin readiness/shutdown.
//! RO:INVARIANTS — old /ingress/ping is absent; /app/healthz traverses the real BFF path.
//! RO:CONFIG — all listeners use isolated ephemeral loopback ports.
//! RO:SECURITY — no dev-ready override, public bind, wallet mutation, or secret material.
//! RO:TEST — cargo test -p macronode --test crabnode_cn3_ingress -- --test-threads=1.

use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::Client;

fn macronode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_macronode")
}

struct RunningNode {
    child: Child,
    root: PathBuf,
}

impl Drop for RunningNode {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn reserve_ports() -> [u16; 10] {
    let mut listeners = Vec::new();

    let mut ports = Vec::new();

    for _ in 0..10 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind");

        ports.push(listener.local_addr().expect("local addr").port());

        listeners.push(listener);
    }

    let ports: [u16; 10] = ports.try_into().expect("exactly ten ports");

    drop(listeners);

    ports
}

#[tokio::test]
async fn canonical_gateway_reaches_real_omnigate_before_ready() {
    let ports = reserve_ports();

    let [admin, gateway, storage, index, overlay, dht, mailbox, omnigate, omnigate_metrics, passport] =
        ports;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    let root = std::env::temp_dir().join(format!("crabnode-cn3-{}-{nonce}", std::process::id()));

    fs::create_dir_all(root.join("data")).expect("create test state");

    let index_db = root.join("data/index");

    let child = Command::new(macronode_bin())
        .arg("run")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{admin}"))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{gateway}"))
        .env("SVC_GATEWAY_BIND_ADDR", format!("127.0.0.1:{gateway}"))
        .env("OMNIGATE_BIND", format!("127.0.0.1:{omnigate}"))
        .env(
            "OMNIGATE_METRICS_ADDR",
            format!("127.0.0.1:{omnigate_metrics}"),
        )
        .env("RON_PASSPORT_ADDR", format!("127.0.0.1:{passport}"))
        .env(
            "OMNIGATE_PASSPORT_BASE_URL",
            format!("http://127.0.0.1:{passport}"),
        )
        .env(
            "SVC_GATEWAY_OMNIGATE_BASE_URL",
            format!("http://127.0.0.1:{omnigate}"),
        )
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{storage}"))
        .env(
            "SVC_GATEWAY_STORAGE_BASE_URL",
            format!("http://127.0.0.1:{storage}"),
        )
        .env("INDEX_BIND", format!("127.0.0.1:{index}"))
        .env("RON_OVERLAY_ADDR", format!("127.0.0.1:{overlay}"))
        .env("RON_DHT_ADDR", format!("127.0.0.1:{dht}"))
        .env("RON_MAILBOX_ADDR", format!("127.0.0.1:{mailbox}"))
        .env("RON_INDEX_DB", &index_db)
        .env("INDEX_DB", &index_db)
        .env("RON_HEADLESS_MODE", "true")
        .env("RON_ADMIN_UI_ENABLED", "false")
        .env("RON_ADMIN_UI_RUNTIME_REQUIRED", "false")
        .env("RUST_LOG", "info")
        .env_remove("RON_DEV_READY")
        .env_remove("MACRONODE_DEV_READY")
        .env_remove("OMNIGATE_DEV_READY")
        .env_remove("CRABNODE_ADMIN_TOKEN")
        .env_remove("RON_ADMIN_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn macronode");

    let mut node = RunningNode { child, root };

    let client = Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .expect("HTTP client");

    let admin_base = format!("http://127.0.0.1:{admin}");

    let gateway_base = format!("http://127.0.0.1:{gateway}");

    let omnigate_base = format!("http://127.0.0.1:{omnigate}");

    let mut ready_body = None;

    for _ in 0..100 {
        if let Some(status) = node
            .child
            .try_wait()
            .expect("poll macronode during CN-3 readiness")
        {
            panic!("macronode exited before truthful CN-3 readiness: {status}");
        }

        if let Ok(response) = client.get(format!("{admin_base}/readyz")).send().await {
            if response.status().is_success() {
                if let Ok(body) = response.text().await {
                    if body.contains("\"ready\":true") && body.contains("\"mode\":\"truthful\"") {
                        ready_body = Some(body);

                        break;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let ready_body = ready_body.expect("macronode never reached truthful CN-3 readiness");

    assert!(ready_body.contains("\"gateway\":\"ok\"",));
    assert!(ready_body.contains("\"omnigate\":\"ok\"",));
    assert!(ready_body.contains("\"passport\":\"ok\"",));
    assert!(ready_body.contains("\"product_ingress\":\"ready\"",));
    assert!(ready_body.contains("\"identity\":\"ready\"",));

    let app_response = client
        .get(format!("{gateway_base}/app/healthz"))
        .send()
        .await
        .expect("gateway app health request");

    assert!(app_response.status().is_success());

    let app_body = app_response.text().await.expect("gateway app health body");

    assert!(app_body.contains("\"ok\":true",));

    assert!(app_body.contains("app plane mounted",));

    let direct_omnigate = client
        .get(format!("{omnigate_base}/v1/app/healthz"))
        .send()
        .await
        .expect("direct Omnigate app health");

    assert!(direct_omnigate.status().is_success());

    let old_ping = client
        .get(format!("{gateway_base}/ingress/ping"))
        .send()
        .await
        .expect("old MVP route probe");

    assert_eq!(
        old_ping.status(),
        reqwest::StatusCode::NOT_FOUND,
        "CN-3 must retire the ping-only MVP route owner"
    );

    let gateway_health = client
        .get(format!("{gateway_base}/healthz"))
        .send()
        .await
        .expect("canonical gateway health");

    assert!(gateway_health.status().is_success());

    let shutdown = client
        .post(format!("{admin_base}/api/v1/shutdown"))
        .send()
        .await
        .expect("shutdown request");

    assert_eq!(shutdown.status(), reqwest::StatusCode::ACCEPTED,);

    let mut exited = false;

    for _ in 0..60 {
        if node.child.try_wait().expect("poll macronode").is_some() {
            exited = true;
            break;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(exited, "macronode should exit after graceful shutdown");
}
