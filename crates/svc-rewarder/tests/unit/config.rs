use svc_rewarder::config::{validate_config, Config};

#[test]
fn default_config_is_valid() {
    validate_config(&Config::default()).unwrap();
}

#[test]
fn zero_workers_rejects() {
    let mut cfg = Config::default();
    cfg.concurrency.compute_workers = 0;
    let err = validate_config(&cfg).unwrap_err();
    assert_eq!(err.reason(), "config");
}

#[test]
fn tls_enabled_requires_paths() {
    let mut cfg = Config::default();
    cfg.tls.enabled = true;
    let err = validate_config(&cfg).unwrap_err();
    assert_eq!(err.reason(), "config");
}

#[test]
fn wallet_issue_path_must_start_with_slash() {
    let mut cfg = Config::default();
    cfg.ingress.wallet_issue_path = "v1/issue".into();
    let err = validate_config(&cfg).unwrap_err();
    assert_eq!(err.reason(), "config");
}

#[test]
fn wallet_base_url_must_be_http_or_https() {
    let mut cfg = Config::default();
    cfg.ingress.wallet_base_url = "unix:///tmp/svc-wallet.sock".into();
    let err = validate_config(&cfg).unwrap_err();
    assert_eq!(err.reason(), "config");
}

#[test]
fn wallet_cap_scope_must_not_be_empty() {
    let mut cfg = Config::default();
    cfg.ingress.wallet_cap_scope = " ".into();
    let err = validate_config(&cfg).unwrap_err();
    assert_eq!(err.reason(), "config");
}
