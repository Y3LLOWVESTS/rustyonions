//! RO:WHAT — Integration tests for crabnode exact-b3 local policy-file controls.
//! RO:WHY — BUILD_PLAN_Z Phase 10 requires CLI-first operator moderation without fake pruning.
//! RO:INTERACTS — crabnode binary, ron-policy, svc-storage bounded policy loader.
//! RO:INVARIANTS — strict b3 IDs; atomic canonical JSON; stronger refusal states retain precedence.
//! RO:SECURITY — no storage delete, provider withdrawal, wallet/ledger mutation, or reward finality.
//! RO:TEST — cargo test -p macronode --test crabnode_policy_cli.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use ron_policy::{B3Id, ModerationPolicy, ModerationReasonCode};
use serde_json::Value;

const OBJECT: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

struct PolicyPath {
    path: PathBuf,
}

impl PolicyPath {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "crabnode-policy-{label}-{}-{nonce}.json",
            std::process::id()
        ));

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, policy: &ModerationPolicy) {
        let bytes = serde_json::to_vec_pretty(policy).expect("policy should serialize");

        fs::write(&self.path, bytes).expect("policy fixture should write");
    }

    fn read(&self) -> ModerationPolicy {
        let bytes = fs::read(&self.path).expect("policy file should exist");

        serde_json::from_slice(&bytes).expect("policy file should remain canonical")
    }
}

impl Drop for PolicyPath {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn run(policy: &PolicyPath, args: &[&str]) -> Output {
    Command::new(crabnode_bin())
        .arg("--policy-file")
        .arg(policy.path())
        .args(args)
        .output()
        .expect("crabnode policy command should run")
}

fn stdout_json(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("stdout should be JSON")
}

#[test]
fn block_status_and_unblock_round_trip_canonical_policy() {
    let policy_path = PolicyPath::new("block-round-trip");
    let object: B3Id = OBJECT.parse().expect("OBJECT should be canonical");

    let blocked = stdout_json(&run(&policy_path, &["block", OBJECT]));

    assert_eq!(blocked["action"], "block");
    assert_eq!(blocked["changed"], true);
    assert_eq!(blocked["created_policy_file"], true);
    assert_eq!(blocked["restart_required"], true);
    assert_eq!(blocked["runtime_hot_reload"], false);
    assert_eq!(blocked["effective"]["permits_serve"], false);
    assert_eq!(blocked["effective"]["reason"], "local_block");
    assert_eq!(blocked["storage_delete"], false);
    assert_eq!(blocked["provider_withdrawal"], false);
    assert_eq!(blocked["wallet_mutation"], false);
    assert_eq!(blocked["ledger_mutation"], false);
    assert_eq!(blocked["reward_finality"], false);

    let policy = policy_path.read();

    assert_eq!(
        policy.evaluate(&object).reason,
        ModerationReasonCode::LocalBlock
    );

    let status = stdout_json(&run(&policy_path, &["policy", "status"]));

    assert_eq!(status["state"], "loaded");
    assert_eq!(status["entries"]["local_block"], 1);
    assert_eq!(status["entries"]["total"], 1);
    assert_eq!(status["activation"], "startup_snapshot");
    assert_eq!(status["hot_reload"], false);

    let unblocked = stdout_json(&run(&policy_path, &["unblock", OBJECT]));

    assert_eq!(unblocked["action"], "unblock");
    assert_eq!(unblocked["changed"], true);
    assert_eq!(unblocked["effective"]["permits_serve"], true);
    assert_eq!(unblocked["effective"]["reason"], "no_rule");

    let policy = policy_path.read();

    assert_eq!(
        policy.evaluate(&object).reason,
        ModerationReasonCode::NoRule
    );
}

#[test]
fn local_allow_cannot_override_global_deny() {
    let policy_path = PolicyPath::new("allow-precedence");
    let object: B3Id = OBJECT.parse().expect("OBJECT should be canonical");

    let mut policy = ModerationPolicy::default();
    assert!(policy.insert_global_deny(object.clone()));
    policy_path.write(&policy);

    let allowed = stdout_json(&run(&policy_path, &["allow", OBJECT]));

    assert_eq!(allowed["changed"], true);
    assert_eq!(allowed["effective"]["permits_serve"], false);
    assert_eq!(allowed["effective"]["reason"], "global_deny");
    assert_eq!(allowed["entries"]["global_deny"], 1);
    assert_eq!(allowed["entries"]["local_allow"], 1);

    let policy = policy_path.read();

    assert_eq!(
        policy.evaluate(&object).reason,
        ModerationReasonCode::GlobalDeny
    );

    let removed = stdout_json(&run(&policy_path, &["remove-allow", OBJECT]));

    assert_eq!(removed["changed"], true);
    assert_eq!(removed["entries"]["local_allow"], 0);
    assert_eq!(removed["effective"]["reason"], "global_deny");
}

#[test]
fn quarantine_and_release_use_canonical_policy_state() {
    let policy_path = PolicyPath::new("quarantine-round-trip");
    let object: B3Id = OBJECT.parse().expect("OBJECT should be canonical");

    let quarantined = stdout_json(&run(&policy_path, &["quarantine", OBJECT]));

    assert_eq!(quarantined["effective"]["reason"], "quarantined");
    assert_eq!(quarantined["effective"]["permits_serve"], false);

    let released = stdout_json(&run(&policy_path, &["release-quarantine", OBJECT]));

    assert_eq!(released["changed"], true);
    assert_eq!(released["effective"]["reason"], "no_rule");
    assert_eq!(released["effective"]["permits_serve"], true);

    let policy = policy_path.read();

    assert_eq!(
        policy.evaluate(&object).reason,
        ModerationReasonCode::NoRule
    );
}

#[test]
fn dry_run_and_invalid_ids_do_not_create_policy_files() {
    let dry_run_path = PolicyPath::new("dry-run");

    let dry_run = Command::new(crabnode_bin())
        .arg("--policy-file")
        .arg(dry_run_path.path())
        .args(["--dry-run", "block", OBJECT])
        .output()
        .expect("crabnode dry-run should run");

    assert!(dry_run.status.success());
    assert!(!dry_run_path.path().exists());

    let stdout = String::from_utf8_lossy(&dry_run.stdout);

    assert!(stdout.contains("would update"));
    assert!(stdout.contains("restart required"));
    assert!(stdout.contains("no storage deletion"));

    let invalid_path = PolicyPath::new("invalid-id");
    let invalid = run(&invalid_path, &["block", "not-a-b3"]);

    assert!(!invalid.status.success());
    assert!(!invalid_path.path().exists());

    let stderr = String::from_utf8_lossy(&invalid.stderr);

    assert!(stderr.contains("expected b3:<64 lowercase hexadecimal characters>"));
}

#[test]
fn invalid_existing_policy_fails_closed_without_rewrite() {
    let policy_path = PolicyPath::new("invalid-existing");

    let original = br#"{"local_block":["not-a-b3"]}"#;

    fs::write(policy_path.path(), original).expect("invalid fixture should write");

    let output = run(&policy_path, &["block", OBJECT]);

    assert!(!output.status.success());

    assert_eq!(
        fs::read(policy_path.path()).expect("invalid fixture should remain"),
        original
    );

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("configured moderation policy was rejected"));
}
