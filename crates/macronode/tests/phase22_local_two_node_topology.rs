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
use ron_kms::backends::ed25519;
use ron_proto::{
    service_node_signature_message_bytes, ContentId, EpochEligibilityStatusV1, EpochEligibilityV1,
    EpochQuorumThresholdV1, EpochRewardAllocationV1, RocEpochTransitionExpectationV1,
    RocEpochTransitionIdentityV1, ServiceNodeQuorumV1, ServiceNodeSignatureV1,
    EPOCH_REWARD_ALLOCATION_SCHEMA, ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA,
    ROC_EPOCH_TRANSITION_VERSION, SERVICE_NODE_QUORUM_SCHEMA,
};
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

fn spawn_quorum_macronode(
    ports: MacronodePorts,
    index_db: &Path,
    service_node_id: &str,
    logical_key_ref: &str,
    seed_hex: &str,
    admin_token: &str,
) -> Result<ChildGuard> {
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
        .env("RON_ADMIN_TOKEN", admin_token)
        .env("RON_QUICKCHAIN_PRIVATE_BETA_SIGNING_ENABLED", "true")
        .env("RON_QUICKCHAIN_PRIVATE_BETA_CHAIN_ID", "rustyonions-dev")
        .env(
            "RON_QUICKCHAIN_PRIVATE_BETA_SERVICE_NODE_ID",
            service_node_id,
        )
        .env("RON_QUICKCHAIN_PRIVATE_BETA_KEY_REF", logical_key_ref)
        .env("RON_QUICKCHAIN_PRIVATE_BETA_SEED_HEX", seed_hex)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("spawn Phase 19 private-beta quorum macronode")?;

    Ok(ChildGuard::new(
        "Phase 19 quorum macronode Service Node",
        child,
    ))
}

fn phase19_seed_hex(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn phase19_seed(byte: u8) -> [u8; 32] {
    [byte; 32]
}

fn phase19_cid(character: char) -> ContentId {
    format!("b3:{}", character.to_string().repeat(64),)
        .parse()
        .expect("Phase 19 fixture ContentId must parse")
}

fn phase19_eligibility(label: &str) -> EpochEligibilityV1 {
    EpochEligibilityV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,

        service_node_id: format!("service_node:{label}"),

        registry_entry_id: format!("registry:phase19:{label}"),

        reward_binding_id: format!("binding:phase19:{label}"),

        key_id: format!("key:phase19:{label}"),

        status: EpochEligibilityStatusV1::Eligible,
    }
}

fn phase19_quorum_threshold() -> EpochQuorumThresholdV1 {
    EpochQuorumThresholdV1 {
        version: ROC_EPOCH_TRANSITION_VERSION,

        eligible_service_nodes: 3,
        quorum_bps: 6_666,
        minimum_signatures: 2,
        required_signatures: 2,
    }
}

fn phase19_transition_identity() -> RocEpochTransitionIdentityV1 {
    let expectation = RocEpochTransitionExpectationV1 {
        schema: ROC_EPOCH_TRANSITION_EXPECTATION_SCHEMA.to_owned(),

        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: "rustyonions-dev".to_owned(),

        epoch_id: "epoch:phase19-live".to_owned(),

        accounting_snapshot_hash: phase19_cid('a'),

        reward_plan_hash: phase19_cid('b'),

        policy_hash: phase19_cid('c'),

        economics_config_hash: phase19_cid('d'),

        registry_root: phase19_cid('e'),

        reward_binding_root: phase19_cid('f'),

        evidence_root: phase19_cid('1'),

        reward_cap_minor_units: "1000".to_owned(),

        threshold: phase19_quorum_threshold(),

        eligibilities: vec![
            phase19_eligibility("alpha"),
            phase19_eligibility("beta"),
            phase19_eligibility("gamma"),
        ],
    };

    let identity = RocEpochTransitionIdentityV1::from_expectation_and_allocations(
        &expectation,
        "1000",
        vec![
            EpochRewardAllocationV1 {
                schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),

                version: ROC_EPOCH_TRANSITION_VERSION,

                allocation_id: "allocation:phase19:alpha".to_owned(),

                reward_plan_allocation_id: "reward_plan_allocation:phase19:alpha".to_owned(),

                service_node_id: "service_node:alpha".to_owned(),

                source_pool: "node_delivery".to_owned(),

                amount_minor_units: "400".to_owned(),
            },
            EpochRewardAllocationV1 {
                schema: EPOCH_REWARD_ALLOCATION_SCHEMA.to_owned(),

                version: ROC_EPOCH_TRANSITION_VERSION,

                allocation_id: "allocation:phase19:beta".to_owned(),

                reward_plan_allocation_id: "reward_plan_allocation:phase19:beta".to_owned(),

                service_node_id: "service_node:beta".to_owned(),

                source_pool: "node_delivery".to_owned(),

                amount_minor_units: "600".to_owned(),
            },
        ],
    );

    identity
        .validate()
        .expect("Phase 19 canonical live transition identity must validate");

    identity
}

async fn phase19_request_signature(
    client: &Client,
    base: &str,
    admin_token: &str,
    identity: &RocEpochTransitionIdentityV1,
    label: &str,
) -> Result<ServiceNodeSignatureV1> {
    let response = client
        .post(format!("{base}/api/v1/quickchain/quorum/sign"))
        .bearer_auth(admin_token)
        .json(identity)
        .send()
        .await
        .with_context(|| format!("POST Phase 19 quorum signature request to {label}"))?;

    ensure!(
        response.status().is_success(),
        "{label} quorum signing returned HTTP {}",
        response.status()
    );

    let body = response
        .json::<Value>()
        .await
        .with_context(|| format!("decode Phase 19 quorum signing response from {label}"))?;

    ensure!(
        body["status"].as_str() == Some("signed"),
        "{label} must truthfully report one signed result"
    );

    ensure!(
        body["serviceNodeSignatureCreated"].as_bool() == Some(true),
        "{label} must report exactly one Service Node signature"
    );

    for field in [
        "quorumAggregated",
        "quorumFinalized",
        "checkpointFinalized",
        "walletMutation",
        "ledgerMutation",
        "payoutExecuted",
        "receiptCreated",
        "confirmedRocReported",
        "paidUnlock",
        "finality",
        "crabLinkFinalityAuthority",
    ] {
        ensure!(
            body[field].as_bool() == Some(false),
            "{label} must not claim authority in {field}"
        );
    }

    serde_json::from_value(body["signature"].clone())
        .with_context(|| format!("decode ServiceNodeSignatureV1 from {label}"))
}

fn phase19_decode_signature(wire: &str) -> Result<[u8; 64]> {
    ensure!(
        wire.len() == 128,
        "Ed25519 signature wire must contain 128 hex characters"
    );

    let bytes = wire.as_bytes();
    let mut output = [0_u8; 64];

    for index in 0..64 {
        let high = phase19_hex_nibble(bytes[index * 2])?;

        let low = phase19_hex_nibble(bytes[index * 2 + 1])?;

        output[index] = (high << 4) | low;
    }

    Ok(output)
}

fn phase19_hex_nibble(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),

        _ => Err(anyhow!(
            "Phase 19 signature contains non-lowercase-hex byte"
        )),
    }
}

fn phase19_verify_live_signature(
    signature: &ServiceNodeSignatureV1,
    expected_node_id: &str,
    expected_key_ref: &str,
    seed: &[u8; 32],
) -> Result<()> {
    ensure!(
        signature.chain_id == "rustyonions-dev",
        "{expected_node_id} signature chain mismatch"
    );

    ensure!(
        signature.epoch_id == "epoch:phase19-live",
        "{expected_node_id} signature epoch mismatch"
    );

    ensure!(
        signature.service_node_id == expected_node_id,
        "{expected_node_id} signature node identity mismatch"
    );

    ensure!(
        signature.key_id == expected_key_ref,
        "{expected_node_id} signature logical key mismatch"
    );

    let message = service_node_signature_message_bytes(signature)
        .context("encode canonical Service Node signature message")?;

    let raw_signature = phase19_decode_signature(&signature.signature_wire)?;

    let public_key = ed25519::public_key(seed);

    ensure!(
        ed25519::verify(&public_key, &message, &raw_signature,),
        "{expected_node_id} returned signature failed independent Ed25519 verification"
    );

    Ok(())
}

fn phase19_quorum_from_signatures(
    identity: &RocEpochTransitionIdentityV1,
    signatures: Vec<ServiceNodeSignatureV1>,
) -> ServiceNodeQuorumV1 {
    let transition_hash = signatures
        .first()
        .expect("Phase 19 quorum requires at least one signature fixture")
        .transition_hash
        .clone();

    ServiceNodeQuorumV1 {
        schema: SERVICE_NODE_QUORUM_SCHEMA.to_owned(),

        version: ROC_EPOCH_TRANSITION_VERSION,

        chain_id: identity.chain_id.clone(),

        epoch_id: identity.epoch_id.clone(),

        transition_hash,

        threshold: identity.threshold.clone(),

        eligibilities: identity.eligibilities.clone(),

        signatures,
    }
}

fn phase19_live_two_of_three_quorum_after_member_loss(
    identity: &RocEpochTransitionIdentityV1,
    first: ServiceNodeSignatureV1,
    second: ServiceNodeSignatureV1,
) -> Result<()> {
    let single = phase19_quorum_from_signatures(identity, vec![first.clone()]);

    ensure!(
        single.validate().is_err(),
        "one live Service Node signature must not satisfy 2-of-3 quorum"
    );

    let two = phase19_quorum_from_signatures(identity, vec![first, second]);

    ensure!(
        two.validate().is_ok(),
        "two independently verified live Service Node signatures must satisfy canonical 2-of-3 quorum"
    );

    Ok(())
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

// RO:WHAT — FINAL_BETA Phase 19 live three-Service-Node plus one-User-Node topology.
// RO:WHY — Prove independent Service Node processes survive one-member loss while the
// remaining Service Nodes and User Node stay truthful, then prove the failed member
// can restart on the same local identity surface.
// RO:INTERACTS — real macronode and micronode binaries plus existing health,
// readiness, status, and child-process lifecycle helpers.
// RO:INVARIANTS — loopback only; isolated index stores; no fake quorum/finality;
// no wallet or ledger mutation; surviving nodes must remain independently healthy.
// RO:SECURITY — no public binds, production calls, minting, burning, bridge,
// staking, liquidity, or live external network mutation.
// RO:TEST — build micronode first, then run this ignored macronode integration test.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live FINAL_BETA Phase 19 topology; launches three real macronodes and one real micronode"]
async fn phase19_live_three_service_one_user_topology_survives_member_loss_and_restart(
) -> Result<()> {
    let ports = reserve_loopback_ports(22)?;

    let service_a_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let service_b_ports = MacronodePorts {
        admin: ports[7],
        gateway: ports[8],
        storage: ports[9],
        index: ports[10],
        overlay: ports[11],
        dht: ports[12],
        mailbox: ports[13],
    };

    let service_c_ports = MacronodePorts {
        admin: ports[14],
        gateway: ports[15],
        storage: ports[16],
        index: ports[17],
        overlay: ports[18],
        dht: ports[19],
        mailbox: ports[20],
    };

    let user_port = ports[21];

    let service_a_db = unique_index_db();
    let service_b_db = unique_index_db();
    let service_c_db = unique_index_db();

    ensure!(
        service_a_db != service_b_db
            && service_a_db != service_c_db
            && service_b_db != service_c_db,
        "Phase 19 Service Nodes must receive isolated index stores"
    );

    let client = test_client()?;

    let service_a_base = format!("http://127.0.0.1:{}", service_a_ports.admin);
    let service_b_base = format!("http://127.0.0.1:{}", service_b_ports.admin);
    let service_c_base = format!("http://127.0.0.1:{}", service_c_ports.admin);
    let user_base = format!("http://127.0.0.1:{user_port}");

    let admin_token = "phase19-live-quorum-admin";

    let seed_a = phase19_seed(0x11);
    let seed_b = phase19_seed(0x22);
    let seed_c = phase19_seed(0x33);

    let seed_a_hex = phase19_seed_hex(0x11);
    let seed_b_hex = phase19_seed_hex(0x22);
    let seed_c_hex = phase19_seed_hex(0x33);

    let transition_identity = phase19_transition_identity();

    let mut service_a = spawn_quorum_macronode(
        service_a_ports,
        &service_a_db,
        "service_node:alpha",
        "key:phase19:alpha",
        &seed_a_hex,
        admin_token,
    )?;

    let mut service_b = spawn_quorum_macronode(
        service_b_ports,
        &service_b_db,
        "service_node:beta",
        "key:phase19:beta",
        &seed_b_hex,
        admin_token,
    )?;

    let mut service_c = spawn_quorum_macronode(
        service_c_ports,
        &service_c_db,
        "service_node:gamma",
        "key:phase19:gamma",
        &seed_c_hex,
        admin_token,
    )?;

    let mut user_node = spawn_micronode(user_port)?;

    println!(
        "Phase 19: waiting for Service Node A, Service Node B, Service Node C, and User Node..."
    );

    let service_a_status = wait_for_macronode(&client, &service_a_base).await?;
    let service_b_status = wait_for_macronode(&client, &service_b_base).await?;
    let service_c_status = wait_for_macronode(&client, &service_c_base).await?;
    let user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&service_a_status)?;
    assert_macronode_truth(&service_b_status)?;
    assert_macronode_truth(&service_c_status)?;
    assert_micronode_truth(&user_status)?;

    service_a.assert_running()?;
    service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    println!("Phase 19: all three independent Service Nodes and the User Node are healthy.");

    println!(
        "Phase 19: requesting the same canonical transition signature from all three real Service Nodes..."
    );

    let signature_a = phase19_request_signature(
        &client,
        &service_a_base,
        admin_token,
        &transition_identity,
        "Service Node A",
    )
    .await?;

    let signature_b = phase19_request_signature(
        &client,
        &service_b_base,
        admin_token,
        &transition_identity,
        "Service Node B",
    )
    .await?;

    let signature_c = phase19_request_signature(
        &client,
        &service_c_base,
        admin_token,
        &transition_identity,
        "Service Node C",
    )
    .await?;

    phase19_verify_live_signature(
        &signature_a,
        "service_node:alpha",
        "key:phase19:alpha",
        &seed_a,
    )?;

    phase19_verify_live_signature(
        &signature_b,
        "service_node:beta",
        "key:phase19:beta",
        &seed_b,
    )?;

    phase19_verify_live_signature(
        &signature_c,
        "service_node:gamma",
        "key:phase19:gamma",
        &seed_c,
    )?;

    ensure!(
        signature_a.transition_hash == signature_b.transition_hash
            && signature_b.transition_hash == signature_c.transition_hash,
        "all three live Service Nodes must independently derive the same transition hash"
    );

    phase19_live_two_of_three_quorum_after_member_loss(
        &transition_identity,
        signature_a.clone(),
        signature_b.clone(),
    )?;

    println!(
        "Phase 19: three process signatures verified independently; one signature failed quorum and two signatures satisfied canonical 2-of-3 quorum."
    );

    println!("Phase 19: terminating Service Node B to prove one-member failure isolation...");

    let _ = service_b.kill_and_wait()?;

    wait_for_unreachable(
        &client,
        &format!("{service_b_base}/healthz"),
        "Phase 19 Service Node B",
    )
    .await?;

    service_a.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let surviving_a_status = get_json(
        &client,
        &format!("{service_a_base}/api/v1/status"),
        "Phase 19 surviving Service Node A",
    )
    .await?;

    let surviving_c_status = get_json(
        &client,
        &format!("{service_c_base}/api/v1/status"),
        "Phase 19 surviving Service Node C",
    )
    .await?;

    let surviving_user_status = get_json(
        &client,
        &format!("{user_base}/api/v1/status"),
        "Phase 19 surviving User Node",
    )
    .await?;

    assert_macronode_truth(&surviving_a_status)?;
    assert_macronode_truth(&surviving_c_status)?;
    assert_micronode_truth(&surviving_user_status)?;

    let signature_a_after_loss = phase19_request_signature(
        &client,
        &service_a_base,
        admin_token,
        &transition_identity,
        "surviving Service Node A",
    )
    .await?;

    let signature_c_after_loss = phase19_request_signature(
        &client,
        &service_c_base,
        admin_token,
        &transition_identity,
        "surviving Service Node C",
    )
    .await?;

    phase19_verify_live_signature(
        &signature_a_after_loss,
        "service_node:alpha",
        "key:phase19:alpha",
        &seed_a,
    )?;

    phase19_verify_live_signature(
        &signature_c_after_loss,
        "service_node:gamma",
        "key:phase19:gamma",
        &seed_c,
    )?;

    phase19_live_two_of_three_quorum_after_member_loss(
        &transition_identity,
        signature_a_after_loss,
        signature_c_after_loss,
    )?;

    println!(
        "Phase 19: Service Nodes A and C plus the User Node remained healthy after Service Node B loss, and A+C still satisfied canonical 2-of-3 quorum."
    );

    println!("Phase 19: restarting Service Node B on the same ports and isolated index store...");

    let mut restarted_service_b = spawn_quorum_macronode(
        service_b_ports,
        &service_b_db,
        "service_node:beta",
        "key:phase19:beta",
        &seed_b_hex,
        admin_token,
    )?;

    let restarted_b_status = wait_for_macronode(&client, &service_b_base).await?;

    assert_macronode_truth(&restarted_b_status)?;

    service_a.assert_running()?;
    restarted_service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let final_a_status = get_json(
        &client,
        &format!("{service_a_base}/api/v1/status"),
        "Phase 19 final Service Node A",
    )
    .await?;

    let final_c_status = get_json(
        &client,
        &format!("{service_c_base}/api/v1/status"),
        "Phase 19 final Service Node C",
    )
    .await?;

    let final_user_status = get_json(
        &client,
        &format!("{user_base}/api/v1/status"),
        "Phase 19 final User Node",
    )
    .await?;

    assert_macronode_truth(&final_a_status)?;
    assert_macronode_truth(&final_c_status)?;
    assert_micronode_truth(&final_user_status)?;

    ensure!(
        restarted_b_status["service_quorum_enabled"].as_bool() == Some(false),
        "live macronode must not falsely claim runtime quorum activation"
    );

    ensure!(
        restarted_b_status["wallet_execution_participant"].as_bool() == Some(false),
        "live macronode must not falsely claim wallet execution authority"
    );

    let restarted_signature_b = phase19_request_signature(
        &client,
        &service_b_base,
        admin_token,
        &transition_identity,
        "restarted Service Node B",
    )
    .await?;

    phase19_verify_live_signature(
        &restarted_signature_b,
        "service_node:beta",
        "key:phase19:beta",
        &seed_b,
    )?;

    ensure!(
        restarted_signature_b == signature_b,
        "restarted Service Node B must reproduce the same deterministic signature for the same explicit identity, seed, and transition"
    );

    phase19_live_two_of_three_quorum_after_member_loss(
        &transition_identity,
        signature_a.clone(),
        restarted_signature_b,
    )?;

    println!(
        "Phase 19: restarted Service Node B returned with the same deterministic signing identity and again formed canonical quorum with Service Node A."
    );

    let _ = restarted_service_b.kill_and_wait()?;
    let _ = service_a.kill_and_wait()?;
    let _ = service_c.kill_and_wait()?;
    let _ = user_node.kill_and_wait()?;

    let _ = fs::remove_dir_all(&service_a_db);
    let _ = fs::remove_dir_all(&service_b_db);
    let _ = fs::remove_dir_all(&service_c_db);

    println!(
        "Phase 19 live quorum passed: three real Service Nodes independently signed the same canonical transition; signatures verified against distinct deterministic Ed25519 identities; one signature failed 2-of-3 quorum; two signatures passed; A+C retained quorum after B loss; and restarted B reproduced its deterministic signature without wallet, ledger, or finality authority."
    );

    Ok(())
}

// RO:WHAT — FINAL_BETA Phase 19 live checkpoint-validator committee/finality proof.
// RO:WHY — Join deterministic checkpoint evidence with three independently running
// macronode checkpoint validators and prove threshold-backed finality survives member loss.
// RO:INTERACTS — checkpoint/sign HTTP route, ron-proto checkpoint validator signatures,
// finalized-checkpoint contract, real macronode processes, real micronode process.
// RO:INVARIANTS — distinct validator identities/seeds; same deterministic checkpoint hash;
// signatures independently verified; one-of-three cannot finalize; two-of-three can;
// wrong-candidate signature cannot finalize; restart preserves deterministic identity.
// RO:SECURITY — loopback-only private-beta proof; no finality HTTP route, wallet/ledger
// mutation, payout, receipt creation, bridge, staking, liquidity, or CrabLink authority.
// RO:TEST — run explicitly with --ignored --nocapture after building micronode.

const PHASE19_CHECKPOINT_CHAIN_ID: &str = "rustyonions-dev";

const PHASE19_CHECKPOINT_EPOCH_ID: &str = "epoch_phase19_checkpoint_live";

fn phase19_checkpoint_validator_identity(suffix: &str) -> ron_proto::QuickChainValidatorIdentityV1 {
    ron_proto::QuickChainValidatorIdentityV1 {
        schema: ron_proto::QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),

        version: ron_proto::QUICKCHAIN_DTO_VERSION,

        chain_id: PHASE19_CHECKPOINT_CHAIN_ID.to_owned(),

        epoch_id: PHASE19_CHECKPOINT_EPOCH_ID.to_owned(),

        validator_id: format!("validator-{suffix}"),

        passport_subject: format!("@validator-{suffix}"),

        registry_entry_id: format!("registry:validator-{suffix}"),

        key_id: format!("key:validator-{suffix}:001"),

        capability_id: format!("cap:validator-{suffix}:verify:001"),

        signature_algorithm: ron_proto::SignatureAlg::Ed25519,

        lifecycle_status: ron_proto::QuickChainValidatorLifecycleStatusV1::Active,

        not_before_ms: 1_800_000_000_000,

        expires_at_ms: 1_800_086_400_000,
    }
}

fn phase19_checkpoint_validator_set() -> ron_proto::QuickChainValidatorSetV1 {
    ron_proto::QuickChainValidatorSetV1 {
        schema: ron_proto::QUICKCHAIN_VALIDATOR_SET_SCHEMA.to_owned(),

        version: ron_proto::QUICKCHAIN_DTO_VERSION,

        chain_id: PHASE19_CHECKPOINT_CHAIN_ID.to_owned(),

        epoch_id: PHASE19_CHECKPOINT_EPOCH_ID.to_owned(),

        validator_set_hash: phase19_cid('c'),

        policy_hash: phase19_cid('b'),

        registry_snapshot_hash: phase19_cid('d'),

        passport_required: true,

        bond_required: false,

        validator_set_algorithm: ron_proto::QUICKCHAIN_VALIDATOR_SET_ALGORITHM_PASSPORT_REGISTRY_V1
            .to_owned(),

        members: vec![
            phase19_checkpoint_validator_identity("alpha"),
            phase19_checkpoint_validator_identity("beta"),
            phase19_checkpoint_validator_identity("gamma"),
        ],
    }
}

fn phase19_checkpoint_candidate(
    set: &ron_proto::QuickChainValidatorSetV1,
) -> ron_proto::QuickChainCommitteeCheckpointPayloadV1 {
    ron_proto::QuickChainCommitteeCheckpointPayloadV1 {
        schema: ron_proto::QUICKCHAIN_COMMITTEE_CHECKPOINT_PAYLOAD_SCHEMA.to_owned(),

        version: ron_proto::QUICKCHAIN_DTO_VERSION,

        chain_id: PHASE19_CHECKPOINT_CHAIN_ID.to_owned(),

        height: 19,

        epoch_id: PHASE19_CHECKPOINT_EPOCH_ID.to_owned(),

        execution_spec_version: "quickchain-execution-v1".to_owned(),

        previous_checkpoint_hash: phase19_cid('a'),

        previous_state_root: phase19_cid('b'),

        new_state_root: phase19_cid('c'),

        receipt_root: phase19_cid('d'),

        accounting_snapshot_root: phase19_cid('e'),

        reward_manifest_root: phase19_cid('f'),

        data_availability_root: phase19_cid('a'),

        policy_hash: set.policy_hash.clone(),

        validator_set_hash: set.validator_set_hash.clone(),

        chain_params_hash: phase19_cid('d'),

        canonical_encoding: ron_proto::QuickChainCanonicalEncodingV1::JsonV1,

        state_root_scheme: ron_proto::QuickChainStateRootSchemeV1::SortedMerkleMapV1,

        receipt_root_scheme: ron_proto::QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,

        supply_delta: ron_proto::QuickChainSupplyDeltaV1 {
            issued_minor: "0".to_owned(),

            burned_minor: "0".to_owned(),

            net_minor: "0".to_owned(),
        },

        conservation: ron_proto::QuickChainConservationV1 {
            debits_minor: "100".to_owned(),

            credits_minor: "100".to_owned(),

            issue_exceptions_minor: "0".to_owned(),

            burn_exceptions_minor: "0".to_owned(),

            valid: true,
        },

        settlement_mode: ron_proto::QuickChainSettlementModeV1::LocalRoot,

        started_at_ms: 1_800_000_000_000,

        ended_at_ms: 1_800_000_060_000,

        produced_at_ms: 1_800_000_061_000,
    }
}

fn phase19_checkpoint_hash(
    candidate: &ron_proto::QuickChainCommitteeCheckpointPayloadV1,
) -> Result<ContentId> {
    candidate
        .validate()
        .map_err(|error| anyhow!("Phase 19 checkpoint candidate validation failed: {error:?}"))?;

    let canonical = ron_proto::to_canonical_json_vec(candidate)
        .map_err(|error| anyhow!("Phase 19 checkpoint canonicalization failed: {error}"))?;

    let mut hasher = blake3::Hasher::new();

    hasher.update(ron_proto::QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1.as_bytes());

    hasher.update(&[0]);

    hasher.update(&canonical);

    format!("b3:{}", hasher.finalize().to_hex(),)
        .parse()
        .map_err(|error| anyhow!("Phase 19 checkpoint hash parse failed: {error}"))
}

fn spawn_checkpoint_validator_macronode(
    ports: MacronodePorts,
    index_db: &Path,
    identity: &ron_proto::QuickChainValidatorIdentityV1,
    seed_hex: &str,
    admin_token: &str,
) -> Result<ChildGuard> {
    let validator_json =
        serde_json::to_string(identity).context("encode Phase 19 checkpoint validator identity")?;

    let mut command = Command::new(macronode_bin());

    clear_project_environment(&mut command);

    command
        .arg("run")
        .env("RUST_LOG", "info,macronode=debug")
        .env("RON_HTTP_ADDR", format!("127.0.0.1:{}", ports.admin,))
        .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", ports.gateway,))
        .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", ports.storage,))
        .env("INDEX_BIND", format!("127.0.0.1:{}", ports.index,))
        .env("RON_OVERLAY_ADDR", format!("127.0.0.1:{}", ports.overlay,))
        .env("RON_DHT_ADDR", format!("127.0.0.1:{}", ports.dht,))
        .env("RON_MAILBOX_ADDR", format!("127.0.0.1:{}", ports.mailbox,))
        .env("RON_INDEX_DB", index_db)
        .env("RON_HEADLESS_MODE", "true")
        .env("RON_ADMIN_UI_ENABLED", "false")
        .env("RON_ADMIN_UI_RUNTIME_REQUIRED", "false")
        .env("RON_OPERATOR_UI_PROFILE", "service_node_local")
        .env("RON_ADMIN_TOKEN", admin_token)
        .env(
            "RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_SIGNING_ENABLED",
            "true",
        )
        .env(
            "RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_VALIDATOR_JSON",
            validator_json,
        )
        .env("RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_SEED_HEX", seed_hex)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command
        .spawn()
        .context("spawn Phase 19 checkpoint-validator macronode")?;

    Ok(ChildGuard::new(
        "Phase 19 checkpoint-validator macronode",
        child,
    ))
}

async fn phase19_request_checkpoint_signature(
    client: &Client,
    base: &str,
    admin_token: &str,
    checkpoint_hash: &ContentId,
    label: &str,
) -> Result<ron_proto::QuickChainCheckpointValidatorSignatureV1> {
    let response = client
        .post(format!("{base}/api/v1/quickchain/checkpoint/sign"))
        .bearer_auth(admin_token)
        .json(&serde_json::json!({
            "height": 19,
            "checkpoint_hash":
                checkpoint_hash
        }))
        .send()
        .await
        .with_context(|| format!("POST Phase 19 checkpoint signature request to {label}"))?;

    ensure!(
        response.status().is_success(),
        "{label} checkpoint signing returned HTTP {}",
        response.status(),
    );

    let body = response
        .json::<Value>()
        .await
        .with_context(|| format!("decode Phase 19 checkpoint signature response from {label}"))?;

    ensure!(
        body["status"].as_str() == Some("signed"),
        "{label} must report one signed checkpoint result",
    );

    ensure!(
        body["checkpointValidatorSignatureCreated"].as_bool() == Some(true),
        "{label} must report one checkpoint-validator signature",
    );

    for field in [
        "committeeAggregated",
        "committeeThresholdSatisfied",
        "checkpointFinalized",
        "walletMutation",
        "ledgerMutation",
        "payoutExecuted",
        "receiptCreated",
        "bridgeSettled",
        "finality",
        "crabLinkFinalityAuthority",
    ] {
        ensure!(
            body[field].as_bool() == Some(false),
            "{label} must not claim authority in {field}",
        );
    }

    serde_json::from_value(body["signature"].clone())
        .with_context(|| format!("decode checkpoint validator signature from {label}"))
}

fn phase19_verify_live_checkpoint_signature(
    signature: &ron_proto::QuickChainCheckpointValidatorSignatureV1,
    expected_identity: &ron_proto::QuickChainValidatorIdentityV1,
    checkpoint_hash: &ContentId,
    seed: &[u8; 32],
) -> Result<()> {
    signature
        .validate()
        .map_err(|error| anyhow!("live checkpoint signature failed DTO validation: {error:?}"))?;

    ensure!(
        signature.chain_id == expected_identity.chain_id,
        "{} checkpoint signature chain mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        signature.epoch_id == expected_identity.epoch_id,
        "{} checkpoint signature epoch mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        signature.validator_id == expected_identity.validator_id,
        "{} checkpoint signature validator mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        signature.key_id == expected_identity.key_id,
        "{} checkpoint signature key mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        signature.algorithm == expected_identity.signature_algorithm,
        "{} checkpoint signature algorithm mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        signature.height == 19,
        "{} checkpoint signature height mismatch",
        expected_identity.validator_id,
    );

    ensure!(
        &signature.checkpoint_hash == checkpoint_hash,
        "{} checkpoint signature hash mismatch",
        expected_identity.validator_id,
    );

    let message =
        ron_proto::checkpoint_validator_signature_message_bytes(&signature.signing_payload())
            .map_err(|error| anyhow!("encode live checkpoint signature message failed: {error}"))?;

    let raw_signature = phase19_decode_signature(&signature.signature_wire)?;

    let public_key = ed25519::public_key(seed);

    ensure!(
        ed25519::verify(&public_key, &message, &raw_signature,),
        "{} checkpoint signature failed independent Ed25519 verification",
        expected_identity.validator_id,
    );

    Ok(())
}

fn phase19_finalized_checkpoint_from_live_signatures(
    candidate: &ron_proto::QuickChainCommitteeCheckpointPayloadV1,
    checkpoint_hash: &ContentId,
    validator_set: &ron_proto::QuickChainValidatorSetV1,
    mut signatures: Vec<ron_proto::QuickChainCheckpointValidatorSignatureV1>,
) -> ron_proto::QuickChainFinalizedCheckpointV1 {
    signatures.sort_by(|left, right| left.validator_id.cmp(&right.validator_id));

    ron_proto::QuickChainFinalizedCheckpointV1 {
        schema: ron_proto::QUICKCHAIN_FINALIZED_CHECKPOINT_SCHEMA.to_owned(),

        version: ron_proto::QUICKCHAIN_DTO_VERSION,

        candidate: candidate.clone(),

        checkpoint_hash: checkpoint_hash.clone(),

        validator_set: validator_set.clone(),

        required_signatures: 2,

        verified_unique_signatures: u32::try_from(signatures.len())
            .expect("Phase 19 live signature count fits u32"),

        signatures,

        finalization_state: ron_proto::QuickChainCheckpointFinalityStateV1::Finalized,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live FINAL_BETA Phase 19 checkpoint finality; launches three checkpoint-validator macronodes and one micronode"]
async fn phase19_live_checkpoint_validator_committee_reaches_finality_after_member_loss_and_restart(
) -> Result<()> {
    let ports = reserve_loopback_ports(22)?;

    let service_a_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let service_b_ports = MacronodePorts {
        admin: ports[7],
        gateway: ports[8],
        storage: ports[9],
        index: ports[10],
        overlay: ports[11],
        dht: ports[12],
        mailbox: ports[13],
    };

    let service_c_ports = MacronodePorts {
        admin: ports[14],
        gateway: ports[15],
        storage: ports[16],
        index: ports[17],
        overlay: ports[18],
        dht: ports[19],
        mailbox: ports[20],
    };

    let user_port = ports[21];

    let service_a_db = unique_index_db();

    let service_b_db = unique_index_db();

    let service_c_db = unique_index_db();

    ensure!(
        service_a_db != service_b_db
            && service_a_db != service_c_db
            && service_b_db != service_c_db,
        "live checkpoint validators require isolated index stores",
    );

    let client = test_client()?;

    let service_a_base = format!("http://127.0.0.1:{}", service_a_ports.admin,);

    let service_b_base = format!("http://127.0.0.1:{}", service_b_ports.admin,);

    let service_c_base = format!("http://127.0.0.1:{}", service_c_ports.admin,);

    let user_base = format!("http://127.0.0.1:{user_port}");

    let admin_token = "phase19-live-checkpoint-admin";

    let validator_set = phase19_checkpoint_validator_set();

    validator_set
        .validate()
        .map_err(|error| anyhow!("live checkpoint validator set invalid: {error:?}"))?;

    let candidate = phase19_checkpoint_candidate(&validator_set);

    let checkpoint_hash = phase19_checkpoint_hash(&candidate)?;

    let identity_a = phase19_checkpoint_validator_identity("alpha");

    let identity_b = phase19_checkpoint_validator_identity("beta");

    let identity_c = phase19_checkpoint_validator_identity("gamma");

    let seed_a = phase19_seed(0x41);

    let seed_b = phase19_seed(0x42);

    let seed_c = phase19_seed(0x43);

    let seed_a_hex = phase19_seed_hex(0x41);

    let seed_b_hex = phase19_seed_hex(0x42);

    let seed_c_hex = phase19_seed_hex(0x43);

    let mut service_a = spawn_checkpoint_validator_macronode(
        service_a_ports,
        &service_a_db,
        &identity_a,
        &seed_a_hex,
        admin_token,
    )?;

    let mut service_b = spawn_checkpoint_validator_macronode(
        service_b_ports,
        &service_b_db,
        &identity_b,
        &seed_b_hex,
        admin_token,
    )?;

    let mut service_c = spawn_checkpoint_validator_macronode(
        service_c_ports,
        &service_c_db,
        &identity_c,
        &seed_c_hex,
        admin_token,
    )?;

    let mut user_node = spawn_micronode(user_port)?;

    println!(
        "Phase 19 checkpoint: waiting for three independent checkpoint validators and one User Node..."
    );

    let service_a_status = wait_for_macronode(&client, &service_a_base).await?;

    let service_b_status = wait_for_macronode(&client, &service_b_base).await?;

    let service_c_status = wait_for_macronode(&client, &service_c_base).await?;

    let user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&service_a_status)?;

    assert_macronode_truth(&service_b_status)?;

    assert_macronode_truth(&service_c_status)?;

    assert_micronode_truth(&user_status)?;

    service_a.assert_running()?;
    service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    println!(
        "Phase 19 checkpoint: requesting the same deterministic checkpoint hash from validators A, B, and C..."
    );

    let signature_a = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &checkpoint_hash,
        "checkpoint validator A",
    )
    .await?;

    let signature_b = phase19_request_checkpoint_signature(
        &client,
        &service_b_base,
        admin_token,
        &checkpoint_hash,
        "checkpoint validator B",
    )
    .await?;

    let signature_c = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &checkpoint_hash,
        "checkpoint validator C",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(&signature_a, &identity_a, &checkpoint_hash, &seed_a)?;

    phase19_verify_live_checkpoint_signature(&signature_b, &identity_b, &checkpoint_hash, &seed_b)?;

    phase19_verify_live_checkpoint_signature(&signature_c, &identity_c, &checkpoint_hash, &seed_c)?;

    ensure!(
        signature_a.validator_id != signature_b.validator_id
            && signature_b.validator_id != signature_c.validator_id
            && signature_a.validator_id != signature_c.validator_id,
        "all three live checkpoint signatures must come from distinct validator identities",
    );

    ensure!(
        signature_a.signature_wire != signature_b.signature_wire
            && signature_b.signature_wire != signature_c.signature_wire
            && signature_a.signature_wire != signature_c.signature_wire,
        "distinct live checkpoint validators must produce distinct signature wires",
    );

    let one_of_three = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone()],
    );

    ensure!(
        one_of_three.validate().is_err(),
        "one live checkpoint validator must not be able to claim finalized checkpoint evidence",
    );

    let two_of_three = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_b.clone(), signature_a.clone()],
    );

    two_of_three
        .validate()
        .map_err(|error| {
            anyhow!(
                "two independently verified live checkpoint signatures must satisfy finalized-checkpoint contract: {error:?}"
            )
        })?;

    ensure!(
        two_of_three.signatures[0].validator_id == "validator-alpha"
            && two_of_three.signatures[1].validator_id == "validator-beta",
        "live finalized-checkpoint signatures must be deterministically ordered",
    );

    println!(
        "Phase 19 checkpoint: three independent process signatures verified; one-of-three rejected; two-of-three produced valid finalized-checkpoint evidence."
    );

    let wrong_checkpoint_hash = phase19_cid('f');

    ensure!(
        wrong_checkpoint_hash != checkpoint_hash,
        "wrong checkpoint fixture must differ from canonical candidate hash",
    );

    let wrong_signature_c = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &wrong_checkpoint_hash,
        "checkpoint validator C wrong-candidate probe",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &wrong_signature_c,
        &identity_c,
        &wrong_checkpoint_hash,
        &seed_c,
    )?;

    let wrong_candidate_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone(), wrong_signature_c],
    );

    ensure!(
        wrong_candidate_finality
            .validate()
            .is_err(),
        "cryptographically valid signature over a different checkpoint must not finalize the canonical candidate",
    );

    println!(
        "Phase 19 checkpoint: wrong-candidate signature remained cryptographically valid for its own hash but was rejected from canonical checkpoint finality."
    );

    println!("Phase 19 checkpoint: terminating validator B and proving A+C retain threshold...");

    let _ = service_b.kill_and_wait()?;

    wait_for_unreachable(
        &client,
        &format!("{service_b_base}/healthz"),
        "Phase 19 checkpoint validator B",
    )
    .await?;

    service_a.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let surviving_a_status = get_json(
        &client,
        &format!("{service_a_base}/api/v1/status"),
        "Phase 19 surviving checkpoint validator A",
    )
    .await?;

    let surviving_c_status = get_json(
        &client,
        &format!("{service_c_base}/api/v1/status"),
        "Phase 19 surviving checkpoint validator C",
    )
    .await?;

    let surviving_user_status = get_json(
        &client,
        &format!("{user_base}/api/v1/status"),
        "Phase 19 surviving checkpoint User Node",
    )
    .await?;

    assert_macronode_truth(&surviving_a_status)?;

    assert_macronode_truth(&surviving_c_status)?;

    assert_micronode_truth(&surviving_user_status)?;

    let signature_a_after_loss = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &checkpoint_hash,
        "surviving checkpoint validator A",
    )
    .await?;

    let signature_c_after_loss = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &checkpoint_hash,
        "surviving checkpoint validator C",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &signature_a_after_loss,
        &identity_a,
        &checkpoint_hash,
        &seed_a,
    )?;

    phase19_verify_live_checkpoint_signature(
        &signature_c_after_loss,
        &identity_c,
        &checkpoint_hash,
        &seed_c,
    )?;

    let after_member_loss = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a_after_loss.clone(), signature_c_after_loss],
    );

    after_member_loss.validate().map_err(|error| {
        anyhow!("A+C must retain valid checkpoint finality after B loss: {error:?}")
    })?;

    println!(
        "Phase 19 checkpoint: A+C retained two-of-three checkpoint finality after validator B loss."
    );

    println!(
        "Phase 19 checkpoint: restarting validator B with the same reviewed identity and deterministic seed..."
    );

    let mut restarted_service_b = spawn_checkpoint_validator_macronode(
        service_b_ports,
        &service_b_db,
        &identity_b,
        &seed_b_hex,
        admin_token,
    )?;

    let restarted_b_status = wait_for_macronode(&client, &service_b_base).await?;

    assert_macronode_truth(&restarted_b_status)?;

    service_a.assert_running()?;
    restarted_service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let restarted_signature_b = phase19_request_checkpoint_signature(
        &client,
        &service_b_base,
        admin_token,
        &checkpoint_hash,
        "restarted checkpoint validator B",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &restarted_signature_b,
        &identity_b,
        &checkpoint_hash,
        &seed_b,
    )?;

    ensure!(
        restarted_signature_b.validator_id == signature_b.validator_id,
        "restarted validator B must retain reviewed validator identity",
    );

    ensure!(
        restarted_signature_b.key_id == signature_b.key_id,
        "restarted validator B must retain reviewed logical key identity",
    );

    ensure!(
        restarted_signature_b.signature_wire == signature_b.signature_wire,
        "deterministic Ed25519 signing must reproduce validator B signature after restart",
    );

    let after_restart = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a_after_loss, restarted_signature_b],
    );

    after_restart.validate().map_err(|error| {
        anyhow!("A+restarted-B must restore valid checkpoint finality: {error:?}")
    })?;

    ensure!(
        restarted_b_status["service_quorum_enabled"].as_bool() == Some(false),
        "checkpoint-validator signing must not falsely flip Service Node reward-quorum status",
    );

    ensure!(
        restarted_b_status["wallet_execution_participant"].as_bool() == Some(false),
        "checkpoint-validator signing must not grant wallet execution authority",
    );

    let _ = restarted_service_b.kill_and_wait()?;

    let _ = service_a.kill_and_wait()?;

    let _ = service_c.kill_and_wait()?;

    let _ = user_node.kill_and_wait()?;

    let _ = fs::remove_dir_all(&service_a_db);

    let _ = fs::remove_dir_all(&service_b_db);

    let _ = fs::remove_dir_all(&service_c_db);

    println!(
        "Phase 19 live checkpoint finality passed: three real checkpoint-validator macronodes signed one deterministic candidate hash with distinct Ed25519 identities; signatures verified independently; one-of-three could not finalize; two-of-three produced valid finalized-checkpoint evidence; wrong-candidate evidence was rejected; A+C retained finality after B loss; and restarted B reproduced its deterministic identity/signature without HTTP, wallet, ledger, or CrabLink finality authority."
    );

    Ok(())
}

// RO:WHAT — FINAL_BETA Phase 19 Step 7A live signer replay and duplicate-count proof.
// RO:WHY — Replaying deterministic signing requests must not manufacture extra
// checkpoint-finality or Service Node quorum weight.
// RO:INTERACTS — real checkpoint/sign HTTP surface, real quorum/sign HTTP surface,
// checkpoint finality DTO validation, ServiceNodeQuorumV1 validation, live macronode
// processes, and one live micronode process.
// RO:INVARIANTS — identical request to one deterministic signer returns identical
// evidence; duplicate evidence from one identity never counts as two participants.
// RO:SECURITY — test-only; no wallet/ledger mutation, finality HTTP endpoint, payout,
// receipt creation, bridge, staking, or CrabLink authority.
// RO:TEST — run explicitly with --ignored --nocapture.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live FINAL_BETA Phase 19 Step 7A signer replay/duplicate proof"]
async fn phase19_live_replayed_signing_requests_cannot_inflate_checkpoint_or_service_quorum(
) -> Result<()> {
    let ports = reserve_loopback_ports(15)?;

    let checkpoint_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let quorum_ports = MacronodePorts {
        admin: ports[7],
        gateway: ports[8],
        storage: ports[9],
        index: ports[10],
        overlay: ports[11],
        dht: ports[12],
        mailbox: ports[13],
    };

    let user_port = ports[14];

    let checkpoint_db = unique_index_db();

    let quorum_db = unique_index_db();

    ensure!(
        checkpoint_db != quorum_db,
        "Phase 19 Step 7A processes require isolated index stores",
    );

    let client = test_client()?;

    let checkpoint_base = format!("http://127.0.0.1:{}", checkpoint_ports.admin,);

    let quorum_base = format!("http://127.0.0.1:{}", quorum_ports.admin,);

    let user_base = format!("http://127.0.0.1:{user_port}");

    let admin_token = "phase19-live-replay-duplicate-admin";

    let checkpoint_set = phase19_checkpoint_validator_set();

    let checkpoint_candidate = phase19_checkpoint_candidate(&checkpoint_set);

    let checkpoint_hash = phase19_checkpoint_hash(&checkpoint_candidate)?;

    let checkpoint_identity = phase19_checkpoint_validator_identity("alpha");

    let checkpoint_seed = phase19_seed(0x41);

    let checkpoint_seed_hex = phase19_seed_hex(0x41);

    let quorum_seed = phase19_seed(0x11);

    let quorum_seed_hex = phase19_seed_hex(0x11);

    let transition_identity = phase19_transition_identity();

    let mut checkpoint_node = spawn_checkpoint_validator_macronode(
        checkpoint_ports,
        &checkpoint_db,
        &checkpoint_identity,
        &checkpoint_seed_hex,
        admin_token,
    )?;

    let mut quorum_node = spawn_quorum_macronode(
        quorum_ports,
        &quorum_db,
        "service_node:alpha",
        "key:phase19:alpha",
        &quorum_seed_hex,
        admin_token,
    )?;

    let mut user_node = spawn_micronode(user_port)?;

    println!(
        "Phase 19 Step 7A: waiting for checkpoint validator, Service Node signer, and User Node..."
    );

    let checkpoint_status = wait_for_macronode(&client, &checkpoint_base).await?;

    let quorum_status = wait_for_macronode(&client, &quorum_base).await?;

    let user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&checkpoint_status)?;

    assert_macronode_truth(&quorum_status)?;

    assert_micronode_truth(&user_status)?;

    checkpoint_node.assert_running()?;

    quorum_node.assert_running()?;

    user_node.assert_running()?;

    println!(
        "Phase 19 Step 7A: replaying the exact checkpoint signing request against one real validator..."
    );

    let checkpoint_signature_first = phase19_request_checkpoint_signature(
        &client,
        &checkpoint_base,
        admin_token,
        &checkpoint_hash,
        "checkpoint validator replay first request",
    )
    .await?;

    let checkpoint_signature_replay = phase19_request_checkpoint_signature(
        &client,
        &checkpoint_base,
        admin_token,
        &checkpoint_hash,
        "checkpoint validator replay second request",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &checkpoint_signature_first,
        &checkpoint_identity,
        &checkpoint_hash,
        &checkpoint_seed,
    )?;

    phase19_verify_live_checkpoint_signature(
        &checkpoint_signature_replay,
        &checkpoint_identity,
        &checkpoint_hash,
        &checkpoint_seed,
    )?;

    ensure!(
        checkpoint_signature_first == checkpoint_signature_replay,
        "identical checkpoint signing replay must return identical deterministic evidence",
    );

    let duplicated_checkpoint_finality = phase19_finalized_checkpoint_from_live_signatures(
        &checkpoint_candidate,
        &checkpoint_hash,
        &checkpoint_set,
        vec![
            checkpoint_signature_first.clone(),
            checkpoint_signature_replay.clone(),
        ],
    );

    ensure!(
        duplicated_checkpoint_finality.validate().is_err(),
        "replaying one validator signature must not satisfy two-of-three checkpoint finality",
    );

    println!(
        "Phase 19 Step 7A: checkpoint signing replay was deterministic and duplicate validator evidence did not inflate finality."
    );

    println!(
        "Phase 19 Step 7A: replaying the exact Service Node epoch-transition signing request..."
    );

    let service_signature_first = phase19_request_signature(
        &client,
        &quorum_base,
        admin_token,
        &transition_identity,
        "Service Node replay first request",
    )
    .await?;

    let service_signature_replay = phase19_request_signature(
        &client,
        &quorum_base,
        admin_token,
        &transition_identity,
        "Service Node replay second request",
    )
    .await?;

    phase19_verify_live_signature(
        &service_signature_first,
        "service_node:alpha",
        "key:phase19:alpha",
        &quorum_seed,
    )?;

    phase19_verify_live_signature(
        &service_signature_replay,
        "service_node:alpha",
        "key:phase19:alpha",
        &quorum_seed,
    )?;

    ensure!(
        service_signature_first == service_signature_replay,
        "identical Service Node signing replay must return identical deterministic evidence",
    );

    let duplicated_service_quorum = ron_proto::ServiceNodeQuorumV1 {
        schema: ron_proto::SERVICE_NODE_QUORUM_SCHEMA.to_owned(),

        version: ron_proto::SERVICE_NODE_QUORUM_VERSION,

        chain_id: "rustyonions-dev".to_owned(),

        epoch_id: "epoch:phase19-live".to_owned(),

        transition_hash: service_signature_first.transition_hash.clone(),

        threshold: phase19_quorum_threshold(),

        eligibilities: vec![
            phase19_eligibility("alpha"),
            phase19_eligibility("beta"),
            phase19_eligibility("gamma"),
        ],

        signatures: vec![
            service_signature_first.clone(),
            service_signature_replay.clone(),
        ],
    };

    ensure!(
        duplicated_service_quorum.validate().is_err(),
        "replaying one Service Node signature must not satisfy two-of-three Service Node quorum",
    );

    println!(
        "Phase 19 Step 7A: Service Node signing replay was deterministic and duplicate signer evidence did not inflate quorum."
    );

    ensure!(
        checkpoint_status["wallet_execution_participant"].as_bool() == Some(false),
        "checkpoint replay proof must not grant wallet execution authority",
    );

    ensure!(
        quorum_status["wallet_execution_participant"].as_bool() == Some(false),
        "Service Node signing replay proof must not grant wallet execution authority",
    );

    checkpoint_node.assert_running()?;

    quorum_node.assert_running()?;

    user_node.assert_running()?;

    let _ = checkpoint_node.kill_and_wait()?;

    let _ = quorum_node.kill_and_wait()?;

    let _ = user_node.kill_and_wait()?;

    let _ = fs::remove_dir_all(&checkpoint_db);

    let _ = fs::remove_dir_all(&quorum_db);

    println!(
        "Phase 19 Step 7A live signer replay passed: repeated checkpoint signing returned identical valid evidence without inflating validator count; duplicate validator evidence could not finalize; repeated Service Node transition signing returned identical valid evidence without inflating Service Node quorum; no wallet, ledger, payout, receipt, or CrabLink authority was created."
    );

    Ok(())
}

// RO:WHAT — FINAL_BETA Phase 19 integrated checkpoint/network chaos.
// RO:WHY — Prove real checkpoint-validator processes remain safe during
// member/User-Node loss while malformed, stale, replayed, wrongly keyed,
// cryptographically bad, and wrong-candidate evidence cannot manufacture
// checkpoint finality.
// RO:INTERACTS — three real macronode checkpoint validators, one real
// micronode User Node, guarded checkpoint signing HTTP, checkpoint finality
// contract, process kill/restart, deterministic Ed25519 verification.
// RO:INVARIANTS — two surviving validators retain threshold when possible;
// User Node absence grants no authority; duplicate signer weight is rejected;
// stale/wrong evidence does not finalize; restarted identities are stable.
// RO:SECURITY — loopback private-beta chaos only. No wallet/ledger mutation,
// payout, receipt creation, bridge, slashing, or CrabLink finality authority.
// RO:TEST — run explicitly with --ignored --nocapture --test-threads=1.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live FINAL_BETA Phase 19 integrated chaos; launches three checkpoint-validator macronodes and one micronode"]
async fn phase19_integrated_checkpoint_chaos_survives_partial_availability_and_rejects_invalid_evidence(
) -> Result<()> {
    let ports = reserve_loopback_ports(22)?;

    let service_a_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let service_b_ports = MacronodePorts {
        admin: ports[7],
        gateway: ports[8],
        storage: ports[9],
        index: ports[10],
        overlay: ports[11],
        dht: ports[12],
        mailbox: ports[13],
    };

    let service_c_ports = MacronodePorts {
        admin: ports[14],
        gateway: ports[15],
        storage: ports[16],
        index: ports[17],
        overlay: ports[18],
        dht: ports[19],
        mailbox: ports[20],
    };

    let user_port = ports[21];

    let service_a_db = unique_index_db();

    let service_b_db = unique_index_db();

    let service_c_db = unique_index_db();

    ensure!(
        service_a_db != service_b_db
            && service_a_db != service_c_db
            && service_b_db != service_c_db,
        "Phase 19 chaos validators require isolated index stores",
    );

    let client = test_client()?;

    let service_a_base = format!("http://127.0.0.1:{}", service_a_ports.admin,);

    let service_b_base = format!("http://127.0.0.1:{}", service_b_ports.admin,);

    let service_c_base = format!("http://127.0.0.1:{}", service_c_ports.admin,);

    let user_base = format!("http://127.0.0.1:{user_port}");

    let admin_token = "phase19-integrated-chaos-admin";

    let validator_set = phase19_checkpoint_validator_set();

    validator_set
        .validate()
        .map_err(|error| anyhow!("Phase 19 chaos validator set invalid: {error:?}"))?;

    let candidate = phase19_checkpoint_candidate(&validator_set);

    let checkpoint_hash = phase19_checkpoint_hash(&candidate)?;

    let identity_a = phase19_checkpoint_validator_identity("alpha");

    let identity_b = phase19_checkpoint_validator_identity("beta");

    let identity_c = phase19_checkpoint_validator_identity("gamma");

    let seed_a = phase19_seed(0x41);

    let seed_b = phase19_seed(0x42);

    let seed_c = phase19_seed(0x43);

    let seed_a_hex = phase19_seed_hex(0x41);

    let seed_b_hex = phase19_seed_hex(0x42);

    let seed_c_hex = phase19_seed_hex(0x43);

    let mut service_a = spawn_checkpoint_validator_macronode(
        service_a_ports,
        &service_a_db,
        &identity_a,
        &seed_a_hex,
        admin_token,
    )?;

    let mut service_b = spawn_checkpoint_validator_macronode(
        service_b_ports,
        &service_b_db,
        &identity_b,
        &seed_b_hex,
        admin_token,
    )?;

    let mut service_c = spawn_checkpoint_validator_macronode(
        service_c_ports,
        &service_c_db,
        &identity_c,
        &seed_c_hex,
        admin_token,
    )?;

    let mut user_node = spawn_micronode(user_port)?;

    println!("Phase 19 chaos: waiting for three checkpoint validators and one User Node...");

    let service_a_status = wait_for_macronode(&client, &service_a_base).await?;

    let service_b_status = wait_for_macronode(&client, &service_b_base).await?;

    let service_c_status = wait_for_macronode(&client, &service_c_base).await?;

    let user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&service_a_status)?;

    assert_macronode_truth(&service_b_status)?;

    assert_macronode_truth(&service_c_status)?;

    assert_micronode_truth(&user_status)?;

    service_a.assert_running()?;
    service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    println!("Phase 19 chaos: establishing healthy two-of-three checkpoint finality...");

    let signature_a = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &checkpoint_hash,
        "chaos validator A",
    )
    .await?;

    let signature_b = phase19_request_checkpoint_signature(
        &client,
        &service_b_base,
        admin_token,
        &checkpoint_hash,
        "chaos validator B",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(&signature_a, &identity_a, &checkpoint_hash, &seed_a)?;

    phase19_verify_live_checkpoint_signature(&signature_b, &identity_b, &checkpoint_hash, &seed_b)?;

    let healthy_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone(), signature_b.clone()],
    );

    healthy_finality
        .validate()
        .map_err(|error| anyhow!("healthy Phase 19 chaos baseline must finalize: {error:?}"))?;

    println!("Phase 19 chaos: injecting malformed checkpoint request...");

    let malformed_response = client
        .post(format!(
            "{service_a_base}/api/v1/quickchain/checkpoint/sign"
        ))
        .bearer_auth(admin_token)
        .json(&serde_json::json!({
            "height": 19,
            "checkpoint_hash": "not-a-valid-content-id"
        }))
        .send()
        .await
        .context("send malformed checkpoint chaos request")?;

    ensure!(
        !malformed_response.status().is_success(),
        "malformed checkpoint request must fail closed",
    );

    service_a.assert_running()?;

    println!("Phase 19 chaos: replaying and duplicating one validator request...");

    let signature_a_replay = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &checkpoint_hash,
        "chaos validator A replay",
    )
    .await?;

    ensure!(
        signature_a_replay == signature_a,
        "identical checkpoint signing retry must reproduce deterministic evidence",
    );

    let duplicate_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone(), signature_a_replay],
    );

    ensure!(
        duplicate_finality.validate().is_err(),
        "duplicate evidence from one validator must not satisfy two-of-three finality",
    );

    println!("Phase 19 chaos: injecting wrong checkpoint candidate...");

    let wrong_checkpoint_hash = phase19_cid('f');

    ensure!(
        wrong_checkpoint_hash != checkpoint_hash,
        "wrong checkpoint chaos fixture must differ from canonical checkpoint",
    );

    let wrong_candidate_signature = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &wrong_checkpoint_hash,
        "chaos validator C wrong candidate",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &wrong_candidate_signature,
        &identity_c,
        &wrong_checkpoint_hash,
        &seed_c,
    )?;

    let wrong_candidate_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone(), wrong_candidate_signature],
    );

    ensure!(
        wrong_candidate_finality.validate().is_err(),
        "signature over wrong checkpoint candidate must not finalize canonical candidate",
    );

    println!("Phase 19 chaos: injecting wrong signing-key binding...");

    let mut wrong_key_signature = signature_b.clone();

    wrong_key_signature.key_id = "key:validator-chaos-wrong:001".to_owned();

    ensure!(
        phase19_verify_live_checkpoint_signature(
            &wrong_key_signature,
            &identity_b,
            &checkpoint_hash,
            &seed_b,
        )
        .is_err(),
        "wrong checkpoint signing-key binding must fail independent verification",
    );

    let wrong_key_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![signature_a.clone(), wrong_key_signature],
    );

    ensure!(
        wrong_key_finality.validate().is_err(),
        "wrong validator key binding must not satisfy finalized-checkpoint contract",
    );

    println!("Phase 19 chaos: injecting cryptographically bad signature...");

    let mut bad_signature = signature_b.clone();

    let replacement = if bad_signature.signature_wire.starts_with('0') {
        "1"
    } else {
        "0"
    };

    bad_signature
        .signature_wire
        .replace_range(0..1, replacement);

    ensure!(
        phase19_verify_live_checkpoint_signature(
            &bad_signature,
            &identity_b,
            &checkpoint_hash,
            &seed_b,
        )
        .is_err(),
        "cryptographically bad checkpoint signature must fail independent verification",
    );

    println!("Phase 19 chaos: injecting stale validator set...");

    let mut stale_validator_set = validator_set.clone();

    stale_validator_set.epoch_id = "epoch_phase19_stale_validator_set".to_owned();

    stale_validator_set.validator_set_hash = phase19_cid('e');

    for member in &mut stale_validator_set.members {
        member.epoch_id = stale_validator_set.epoch_id.clone();
    }

    stale_validator_set.validate().map_err(|error| {
        anyhow!("stale validator-set chaos fixture must remain structurally valid: {error:?}")
    })?;

    let stale_set_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &stale_validator_set,
        vec![signature_a.clone(), signature_b.clone()],
    );

    ensure!(
        stale_set_finality.validate().is_err(),
        "stale validator set must not finalize current checkpoint",
    );

    println!(
        "Phase 19 chaos: killing validator B and User Node simultaneously to simulate partial network availability..."
    );

    let _ = service_b.kill_and_wait()?;

    wait_for_unreachable(
        &client,
        &format!("{service_b_base}/healthz"),
        "Phase 19 chaos validator B",
    )
    .await?;

    let _ = user_node.kill_and_wait()?;

    wait_for_unreachable(
        &client,
        &format!("{user_base}/healthz"),
        "Phase 19 chaos User Node",
    )
    .await?;

    service_a.assert_running()?;
    service_c.assert_running()?;

    let surviving_a_status = get_json(
        &client,
        &format!("{service_a_base}/api/v1/status"),
        "Phase 19 chaos surviving validator A",
    )
    .await?;

    let surviving_c_status = get_json(
        &client,
        &format!("{service_c_base}/api/v1/status"),
        "Phase 19 chaos surviving validator C",
    )
    .await?;

    assert_macronode_truth(&surviving_a_status)?;

    assert_macronode_truth(&surviving_c_status)?;

    let surviving_signature_a = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &checkpoint_hash,
        "chaos surviving validator A",
    )
    .await?;

    let surviving_signature_c = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &checkpoint_hash,
        "chaos surviving validator C",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &surviving_signature_a,
        &identity_a,
        &checkpoint_hash,
        &seed_a,
    )?;

    phase19_verify_live_checkpoint_signature(
        &surviving_signature_c,
        &identity_c,
        &checkpoint_hash,
        &seed_c,
    )?;

    let partial_availability_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![surviving_signature_a.clone(), surviving_signature_c],
    );

    partial_availability_finality
        .validate()
        .map_err(
            |error| {
                anyhow!(
                    "A+C must retain valid checkpoint finality while B and User Node are unavailable: {error:?}"
                )
            },
        )?;

    println!(
        "Phase 19 chaos: A+C retained threshold while validator B and User Node were unavailable."
    );

    println!("Phase 19 chaos: restarting validator B and User Node...");

    let mut restarted_service_b = spawn_checkpoint_validator_macronode(
        service_b_ports,
        &service_b_db,
        &identity_b,
        &seed_b_hex,
        admin_token,
    )?;

    let mut restarted_user_node = spawn_micronode(user_port)?;

    let restarted_b_status = wait_for_macronode(&client, &service_b_base).await?;

    let restarted_user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&restarted_b_status)?;

    assert_micronode_truth(&restarted_user_status)?;

    service_a.assert_running()?;
    restarted_service_b.assert_running()?;
    service_c.assert_running()?;
    restarted_user_node.assert_running()?;

    let restarted_signature_b = phase19_request_checkpoint_signature(
        &client,
        &service_b_base,
        admin_token,
        &checkpoint_hash,
        "chaos restarted validator B",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &restarted_signature_b,
        &identity_b,
        &checkpoint_hash,
        &seed_b,
    )?;

    ensure!(
        restarted_signature_b == signature_b,
        "restarted validator B must reproduce deterministic signing evidence",
    );

    let recovered_finality = phase19_finalized_checkpoint_from_live_signatures(
        &candidate,
        &checkpoint_hash,
        &validator_set,
        vec![surviving_signature_a, restarted_signature_b],
    );

    recovered_finality.validate().map_err(|error| {
        anyhow!("recovered validator set must restore canonical finality: {error:?}")
    })?;

    ensure!(
        restarted_b_status["wallet_execution_participant"].as_bool() == Some(false),
        "checkpoint chaos recovery must not grant wallet execution authority",
    );

    let _ = restarted_service_b.kill_and_wait()?;

    let _ = service_a.kill_and_wait()?;

    let _ = service_c.kill_and_wait()?;

    let _ = restarted_user_node.kill_and_wait()?;

    let _ = fs::remove_dir_all(&service_a_db);

    let _ = fs::remove_dir_all(&service_b_db);

    let _ = fs::remove_dir_all(&service_c_db);

    println!(
        "Phase 19 integrated chaos passed: malformed input failed closed; replayed/duplicate evidence could not inflate finality; wrong candidate, wrong key, bad signature, and stale validator set were rejected; A+C retained threshold while B and the User Node were unavailable; and both lost processes restarted without phantom wallet, ledger, or CrabLink authority."
    );

    Ok(())
}

// RO:WHAT — FINAL_BETA Phase 19 meaningful multi-node checkpoint soak.
// RO:WHY — Keep the real checkpoint-validator/User-Node topology active across
// repeated checkpoint verification/signing cycles and restarts while checking
// deterministic roots, candidate convergence, signature stability, process
// health, memory posture, and finality authority.
// RO:INTERACTS — three checkpoint-validator macronodes, one micronode,
// canonical checkpoint candidate/hash helpers, guarded signing HTTP, process
// restart harness, and local process RSS observation.
// RO:INVARIANTS — repeated identical candidate construction does not drift;
// validator signatures remain deterministic and unique per validator; one
// signer or duplicate signer weight never finalizes; valid two-of-three
// remains sufficient; restarts recover; no wallet execution authority appears.
// RO:SECURITY — loopback/private-beta soak only. No wallet/ledger mutation,
// payout, receipt creation, bridge, slashing, or CrabLink finality authority.
// RO:TEST — run explicitly with --ignored --nocapture --test-threads=1.

fn phase19_process_rss_kib(guard: &ChildGuard) -> Result<u64> {
    let child = guard.child.as_ref().ok_or_else(|| {
        anyhow!(
            "{} child is no longer owned for RSS observation",
            guard.label,
        )
    })?;

    let pid = child.id();

    let pid_string = pid.to_string();

    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid_string])
        .output()
        .with_context(|| format!("observe RSS for {} pid {pid}", guard.label,))?;

    ensure!(
        output.status.success(),
        "ps RSS observation failed for {} pid {pid}",
        guard.label,
    );

    let stdout = String::from_utf8(output.stdout).context("decode ps RSS output")?;

    stdout.trim().parse::<u64>().with_context(|| {
        format!(
            "parse RSS for {} pid {pid} from {:?}",
            guard.label,
            stdout.trim(),
        )
    })
}

fn phase19_assert_soak_memory_bound(
    label: &str,
    baseline_kib: u64,
    observed_kib: u64,
) -> Result<()> {
    const MAX_SOAK_RSS_GROWTH_KIB: u64 = 128 * 1024;

    ensure!(
        observed_kib
            <= baseline_kib
                .saturating_add(
                    MAX_SOAK_RSS_GROWTH_KIB,
                ),
        "Phase 19 soak detected possible memory runaway for {label}: baseline={baseline_kib}KiB observed={observed_kib}KiB allowed_growth={MAX_SOAK_RSS_GROWTH_KIB}KiB",
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "live FINAL_BETA Phase 19 soak; launches three checkpoint-validator macronodes and one micronode"]
async fn phase19_multi_node_checkpoint_soak_preserves_determinism_health_and_authority(
) -> Result<()> {
    const SOAK_ROUNDS: usize = 120;

    const SOAK_INTERVAL: Duration = Duration::from_millis(250);

    const VALIDATOR_B_RESTART_ROUND: usize = 60;

    const USER_NODE_RESTART_ROUND: usize = 90;

    let ports = reserve_loopback_ports(22)?;

    let service_a_ports = MacronodePorts {
        admin: ports[0],
        gateway: ports[1],
        storage: ports[2],
        index: ports[3],
        overlay: ports[4],
        dht: ports[5],
        mailbox: ports[6],
    };

    let service_b_ports = MacronodePorts {
        admin: ports[7],
        gateway: ports[8],
        storage: ports[9],
        index: ports[10],
        overlay: ports[11],
        dht: ports[12],
        mailbox: ports[13],
    };

    let service_c_ports = MacronodePorts {
        admin: ports[14],
        gateway: ports[15],
        storage: ports[16],
        index: ports[17],
        overlay: ports[18],
        dht: ports[19],
        mailbox: ports[20],
    };

    let user_port = ports[21];

    let service_a_db = unique_index_db();

    let service_b_db = unique_index_db();

    let service_c_db = unique_index_db();

    ensure!(
        service_a_db != service_b_db
            && service_a_db != service_c_db
            && service_b_db != service_c_db,
        "Phase 19 soak validators require isolated index stores",
    );

    let client = test_client()?;

    let service_a_base = format!("http://127.0.0.1:{}", service_a_ports.admin,);

    let service_b_base = format!("http://127.0.0.1:{}", service_b_ports.admin,);

    let service_c_base = format!("http://127.0.0.1:{}", service_c_ports.admin,);

    let user_base = format!("http://127.0.0.1:{user_port}");

    let admin_token = "phase19-multi-node-soak-admin";

    let validator_set = phase19_checkpoint_validator_set();

    validator_set
        .validate()
        .map_err(|error| anyhow!("Phase 19 soak validator set invalid: {error:?}"))?;

    let baseline_candidate = phase19_checkpoint_candidate(&validator_set);

    let baseline_checkpoint_hash = phase19_checkpoint_hash(&baseline_candidate)?;

    let baseline_state_root = baseline_candidate.new_state_root.clone();

    let baseline_receipt_root = baseline_candidate.receipt_root.clone();

    let baseline_da_root = baseline_candidate.data_availability_root.clone();

    let identity_a = phase19_checkpoint_validator_identity("alpha");

    let identity_b = phase19_checkpoint_validator_identity("beta");

    let identity_c = phase19_checkpoint_validator_identity("gamma");

    let seed_a = phase19_seed(0x41);

    let seed_b = phase19_seed(0x42);

    let seed_c = phase19_seed(0x43);

    let seed_a_hex = phase19_seed_hex(0x41);

    let seed_b_hex = phase19_seed_hex(0x42);

    let seed_c_hex = phase19_seed_hex(0x43);

    let mut service_a = spawn_checkpoint_validator_macronode(
        service_a_ports,
        &service_a_db,
        &identity_a,
        &seed_a_hex,
        admin_token,
    )?;

    let mut service_b = spawn_checkpoint_validator_macronode(
        service_b_ports,
        &service_b_db,
        &identity_b,
        &seed_b_hex,
        admin_token,
    )?;

    let mut service_c = spawn_checkpoint_validator_macronode(
        service_c_ports,
        &service_c_db,
        &identity_c,
        &seed_c_hex,
        admin_token,
    )?;

    let mut user_node = spawn_micronode(user_port)?;

    println!("Phase 19 soak: waiting for three checkpoint validators and one User Node...");

    let service_a_status = wait_for_macronode(&client, &service_a_base).await?;

    let service_b_status = wait_for_macronode(&client, &service_b_base).await?;

    let service_c_status = wait_for_macronode(&client, &service_c_base).await?;

    let user_status = wait_for_micronode(&client, &user_base).await?;

    assert_macronode_truth(&service_a_status)?;

    assert_macronode_truth(&service_b_status)?;

    assert_macronode_truth(&service_c_status)?;

    assert_micronode_truth(&user_status)?;

    service_a.assert_running()?;
    service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let baseline_signature_a = phase19_request_checkpoint_signature(
        &client,
        &service_a_base,
        admin_token,
        &baseline_checkpoint_hash,
        "soak baseline validator A",
    )
    .await?;

    let baseline_signature_b = phase19_request_checkpoint_signature(
        &client,
        &service_b_base,
        admin_token,
        &baseline_checkpoint_hash,
        "soak baseline validator B",
    )
    .await?;

    let baseline_signature_c = phase19_request_checkpoint_signature(
        &client,
        &service_c_base,
        admin_token,
        &baseline_checkpoint_hash,
        "soak baseline validator C",
    )
    .await?;

    phase19_verify_live_checkpoint_signature(
        &baseline_signature_a,
        &identity_a,
        &baseline_checkpoint_hash,
        &seed_a,
    )?;

    phase19_verify_live_checkpoint_signature(
        &baseline_signature_b,
        &identity_b,
        &baseline_checkpoint_hash,
        &seed_b,
    )?;

    phase19_verify_live_checkpoint_signature(
        &baseline_signature_c,
        &identity_c,
        &baseline_checkpoint_hash,
        &seed_c,
    )?;

    ensure!(
        baseline_signature_a.validator_id != baseline_signature_b.validator_id
            && baseline_signature_a.validator_id != baseline_signature_c.validator_id
            && baseline_signature_b.validator_id != baseline_signature_c.validator_id,
        "Phase 19 soak requires three unique validator identities",
    );

    ensure!(
        baseline_signature_a.signature_wire != baseline_signature_b.signature_wire
            && baseline_signature_a.signature_wire != baseline_signature_c.signature_wire
            && baseline_signature_b.signature_wire != baseline_signature_c.signature_wire,
        "distinct checkpoint validators must produce distinct signature evidence",
    );

    let healthy_baseline = phase19_finalized_checkpoint_from_live_signatures(
        &baseline_candidate,
        &baseline_checkpoint_hash,
        &validator_set,
        vec![baseline_signature_a.clone(), baseline_signature_b.clone()],
    );

    healthy_baseline
        .validate()
        .map_err(|error| anyhow!("Phase 19 soak baseline finality must validate: {error:?}"))?;

    let baseline_rss_a = phase19_process_rss_kib(&service_a)?;

    let mut baseline_rss_b = phase19_process_rss_kib(&service_b)?;

    let baseline_rss_c = phase19_process_rss_kib(&service_c)?;

    let mut baseline_rss_user = phase19_process_rss_kib(&user_node)?;

    let mut peak_rss_a = baseline_rss_a;

    let mut peak_rss_b = baseline_rss_b;

    let mut peak_rss_c = baseline_rss_c;

    let mut peak_rss_user = baseline_rss_user;

    println!(
        "Phase 19 soak: baseline RSS KiB A={baseline_rss_a} B={baseline_rss_b} C={baseline_rss_c} User={baseline_rss_user}"
    );

    for round in 1..=SOAK_ROUNDS {
        service_a.assert_running()?;
        service_b.assert_running()?;
        service_c.assert_running()?;
        user_node.assert_running()?;

        let candidate_left = phase19_checkpoint_candidate(&validator_set);

        let candidate_right = phase19_checkpoint_candidate(&validator_set);

        let hash_left = phase19_checkpoint_hash(&candidate_left)?;

        let hash_right = phase19_checkpoint_hash(&candidate_right)?;

        ensure!(
            hash_left == baseline_checkpoint_hash && hash_right == baseline_checkpoint_hash,
            "Phase 19 soak candidate/hash drift detected at round {round}",
        );

        ensure!(
            candidate_left.new_state_root == baseline_state_root
                && candidate_left.receipt_root == baseline_receipt_root
                && candidate_left.data_availability_root == baseline_da_root,
            "Phase 19 soak checkpoint-root drift detected at round {round}",
        );

        let signature_a = phase19_request_checkpoint_signature(
            &client,
            &service_a_base,
            admin_token,
            &hash_left,
            "soak validator A",
        )
        .await?;

        let signature_b = phase19_request_checkpoint_signature(
            &client,
            &service_b_base,
            admin_token,
            &hash_left,
            "soak validator B",
        )
        .await?;

        let signature_c = phase19_request_checkpoint_signature(
            &client,
            &service_c_base,
            admin_token,
            &hash_left,
            "soak validator C",
        )
        .await?;

        phase19_verify_live_checkpoint_signature(&signature_a, &identity_a, &hash_left, &seed_a)?;

        phase19_verify_live_checkpoint_signature(&signature_b, &identity_b, &hash_left, &seed_b)?;

        phase19_verify_live_checkpoint_signature(&signature_c, &identity_c, &hash_left, &seed_c)?;

        ensure!(
            signature_a == baseline_signature_a,
            "validator A checkpoint signature drift detected at soak round {round}",
        );

        ensure!(
            signature_b == baseline_signature_b,
            "validator B checkpoint signature drift detected at soak round {round}",
        );

        ensure!(
            signature_c == baseline_signature_c,
            "validator C checkpoint signature drift detected at soak round {round}",
        );

        let one_signer = phase19_finalized_checkpoint_from_live_signatures(
            &candidate_left,
            &hash_left,
            &validator_set,
            vec![signature_a.clone()],
        );

        ensure!(
            one_signer.validate().is_err(),
            "one validator manufactured phantom finality at soak round {round}",
        );

        let duplicated_signer = phase19_finalized_checkpoint_from_live_signatures(
            &candidate_left,
            &hash_left,
            &validator_set,
            vec![signature_a.clone(), signature_a.clone()],
        );

        ensure!(
            duplicated_signer.validate().is_err(),
            "duplicate validator evidence inflated finality at soak round {round}",
        );

        let two_of_three = phase19_finalized_checkpoint_from_live_signatures(
            &candidate_left,
            &hash_left,
            &validator_set,
            vec![signature_a, signature_b],
        );

        two_of_three.validate().map_err(|error| {
            anyhow!("legitimate two-of-three finality failed at soak round {round}: {error:?}")
        })?;

        if round % 10 == 0 {
            let status_a = get_json(
                &client,
                &format!("{service_a_base}/api/v1/status"),
                "Phase 19 soak validator A",
            )
            .await?;

            let status_b = get_json(
                &client,
                &format!("{service_b_base}/api/v1/status"),
                "Phase 19 soak validator B",
            )
            .await?;

            let status_c = get_json(
                &client,
                &format!("{service_c_base}/api/v1/status"),
                "Phase 19 soak validator C",
            )
            .await?;

            let status_user = get_json(
                &client,
                &format!("{user_base}/api/v1/status"),
                "Phase 19 soak User Node",
            )
            .await?;

            assert_macronode_truth(&status_a)?;

            assert_macronode_truth(&status_b)?;

            assert_macronode_truth(&status_c)?;

            assert_micronode_truth(&status_user)?;

            ensure!(
                status_a["wallet_execution_participant"].as_bool() == Some(false)
                    && status_b["wallet_execution_participant"].as_bool() == Some(false)
                    && status_c["wallet_execution_participant"].as_bool() == Some(false),
                "checkpoint-validator soak must never gain wallet execution authority",
            );

            let rss_a = phase19_process_rss_kib(&service_a)?;

            let rss_b = phase19_process_rss_kib(&service_b)?;

            let rss_c = phase19_process_rss_kib(&service_c)?;

            let rss_user = phase19_process_rss_kib(&user_node)?;

            peak_rss_a = peak_rss_a.max(rss_a);

            peak_rss_b = peak_rss_b.max(rss_b);

            peak_rss_c = peak_rss_c.max(rss_c);

            peak_rss_user = peak_rss_user.max(rss_user);

            phase19_assert_soak_memory_bound("validator A", baseline_rss_a, rss_a)?;

            phase19_assert_soak_memory_bound("validator B", baseline_rss_b, rss_b)?;

            phase19_assert_soak_memory_bound("validator C", baseline_rss_c, rss_c)?;

            phase19_assert_soak_memory_bound("User Node", baseline_rss_user, rss_user)?;

            println!(
                "Phase 19 soak round {round}/{SOAK_ROUNDS}: health/root/signature/finality stable; RSS KiB A={rss_a} B={rss_b} C={rss_c} User={rss_user}"
            );
        }

        if round == VALIDATOR_B_RESTART_ROUND {
            println!("Phase 19 soak: restarting validator B at round {round}...");

            let _ = service_b.kill_and_wait()?;

            wait_for_unreachable(
                &client,
                &format!("{service_b_base}/healthz"),
                "Phase 19 soak validator B restart",
            )
            .await?;

            service_b = spawn_checkpoint_validator_macronode(
                service_b_ports,
                &service_b_db,
                &identity_b,
                &seed_b_hex,
                admin_token,
            )?;

            let restarted_status = wait_for_macronode(&client, &service_b_base).await?;

            assert_macronode_truth(&restarted_status)?;

            service_b.assert_running()?;

            let restarted_signature = phase19_request_checkpoint_signature(
                &client,
                &service_b_base,
                admin_token,
                &baseline_checkpoint_hash,
                "soak restarted validator B",
            )
            .await?;

            phase19_verify_live_checkpoint_signature(
                &restarted_signature,
                &identity_b,
                &baseline_checkpoint_hash,
                &seed_b,
            )?;

            ensure!(
                restarted_signature == baseline_signature_b,
                "validator B identity/signature drifted across soak restart",
            );

            baseline_rss_b = phase19_process_rss_kib(&service_b)?;

            peak_rss_b = baseline_rss_b;

            println!(
                "Phase 19 soak: validator B restarted cleanly with stable identity/signature."
            );
        }

        if round == USER_NODE_RESTART_ROUND {
            println!("Phase 19 soak: restarting User Node at round {round}...");

            let _ = user_node.kill_and_wait()?;

            wait_for_unreachable(
                &client,
                &format!("{user_base}/healthz"),
                "Phase 19 soak User Node restart",
            )
            .await?;

            user_node = spawn_micronode(user_port)?;

            let restarted_status = wait_for_micronode(&client, &user_base).await?;

            assert_micronode_truth(&restarted_status)?;

            user_node.assert_running()?;

            baseline_rss_user = phase19_process_rss_kib(&user_node)?;

            peak_rss_user = baseline_rss_user;

            println!("Phase 19 soak: User Node restarted cleanly.");
        }

        sleep(SOAK_INTERVAL).await;
    }

    service_a.assert_running()?;
    service_b.assert_running()?;
    service_c.assert_running()?;
    user_node.assert_running()?;

    let final_candidate = phase19_checkpoint_candidate(&validator_set);

    let final_hash = phase19_checkpoint_hash(&final_candidate)?;

    ensure!(
        final_hash == baseline_checkpoint_hash,
        "Phase 19 soak ended with candidate/hash drift",
    );

    ensure!(
        final_candidate.new_state_root == baseline_state_root
            && final_candidate.receipt_root == baseline_receipt_root
            && final_candidate.data_availability_root == baseline_da_root,
        "Phase 19 soak ended with root drift",
    );

    println!(
        "Phase 19 soak passed {SOAK_ROUNDS} rounds: process health remained green; checkpoint roots and candidate hash converged without drift; three validator identities/signatures remained unique and deterministic; one signer and duplicate signer weight never produced finality; valid two-of-three finality remained available; validator and User Node restarts recovered; and RSS remained bounded. Peak RSS KiB A={peak_rss_a} B={peak_rss_b} C={peak_rss_c} User={peak_rss_user}."
    );

    let _ = service_a.kill_and_wait()?;

    let _ = service_b.kill_and_wait()?;

    let _ = service_c.kill_and_wait()?;

    let _ = user_node.kill_and_wait()?;

    let _ = fs::remove_dir_all(&service_a_db);

    let _ = fs::remove_dir_all(&service_b_db);

    let _ = fs::remove_dir_all(&service_c_db);

    Ok(())
}
