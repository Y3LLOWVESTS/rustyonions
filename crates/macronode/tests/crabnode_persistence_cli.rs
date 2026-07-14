//! RO:WHAT — Focused tests for `crabnode persistence` operator commands.
//!
//! RO:WHY — Phase 11F requires CLI commands to reach the real macronode
//! persistence HTTP contract without fabricating durability or success.
//!
//! RO:INTERACTS — crabnode binary and macronode persistence admin routes.
//!
//! RO:INVARIANTS — exact B3 and canonical asset-kind validation happens
//! before HTTP; real response bodies and failures are preserved.
//!
//! RO:SECURITY — loopback-only fake servers; no durable-byte, reward, wallet,
//! or ledger mutation.
//!
//! RO:TEST — cargo test -p macronode --test crabnode_persistence_cli.

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
    time::Duration,
};

const CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

fn spawn_one_request_server(
    status: &'static str,
    response_body: &'static str,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake macronode");
    let address = listener.local_addr().expect("fake listener address");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept fake macronode request");

        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fake read timeout");

        let mut bytes = [0_u8; 4096];
        let read = socket.read(&mut bytes).expect("read CLI request");
        let request = String::from_utf8_lossy(&bytes[..read]).to_string();

        let response = format!(
            "HTTP/1.1 {status}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {response_body}",
            response_body.len()
        );

        socket
            .write_all(response.as_bytes())
            .expect("write fake macronode response");

        request
    });

    (format!("http://{address}"), handle)
}

fn request_json(request: &str) -> serde_json::Value {
    let (_, body) = request
        .split_once("\r\n\r\n")
        .expect("HTTP request must contain a body separator");

    serde_json::from_str(body).expect("request body must be JSON")
}

#[test]
fn help_lists_real_persistence_commands_without_durability_claims() {
    let output = Command::new(crabnode_bin())
        .arg("--help")
        .output()
        .expect("crabnode help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("persistence register b3:<hash> KIND"));
    assert!(stdout.contains("persistence status b3:<hash>"));
    assert!(stdout.contains("persistence pending [LIMIT]"));
    assert!(stdout.contains("persistence submit b3:<hash>"));
    assert!(stdout.contains("persistence approve b3:<hash>"));
    assert!(stdout.contains("persistence pin b3:<hash>"));
    assert!(stdout.contains("persistence unpin b3:<hash>"));
    assert!(stdout.contains("persistence reject b3:<hash>"));
    assert!(stdout.contains("eligibility does not prove durable bytes"));
    assert!(!stdout.contains("durable write completed"));
}

#[test]
fn register_posts_exact_object_and_canonical_asset_kind() {
    let response = r#"{"action":"register","changed":true,"candidate":{"state":"ephemeral_unvetted"},"durableBytesWritten":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "register", CID, "IMAGE"])
        .output()
        .expect("crabnode persistence register");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/register HTTP/1.1"));

    let body = request_json(&request);
    assert_eq!(body["object"], CID);
    assert_eq!(body["assetKind"], "image");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""action":"register""#));
    assert!(stdout.contains(r#""durableBytesWritten":false"#));
}

#[test]
fn status_queries_the_exact_runtime_candidate() {
    let response = r#"{"candidate":{"object":"b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","state":"pending_review"},"durableBytesWritten":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "status", CID])
        .output()
        .expect("crabnode persistence status");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with(&format!("GET /api/v1/persistence/status/{CID} HTTP/1.1")));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""state":"pending_review""#));
}

#[test]
fn pending_queries_the_requested_bounded_limit() {
    let response = r#"{"limit":7,"count":0,"items":[],"durableBytesWritten":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "pending", "7"])
        .output()
        .expect("crabnode persistence pending");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("GET /api/v1/persistence/pending?limit=7 HTTP/1.1"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""limit":7"#));
}

#[test]
fn submit_posts_the_exact_object() {
    let response = r#"{"action":"submit_for_review","changed":true,"candidate":{"state":"pending_review"},"durableBytesWritten":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "submit", CID])
        .output()
        .expect("crabnode persistence submit");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/submit HTTP/1.1"));
    assert_eq!(request_json(&request)["object"], CID);
}

#[test]
fn approve_posts_the_exact_object_and_preserves_runtime_response() {
    let response = r#"{"action":"approve","changed":true,"candidate":{"state":"verified_persistent","durableStorageEligible":true},"durableBytesWritten":false,"walletMutation":false,"ledgerMutation":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "approve", CID])
        .output()
        .expect("crabnode persistence approve");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/approve HTTP/1.1"));
    assert_eq!(request_json(&request)["object"], CID);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(r#""action":"approve""#));
    assert!(stdout.contains(r#""state":"verified_persistent""#));
    assert!(stdout.contains(r#""durableBytesWritten":false"#));
    assert!(stdout.contains(r#""walletMutation":false"#));
    assert!(stdout.contains(r#""ledgerMutation":false"#));
}

#[test]
fn pin_posts_the_exact_object_and_preserves_runtime_response() {
    let response = r#"{"action":"pin","changed":true,"candidate":{"state":"pinned_by_operator","durableStorageEligible":true},"durableBytesWritten":false,"walletMutation":false,"ledgerMutation":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "pin", CID])
        .output()
        .expect("crabnode persistence pin");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/pin HTTP/1.1"));
    assert_eq!(request_json(&request)["object"], CID);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(r#""action":"pin""#));
    assert!(stdout.contains(r#""state":"pinned_by_operator""#));
    assert!(stdout.contains(r#""durableBytesWritten":false"#));
    assert!(stdout.contains(r#""walletMutation":false"#));
    assert!(stdout.contains(r#""ledgerMutation":false"#));
}

#[test]
fn unpin_posts_the_exact_object_and_preserves_runtime_response() {
    let response = r#"{"action":"unpin","changed":true,"candidate":{"state":"verified_persistent","durableStorageEligible":true},"durableBytesWritten":false,"walletMutation":false,"ledgerMutation":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "unpin", CID])
        .output()
        .expect("crabnode persistence unpin");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/unpin HTTP/1.1"));
    assert_eq!(request_json(&request)["object"], CID);

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(r#""action":"unpin""#));
    assert!(stdout.contains(r#""state":"verified_persistent""#));
    assert!(stdout.contains(r#""durableBytesWritten":false"#));
    assert!(stdout.contains(r#""walletMutation":false"#));
    assert!(stdout.contains(r#""ledgerMutation":false"#));
}

#[test]
fn approve_pin_and_unpin_dry_runs_are_explicitly_non_mutating() {
    for (action, path) in [
        ("approve", "/api/v1/persistence/approve"),
        ("pin", "/api/v1/persistence/pin"),
        ("unpin", "/api/v1/persistence/unpin"),
    ] {
        let output = Command::new(crabnode_bin())
            .args(["--dry-run", "persistence", action, CID])
            .output()
            .expect("persistence policy action dry-run");

        assert!(
            output.status.success(),
            "{action} dry-run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(stdout.contains("would POST"));
        assert!(stdout.contains(path));
        assert!(stdout.contains("changes persistence-review metadata only"));
        assert!(stdout.contains("durable bytes written=false"));
        assert!(stdout.contains("no wallet or ledger mutation"));
    }
}

#[test]
fn reject_posts_the_exact_object_and_preserves_runtime_response() {
    let response = r#"{"action":"reject","changed":true,"candidate":{"state":"operator_blocked"},"durableBytesWritten":false,"walletMutation":false,"ledgerMutation":false}"#;
    let (url, handle) = spawn_one_request_server("200 OK", response);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "reject", CID])
        .output()
        .expect("crabnode persistence reject");

    let request = handle.join().expect("fake server thread");

    assert!(output.status.success());
    assert!(request.starts_with("POST /api/v1/persistence/reject HTTP/1.1"));
    assert_eq!(request_json(&request)["object"], CID);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""state":"operator_blocked""#));
    assert!(stdout.contains(r#""walletMutation":false"#));
    assert!(stdout.contains(r#""ledgerMutation":false"#));
}

#[test]
fn invalid_object_asset_kind_and_limit_reject_before_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind no-request listener");
    listener
        .set_nonblocking(true)
        .expect("set listener nonblocking");

    let url = format!(
        "http://{}",
        listener.local_addr().expect("listener address")
    );

    let invalid_object = Command::new(crabnode_bin())
        .args([
            "--admin-url",
            &url,
            "persistence",
            "register",
            "b3:not-valid",
            "image",
        ])
        .output()
        .expect("invalid object command");

    assert!(!invalid_object.status.success());
    assert!(String::from_utf8_lossy(&invalid_object.stderr).contains("invalid persistence object"));

    let invalid_kind = Command::new(crabnode_bin())
        .args([
            "--admin-url",
            &url,
            "persistence",
            "register",
            CID,
            "executable",
        ])
        .output()
        .expect("invalid asset-kind command");

    assert!(!invalid_kind.status.success());
    assert!(
        String::from_utf8_lossy(&invalid_kind.stderr).contains("invalid persistence asset kind")
    );

    let invalid_limit = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "persistence", "pending", "0"])
        .output()
        .expect("invalid pending-limit command");

    assert!(!invalid_limit.status.success());
    assert!(String::from_utf8_lossy(&invalid_limit.stderr).contains("must be within 1..=1024"));

    let error = listener
        .accept()
        .expect_err("invalid inputs must not contact macronode");

    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn persistence_dry_runs_are_explicitly_non_mutating() {
    let register = Command::new(crabnode_bin())
        .args(["--dry-run", "persistence", "register", CID, "image"])
        .output()
        .expect("register dry-run");

    assert!(register.status.success());

    let register_stdout = String::from_utf8_lossy(&register.stdout);

    assert!(register_stdout.contains("would POST"));
    assert!(register_stdout.contains("/api/v1/persistence/register"));
    assert!(register_stdout.contains("candidate begins amnesia-first"));
    assert!(register_stdout.contains("durable bytes written=false"));
    assert!(register_stdout.contains("no wallet or ledger mutation"));

    let reject = Command::new(crabnode_bin())
        .args(["--dry-run", "persistence", "reject", CID])
        .output()
        .expect("reject dry-run");

    assert!(reject.status.success());

    let reject_stdout = String::from_utf8_lossy(&reject.stdout);

    assert!(reject_stdout.contains("would POST"));
    assert!(reject_stdout.contains("/api/v1/persistence/reject"));
    assert!(reject_stdout.contains("changes persistence-review metadata only"));
    assert!(reject_stdout.contains("durable bytes written=false"));
}

#[test]
fn persistence_requests_prefer_crabnode_admin_token() {
    let response_body = r#"{
        "version":1,
        "limit":1,
        "count":0,
        "maximumLimit":4096,
        "items":[],
        "durableBytesWritten":false,
        "walletMutation":false,
        "ledgerMutation":false
    }"#;

    let (admin_url, handle) = spawn_one_request_server("200 OK", response_body);

    let preferred = "phase21g-crabnode-secret";
    let fallback = "phase21g-ron-secret";

    let output = Command::new(crabnode_bin())
        .env("CRABNODE_ADMIN_TOKEN", preferred)
        .env("RON_ADMIN_TOKEN", fallback)
        .args(["--admin-url", &admin_url, "persistence", "pending", "1"])
        .output()
        .expect("authenticated crabnode persistence request");

    let request = handle.join().expect("fake server joins");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        request.contains(
            "Authorization: Bearer \
             phase21g-crabnode-secret\r\n"
        ),
        "preferred administrator-token header missing: \
         {request}",
    );

    assert!(
        !request.contains(fallback),
        "fallback token must not be sent when the \
         crabnode-specific token exists",
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stdout.contains(preferred));
    assert!(!stderr.contains(preferred));
    assert!(!stdout.contains(fallback));
    assert!(!stderr.contains(fallback));
}

#[test]
fn persistence_requests_use_ron_admin_token_fallback() {
    let response_body = r#"{
        "version":1,
        "limit":1,
        "count":0,
        "maximumLimit":4096,
        "items":[],
        "durableBytesWritten":false,
        "walletMutation":false,
        "ledgerMutation":false
    }"#;

    let (admin_url, handle) = spawn_one_request_server("200 OK", response_body);

    let fallback = "phase21g-fallback-secret";

    let output = Command::new(crabnode_bin())
        .env_remove("CRABNODE_ADMIN_TOKEN")
        .env("RON_ADMIN_TOKEN", fallback)
        .args(["--admin-url", &admin_url, "persistence", "pending", "1"])
        .output()
        .expect("fallback-authenticated persistence request");

    let request = handle.join().expect("fake server joins");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        request.contains(
            "Authorization: Bearer \
             phase21g-fallback-secret\r\n"
        ),
        "RON_ADMIN_TOKEN fallback header missing: \
         {request}",
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stdout.contains(fallback));
    assert!(!stderr.contains(fallback));
}

#[test]
fn persistence_requests_omit_auth_when_tokens_are_absent() {
    let response_body = r#"{
        "version":1,
        "limit":1,
        "count":0,
        "maximumLimit":4096,
        "items":[],
        "durableBytesWritten":false,
        "walletMutation":false,
        "ledgerMutation":false
    }"#;

    let (admin_url, handle) = spawn_one_request_server("200 OK", response_body);

    let output = Command::new(crabnode_bin())
        .env_remove("CRABNODE_ADMIN_TOKEN")
        .env_remove("RON_ADMIN_TOKEN")
        .args(["--admin-url", &admin_url, "persistence", "pending", "1"])
        .output()
        .expect("unauthenticated loopback development request");

    let request = handle.join().expect("fake server joins");

    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );

    assert!(
        !request.to_ascii_lowercase().contains("\r\nauthorization:"),
        "authorization header must be absent when no \
         token is configured: {request}",
    );
}
