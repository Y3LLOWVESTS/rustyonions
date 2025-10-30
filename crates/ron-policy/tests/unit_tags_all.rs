use ron_policy::ctx::clock::SystemClock;
use ron_policy::engine::eval::{DecisionEffect, Evaluator};
use ron_policy::{load_json, Context};

#[test]
fn all_required_tags_must_exist() {
    let b = load_json(include_bytes!("vectors/tags_all.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();

    let ok = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("paid")
        .tag("verified")
        .build(&SystemClock);

    let missing_one = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .tag("paid")
        .build(&SystemClock);

    let d1 = ev.evaluate(&ok).unwrap();
    assert!(matches!(d1.effect, DecisionEffect::Allow));
    assert_eq!(d1.reason.as_deref(), Some("all-required-tags-present"));

    let d2 = ev.evaluate(&missing_one).unwrap();
    assert!(matches!(d2.effect, DecisionEffect::Deny));
    assert_eq!(d2.reason.as_deref(), Some("default"));
}
