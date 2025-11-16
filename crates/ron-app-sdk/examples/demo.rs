//! Minimal demo for ron-app-sdk.
//!
//! RO:WHAT — Tiny example that exercises config loading, readiness
//!           checks, and constructing `RonAppSdk`.
//! RO:WHY  — Gives integrators a copy-paste starting point without
//!           requiring a running Micronode/Macronode or real gateway.
//! RO:NOTE — This example does *not* perform real plane calls yet,
//!           because the default `RON_SDK_GATEWAY_ADDR` in docs is
//!           usually `https://example.invalid`. It focuses on:
//!             - config-from-env
//!             - readiness (`check_ready`)
//!             - constructing the SDK client safely.

use ron_app_sdk::{check_ready, RonAppSdk, SdkConfig};

/// Run with something like:
///
/// ```bash
/// RON_SDK_GATEWAY_ADDR="https://example.invalid" \
/// RON_SDK_TRANSPORT="tls" \
/// RON_SDK_OVERALL_TIMEOUT_MS="30000" \
/// cargo run -p ron-app-sdk --example demo
/// ```
///
/// For real usage, point `RON_SDK_GATEWAY_ADDR` at a live Micronode /
/// Macronode gateway and provide a real capability when calling the
/// plane helpers (`storage_*`, `edge_get`, `mailbox_*`, `index_resolve`).
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) Load config from environment.
    let cfg = SdkConfig::from_env()?;

    println!(
        "[demo] loaded config: transport={:?}, gateway_addr={}",
        cfg.transport, cfg.gateway_addr
    );

    // 2) Run in-process readiness check (no network I/O).
    let report = check_ready(&cfg);

    println!(
        "[demo] ready_report: config_ok={}, transport_ok={}, tor_ok={:?}, missing={:?}",
        report.config_ok, report.transport_ok, report.tor_ok, report.missing
    );

    if !report.is_ready() {
        eprintln!("[demo] SDK configuration is NOT ready; refusing to construct client");
        eprintln!("[demo] fix the above `missing` fields and try again.");
        return Ok(());
    }

    // 3) Construct the SDK client. This re-validates config and
    // prepares the internal transport handle.
    let sdk = RonAppSdk::new(cfg).await?;
    let ctx = sdk.context();

    println!(
        "[demo] SDK context: profile={:?}, amnesia={}",
        ctx.profile(),
        ctx.amnesia()
    );

    // 4) (Optional) Sketch of how plane calls will look — left as
    // comments so this example stays safe even when pointing at
    // example.invalid.
    //
    // use ron_app_sdk::types::{Capability, IdemKey};
    //
    // let cap: Capability = /* obtain from svc-passport / auth flow */;
    //
    // // Example: storage_put (with optional idempotency key)
    // let bytes = bytes::Bytes::from_static(b"hello world");
    // let idem = None::<IdemKey>;
    // let addr = sdk
    //     .storage_put(cap.clone(), bytes, std::time::Duration::from_secs(5), idem)
    //     .await?;
    // println!("[demo] stored blob at addr_b3={}", addr);
    //
    // // Example: storage_get
    // let fetched = sdk
    //     .storage_get(cap.clone(), &addr, std::time::Duration::from_secs(5))
    //     .await?;
    // println!("[demo] fetched {} bytes", fetched.len());

    println!("[demo] done (no plane calls issued)");

    Ok(())
}
