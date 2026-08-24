//! RO:WHAT — Integration proof for managed CrabNode start/stop/restart and stale-PID safety.
//! RO:WHY — CN-1 requires ordinary operator lifecycle without unsafe PID-only authority.
//! RO:INTERACTS — crabnode runtime.rs, real macronode child, sysinfo, isolated service ports.
//! RO:INVARIANTS — restart preserves ID; repeated start/stop are idempotent; stale PID is harmless.
//! RO:SECURITY — unrelated live process survives even when PID/executable/start-time are copied.
//! RO:TEST — cargo test -p macronode --test crabnode_lifecycle_cli.

use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use sysinfo::{Pid, System};

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

fn macronode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_macronode")
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
                "crabnode-cn1d-{label}-{}-{nonce}",
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
            .expect("crabnode lifecycle command should execute")
    }

    fn initialize(&self) {
        let output = self.output(&["init"]);

        assert!(
            output.status.success(),
            "init stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn spawn_unrelated_macronode(&self) -> Child {
        let mut command = Command::new(macronode_bin());

        self.apply_runtime_env(&mut command);

        command
            .arg("run")
            .env_remove("RON_CRABNODE_INSTANCE_ID")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("unrelated macronode should spawn")
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
            .env("RUST_LOG", "warn")
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
        let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral test port should bind");

        ports.push(listener.local_addr().expect("test local addr").port());

        listeners.push(listener);
    }

    let ports: [u16; 7] = ports.try_into().expect("exactly seven ports");

    drop(listeners);

    ports
}

#[test]
fn managed_start_stop_restart_is_real_idempotent_and_preserves_node_id() {
    let node = TestNode::new("managed");
    node.initialize();

    let node_id_path = node.home().join("data/node-id");

    let first_node_id = fs::read_to_string(&node_id_path).expect("initial node id");

    let start = node.output(&["start"]);

    assert!(
        start.status.success(),
        "start stderr={}",
        String::from_utf8_lossy(&start.stderr)
    );

    assert!(String::from_utf8_lossy(&start.stdout).contains("CrabNode started pid="));

    let second_start = node.output(&["start"]);

    assert!(
        second_start.status.success(),
        "second start stderr={}",
        String::from_utf8_lossy(&second_start.stderr)
    );

    assert!(String::from_utf8_lossy(&second_start.stdout).contains("already running"));

    let ready = node.output(&["ready"]);

    assert!(
        ready.status.success(),
        "ready stderr={}",
        String::from_utf8_lossy(&ready.stderr)
    );

    assert!(String::from_utf8_lossy(&ready.stdout).contains("\"ready\":true"));

    let restart = node.output(&["restart"]);

    assert!(
        restart.status.success(),
        "restart stderr={}",
        String::from_utf8_lossy(&restart.stderr)
    );

    let second_node_id = fs::read_to_string(&node_id_path).expect("post-restart node id");

    assert_eq!(first_node_id, second_node_id);

    let stop = node.output(&["stop"]);

    assert!(
        stop.status.success(),
        "stop stderr={}",
        String::from_utf8_lossy(&stop.stderr)
    );

    let second_stop = node.output(&["stop"]);

    assert!(
        second_stop.status.success(),
        "second stop stderr={}",
        String::from_utf8_lossy(&second_stop.stderr)
    );

    assert!(String::from_utf8_lossy(&second_stop.stdout).contains("already stopped"));

    assert!(!node.home().join("run/crabnode-process.json").exists());
}

#[test]
fn stale_pid_with_matching_executable_and_start_time_is_never_signaled() {
    let node = TestNode::new("stale-pid");

    node.initialize();

    let mut unrelated = node.spawn_unrelated_macronode();

    thread::sleep(Duration::from_millis(300));

    assert!(
        unrelated
            .try_wait()
            .expect("inspect unrelated process")
            .is_none(),
        "unrelated macronode must be live before stale-PID test"
    );

    let unrelated_pid = unrelated.id();

    let system = System::new_all();

    let process = system
        .process(Pid::from_u32(unrelated_pid))
        .expect("unrelated process visible to sysinfo");

    let actual_start_time = process.start_time();

    let executable = fs::canonicalize(macronode_bin()).expect("canonical macronode executable");

    let node_id = fs::read_to_string(node.home().join("data/node-id")).expect("node id");

    let fake_instance_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let fake_state = serde_json::json!({
        "schema_version": 1,
        "pid": unrelated_pid,
        "process_start_time": actual_start_time,
        "executable": executable
            .to_str()
            .expect("UTF-8 executable"),
        "instance_id": fake_instance_id,
        "node_id": node_id.trim(),
        "admin_addr": node.admin_addr(),
    });

    let state_path = node.home().join("run/crabnode-process.json");

    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&fake_state).expect("serialize fake state"),
    )
    .expect("write fake state");

    let stop = node.output(&["stop"]);

    let unrelated_survived = unrelated
        .try_wait()
        .expect("inspect unrelated process after stop")
        .is_none();

    let _ = unrelated.kill();
    let _ = unrelated.wait();

    assert!(
        stop.status.success(),
        "stale stop stderr={}",
        String::from_utf8_lossy(&stop.stderr)
    );

    assert!(String::from_utf8_lossy(&stop.stdout).contains("was not signaled"));

    assert!(
        unrelated_survived,
        "stale process state must never terminate an unrelated live process"
    );

    assert!(
        !state_path.exists(),
        "stale state should be recovered after proving the process is unrelated"
    );
}

#[test]
fn managed_lifecycle_dry_runs_are_non_mutating() {
    let node = TestNode::new("dry-run");

    node.initialize();

    for action in ["start", "stop", "restart"] {
        let output = node.output(&["--dry-run", action]);

        assert!(
            output.status.success(),
            "{action} dry-run stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert!(!node.home().join("run/crabnode-process.json").exists());
}
