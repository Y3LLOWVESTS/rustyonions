use ron_policy::engine::eval::DecisionEffect;
use ron_policy::explain::trace::TraceStep;
use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

#[test]
fn explain_trace_is_stable() {
    let b = load_json(include_bytes!("vectors/decompress_guard.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();
    let ctx = Context::builder()
        .tenant("t")
        .method("PUT")
        .region("US")
        .body_bytes(512)
        .build(&SystemClock);
    let d = ev.evaluate(&ctx).unwrap();

    assert!(matches!(d.effect, DecisionEffect::Deny));
    assert_eq!(d.reason.as_deref(), Some("per-rule cap"));
    assert_eq!(d.trace.steps.len(), 1);
    match &d.trace.steps[0] {
        TraceStep::RuleHit { id, reason } => {
            assert_eq!(id, "deny-large-put");
            assert_eq!(reason, "per-rule cap");
        }
        other => panic!("unexpected trace step: {other:?}"),
    }
}
