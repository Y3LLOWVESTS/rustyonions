use ron_policy::ctx::clock::SystemClock;
use ron_policy::engine::eval::{DecisionEffect, Evaluator};
use ron_policy::{load_json, Context};

#[test]
fn method_matrix_behaves() {
    let b = load_json(include_bytes!("vectors/method_matrix.json")).unwrap();
    let ev = Evaluator::new(&b).unwrap();

    let getc = Context::builder()
        .tenant("t")
        .method("GET")
        .region("US")
        .build(&SystemClock);
    let putc = Context::builder()
        .tenant("t")
        .method("PUT")
        .region("US")
        .build(&SystemClock);
    let postc = Context::builder()
        .tenant("t")
        .method("POST")
        .region("US")
        .build(&SystemClock);

    let d_get = ev.evaluate(&getc).unwrap();
    assert!(matches!(d_get.effect, DecisionEffect::Allow));
    assert_eq!(d_get.reason.as_deref(), Some("get ok"));

    let d_put = ev.evaluate(&putc).unwrap();
    assert!(matches!(d_put.effect, DecisionEffect::Deny));
    assert_eq!(d_put.reason.as_deref(), Some("put blocked"));

    let d_post = ev.evaluate(&postc).unwrap();
    assert!(matches!(d_post.effect, DecisionEffect::Deny));
    assert_eq!(d_post.reason.as_deref(), Some("default"));
}
