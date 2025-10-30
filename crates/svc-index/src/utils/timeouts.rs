//! RO:WHAT — Common timeout helpers.
//! RO:WHY  — Keep await-time discipline explicit.

use tokio::time::{timeout, Duration};

pub async fn with_timeout<F, T>(ms: u64, fut: F) -> Result<T, ()>
where
    F: std::future::Future<Output = T>,
{
    timeout(Duration::from_millis(ms), fut)
        .await
        .map_err(|_| ())
}
