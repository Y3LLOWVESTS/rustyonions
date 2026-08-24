//! RO:WHAT — CN-2 canonical CrabNode profile CLI acceptance tests.
//! RO:WHY — Doctor must reject collisions, unsafe exposure, invalid binds, and bad local permissions.
//! RO:INTERACTS — crabnode init/doctor and profile.rs.
//! RO:INVARIANTS — one ingress, private admin, collision-free canonical topology.
//! RO:SECURITY — unsafe binds and permissive state fail closed.
//! RO:TEST — cargo test -p macronode --test crabnode_profile_cli.

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
            .expect("clock after epoch")
            .as_nanos();

        Self {
            path: std::env::temp_dir().join(format!(
                "crabnode-cn2b-{label}-{}-{nonce}",
                std::process::id()
            )),
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(crabnode_bin());

        command
            .env("CRABNODE_HOME", &self.path)
            .env_remove("RON_GATEWAY_ADDR")
            .env_remove("SVC_GATEWAY_BIND_ADDR")
            .env_remove("OMNIGATE_BIND")
            .env_remove("RON_OVERLAY_ADDR")
            .env_remove("RON_DHT_ADDR")
            .env_remove("RON_STORAGE_ADDR")
            .env_remove("INDEX_BIND")
            .env_remove("RON_MAILBOX_ADDR")
            .env_remove("RON_PASSPORT_ADDR")
            .args(args);

        command
    }

    fn output(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("crabnode command")
    }

    fn initialize(&self) {
        let output = self.output(&["init"]);

        assert!(
            output.status.success(),
            "init stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn stopped_doctor_reports_cn2_contract() {
    let home = TestHome::new("canonical");

    home.initialize();

    let output = home.output(&["doctor"]);

    assert!(
        output.status.success(),
        "doctor stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    for required in [
        "cn2_profile=private_beta",
        "cn2_port_map=collision_free",
        "cn2_public_admin_default=no",
        "cn2_single_client_ingress=yes",
        "client_ingress_bind=127.0.0.1:8090",
        "omnigate_bind=127.0.0.1:9090",
        "passport_bind=127.0.0.1:5307",
        "passport_runtime_owner=embedded_svc-passport_profile_only",
        "client_ingress_active_owner=svc-gateway",
        "client_ingress_route_surface=full_product_router",
        "client_ingress_handoff=complete_cn3",
        "operator_state_permissions=verified",
        "doctor_result=GREEN",
    ] {
        assert!(stdout.contains(required), "missing {required}: {stdout}");
    }
}

#[test]
fn doctor_rejects_public_admin() {
    let home = TestHome::new("public-admin");

    home.initialize();

    let output = home
        .command(&[
            "--admin-url",
            "http://0.0.0.0:8080",
            "--allow-non-loopback",
            "doctor",
        ])
        .output()
        .expect("doctor");

    assert!(!output.status.success());

    assert!(String::from_utf8_lossy(&output.stderr).contains("loopback-only"));
}

#[test]
fn doctor_rejects_invalid_internal_bind() {
    let home = TestHome::new("bad-bind");

    home.initialize();

    let output = home
        .command(&["doctor"])
        .env("RON_STORAGE_ADDR", "not-a-socket")
        .output()
        .expect("doctor");

    assert!(!output.status.success());

    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid CrabNode storage bind"));
}

#[test]
fn doctor_rejects_port_collision() {
    let home = TestHome::new("collision");

    home.initialize();

    let output = home
        .command(&["doctor"])
        .env("RON_STORAGE_ADDR", "127.0.0.1:5302")
        .output()
        .expect("doctor");

    assert!(!output.status.success());

    assert!(String::from_utf8_lossy(&output.stderr).contains("port collision"));
}

#[test]
fn doctor_rejects_divergent_gateway_configuration() {
    let home = TestHome::new("gateway");

    home.initialize();

    let output = home
        .command(&["doctor"])
        .env("RON_GATEWAY_ADDR", "127.0.0.1:8090")
        .env("SVC_GATEWAY_BIND_ADDR", "127.0.0.1:18090")
        .output()
        .expect("doctor");

    assert!(!output.status.success());

    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("unsupported CrabNode profile combination"));
}

#[cfg(unix)]
#[test]
fn doctor_rejects_permissive_operator_state() {
    use std::os::unix::fs::PermissionsExt;

    let home = TestHome::new("permissions");

    home.initialize();

    fs::set_permissions(home.path().join("log"), fs::Permissions::from_mode(0o755))
        .expect("permissions");

    let output = home.output(&["doctor"]);

    assert!(!output.status.success());

    assert!(String::from_utf8_lossy(&output.stderr).contains("too permissive"));
}
