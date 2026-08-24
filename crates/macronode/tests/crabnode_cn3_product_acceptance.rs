//! RO:WHAT — End-to-end CN-3 acceptance through CrabNode's one public svc-gateway ingress.
//! RO:WHY — Proves the standalone node is a real CrabLink backend rather than a ping-only shell.
//! RO:INTERACTS — macronode, svc-gateway, Omnigate, svc-passport, svc-storage, svc-index.
//! RO:INVARIANTS — clients use gateway only; B3 bytes come from storage; profile truth comes from svc-passport; index truth comes from svc-index.
//! RO:METRICS — no new collectors; exercises existing service instrumentation.
//! RO:CONFIG — isolated loopback ephemeral ports and deterministic RON_SERVICE_NODE_SEED_OBJECT.
//! RO:SECURITY — no dev-ready bypass, no DevKms route, no wallet/ledger mutation, no direct internal-service client shortcut.
//! RO:TEST — cargo test -p macronode --test crabnode_cn3_product_acceptance -- --nocapture --test-threads=1.

use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::Duration,
};

use reqwest::{Client, StatusCode};

use serde_json::{json, Value};

use tokio::time::sleep;

const SEED_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

const PROFILE_SUBJECT: &str = "passport:main:cn3acceptance";

const PROFILE_USERNAME: &str = "cn3acceptance";

struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();

        let _ = self.child.wait();
    }
}

fn reserve_ports<const N: usize>() -> [u16; N] {
    let listeners = (0..N)
        .map(|_| TcpListener::bind("127.0.0.1:0").expect("reserve loopback port"))
        .collect::<Vec<_>>();

    std::array::from_fn(|index| {
        listeners[index]
            .local_addr()
            .expect("reserved loopback address")
            .port()
    })
}

async fn wait_for_truthful_ready(client: &Client, admin_base: &str, child: &mut Child) -> Value {
    for _ in 0..240 {
        if let Some(status) = child.try_wait().expect("poll macronode") {
            panic!("macronode exited before CN-3 product readiness: {status}");
        }

        if let Ok(response) = client.get(format!("{admin_base}/readyz")).send().await {
            if response.status() == StatusCode::OK {
                if let Ok(body) = response.json::<Value>().await {
                    if body.get("ready").and_then(Value::as_bool) == Some(true)
                        && body.get("mode").and_then(Value::as_str) == Some("truthful")
                    {
                        return body;
                    }
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }

    panic!("macronode never reached truthful CN-3 product readiness");
}

fn assert_ready_value(body: &Value, section: &str, key: &str, expected: &str) {
    assert_eq!(
        body.get(section,)
            .and_then(|value| { value.get(key,) },)
            .and_then(Value::as_str,),
        Some(expected,),
        "{section}.{key} readiness mismatch",
    );
}

#[tokio::test]
async fn cn3_public_gateway_is_real_crablink_backend() {
    let [admin_port, gateway_port, storage_port, index_port, overlay_port, dht_port, mailbox_port, omnigate_port, omnigate_metrics_port, passport_port] =
        reserve_ports::<10>();

    let admin_addr = format!("127.0.0.1:{admin_port}");

    let gateway_addr = format!("127.0.0.1:{gateway_port}");

    let storage_addr = format!("127.0.0.1:{storage_port}");

    let index_addr = format!("127.0.0.1:{index_port}");

    let overlay_addr = format!("127.0.0.1:{overlay_port}");

    let dht_addr = format!("127.0.0.1:{dht_port}");

    let mailbox_addr = format!("127.0.0.1:{mailbox_port}");

    let omnigate_addr = format!("127.0.0.1:{omnigate_port}");

    let omnigate_metrics_addr = format!("127.0.0.1:{omnigate_metrics_port}");

    let passport_addr = format!("127.0.0.1:{passport_port}");

    let admin_base = format!("http://{admin_addr}");

    let gateway_base = format!("http://{gateway_addr}");

    let omnigate_base = format!("http://{omnigate_addr}");

    let mut command = Command::new(env!("CARGO_BIN_EXE_macronode"));

    command
        .arg("run")
        .env("RUST_LOG", "info")
        .env("RON_HTTP_ADDR", &admin_addr)
        .env("RON_GATEWAY_ADDR", &gateway_addr)
        .env("SVC_GATEWAY_BIND_ADDR", &gateway_addr)
        .env("RON_STORAGE_ADDR", &storage_addr)
        .env(
            "SVC_GATEWAY_STORAGE_BASE_URL",
            format!("http://{storage_addr}"),
        )
        .env("INDEX_BIND", &index_addr)
        .env("OMNIGATE_INDEX_BASE_URL", format!("http://{index_addr}"))
        .env("RON_OVERLAY_ADDR", &overlay_addr)
        .env("RON_DHT_ADDR", &dht_addr)
        .env("RON_MAILBOX_ADDR", &mailbox_addr)
        .env("OMNIGATE_BIND", &omnigate_addr)
        .env("OMNIGATE_METRICS_ADDR", &omnigate_metrics_addr)
        .env("SVC_GATEWAY_OMNIGATE_BASE_URL", &omnigate_base)
        .env("RON_PASSPORT_ADDR", &passport_addr)
        .env(
            "OMNIGATE_PASSPORT_BASE_URL",
            format!("http://{passport_addr}"),
        )
        .env("RON_SERVICE_NODE_SEED_OBJECT", "1")
        .env_remove("MACRONODE_DEV_READY")
        .env_remove("RON_DEV_READY")
        .env_remove("SVC_GATEWAY_DEV_READY")
        .env_remove("OMNIGATE_DEV_READY")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let child = command.spawn().expect("spawn CN-3 product node");

    let mut node = ChildGuard { child };

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("build CN-3 client");

    let ready = wait_for_truthful_ready(&client, &admin_base, &mut node.child).await;

    assert_ready_value(&ready, "deps", "gateway", "ok");

    assert_ready_value(&ready, "deps", "omnigate", "ok");

    assert_ready_value(&ready, "deps", "passport", "ok");

    assert_ready_value(&ready, "deps", "storage", "ok");

    assert_ready_value(&ready, "deps", "index", "ok");

    assert_ready_value(&ready, "capabilities", "operator_core", "ready");

    assert_ready_value(&ready, "capabilities", "product_ingress", "ready");

    assert_ready_value(&ready, "capabilities", "content_store", "ready");

    assert_ready_value(&ready, "capabilities", "identity", "ready");

    assert_ready_value(&ready, "capabilities", "peer_transport", "disabled");

    assert_ready_value(&ready, "capabilities", "messaging", "disabled");

    assert_ready_value(&ready, "capabilities", "economic_participation", "disabled");

    let gateway_health = client
        .get(format!("{gateway_base}/healthz"))
        .send()
        .await
        .expect("gateway health");

    assert_eq!(
        gateway_health.status(),
        StatusCode::OK,
        "canonical public gateway health must succeed",
    );

    let gateway_ready = client
        .get(format!("{gateway_base}/readyz"))
        .send()
        .await
        .expect("gateway readiness");

    assert_eq!(
        gateway_ready.status(),
        StatusCode::OK,
        "canonical public gateway readiness must succeed",
    );

    let product_health = client
        .get(format!("{gateway_base}/app/healthz"))
        .send()
        .await
        .expect("safe product health");

    assert_eq!(
        product_health.status(),
        StatusCode::OK,
        "gateway → Omnigate safe product route must succeed",
    );

    let claim = client
        .post(format!("{gateway_base}/identity/passport/profile/claim"))
        .json(&json!({
            "passport_subject":
                PROFILE_SUBJECT,
            "requested_username":
                PROFILE_USERNAME,
            "display_name":
                "CN3 Acceptance",
            "bio":
                "CrabNode CN-3 public identity acceptance.",
            "avatar_image":
                null
        }))
        .send()
        .await
        .expect("public gateway profile claim");

    assert_eq!(
        claim.status(),
        StatusCode::CREATED,
        "profile claim must traverse gateway → Omnigate → svc-passport",
    );

    let claim_body = claim.json::<Value>().await.expect("claim response JSON");

    assert_eq!(
        claim_body.get("schema",).and_then(Value::as_str,),
        Some("svc-passport.public-profile.v1",),
    );

    assert_eq!(
        claim_body.get("passport_subject",).and_then(Value::as_str,),
        Some(PROFILE_SUBJECT,),
    );

    assert_eq!(
        claim_body.get("username",).and_then(Value::as_str,),
        Some(PROFILE_USERNAME,),
    );

    assert_eq!(
        claim_body.get("username_status",).and_then(Value::as_str,),
        Some("confirmed",),
    );

    let profile = client
        .get(format!(
            "{gateway_base}/identity/passport/profile/{PROFILE_USERNAME}"
        ))
        .send()
        .await
        .expect("public gateway profile lookup");

    assert_eq!(
        profile.status(),
        StatusCode::OK,
        "public profile read must traverse the real backend",
    );

    let profile_body = profile
        .json::<Value>()
        .await
        .expect("profile response JSON");

    assert_eq!(
        profile_body
            .get("passport_subject",)
            .and_then(Value::as_str,),
        Some(PROFILE_SUBJECT,),
    );

    assert_eq!(
        profile_body.get("username",).and_then(Value::as_str,),
        Some(PROFILE_USERNAME,),
    );

    assert_eq!(
        profile_body
            .get("profile_crab_url",)
            .and_then(Value::as_str,),
        Some("crab://@cn3acceptance",),
    );

    for forbidden in [
        "private_key",
        "seed_phrase",
        "spend_authority",
        "wallet_spend_authority",
        "private_alt_mapping",
        "parent_passport",
        "main_passport_subject",
    ] {
        assert!(
            profile_body.get(forbidden,).is_none(),
            "public profile leaked forbidden field {forbidden}",
        );
    }

    let object_url = format!("{gateway_base}/o/{SEED_CID}");

    let object_get = client.get(&object_url).send().await.expect("public B3 GET");

    assert_eq!(
        object_get.status(),
        StatusCode::OK,
        "public B3 GET must reach canonical svc-storage",
    );

    let object_bytes = object_get.bytes().await.expect("public B3 bytes");

    assert_eq!(
        object_bytes.as_ref(),
        b"abc",
        "seeded B3 content must be exact",
    );

    let object_head = client
        .head(&object_url)
        .send()
        .await
        .expect("public B3 HEAD");

    assert_eq!(
        object_head.status(),
        StatusCode::OK,
        "public B3 HEAD must reach canonical svc-storage",
    );

    assert_eq!(
        object_head
            .headers()
            .get(reqwest::header::CONTENT_LENGTH,)
            .and_then(|value| { value.to_str().ok() },),
        Some("3",),
        "seeded object HEAD must preserve exact content length",
    );

    let explore = client
        .get(format!("{gateway_base}/explore"))
        .send()
        .await
        .expect("public Explore request");

    assert_eq!(
        explore.status(),
        StatusCode::OK,
        "Explore must traverse gateway → Omnigate → svc-index",
    );

    let explore_body = explore.json::<Value>().await.expect("Explore JSON");

    assert_eq!(
        explore_body.get("schema",).and_then(Value::as_str,),
        Some("crablink.explore-discovery.v1",),
        "Explore schema changed",
    );

    for field in ["recentPublications", "publicCreators", "templateSites"] {
        assert!(
            explore_body
                .get(field,)
                .and_then(Value::as_array,)
                .is_some(),
            "Explore response missing array {field}",
        );
    }

    let old_ping = client
        .get(format!("{gateway_base}/ingress/ping"))
        .send()
        .await
        .expect("retired MVP ping probe");

    assert_eq!(
        old_ping.status(),
        StatusCode::NOT_FOUND,
        "ping-only CrabNode backend must remain retired",
    );

    let shutdown = client
        .post(format!("{admin_base}/api/v1/shutdown"))
        .send()
        .await
        .expect("shutdown request");

    assert_eq!(shutdown.status(), StatusCode::ACCEPTED,);

    for _ in 0..80 {
        if node
            .child
            .try_wait()
            .expect("poll graceful shutdown")
            .is_some()
        {
            return;
        }

        sleep(Duration::from_millis(100)).await;
    }

    panic!("macronode did not exit after bounded graceful shutdown");
}
