//! Minimal demo for ron-app-sdk.
//!
//! RO:WHAT — Tiny example that exercises config loading, readiness
//!           checks, and constructing `RonAppSdk`.
//! RO:WHY  — Gives integrators a copy-paste starting point without
//!           requiring a running Micronode/Macronode or real transport.
//! RO:NOTE — This example intentionally **does not** call any plane
//!           methods yet (storage/edge/mailbox/index) so we avoid the
//!           unfinished `todo!()` implementations.

use ron_app_sdk::{check_ready, RonAppSdk, SdkConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load config from environment.
    //
    // In a real deployment you’d set variables like:
    //   RON_APP_SDK_GATEWAY_ADDR=https://micronode.example
    //   RON_APP_SDK_TRANSPORT=tls
    //   RON_APP_SDK_OVERALL_TIMEOUT=30s
    let cfg = match SdkConfig::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("[demo] failed to load SdkConfig from env: {err}");
            std::process::exit(1);
        }
    };

    // Run a lightweight readiness check (no network I/O).
    let ready = check_ready(&cfg);
    println!("[demo] ReadyReport: {:?}", ready);

    if !ready.is_ready() {
        eprintln!("[demo] SDK not ready; missing: {:?}", ready.missing);
        std::process::exit(1);
    }

    // Construct the SDK client. This validates config again and
    // prepares the internal transport handle (stubbed for now).
    let sdk = RonAppSdk::new(cfg).await?;
    let ctx = sdk.context();

    println!(
        "[demo] SDK context: profile={:?}, amnesia={}",
        ctx.profile(),
        ctx.amnesia()
    );

    // NOTE: We intentionally do *not* call `storage_get`, `edge_get`,
    // or any other plane methods yet because the underlying transport
    // wiring is still `todo!()`. Once transport is implemented, this
    // example can be extended to make a real call.

    Ok(())
}
