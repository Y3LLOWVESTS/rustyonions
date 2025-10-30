use ron_policy::{load_json, model::*};

#[test]
fn strict_deny_unknown_fields() {
    let bad = br#"{"version":1,"rules":[],"oops":1}"#;
    assert!(load_json(bad).is_err());
}

#[test]
fn round_trip_minimal() {
    let good =
        br#"{"version":1,"rules":[{"id":"allow-get","when":{"method":"GET"},"action":"allow"}]}"#;
    let b = load_json(good).unwrap();
    assert_eq!(b.rules.len(), 1);
}
