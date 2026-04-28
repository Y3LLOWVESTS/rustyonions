use svc_rewarder::outputs::IntentStore;

#[test]
fn settlement_intent_egress_is_idempotent_by_run_key() {
    let store = IntentStore::default();
    let first = store.emit_once("b3:run", false);
    let second = store.emit_once("b3:run", false);
    assert_eq!(first.as_str(), "accepted");
    assert_eq!(second.as_str(), "dup");
}
