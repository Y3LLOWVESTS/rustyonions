//! RO:WHAT — Integration tests for crabnode rewards operator commands.
//! RO:WHY — Phase 5B begins reward-recipient CLI surface without fake payout success.
//! RO:INVARIANTS — commands call real macronode endpoints; no wallet/ledger mutation is implied.

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

fn spawn_one_request_server(
    response_status: &'static str,
    response_body: &'static str,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake macronode listener");
    let addr = listener.local_addr().expect("fake macronode addr");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("fake macronode accepts request");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fake read timeout");

        let mut buf = [0_u8; 4096];
        let n = socket.read(&mut buf).expect("read request");
        let request = String::from_utf8_lossy(&buf[..n]).to_string();

        let response = format!(
            "HTTP/1.1 {response_status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );

        socket
            .write_all(response.as_bytes())
            .expect("write fake response");

        request
    });

    (format!("http://{addr}"), handle)
}

#[test]
fn crabnode_help_lists_rewards_commands_without_claiming_payout_finality() {
    let output = Command::new(crabnode_bin())
        .arg("--help")
        .output()
        .expect("crabnode --help should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rewards show"));
    assert!(stdout.contains("rewards bind @operator"));
    assert!(stdout.contains("rewards rotate @new"));
    assert!(stdout.contains("do not mutate wallet or ledger state"));
    assert!(!stdout.contains("confirmed ROC"));
}

#[test]
fn crabnode_rewards_show_queries_real_macronode_endpoint() {
    let body =
        r#"{"version":1,"serviceNodeId":"node_b3c9","state":"unbound","observedRegistryEpoch":7}"#;
    let (url, handle) = spawn_one_request_server("200 OK", body);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "rewards", "show"])
        .output()
        .expect("crabnode rewards show should run");

    let request = handle.join().expect("fake macronode thread joins");

    assert!(output.status.success());
    assert!(request.starts_with("GET /api/v1/rewards/status HTTP/1.1"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"state\":\"unbound\""));
}

#[test]
fn crabnode_rewards_show_dry_run_is_explicitly_non_mutating() {
    let output = Command::new(crabnode_bin())
        .args(["--dry-run", "rewards", "show"])
        .output()
        .expect("crabnode rewards show --dry-run should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("would query"));
    assert!(stdout.contains("/api/v1/rewards/status"));
    assert!(stdout.contains("wallet/ledger receipts"));
}

#[test]
fn crabnode_rewards_bind_posts_registry_binding_request() {
    let body = r#"{"status":"binding_request_received","state":"pending"}"#;
    let (url, handle) = spawn_one_request_server("202 Accepted", body);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "rewards", "bind", "@operator"])
        .output()
        .expect("crabnode rewards bind should run");

    let request = handle.join().expect("fake macronode thread joins");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/rewards/bind HTTP/1.1"));
    assert!(request.contains(r#""rewardRecipientDisplayAddress":"@operator""#));
    assert!(request.contains("no wallet or ledger mutation"));
    assert!(!request.contains("confirmedRoc"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("binding_request_received"));
}

#[test]
fn crabnode_rewards_rotate_posts_future_epoch_rotation_request() {
    let body = r#"{"status":"rotation_request_received","state":"pending_rotation"}"#;
    let (url, handle) = spawn_one_request_server("202 Accepted", body);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "rewards", "rotate", "@new-operator"])
        .output()
        .expect("crabnode rewards rotate should run");

    let request = handle.join().expect("fake macronode thread joins");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/rewards/rotate HTTP/1.1"));
    assert!(request.contains(r#""newRewardRecipientDisplayAddress":"@new-operator""#));
    assert!(request.contains("future-epoch registry rotation request only"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pending_rotation"));
}

#[test]
fn crabnode_rewards_bind_rejects_non_address_without_http_call() {
    let output = Command::new(crabnode_bin())
        .args(["rewards", "bind", "operator"])
        .output()
        .expect("crabnode rewards bind should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("@ address"));
}

#[test]
fn crabnode_rewards_bind_refuses_non_loopback_by_default() {
    let output = Command::new(crabnode_bin())
        .args([
            "--admin-url",
            "http://198.51.100.10:8080",
            "rewards",
            "bind",
            "@operator",
        ])
        .output()
        .expect("crabnode rewards bind should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("refusing non-loopback admin host"));
}
