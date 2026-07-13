//! RO:WHAT — Integration tests for the crabnode exact local prune command.
//!
//! RO:WHY — BUILD_PLAN_Z Phase 10 requires CLI-first local byte deletion
//! and provider withdrawal without fake network-wide or economic success.
//!
//! RO:INVARIANTS — Canonical B3 validation occurs before HTTP; real runs
//! call the macronode prune endpoint; HTTP failures remain failures.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    thread,
    time::Duration,
};

const CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

fn crabnode_bin() -> &'static str {
    env!("CARGO_BIN_EXE_crabnode")
}

fn read_http_request(socket: &mut TcpStream) -> String {
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set fake macronode read timeout");

    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let read = socket.read(&mut chunk).expect("read HTTP request");
        assert!(read > 0, "client closed before request completed");

        bytes.extend_from_slice(&chunk[..read]);

        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };

        let header_text = String::from_utf8_lossy(&bytes[..header_end + 4]);

        let content_length = header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;

                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let expected_length = header_end + 4 + content_length;

        if bytes.len() >= expected_length {
            break;
        }
    }

    String::from_utf8(bytes).expect("HTTP request must be UTF-8")
}

fn spawn_one_request_server(
    response_status: &'static str,
    response_body: &'static str,
) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake macronode");

    let addr = listener.local_addr().expect("fake macronode local address");

    let handle = thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept crabnode request");

        let request = read_http_request(&mut socket);

        let response = format!(
            "HTTP/1.1 {response_status}\r\n\
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

    (format!("http://{addr}"), handle)
}

#[test]
fn help_lists_real_local_prune_without_network_wide_claims() {
    let output = Command::new(crabnode_bin())
        .arg("--help")
        .output()
        .expect("crabnode help");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("prune b3:<hash>"));
    assert!(stdout.contains("Delete exact local bytes"));
    assert!(stdout.contains("withdraw this node's provider record"));
    assert!(stdout.contains("no network-wide deletion"));
    assert!(stdout.contains("storage, provider, and index-cache outcomes"));
    assert!(stdout.contains("no network-wide deletion"));
    assert!(stdout.contains("resolve-cache removal"));
    assert!(stdout.contains("manifest-pointer deletion"));
    assert!(!stdout.contains("prune is not available"));
}

#[test]
fn prune_posts_canonical_object_to_real_macronode_endpoint() {
    let response_body = r#"{"version":1,"object":"b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85","scope":"local_storage_provider_and_index_cache","status":"pruned","complete":true,"changed":true,"local_bytes":{"status":"removed","bytes":3},"provider":{"status":"withdrawn"},"network_propagation":false,"index_cache_invalidation":"invalidated","resolve_cache_invalidation":false,"manifest_pointer_removal":false,"wallet_mutation":false,"ledger_mutation":false,"reward_finality":false}"#;

    let (url, handle) = spawn_one_request_server("200 OK", response_body);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "prune", CID])
        .output()
        .expect("crabnode prune");

    let request = handle.join().expect("fake macronode thread");

    assert!(output.status.success());

    assert!(request.starts_with("POST /api/v1/moderation/prune HTTP/1.1"));

    assert!(request.contains(&format!(r#"{{"object":"{CID}"}}"#)));

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(r#""status":"pruned""#));
    assert!(stdout.contains(r#""index_cache_invalidation":"invalidated","resolve_cache_invalidation":false,"manifest_pointer_removal":false"#));
    assert!(stdout.contains(r#""network_propagation":false"#));
}

#[test]
fn invalid_prune_object_is_rejected_before_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind unused listener");

    listener
        .set_nonblocking(true)
        .expect("set unused listener nonblocking");

    let addr = listener.local_addr().expect("unused listener address");

    let output = Command::new(crabnode_bin())
        .args([
            "--admin-url",
            &format!("http://{addr}"),
            "prune",
            "b3:not-valid",
        ])
        .output()
        .expect("invalid crabnode prune");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid prune object"));

    let error = listener
        .accept()
        .expect_err("invalid object must not connect");

    assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
}

#[test]
fn prune_dry_run_is_explicit_about_local_scope_and_missing_steps() {
    let output = Command::new(crabnode_bin())
        .args(["--dry-run", "prune", CID])
        .output()
        .expect("crabnode prune dry-run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("would POST"));
    assert!(stdout.contains("/api/v1/moderation/prune"));
    assert!(stdout.contains("exact local byte deletion"));
    assert!(stdout.contains("configured-node provider withdrawal"));
    assert!(stdout.contains("network propagation=false"));
    assert!(stdout.contains("exact provider-cache invalidation"));
    assert!(stdout.contains("resolve cache invalidation=false"));
    assert!(stdout.contains("manifest pointer removal=false"));
    assert!(stdout.contains("no wallet or ledger mutation"));
    assert!(stdout.contains("no reward finality"));
}

#[test]
fn prune_preserves_macronode_failure_instead_of_printing_success() {
    let response_body = r#"{"version":1,"object":"b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85","status":"failed","complete":false,"changed":false,"local_bytes":{"status":"unavailable"},"provider":{"status":"unavailable"}}"#;

    let (url, handle) = spawn_one_request_server("503 Service Unavailable", response_body);

    let output = Command::new(crabnode_bin())
        .args(["--admin-url", &url, "prune", CID])
        .output()
        .expect("failed crabnode prune");

    let request = handle.join().expect("fake macronode thread");

    assert!(request.starts_with("POST /api/v1/moderation/prune HTTP/1.1"));

    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stdout.trim().is_empty());
    assert!(stderr.contains("HTTP error"));
    assert!(stderr.contains("503"));
    assert!(stderr.contains(r#""status":"failed""#));
}
