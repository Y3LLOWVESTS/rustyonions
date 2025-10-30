#![no_main]
use libfuzzer_sys::fuzz_target;
use ron_policy::{load_json, Evaluator, Context, ctx::clock::SystemClock};

fuzz_target!(|data: &[u8]| {
    if let Ok(b) = load_json(data) {
        if let Ok(ev) = Evaluator::new(&b) {
            let c = Context::builder().tenant("t").method("GET").region("US").build(&SystemClock);
            let _ = ev.evaluate(&c);
        }
    }
});
