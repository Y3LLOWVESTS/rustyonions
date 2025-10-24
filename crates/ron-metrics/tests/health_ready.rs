use ron_metrics::HealthState;

#[test]
fn health_state_roundtrip() {
    let h = HealthState::new();
    h.set("db".to_string(), false);
    h.set("cache".to_string(), true);

    let snap = h.snapshot();
    assert_eq!(snap.get("db"), Some(&false));
    assert_eq!(snap.get("cache"), Some(&true));
    assert!(!h.all_ready());

    h.set("db".to_string(), true);
    assert!(h.all_ready());
}
