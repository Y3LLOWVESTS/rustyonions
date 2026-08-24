//! RO:WHAT — Integration tests for CrabNode bootstrap config show/validate.
//! RO:WHY — CN-1 requires truthful config inspection before managed lifecycle.
//! RO:INTERACTS — crabnode binary, state.rs, CRABNODE_HOME test isolation.
//! RO:INVARIANTS — strict schema; unsafe/unknown fields reject; invalid contents never echoed.
//! RO:SECURITY — test secret markers must never appear in command output.
//! RO:TEST — cargo test -p macronode --test crabnode_config_cli.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
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

        Self {
            path: std::env::temp_dir().join(format!(
                "crabnode-cn1c-{label}-{}-{nonce}",
                std::process::id()
            )),
        }
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

fn run(home: &TestHome, args: &[&str]) -> Output {
    Command::new(crabnode_bin())
        .env("CRABNODE_HOME", home.path())
        .args(args)
        .output()
        .expect("crabnode command should execute")
}

fn initialize(home: &TestHome) {
    let output = run(home, &["init"]);

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn initialized_config_show_is_validated_and_bounded_to_bootstrap_fields() {
    let home = TestHome::new("show");
    initialize(&home);

    let output = run(&home, &["config", "show"]);

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("schema_version = 1"));
    assert!(stdout.contains("profile = \"private_beta\""));
    assert!(stdout.contains("[operator]"));
    assert!(stdout.contains("headless = true"));
    assert!(stdout.contains("admin_ui_required = false"));
    assert!(stdout.contains("[security]"));
    assert!(stdout.contains("admin_loopback_only = true"));
}

#[test]
fn initialized_config_validate_succeeds_without_starting_runtime() {
    let home = TestHome::new("validate");
    initialize(&home);

    let output = run(&home, &["config", "validate"]);

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("crabnode config validate: OK"));
}

#[test]
fn unknown_config_field_fails_closed_without_echoing_secret_value() {
    let home = TestHome::new("unknown");
    initialize(&home);

    let path = home.path().join("config/crabnode.toml");
    let secret_marker = "DO_NOT_PRINT_THIS_ADMIN_TOKEN";

    let mut config = fs::read_to_string(&path).expect("initialized config should exist");

    config.push_str(&format!(
        "\nunsupported_admin_token = \"{secret_marker}\"\n"
    ));

    fs::write(&path, config).expect("test config mutation");

    let output = run(&home, &["config", "validate"]);

    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stdout.contains(secret_marker));
    assert!(!stderr.contains(secret_marker));
    assert!(stderr.contains("invalid or contains unsupported fields"));
}

#[test]
fn unsafe_bootstrap_posture_fails_closed() {
    let home = TestHome::new("unsafe");
    initialize(&home);

    let path = home.path().join("config/crabnode.toml");

    let unsafe_config = r#"schema_version = 1
profile = "private_beta"

[operator]
headless = false
admin_ui_required = true

[security]
admin_loopback_only = false
"#;

    fs::write(&path, unsafe_config).expect("unsafe test config");

    let output = run(&home, &["config", "validate"]);

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("operator.headless must remain true"));
}

#[test]
fn missing_config_fails_with_init_guidance() {
    let home = TestHome::new("missing");

    let output = run(&home, &["config", "validate"]);

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("run `crabnode init` first"));
}
