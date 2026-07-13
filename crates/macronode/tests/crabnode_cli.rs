//! RO:WHAT — Integration tests for the crabnode headless operator CLI skeleton.
//! RO:WHY — BUILD_PLAN_Z requires CLI status and truthful moderation controls without mandatory UI.
//! RO:INTERACTS — macronode/src/bin/crabnode.rs, loopback admin HTTP surfaces.
//! RO:INVARIANTS — no fake admin mutation success; status can query loopback admin JSON.
//! RO:TEST — cargo test -p macronode --test crabnode_cli.

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::Duration,
};

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

#[test]
fn crabnode_help_lists_required_phase3_commands() {
    let output = Command::new(crabnode_bin())
        .arg("--help")
        .output()
        .expect("crabnode --help should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("crabnode"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("admin enable-web"));
    assert!(stdout.contains("admin disable-web"));
    assert!(stdout.contains("admin setup-token"));
    assert!(stdout.contains("admin create-user"));
    assert!(stdout.contains("rewards show"));
    assert!(stdout.contains("rewards bind @operator"));
    assert!(stdout.contains("rewards rotate @new"));
    assert!(stdout.contains("policy status"));
    assert!(stdout.contains("block b3:<hash>"));
    assert!(stdout.contains("unblock b3:<hash>"));
    assert!(stdout.contains("allow b3:<hash>"));
    assert!(stdout.contains("remove-allow b3:<hash>"));
    assert!(stdout.contains("quarantine b3:<hash>"));
    assert!(stdout.contains("release-quarantine b3:<hash>"));
    assert!(stdout.contains("prune b3:<hash>"));
    assert!(stdout.contains("Delete exact local bytes"));
    assert!(!stdout.contains("prune is not available"));
    assert!(!stdout.contains("reset-password"));
}

#[test]
fn crabnode_status_queries_loopback_admin_without_ui() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake node admin listener");
    let addr = listener.local_addr().expect("listener local addr");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("fake admin accepts request");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fake read timeout");

        let mut buf = [0_u8; 1024];
        let n = socket.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let body = r#"{"node_role":"service_node","headless_mode":true,"admin_ui_enabled":false}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        socket
            .write_all(response.as_bytes())
            .expect("write fake response");

        request
    });

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &format!("http://{addr}"), "status"])
        .output()
        .expect("crabnode status should run");

    let request = handle.join().expect("fake admin thread joins");

    assert!(output.status.success());
    assert!(request.starts_with("GET /api/v1/status HTTP/1.1"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""node_role":"service_node""#));
    assert!(stdout.contains(r#""headless_mode":true"#));
    assert!(stdout.contains(r#""admin_ui_enabled":false"#));
}

#[test]
fn crabnode_setup_token_posts_to_local_admin_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake node admin listener");
    let addr = listener.local_addr().expect("listener local addr");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("fake admin accepts request");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fake read timeout");

        let mut buf = [0_u8; 1024];
        let n = socket.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let body = r#"{"status":"setup token issued","token":"abc"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );

        socket
            .write_all(response.as_bytes())
            .expect("write fake response");

        request
    });

    let output = Command::new(crabnode_bin())
        .args([
            "--admin-url",
            &format!("http://{addr}"),
            "admin",
            "setup-token",
        ])
        .output()
        .expect("crabnode admin setup-token should run");

    let request = handle.join().expect("fake admin thread joins");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/admin/setup-token HTTP/1.1"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""status":"setup token issued""#));
}

#[test]
fn crabnode_admin_dry_run_is_explicitly_non_mutating() {
    let output = Command::new(crabnode_bin())
        .args(["--dry-run", "admin", "disable-web"])
        .output()
        .expect("crabnode admin disable-web --dry-run should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("would POST"));
    assert!(stdout.contains("/api/v1/admin/ui/disable"));
}
