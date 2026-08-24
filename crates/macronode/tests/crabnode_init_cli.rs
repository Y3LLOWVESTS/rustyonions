//! RO:WHAT — Integration tests for idempotent CrabNode local-state initialization.
//! RO:WHY — CN-1 requires fresh/repeated init and identity preservation before daemon lifecycle.
//! RO:INTERACTS — crabnode binary and crabnode/state.rs through CRABNODE_HOME.
//! RO:INVARIANTS — isolated test home; existing config/identity never overwritten; corrupt ID fails closed.
//! RO:SECURITY — tests create no private key, Passport secret, admin token, or economic authority.
//! RO:TEST — cargo test -p macronode --test crabnode_init_cli.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

struct TestHome {
    path: PathBuf,
}

impl TestHome {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "crabnode-cn1b-{label}-{}-{nonce}",
            std::process::id()
        ));

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_init(home: &TestHome) -> std::process::Output {
    Command::new(crabnode_bin())
        .env("CRABNODE_HOME", home.path())
        .arg("init")
        .output()
        .expect("crabnode init should execute")
}

#[test]
fn fresh_init_creates_required_local_operator_state() {
    let home = TestHome::new("fresh");

    let output = run_init(&home);

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    for relative in ["config", "data", "log", "run"] {
        assert!(
            home.path().join(relative).is_dir(),
            "missing required directory {relative}"
        );
    }

    let config_path = home.path().join("config/crabnode.toml");
    let node_id_path = home.path().join("data/node-id");

    assert!(config_path.is_file());
    assert!(node_id_path.is_file());

    let config = fs::read_to_string(config_path).expect("config should be readable");

    assert!(config.contains("schema_version = 1"));
    assert!(config.contains("profile = \"private_beta\""));
    assert!(config.contains("headless = true"));
    assert!(config.contains("admin_ui_required = false"));
    assert!(config.contains("admin_loopback_only = true"));

    let node_id = fs::read_to_string(node_id_path).expect("node id should be readable");
    let node_id = node_id.trim();

    assert_eq!(node_id.len(), 64);
    assert!(node_id
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}

#[test]
fn repeated_init_preserves_node_identity_and_existing_config() {
    let home = TestHome::new("repeat");

    let first = run_init(&home);
    assert!(first.status.success());

    let config_path = home.path().join("config/crabnode.toml");
    let node_id_path = home.path().join("data/node-id");

    let first_node_id = fs::read_to_string(&node_id_path).expect("first node id");

    let custom_config = "# operator-owned test config\ncustom_marker = \"preserve-me\"\n";

    fs::write(&config_path, custom_config)
        .expect("test should replace config before repeated init");

    let second = run_init(&home);

    assert!(
        second.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );

    let second_node_id = fs::read_to_string(&node_id_path).expect("second node id");
    let second_config = fs::read_to_string(&config_path).expect("second config");

    assert_eq!(first_node_id, second_node_id);
    assert_eq!(second_config, custom_config);

    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(stdout.contains("(preserved)"));
}

#[test]
fn malformed_existing_node_identity_fails_closed_without_replacement() {
    let home = TestHome::new("corrupt");

    fs::create_dir_all(home.path().join("data")).expect("test data directory");

    let node_id_path = home.path().join("data/node-id");
    fs::write(&node_id_path, "not-a-valid-node-id\n").expect("test malformed node id");

    let output = run_init(&home);

    assert!(!output.status.success());

    let retained = fs::read_to_string(node_id_path).expect("malformed ID retained");
    assert_eq!(retained, "not-a-valid-node-id\n");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("malformed"));
    assert!(stderr.contains("refusing to replace"));
}

#[test]
fn init_dry_run_creates_no_state() {
    let home = TestHome::new("dry-run");

    let output = Command::new(crabnode_bin())
        .env("CRABNODE_HOME", home.path())
        .args(["--dry-run", "init"])
        .output()
        .expect("crabnode init dry-run should execute");

    assert!(output.status.success());
    assert!(!home.path().exists());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no directories or files created"));
}
