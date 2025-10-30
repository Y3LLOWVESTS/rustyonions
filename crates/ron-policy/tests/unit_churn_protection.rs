use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

#[test]
fn rule_deny_region() {
    let b = load_json(include_bytes!("vectors/deny_region.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();
    let ctx = Context::builder()
        .tenant("any")
        .method("GET")
        .region("US-FL")
        .build(&SystemClock);
    let d = ev.evaluate(&ctx).unwrap();
    assert!(matches!(
        d.effect,
        ron_policy::engine::eval::DecisionEffect::Deny
    ));
}
