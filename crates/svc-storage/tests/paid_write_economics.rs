//! RO:WHAT — Unit/integration-style tests for optional ROC economics pricing in svc-storage.
//! RO:WHY — Pillar 12; Concerns: ECON/GOV/DX. Paid storage pricing should load from validated policy.
//! RO:INTERACTS — svc_storage::policy::economics and configs/roc-economics.toml.
//! RO:INVARIANTS — no floats; no direct wallet/ledger mutation; env-gated economics fail closed when explicit.
//! RO:METRICS — none.
//! RO:CONFIG — RON_STORAGE_ROC_ECONOMICS_PATH, RON_STORAGE_ROC_ECONOMICS_ACTION.
//! RO:SECURITY — test policy only; no bearer tokens, wallet calls, object bytes, or external services.
//! RO:TEST — cargo test -p svc-storage --test paid_write_economics.

use std::{
    env, fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use svc_storage::policy::economics::{
    legacy_paid_storage_capture_amount, paid_storage_capture_amount_from_env,
    ENV_ROC_ECONOMICS_ACTION, ENV_ROC_ECONOMICS_PATH,
};

const CHECKED_IN_ECONOMICS: &str = include_str!("../../../configs/roc-economics.toml");

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn legacy_pricing_is_preserved_when_policy_env_is_unset() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    assert_eq!(legacy_paid_storage_capture_amount(0), 1);
    assert_eq!(legacy_paid_storage_capture_amount(1), 1);
    assert_eq!(legacy_paid_storage_capture_amount(48), 48);

    assert_eq!(paid_storage_capture_amount_from_env(0).unwrap(), 1);
    assert_eq!(paid_storage_capture_amount_from_env(1).unwrap(), 1);
    assert_eq!(paid_storage_capture_amount_from_env(48).unwrap(), 48);
}

#[test]
fn checked_in_roc_economics_prices_paid_storage_put() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    let path = write_temp_policy(CHECKED_IN_ECONOMICS);
    env::set_var(ENV_ROC_ECONOMICS_PATH, &path);

    assert_eq!(paid_storage_capture_amount_from_env(1).unwrap(), 84);
    assert_eq!(paid_storage_capture_amount_from_env(48).unwrap(), 84);
    assert_eq!(paid_storage_capture_amount_from_env(100).unwrap(), 120);

    clear_economics_env();
    let _ = fs::remove_file(path);
}

#[test]
fn configured_action_can_be_overridden() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    let path = write_temp_policy(CHECKED_IN_ECONOMICS);
    env::set_var(ENV_ROC_ECONOMICS_PATH, &path);
    env::set_var(ENV_ROC_ECONOMICS_ACTION, "paid_content_view");

    assert_eq!(paid_storage_capture_amount_from_env(48).unwrap(), 5);

    clear_economics_env();
    let _ = fs::remove_file(path);
}

#[test]
fn explicit_bad_policy_path_fails_closed() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    env::set_var(
        ENV_ROC_ECONOMICS_PATH,
        "/definitely/not/a/real/roc-economics.toml",
    );

    let err = paid_storage_capture_amount_from_env(48)
        .expect_err("explicit missing economics policy must fail closed");

    assert!(err.contains("failed to read"));

    clear_economics_env();
}

#[test]
fn explicit_empty_policy_path_fails_closed() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    env::set_var(ENV_ROC_ECONOMICS_PATH, "   ");

    let err = paid_storage_capture_amount_from_env(48)
        .expect_err("explicit empty economics policy path must fail closed");

    assert!(err.contains("cannot be empty"));

    clear_economics_env();
}

#[test]
fn explicit_unknown_action_fails_closed() {
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    clear_economics_env();

    let path = write_temp_policy(CHECKED_IN_ECONOMICS);
    env::set_var(ENV_ROC_ECONOMICS_PATH, &path);
    env::set_var(ENV_ROC_ECONOMICS_ACTION, "unknown_paid_action");

    let err = paid_storage_capture_amount_from_env(48)
        .expect_err("unknown paid storage action must fail closed");

    assert!(err.contains("unknown economics action"));

    clear_economics_env();
    let _ = fs::remove_file(path);
}

fn clear_economics_env() {
    env::remove_var(ENV_ROC_ECONOMICS_PATH);
    env::remove_var(ENV_ROC_ECONOMICS_ACTION);
}

fn write_temp_policy(contents: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();

    let path = env::temp_dir().join(format!(
        "rustyonions-roc-economics-{}-{nanos}.toml",
        std::process::id()
    ));

    fs::write(&path, contents).expect("temp economics policy should write");
    path
}
