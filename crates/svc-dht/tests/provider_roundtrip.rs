use std::time::Duration;
use svc_dht::provider::Store;

#[test]
fn provider_add_get_prune_roundtrip() {
    let store = Store::new(Duration::from_secs(2));
    let cid = "b3:deadbeef".to_string();

    // Add two providers; ensure de-dup by node works
    store.add(cid.clone(), "local://A".into(), Some(Duration::from_millis(250)));
    store.add(cid.clone(), "local://B".into(), Some(Duration::from_millis(250)));
    store.add(cid.clone(), "local://A".into(), Some(Duration::from_millis(250))); // refresh

    let mut live = store.get_live(&cid);
    live.sort();
    assert_eq!(live, vec!["local://A", "local://B"]);

    // After expiry window, purge removes both
    std::thread::sleep(Duration::from_millis(300));
    let purged = store.purge_expired();
    assert!(purged >= 1, "expected at least one purged; got {purged}");

    let live2 = store.get_live(&cid);
    assert!(live2.is_empty(), "expected no live providers after purge, got {live2:?}");
}
