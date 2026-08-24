//! RO:WHAT — Integration tests for CN-1 CrabNode logs and lifecycle doctor commands.
//! RO:WHY — Public lifecycle closure requires bounded logs and truthful local/runtime diagnostics.
//! RO:INTERACTS — crabnode runtime/state, real isolated macronode child, CRABNODE_HOME.
//! RO:INVARIANTS — bounded reads; secret values redact; doctor never invents CN-2 topology truth.
//! RO:SECURITY — known secret environment values must not appear in log output.
//! RO:TEST — cargo test -p macronode --test crabnode_operator_cli.

use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

struct TestNode {
    home: PathBuf,
    ports: [u16; 7],
}

impl TestNode {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();

        Self {
            home: std::env::temp_dir().join(format!(
                "crabnode-cn1e-{label}-{}-{nonce}",
                std::process::id()
            )),
            ports: reserve_ports(),
        }
    }

    fn home(&self) -> &Path {
        &self.home
    }

    fn admin_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.ports[0])
    }

    fn admin_addr(&self) -> String {
        format!("127.0.0.1:{}", self.ports[0])
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(crabnode_bin());

        self.apply_runtime_env(&mut command);

        command.arg("--admin-url").arg(self.admin_url()).args(args);

        command
    }

    fn output(&self, args: &[&str]) -> Output {
        self.command(args)
            .output()
            .expect("crabnode operator command should execute")
    }

    fn initialize(&self) {
        let output = self.output(&["init"]);

        assert!(
            output.status.success(),
            "init stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn apply_runtime_env(&self, command: &mut Command) {
        let index_db = self.home.join("data/index");

        command
            .env("CRABNODE_HOME", &self.home)
            .env("RON_HTTP_ADDR", self.admin_addr())
            .env("RON_GATEWAY_ADDR", format!("127.0.0.1:{}", self.ports[1]))
            .env("RON_STORAGE_ADDR", format!("127.0.0.1:{}", self.ports[2]))
            .env("INDEX_BIND", format!("127.0.0.1:{}", self.ports[3]))
            .env("RON_OVERLAY_ADDR", format!("127.0.0.1:{}", self.ports[4]))
            .env("RON_DHT_ADDR", format!("127.0.0.1:{}", self.ports[5]))
            .env("RON_MAILBOX_ADDR", format!("127.0.0.1:{}", self.ports[6]))
            .env("RON_INDEX_DB", &index_db)
            .env("INDEX_DB", &index_db)
            .env("RON_HEADLESS_MODE", "true")
            .env("RON_ADMIN_UI_ENABLED", "false")
            .env("RON_ADMIN_UI_RUNTIME_REQUIRED", "false")
            .env("RUST_LOG", "info")
            .env_remove("CRABNODE_ADMIN_TOKEN")
            .env_remove("RON_ADMIN_TOKEN");
    }
}

impl Drop for TestNode {
    fn drop(&mut self) {
        let _ = self.output(&["stop"]);
        let _ = fs::remove_dir_all(&self.home);
    }
}

fn reserve_ports() -> [u16; 7] {
    let mut listeners = Vec::new();
    let mut ports = Vec::new();

    for _ in 0..7 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral port should bind");

        ports.push(listener.local_addr().expect("local addr").port());

        listeners.push(listener);
    }

    let ports: [u16; 7] = ports.try_into().expect("exactly seven ports");

    drop(listeners);

    ports
}

#[test]
fn managed_start_routes_real_output_and_running_doctor_verifies_identity() {
    let node = TestNode::new("running-doctor");

    node.initialize();

    let start = node.output(&["start"]);

    assert!(
        start.status.success(),
        "start stderr={}",
        String::from_utf8_lossy(&start.stderr)
    );

    let log_path = node.home().join("log/crabnode.log");

    assert!(
        log_path.is_file(),
        "managed start must create the real runtime log"
    );

    let doctor = node.output(&["doctor"]);

    assert!(
        doctor.status.success(),
        "doctor stderr={}",
        String::from_utf8_lossy(&doctor.stderr)
    );

    let doctor_stdout = String::from_utf8_lossy(&doctor.stdout);

    assert!(doctor_stdout.contains("managed_runtime=running"));

    assert!(doctor_stdout.contains("managed_process_identity=verified"));

    assert!(doctor_stdout.contains("admin_identity=verified"));

    assert!(doctor_stdout.contains("cn2_profile=private_beta"));

    let logs = node.output(&["logs"]);

    assert!(
        logs.status.success(),
        "logs stderr={}",
        String::from_utf8_lossy(&logs.stderr)
    );

    assert!(String::from_utf8_lossy(&logs.stdout).contains("bounded_tail"));

    let stop = node.output(&["stop"]);

    assert!(
        stop.status.success(),
        "stop stderr={}",
        String::from_utf8_lossy(&stop.stderr)
    );
}

#[test]
fn logs_are_line_and_byte_bounded_and_redact_known_secret_values() {
    let node = TestNode::new("bounded-logs");

    node.initialize();

    let secret = "CN1E_ADMIN_SECRET_VALUE";

    let mut log = String::new();

    for index in 0..260 {
        log.push_str(&format!("runtime-line-{index:03}\n"));
    }

    log.push_str(&format!("RON_ADMIN_TOKEN={secret}\n"));

    fs::write(node.home().join("log/crabnode.log"), log).expect("write test log");

    let output = node
        .command(&["logs"])
        .env("RON_ADMIN_TOKEN", secret)
        .output()
        .expect("logs command");

    assert!(
        output.status.success(),
        "logs stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("bounded_tail"));

    assert!(stdout.contains("runtime-line-259"));

    assert!(!stdout.contains("runtime-line-000"));

    assert!(!stdout.contains(secret));
    assert!(stdout.contains("[REDACTED]"));

    let runtime_lines = stdout
        .lines()
        .filter(|line| line.starts_with("runtime-line-"))
        .count();

    assert!(runtime_lines <= 200);
}

#[test]
fn doctor_accepts_valid_initialized_state_while_runtime_is_stopped() {
    let node = TestNode::new("doctor-stopped");

    node.initialize();

    let output = node.output(&["doctor"]);

    assert!(
        output.status.success(),
        "doctor stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("local_state=ok"));

    assert!(stdout.contains("bootstrap_config=valid"));

    assert!(stdout.contains("node_identity=valid"));

    assert!(stdout.contains("managed_runtime=stopped"));

    assert!(stdout.contains("cn2_profile=private_beta"));

    assert!(stdout.contains("doctor_result=GREEN"));
}

#[test]
fn doctor_fails_closed_for_corrupt_node_identity() {
    let node = TestNode::new("doctor-corrupt");

    node.initialize();

    fs::write(node.home().join("data/node-id"), "corrupt-node-id\n")
        .expect("write corrupt node identity");

    let output = node.output(&["doctor"]);

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("malformed"));
    assert!(stderr.contains("refusing"));
}

#[test]
fn logs_allow_initialized_node_without_existing_log_file() {
    let node = TestNode::new("logs-empty");

    node.initialize();

    let output = node.output(&["logs"]);

    assert!(
        output.status.success(),
        "logs stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(String::from_utf8_lossy(&output.stdout).contains("no managed runtime log recorded yet"));
}
