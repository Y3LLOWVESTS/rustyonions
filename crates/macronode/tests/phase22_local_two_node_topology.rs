//! RO:WHAT — Live Phase 22 smoke for independent local Service Node and User Node runtimes.
//! RO:WHY — Establish the real two-node topology before adding CrabLink and economic-loop stages.
//! RO:INTERACTS — macronode binary, micronode binary, health/readiness/status/shutdown HTTP surfaces.
//! RO:INVARIANTS — loopback only; independent lifecycle; truthful roles; no fake ROC, payout, quorum, wallet, ledger, or finality.
//! RO:CONFIG — dynamic loopback ports and an isolated temporary index database.
//! RO:SECURITY — no production calls, public binds, live wallet mutation, ledger mutation, minting, burning, bridge, staking, or liquidity.
//! RO:TEST — run explicitly with `--ignored --nocapture` after building the micronode binary.

use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, ensure, Context, Result};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::sleep;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
struct MacronodePorts {
    admin: u16,
    gateway: u16,
    storage: u16,
    index: u16,
    overlay: u16,
    dht: u16,
    mailbox: u16,
}

struct ChildGuard {
    label: &'static str,
    child: Option<Child>,
}

impl ChildGuard {
    fn new(label: &'static str, child: Child) -> Self {
        Self {
            label,
            child: Some(child),
        }
    }

    fn assert_running(&mut self) -> Result<()> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow!("{} child is no longer owned", self.label))?;

        match child
            .try_wait()
            .with_context(|| format!("inspect {} child status", self.label))?
        {
            None => Ok(()),
            Some(status) => Err(anyhow!(
                "{} exited unexpectedly with status {status}",
                self.label
            )),
        }
    }

    fn kill_and_wait(&mut self) -> Result<ExitStatus> {
        let mut child = self
            .child
            .take()
            .ok_or_else(|| anyhow!("{} child is no longer owned", self.label))?;

        if child
            .try_wait()
            .with_context(|| format!("inspect {} before kill", self.label))?
            .is_none()
        {
            child
                .kill()
                .with_context(|| format!("kill {}", self.label))?;
        }

        child
            .wait()
            .with_context(|| format!("wait for {} after kill", self.label))
    }

    async fn wait_for_exit(&mut self, timeout: Duration) -> Result<ExitStatus> {
        let deadline = Instant::now() + timeout;

        loop {
            let child = self
                .child
                .as_mut()
                .ok_or_else(|| anyhow!("{} child is no longer owned", self.label))?;

            if let Some(status) = child
                .try_wait()
                .with_context(|| format!("inspect {} exit status", self.label))?
            {
                self.child = None;
                return Ok(status);
            }

            if Instant::now() >= deadline {
                return Err(anyhow!("{} did not exit within {timeout:?}", self.label));
            }

            sleep(POLL_INTERVAL).await;
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn reserve_loopback_ports(count: usize) -> Result<Vec<u16>> {
    let mut listeners = Vec::with_capacity(count);

    for _ in 0..count {
        listeners.push(TcpListener::bind("127.0.0.1:0").context("reserve dynamic loopback port")?);
    }

    listeners
        .iter()
        .map(|listener| {
            listener
                .local_addr()
                .context("read reserved loopback address")
                .map(|address| address.port())
        })
        .collect()
}

fn macronode_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_macronode"))
}

fn micronode_bin() -> Result<PathBuf> {
    let macronode = macronode_bin();
    let binary_dir = macronode
        .parent()
        .ok_or_else(|| anyhow!("macronode binary has no parent directory"))?;

    let micronode = binary_dir.join(format!("micronode{}", std::env::consts::EXE_SUFFIX));

    ensure!(
        micronode.is_file(),
        "micronode binary is missing at {}; run `cargo build -p micronode --bin micronode` first",
        micronode.display()
    );

    Ok(micronode)
}

fn unique_index_db() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!(
        "rustyonions-phase22-index-{}-{nonce}.sled",
        std::process::id()
    ))
}

fn clear_project_environment(command: &mut Command) {
    for (key, _) in std::env::vars() {
        if key.starts_with("RON_")
            || key.starts_with("MACRONODE_")
            || key.starts_with("MICRONODE_")
            || key.starts_with("CRABNODE_")
        {
            command.env_remove(key);
        }
    }
}

fn spawn_macronode(ports: MacronodePorts, index_db: &Path) -> Result<ChildGuard> {
    let mut command = Command::new(macronode_bin());

    clear_project_environment(&mut command);

    command
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{}", ports.admin))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", ports.gateway))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", ports.storage))
        .env("INDEX_BIND", format!("127.0.0.1:{}", ports.index))
        .env("RON_OVERLAY_ADDR", format!("127.0.0.1:{}", ports.overlay))
        .env("RON_DHT_ADDR", format!("127.0.0.1:{}", ports.dht))
        .env("RON_MAILBOX_ADDR", format!("127.0.0.1:{}", ports.mailbox))
        .env("RON_INDEX_DB", index_db)
        .env("RON_HEADLESS_MODE", "true")
        .env("RON_ADMIN_UI_ENABLED", "false")
        .env("RON_ADMIN_UI_RUNTIME_REQUIRED", "false")
        .env("RON_OPERATOR_UI_PROFILE", "service_node_local")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("spawn Phase 22 macronode Service Node")?;

    Ok(ChildGuard::new("macronode Service Node", child))
}

fn spawn_micronode(port: u16) -> Result<ChildGuard> {
    let mut command = Command::new(micronode_bin()?);

    clear_project_environment(&mut command);

    command
        .arg("serve")
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .arg("--no-dev-routes")
        .env("RUST_LOG", "info,micronode=debug")
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("spawn Phase 22 micronode User Node")?;

    Ok(ChildGuard::new("micronode User Node", child))
}

fn test_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("build Phase 22 HTTP client")
}

async fn wait_for_success(client: &Client, url: &str, label: &str) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    loop {
        let observation = match client.get(url).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => format!("HTTP {}", response.status()),
            Err(error) => error.to_string(),
        };

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "{label} did not become successful at {url}: {observation}"
            ));
        }

        sleep(POLL_INTERVAL).await;
    }
}

async fn wait_for_unreachable(client: &Client, url: &str, label: &str) -> Result<()> {
    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;

    loop {
        if client.get(url).send().await.is_err() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(anyhow!(
                "{label} remained reachable at {url} after shutdown"
            ));
        }

        sleep(POLL_INTERVAL).await;
    }
}

async fn get_json(client: &Client, url: &str, label: &str) -> Result<Value> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {label} from {url}"))?;

    ensure!(
        response.status().is_success(),
        "{label} returned HTTP {}",
        response.status()
    );

    response
        .json::<Value>()
        .await
        .with_context(|| format!("decode {label} JSON"))
}

fn require_string(value: &Value, label: &str, expected: &str) -> Result<()> {
    ensure!(
        value.as_str() == Some(expected),
        "{label} expected {expected:?}, got {value}"
    );

    Ok(())
}

fn require_bool(value: &Value, label: &str, expected: bool) -> Result<()> {
    ensure!(
        value.as_bool() == Some(expected),
        "{label} expected {expected}, got {value}"
    );

    Ok(())
}

fn require_null(value: &Value, label: &str) -> Result<()> {
    ensure!(value.is_null(), "{label} must remain null, got {value}");
    Ok(())
}

fn require_missing(parent: &Value, field: &str, label: &str) -> Result<()> {
    ensure!(
        parent.get(field).is_none(),
        "{label} must not be publicly serialized, got {}",
        parent[field]
    );

    Ok(())
}

fn assert_macronode_truth(status: &Value) -> Result<()> {
    require_string(&status["node_role"], "macronode.node_role", "service_node")?;
    require_string(
        &status["node_profile"],
        "macronode.node_profile",
        "macronode",
    )?;

    require_bool(&status["ready"], "macronode.ready", true)?;
    require_bool(&status["headless_mode"], "macronode.headless_mode", true)?;
    require_bool(
        &status["admin_ui_enabled"],
        "macronode.admin_ui_enabled",
        false,
    )?;
    require_bool(
        &status["admin_ui_runtime_required"],
        "macronode.admin_ui_runtime_required",
        false,
    )?;

    require_bool(
        &status["content_serving_enabled"],
        "macronode.content_serving_enabled",
        true,
    )?;
    require_bool(
        &status["service_quorum_enabled"],
        "macronode.service_quorum_enabled",
        false,
    )?;
    require_bool(
        &status["wallet_execution_participant"],
        "macronode.wallet_execution_participant",
        false,
    )?;
    require_bool(
        &status["public_inbound_enabled"],
        "macronode.public_inbound_enabled",
        false,
    )?;
    require_string(
        &status["user_ip_publication"],
        "macronode.user_ip_publication",
        "not_applicable_service_node",
    )?;

    require_string(
        &status["peer_ip_display"],
        "macronode.peer_ip_display",
        "forbidden",
    )?;
    require_bool(
        &status["admin_bind_publication"],
        "macronode.admin_bind_publication",
        false,
    )?;
    require_string(
        &status["service_socket_publication"],
        "macronode.service_socket_publication",
        "operator_local_only",
    )?;
    require_bool(
        &status["transport_routes_public"],
        "macronode.transport_routes_public",
        false,
    )?;
    require_bool(
        &status["raw_socket_publication"],
        "macronode.raw_socket_publication",
        false,
    )?;
    require_missing(status, "http_addr", "macronode.http_addr")?;
    require_missing(status, "metrics_addr", "macronode.metrics_addr")?;
    require_missing(status, "socket_addr", "macronode.socket_addr")?;

    require_string(
        &status["oap"]["protocol"],
        "macronode.oap.protocol",
        "oap/1",
    )?;
    require_bool(
        &status["oap"]["object_fetch_active"],
        "macronode.oap.object_fetch_active",
        true,
    )?;
    require_bool(
        &status["oap"]["full_digest_verification_active"],
        "macronode.oap.full_digest_verification_active",
        true,
    )?;

    require_bool(
        &status["reward_binding"]["registry_finality"],
        "macronode.reward_binding.registry_finality",
        false,
    )?;
    require_bool(
        &status["reward_binding"]["wallet_mutation"],
        "macronode.reward_binding.wallet_mutation",
        false,
    )?;
    require_bool(
        &status["reward_binding"]["ledger_mutation"],
        "macronode.reward_binding.ledger_mutation",
        false,
    )?;
    require_null(
        &status["reward_binding"]["confirmed_roc_minor_units"],
        "macronode.reward_binding.confirmed_roc_minor_units",
    )?;

    require_bool(
        &status["service_evidence"]["accounting_accepted"],
        "macronode.service_evidence.accounting_accepted",
        false,
    )?;
    require_bool(
        &status["service_evidence"]["reward_truth"],
        "macronode.service_evidence.reward_truth",
        false,
    )?;
    require_bool(
        &status["service_evidence"]["payout_authority"],
        "macronode.service_evidence.payout_authority",
        false,
    )?;
    require_bool(
        &status["service_evidence"]["wallet_mutation"],
        "macronode.service_evidence.wallet_mutation",
        false,
    )?;
    require_bool(
        &status["service_evidence"]["ledger_mutation"],
        "macronode.service_evidence.ledger_mutation",
        false,
    )?;

    Ok(())
}

fn assert_micronode_truth(status: &Value) -> Result<()> {
    require_string(&status["node_role"], "micronode.node_role", "user_node")?;
    require_string(
        &status["node_profile"],
        "micronode.node_profile",
        "micronode",
    )?;

    require_bool(&status["amnesia_mode"], "micronode.amnesia_mode", true)?;
    require_bool(&status["privacy_mode"], "micronode.privacy_mode", true)?;
    require_bool(
        &status["public_inbound_enabled"],
        "micronode.public_inbound_enabled",
        false,
    )?;

    require_string(
        &status["peer_ip_display"],
        "micronode.peer_ip_display",
        "forbidden",
    )?;
    require_string(
        &status["user_ip_publication"],
        "micronode.user_ip_publication",
        "forbidden",
    )?;
    require_bool(
        &status["admin_bind_loopback_only"],
        "micronode.admin_bind_loopback_only",
        true,
    )?;

    require_bool(
        &status["verification_enabled"],
        "micronode.verification_enabled",
        true,
    )?;
    require_bool(
        &status["content_serving_enabled"],
        "micronode.content_serving_enabled",
        false,
    )?;
    require_bool(
        &status["service_quorum_enabled"],
        "micronode.service_quorum_enabled",
        false,
    )?;
    require_bool(
        &status["wallet_execution_participant"],
        "micronode.wallet_execution_participant",
        false,
    )?;

    require_string(
        &status["passive_runtime"]["lifecycle_state"],
        "micronode.passive_runtime.lifecycle_state",
        "active",
    )?;
    require_string(
        &status["passive_runtime"]["confirmed_roc_source"],
        "micronode.passive_runtime.confirmed_roc_source",
        "wallet_ledger_receipt_only",
    )?;
    require_null(
        &status["passive_runtime"]["confirmed_roc_minor_units"],
        "micronode.passive_runtime.confirmed_roc_minor_units",
    )?;
    require_bool(
        &status["passive_runtime"]["wallet_mutation"],
        "micronode.passive_runtime.wallet_mutation",
        false,
    )?;
    require_bool(
        &status["passive_runtime"]["ledger_mutation"],
        "micronode.passive_runtime.ledger_mutation",
        false,
    )?;
    require_bool(
        &status["passive_runtime"]["verification_queue"]["mutates_wallet"],
        "micronode.verification_queue.mutates_wallet",
        false,
    )?;
    require_bool(
        &status["passive_runtime"]["verification_queue"]["mutates_ledger"],
        "micronode.verification_queue.mutates_ledger",
        false,
    )?;
    require_bool(
        &status["passive_runtime"]["economic_replay_worker"]["mutates_wallet"],
        "micronode.economic_replay_worker.mutates_wallet",
        false,
    )?;
    require_bool(
        &status["passive_runtime"]["economic_replay_worker"]["mutates_ledger"],
        "micronode.economic_replay_worker.mutates_ledger",
        false,
    )?;

    Ok(())
}

async fn wait_for_macronode(client: &Client, base: &str) -> Result<Value> {
    wait_for_success(client, &format!("{base}/healthz"), "macronode health").await?;

    wait_for_success(client, &format!("{base}/readyz"), "macronode readiness").await?;

    let status = get_json(client, &format!("{base}/api/v1/status"), "macronode status").await?;

    assert_macronode_truth(&status)?;
    Ok(status)
}

async fn wait_for_micronode(client: &Client, base: &str) -> Result<Value> {
    wait_for_success(client, &format!("{base}/healthz"), "micronode health").await?;

    wait_for_success(client, &format!("{base}/readyz"), "micronode readiness").await?;

    let status = get_json(client, &format!("{base}/api/v1/status"), "micronode status").await?;

    assert_micronode_truth(&status)?;
    Ok(status)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "live Phase 22 smoke; explicitly builds and launches real macronode and micronode binaries"]
async fn phase22_live_two_node_topology_is_independent_and_truthful() -> Result<()> {
    let ports = reserve_loopback_ports(9)?;

    let macronode_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let first_micronode_port = ports[7];
    let second_micronode_port = ports[8];

    let index_db = unique_index_db();
    let client = test_client()?;

    let macronode_base = format!("http://127.0.0.1:{}", macronode_ports.admin);
    let first_micronode_base = format!("http://127.0.0.1:{first_micronode_port}");
    let second_micronode_base = format!("http://127.0.0.1:{second_micronode_port}");

    let mut macronode = spawn_macronode(macronode_ports, &index_db)?;
    let mut first_micronode = spawn_micronode(first_micronode_port)?;

    println!("Phase 22A: waiting for independent Service Node...");
    wait_for_macronode(&client, &macronode_base).await?;

    println!("Phase 22A: waiting for independent User Node...");
    wait_for_micronode(&client, &first_micronode_base).await?;

    macronode.assert_running()?;
    first_micronode.assert_running()?;

    println!("Phase 22A: stopping User Node and proving Service Node remains truthful...");

    let _ = first_micronode.kill_and_wait()?;

    wait_for_unreachable(
        &client,
        &format!("{first_micronode_base}/healthz"),
        "first micronode",
    )
    .await?;

    macronode.assert_running()?;

    let service_status = get_json(
        &client,
        &format!("{macronode_base}/api/v1/status"),
        "macronode status after User Node stop",
    )
    .await?;

    assert_macronode_truth(&service_status)?;

    println!("Phase 22A: restarting User Node independently before Service Node shutdown...");

    let mut second_micronode = spawn_micronode(second_micronode_port)?;

    wait_for_micronode(&client, &second_micronode_base).await?;

    println!(
        "Phase 22A: gracefully stopping Service Node and proving User Node remains truthful..."
    );

    let shutdown = client
        .post(format!("{macronode_base}/api/v1/shutdown"))
        .send()
        .await
        .context("request macronode graceful shutdown")?;

    ensure!(
        shutdown.status().is_success() || shutdown.status() == StatusCode::ACCEPTED,
        "macronode shutdown returned HTTP {}",
        shutdown.status()
    );

    let service_exit = macronode.wait_for_exit(SHUTDOWN_TIMEOUT).await?;

    ensure!(
        service_exit.success(),
        "macronode graceful shutdown exited with {service_exit}"
    );

    wait_for_unreachable(&client, &format!("{macronode_base}/healthz"), "macronode").await?;

    second_micronode.assert_running()?;

    let user_status = get_json(
        &client,
        &format!("{second_micronode_base}/api/v1/status"),
        "micronode status after Service Node shutdown",
    )
    .await?;

    assert_micronode_truth(&user_status)?;

    let _ = second_micronode.kill_and_wait()?;
    let _ = fs::remove_dir_all(&index_db);

    println!(
        "Phase 22A passed: real Service Node and User Node are independently runnable, truthful, loopback-local, and non-authoritative."
    );

    Ok(())
}
