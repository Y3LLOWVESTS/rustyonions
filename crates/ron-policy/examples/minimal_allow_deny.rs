//! RO:WHAT — Minimal usage example: parse bundle → evaluate context → print result.
use ron_policy::{ctx::clock::SystemClock, load_json, Context, Evaluator};

fn main() {
    let bundle = load_json(
        br#"{
        "version":1,
        "rules":[
            {"id":"deny-fl","when":{"region":"US-FL"},"action":"deny","reason":"geo block"},
            {"id":"allow","when":{},"action":"allow","reason":"open"}
        ]
    }"#,
    )
    .unwrap();

    let ev = Evaluator::new(&bundle).unwrap();
    let ctx = Context::builder()
        .tenant("acme")
        .method("GET")
        .region("US")
        .build(&SystemClock);
    let d = ev.evaluate(&ctx).unwrap();
    println!("effect={:?} reason={:?}", d.effect, d.reason);
}
