use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use svc_rewarder::config::{load_config_file, validate_config};

fn unique_temp_file(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let dir = std::env::temp_dir().join(format!(
        "svc_rewarder_config_{label}_{}_{}",
        std::process::id(),
        nanos
    ));

    std::fs::create_dir_all(&dir).unwrap();
    dir.join("svc-rewarder.toml")
}

#[test]
fn checked_in_config_fixture_is_valid() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("configs/svc-rewarder.toml");
    let cfg = load_config_file(path).unwrap();

    validate_config(&cfg).unwrap();
    assert_eq!(cfg.ingress.wallet_base_url, "http://127.0.0.1:8088");
    assert_eq!(cfg.ingress.wallet_issue_path, "/v1/issue");
    assert_eq!(cfg.ingress.wallet_cap_scope, "wallet.issue.rewarder");
}

#[test]
fn partial_config_file_overlays_defaults() {
    let path = unique_temp_file("partial");
    let raw = r#"
bind_addr = "127.0.0.1:8199"

[ingress]
wallet_base_url = "http://127.0.0.1:8088"
wallet_issue_path = "/v1/issue"
wallet_cap_scope = "wallet.issue.rewarder"
"#;

    std::fs::write(&path, raw).unwrap();

    let cfg = load_config_file(&path).unwrap();

    assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:8199");
    assert_eq!(cfg.rewarder.policy_id, "policy:v1");
    assert_eq!(cfg.ingress.wallet_base_url, "http://127.0.0.1:8088");

    let parent = path.parent().unwrap().to_path_buf();
    let _ = std::fs::remove_dir_all(parent);
}

#[test]
fn unknown_config_keys_are_rejected() {
    let path = unique_temp_file("unknown");
    std::fs::write(&path, "surprise = true\n").unwrap();

    let err = load_config_file(&path).unwrap_err();

    assert_eq!(err.reason(), "config");

    let parent = path.parent().unwrap().to_path_buf();
    let _ = std::fs::remove_dir_all(parent);
}
