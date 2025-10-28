//! RO:WHAT — β-hedged race between lookup legs with stagger
//! RO:WHY — Reduce tail latency while respecting deadline; Concerns: PERF/RES

use std::future::Future;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// Race a primary future with up to `beta` hedges, each staggered by `stagger`.
/// Each leg is wrapped with `timeout(leg_budget)`. The first Ok wins; errors are
/// collected and last error is returned if all fail/timeout.
pub async fn race_hedged<F, Fut, T, E>(
    beta: usize,
    stagger: Duration,
    leg_budget: Duration,
    mut mk_leg: F,
) -> Result<T, E>
where
    F: FnMut(usize) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: Send + Clone + Default + 'static,
{
    // beta == 0 means: just one primary
    let hedges = beta.saturating_add(1);
    let mut handles = Vec::with_capacity(hedges);

    for i in 0..hedges {
        let fut = mk_leg(i);
        let h = tokio::spawn(async move {
            let t = timeout(leg_budget, fut).await;
            match t {
                Ok(r) => r,
                Err(_) => Err(timeout_err()),
            }
        });
        handles.push(h);
        if i + 1 < hedges && !stagger.is_zero() {
            sleep(stagger).await;
        }
    }

    let mut last_err = None;
    for h in handles {
        match h.await {
            Ok(Ok(v)) => return Ok(v),
            Ok(Err(e)) => last_err = Some(e),
            Err(_) => {}
        }
    }
    Err(last_err.expect("no legs executed"))
}

// Local error helper for timeouts in the hedge layer.
fn timeout_err<E>() -> E
where
    E: Default,
{
    E::default()
}
