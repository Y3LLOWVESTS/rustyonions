use ron_policy::ctx::clock::SystemClock;
use ron_policy::engine::eval::{DecisionEffect, Evaluator};
use ron_policy::{load_json, Context};

#[test]
fn large_body_defaults_trips_deny() {
    let b = load_json(include_bytes!("vectors/large_body_default_deny.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();
    let ctx = Context::builder()
        .tenant("t")
        .method("PUT")
        .region("US")
        .body_bytes(512 * 1024)
        .build(&SystemClock);
    let d = ev.evaluate(&ctx).unwrap();
    assert!(matches!(d.effect, DecisionEffect::Deny));
    assert_eq!(d.reason.as_deref(), Some("body too large (defaults)"));
}
