use ron_policy::ctx::clock::SystemClock;
use ron_policy::engine::eval::{DecisionEffect, Evaluator};
use ron_policy::{load_json, Context};

#[test]
fn eu_allows_us_denies() {
    let b = load_json(include_bytes!("vectors/eu_only.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();

    let eu = Context::builder()
        .tenant("t")
        .method("GET")
        .region("EU")
        .build(&SystemClock);
    let us = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .build(&SystemClock);

    let de = ev.evaluate(&eu).unwrap();
    assert!(matches!(de.effect, DecisionEffect::Allow));
    assert_eq!(de.reason.as_deref(), Some("eu residency"));

    let du = ev.evaluate(&us).unwrap();
    assert!(matches!(du.effect, DecisionEffect::Deny));
    assert_eq!(du.reason.as_deref(), Some("default"));
}
