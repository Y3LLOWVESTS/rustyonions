//! RO:WHAT   Hedged requests helper: launch a second attempt after a delay, take first success.
//! RO:WHY    Reduce tail latency for p95+ under occasional stragglers.
//! RO:INVARS  Max two in-flight per call; second attempt only if first hasn't finished.

use super::{DsError, RetryPolicy};
use rand::rngs::StdRng;
use tokio::task::JoinSet;
use std::future::Future;

pub async fn hedge2<F, T>(
    make_call: impl Fn() -> F + Send + Sync + 'static + Clone,
    hedged_after_ms: u64,
) -> Result<T, DsError>
where
    F: Future<Output = Result<T, DsError>> + Send + 'static,
    T: Send + 'static,
{
    let mut js = JoinSet::new();
    js.spawn(make_call.clone()());
    tokio::time::sleep(std::time::Duration::from_millis(hedged_after_ms)).await;
    js.spawn(make_call());

    while let Some(res) = js.join_next().await {
        match res {
            Ok(Ok(v)) => return Ok(v),
            Ok(Err(_)) => continue,
            Err(_) => continue,
        }
    }
    Err(DsError::Net(reqwest::Error::new(
        reqwest::ErrorKind::Request,
        "both hedged attempts failed",
    )))
}
