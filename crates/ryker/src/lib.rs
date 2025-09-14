#![forbid(unsafe_code)]
// ryker: tiny supervisor helpers with jittered backoff and a temporary
// compatibility shim for previously in-crate billing helpers.

use std::future::Future;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Spawn a supervised task; restart on error with exponential backoff and jitter.
///
/// Usage:
/// ```ignore
/// ryker::spawn_supervised("overlay-loop", || async {
///     overlay::run_once().await
/// });
/// ```
pub fn spawn_supervised<F, Fut>(name: &'static str, mut factory: F) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = Duration::from_millis(200);
        let max = Duration::from_secs(10);
        loop {
            info!(task = name, "starting");
            match factory().await {
                Ok(()) => {
                    info!(task = name, "completed (no restart)");
                    return;
                }
                Err(e) => {
                    error!(task = name, error = %e, "task failed; will restart");
                    tokio::time::sleep(jitter(backoff)).await;
                    backoff = (backoff * 2).min(max);
                }
            }
        }
    })
}

fn jitter(base: Duration) -> Duration {
    use rand::Rng;
    let b = base.as_millis() as u64;
    let j = rand::thread_rng().gen_range((b / 2)..=(b + b / 2).max(1));
    Duration::from_millis(j)
}

// -------- Temporary compatibility re-exports --------
// These allow existing code importing `ryker::PriceModel` or `ryker::compute_cost`
// to keep compiling while you migrate to `ron_billing::...`.
//
// To remove: set `default-features = false` on `ryker` in dependents,
// then delete this section in a future minor release.
#[cfg(feature = "billing-compat")]
#[allow(deprecated)]
pub use ron_billing::{
    compute_cost as compute_cost,
    validate_payment_block as validate_payment_block,
    validate_wallet_string as validate_wallet_string,
};

#[cfg(feature = "billing-compat")]
#[allow(deprecated)]
pub use ron_billing::PriceModel;

#[cfg(feature = "billing-compat")]
#[deprecated(
    since = "0.2.0",
    note = "moved to `ron-billing`; switch to `ron_billing::PriceModel` and related functions"
)]
pub mod _billing_compat_note {}
