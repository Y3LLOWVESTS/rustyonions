use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

#[test]
fn deterministic_default_deny() {
    let bundle = load_json(br#"{"version":1,"rules":[]}"#).unwrap();
    let ctx = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .build(&SystemClock);
    let ev = Evaluator::new(&bundle).unwrap();
    let d = ev.evaluate(&ctx).unwrap();
    assert!(matches!(
        d.effect,
        ron_policy::engine::eval::DecisionEffect::Deny
    ));
}
