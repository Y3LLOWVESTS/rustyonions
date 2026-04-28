use svc_rewarder::core::run_key;
use svc_rewarder::outputs::IntentStore;

#[test]
fn run_key_is_deterministic_for_same_triple() {
    let a = run_key(
        "epoch-1",
        &format!("b3:{}", "b".repeat(64)),
        &format!("b3:{}", "a".repeat(64)),
        "salt",
    );
    let b = run_key(
        "epoch-1",
        &format!("b3:{}", "b".repeat(64)),
        &format!("b3:{}", "a".repeat(64)),
        "salt",
    );
    assert_eq!(a, b);
}

#[test]
fn intent_store_accepts_then_dups() {
    let store = IntentStore::default();
    assert_eq!(store.emit_once("run-1", false).as_str(), "accepted");
    assert_eq!(store.emit_once("run-1", false).as_str(), "dup");
    assert_eq!(store.emit_once("run-2", true).as_str(), "dry_run");
}
