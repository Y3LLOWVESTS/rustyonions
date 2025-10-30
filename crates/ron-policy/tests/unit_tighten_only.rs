use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

#[test]
fn defaults_cap_applies() {
    let b = load_json(
        br#"{
        "version":1,
        "defaults":{"max_body_bytes": 10},
        "rules":[]
    }"#,
    )
    .unwrap();
    let ev = Evaluator::new(&b).unwrap();
    let ctx = Context::builder()
        .tenant("a")
        .method("PUT")
        .region("US")
        .body_bytes(11)
        .build(&SystemClock);
    let d = ev.evaluate(&ctx).unwrap();
    assert!(matches!(
        d.effect,
        ron_policy::engine::eval::DecisionEffect::Deny
    ));
}
