use ron_policy::load_json;

#[test]
fn strict_deny_unknown_fields() {
    // Unknown field "oops" should be rejected.
    let bad = br#"{"version":1,"oops":true,"rules":[]}"#;
    let err = load_json(bad).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("validation error") || msg.contains("parse error"));
}

#[test]
fn round_trip_minimal() {
    let good = br#"{"version":1,"rules":[]}"#;
    let b = load_json(good).unwrap();
    assert_eq!(b.version, 1);
}
