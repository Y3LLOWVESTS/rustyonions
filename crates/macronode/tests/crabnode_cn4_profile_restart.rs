//! RO:WHAT — CN-4 real managed-process restart acceptance for durable public username/profile state.
//! RO:WHY — Proves public CrabNode persists Passport profile ownership across actual stop/start boundaries.
//! RO:INTERACTS — crabnode lifecycle, macronode, svc-gateway, Omnigate, svc-passport durable store.
//! RO:INVARIANTS — public gateway only; @testmac survives restart; competing subject cannot claim it; @testpc remains independently claimable.
//! RO:METRICS — existing service metrics only.
//! RO:CONFIG — isolated CRABNODE_HOME and ephemeral loopback service ports.
//! RO:SECURITY — no dev readiness, no DevKms, no direct internal Passport request, no wallet/ledger mutation.
//! RO:TEST — this file is the CN-4B focused lifecycle gate.

use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
    process::{Command, Output},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{Client, Response, StatusCode};

use serde_json::{json, Value};

use tokio::time::sleep;

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

fn reserve_ports<const N: usize>() -> [u16; N] {
    let listeners = (0..N)
        .map(|_| TcpListener::bind("127.0.0.1:0").expect("reserve loopback port"))
        .collect::<Vec<_>>();

    std::array::from_fn(|index| {
        listeners[index]
            .local_addr()
            .expect("reserved address")
            .port()
    })
}

fn isolated_home() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("crabnode-cn4-{}-{nonce}", std::process::id(),))
}

struct Node {
    home: PathBuf,
    admin: String,
    gateway: String,
    storage: String,
    index: String,
    overlay: String,
    dht: String,
    mailbox: String,
    omnigate: String,
    omnigate_metrics: String,
    passport: String,
}

impl Node {
    fn new() -> Self {
        let [admin, gateway, storage, index, overlay, dht, mailbox, omnigate, omnigate_metrics, passport] =
            reserve_ports::<10>();

        Self {
            home: isolated_home(),
            admin: format!("127.0.0.1:{admin}"),
            gateway: format!("127.0.0.1:{gateway}"),
            storage: format!("127.0.0.1:{storage}"),
            index: format!("127.0.0.1:{index}"),
            overlay: format!("127.0.0.1:{overlay}"),
            dht: format!("127.0.0.1:{dht}"),
            mailbox: format!("127.0.0.1:{mailbox}"),
            omnigate: format!("127.0.0.1:{omnigate}"),
            omnigate_metrics: format!("127.0.0.1:{omnigate_metrics}"),
            passport: format!("127.0.0.1:{passport}"),
        }
    }

    fn admin_base(&self) -> String {
        format!("http://{}", self.admin,)
    }

    fn gateway_base(&self) -> String {
        format!("http://{}", self.gateway,)
    }

    fn command(&self, args: &[&str]) -> Output {
        Command::new(crabnode_bin())
            .env("CRABNODE_HOME", &self.home)
            .env("RON_GATEWAY_ADDR", &self.gateway)
            .env("SVC_GATEWAY_BIND_ADDR", &self.gateway)
            .env("RON_STORAGE_ADDR", &self.storage)
            .env(
                "SVC_GATEWAY_STORAGE_BASE_URL",
                format!("http://{}", self.storage,),
            )
            .env("INDEX_BIND", &self.index)
            .env("OMNIGATE_INDEX_BASE_URL", format!("http://{}", self.index,))
            .env("RON_OVERLAY_ADDR", &self.overlay)
            .env("RON_DHT_ADDR", &self.dht)
            .env("RON_MAILBOX_ADDR", &self.mailbox)
            .env("OMNIGATE_BIND", &self.omnigate)
            .env("OMNIGATE_METRICS_ADDR", &self.omnigate_metrics)
            .env(
                "SVC_GATEWAY_OMNIGATE_BASE_URL",
                format!("http://{}", self.omnigate,),
            )
            .env("RON_PASSPORT_ADDR", &self.passport)
            .env(
                "OMNIGATE_PASSPORT_BASE_URL",
                format!("http://{}", self.passport,),
            )
            .env_remove("RON_PASSPORT_PROFILE_DATA_DIR")
            .env_remove("MACRONODE_DEV_READY")
            .env_remove("RON_DEV_READY")
            .env_remove("SVC_GATEWAY_DEV_READY")
            .arg("--admin-url")
            .arg(self.admin_base())
            .args(args)
            .output()
            .expect("execute crabnode")
    }

    fn success(&self, args: &[&str]) -> Output {
        let output = self.command(args);

        assert!(
            output.status.success(),
            "crabnode {:?} failed\nstdout={}\nstderr={}",
            args,
            String::from_utf8_lossy(&output.stdout,),
            String::from_utf8_lossy(&output.stderr,),
        );

        output
    }

    fn durable_profile_dir(&self) -> PathBuf {
        self.home.join("data/passport-profile")
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        if self.home.exists() {
            let _ = self.command(&["stop"]);

            let _ = fs::remove_dir_all(&self.home);
        }
    }
}

async fn wait_ready(client: &Client, admin_base: &str) -> Value {
    for _ in 0..240 {
        if let Ok(response) = client.get(format!("{admin_base}/readyz")).send().await {
            if response.status() == StatusCode::OK {
                if let Ok(body) = response.json::<Value>().await {
                    if body["ready"].as_bool() == Some(true)
                        && body["mode"].as_str() == Some("truthful")
                    {
                        return body;
                    }
                }
            }
        }

        sleep(Duration::from_millis(50)).await;
    }

    panic!("CrabNode never reached truthful readiness");
}

async fn claim(client: &Client, gateway_base: &str, subject: &str, username: &str) -> Response {
    client
        .post(format!("{gateway_base}/identity/passport/profile/claim"))
        .json(&json!({
            "passport_subject":
                subject,
            "requested_username":
                username,
            "display_name":
                username,
            "bio":
                "CN-4 restart acceptance",
            "avatar_image":
                null
        }))
        .send()
        .await
        .expect("public profile claim")
}

async fn profile(client: &Client, gateway_base: &str, username: &str) -> Value {
    let response = client
        .get(format!(
            "{gateway_base}/identity/passport/profile/{username}"
        ))
        .send()
        .await
        .expect("public profile lookup");

    assert_eq!(response.status(), StatusCode::OK,);

    response.json::<Value>().await.expect("profile JSON")
}

fn assert_owner(value: &Value, subject: &str, username: &str) {
    assert_eq!(value["passport_subject"].as_str(), Some(subject,),);

    assert_eq!(value["username"].as_str(), Some(username,),);

    let expected_url = format!("crab://@{username}");

    assert_eq!(
        value["profile_crab_url"].as_str(),
        Some(expected_url.as_str(),),
    );
}

fn snapshot_count(node: &Node) -> usize {
    fs::read_dir(node.durable_profile_dir())
        .expect("durable profile directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("claims-v1-") && name.ends_with(".json"))
        })
        .count()
}

#[tokio::test]
async fn public_crabnode_profile_claims_survive_real_managed_restarts() {
    let node = Node::new();

    node.success(&["init"]);

    let node_id_path = node.home.join("data/node-id");

    let original_node_id = fs::read_to_string(&node_id_path).expect("initial node id");

    node.success(&["start"]);

    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("client");

    let admin = node.admin_base();

    let gateway = node.gateway_base();

    let first_ready = wait_ready(&client, &admin).await;

    assert_eq!(
        first_ready["capabilities"]["identity"].as_str(),
        Some("ready",),
    );

    let first_claim = claim(&client, &gateway, "passport:main:testmac", "testmac").await;

    assert_eq!(first_claim.status(), StatusCode::CREATED,);

    assert_owner(
        &profile(&client, &gateway, "testmac").await,
        "passport:main:testmac",
        "testmac",
    );

    assert!(node.durable_profile_dir().is_dir(),);

    assert_eq!(
        snapshot_count(&node,),
        1,
        "one successful new claim should create one generation",
    );

    node.success(&["restart"]);

    let _ = wait_ready(&client, &admin).await;

    assert_owner(
        &profile(&client, &gateway, "testmac").await,
        "passport:main:testmac",
        "testmac",
    );

    let conflict = claim(&client, &gateway, "passport:main:other-device", "testmac").await;

    assert_eq!(
        conflict.status(),
        StatusCode::CONFLICT,
        "@testmac must remain unavailable to another subject after restart",
    );

    let second_claim = claim(&client, &gateway, "passport:main:testpc", "testpc").await;

    assert_eq!(second_claim.status(), StatusCode::CREATED,);

    assert_eq!(
        snapshot_count(&node,),
        2,
        "two distinct successful claims should create two generations",
    );

    node.success(&["restart"]);

    let _ = wait_ready(&client, &admin).await;

    assert_owner(
        &profile(&client, &gateway, "testmac").await,
        "passport:main:testmac",
        "testmac",
    );

    assert_owner(
        &profile(&client, &gateway, "testpc").await,
        "passport:main:testpc",
        "testpc",
    );

    let final_node_id = fs::read_to_string(&node_id_path).expect("node id after restart");

    assert_eq!(
        final_node_id, original_node_id,
        "profile persistence must not rewrite CrabNode identity",
    );

    node.success(&["stop"]);
}
