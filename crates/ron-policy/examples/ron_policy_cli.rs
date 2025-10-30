use ron_policy::ctx::clock::SystemClock;
use ron_policy::{load_json, load_toml, Context, Evaluator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = pico_args::Arguments::from_env();

    let bundle: String = args.value_from_str("--bundle")?;
    let tenant: String = args
        .opt_value_from_str("--tenant")?
        .unwrap_or_else(|| "*".into());
    let method: String = args
        .opt_value_from_str("--method")?
        .unwrap_or_else(|| "*".into());
    let region: String = args
        .opt_value_from_str("--region")?
        .unwrap_or_else(|| "*".into());
    let body: u64 = args.opt_value_from_str("--body")?.unwrap_or(0);

    // Collect repeated flags: --tag paid --tag verified ...
    // Note: values_from_str<A, T>(...) — we specify T and let A be inferred via `_`.
    let tags: Vec<String> = args
        .values_from_str::<_, String>("--tag")
        .unwrap_or_default();

    let bytes = std::fs::read(&bundle)?;
    let bundle_val = if bundle.ends_with(".json") {
        load_json(&bytes)?
    } else if bundle.ends_with(".toml") {
        load_toml(&bytes)?
    } else {
        return Err("bundle must be .json or .toml".into());
    };

    let ev = Evaluator::new(&bundle_val)?;
    let mut b = Context::builder()
        .tenant(tenant)
        .method(method)
        .region(region)
        .body_bytes(body);
    for t in tags {
        b = b.tag(t);
    }
    let ctx = b.build(&SystemClock);

    let d = ev.evaluate(&ctx)?;
    println!("effect={:?} reason={:?}", d.effect, d.reason);
    println!("trace={:?}", d.trace.steps);
    Ok(())
}
