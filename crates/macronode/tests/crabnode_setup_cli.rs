//! RO:WHAT — Integration tests for crabnode -> svc-admin first-run setup wiring.
//! RO:WHY — Prove create-user calls the real setup endpoint and never accepts secrets as args.
//! RO:INVARIANTS — loopback-only by default; no fake success; password/token supplied by env.

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
fn crabnode_create_user_requires_env_secrets() {
    let output = Command::new(crabnode_bin())
        .args(["admin", "create-user", "admin"])
        .env_remove("CRABNODE_USER_PASSWORD")
        .env_remove("CRABNODE_SETUP_TOKEN")
        .output()
        .expect("crabnode create-user should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("CRABNODE_USER_PASSWORD"));
    assert!(!stderr.contains("correct horse"));
}

#[test]
fn crabnode_create_user_posts_to_svc_admin_setup_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake svc-admin listener");
    let addr = listener.local_addr().expect("fake svc-admin addr");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("fake svc-admin accepts request");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fake read timeout");

        let mut buf = [0_u8; 4096];
        let n = socket.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let body =
            r#"{"status":"admin_created","user":{"username":"admin"},"setupRequired":false}"#;
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
            "--svc-admin-url",
            &format!("http://{addr}"),
            "admin",
            "create-user",
            "admin",
        ])
        .env("CRABNODE_USER_PASSWORD", "correct horse battery staple")
        .env("CRABNODE_SETUP_TOKEN", "setup-token-good")
        .output()
        .expect("crabnode create-user should run");

    let request = handle.join().expect("fake svc-admin thread joins");

    assert!(output.status.success());

    assert!(request.starts_with("POST /api/setup/create-admin HTTP/1.1"));
    assert!(request.contains(r#""username":"admin""#));
    assert!(request.contains(r#""password":"correct horse battery staple""#));
    assert!(request.contains(r#""setupToken":"setup-token-good""#));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""status":"admin_created""#));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("correct horse battery staple"));
    assert!(!stderr.contains("setup-token-good"));
}

#[test]
fn crabnode_create_user_dry_run_does_not_require_or_print_secrets() {
    let output = Command::new(crabnode_bin())
        .args(["--dry-run", "admin", "create-user", "admin"])
        .env_remove("CRABNODE_USER_PASSWORD")
        .env_remove("CRABNODE_SETUP_TOKEN")
        .output()
        .expect("crabnode create-user dry-run should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("would POST"));
    assert!(stdout.contains("/api/setup/create-admin"));
    assert!(stdout.contains("secrets are not printed"));
    assert!(!stdout.contains("correct horse"));
    assert!(!stdout.contains("setup-token-good"));
}
