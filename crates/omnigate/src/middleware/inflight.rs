//! RO:WHAT  Global inflight bridge: measure actual concurrent requests across the whole stack.
//! RO:WHY   Guarantees that /readyz sees truthful concurrency no matter which path a request takes.

use crate::readiness::policy::ReadyPolicy;
use axum::{extract::State, middleware::from_fn_with_state, Router};
use std::sync::Arc;

pub async fn inflight_bridge(
    State(rp): State<Arc<ReadyPolicy>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    rp.inc();
    struct Guard(Arc<ReadyPolicy>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.dec();
        }
    }
    let _g = Guard(rp);
    next.run(req).await
}

/// Attach the inflight bridge as the outermost layer.
pub fn attach<S>(router: Router<S>, rp: Arc<ReadyPolicy>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn_with_state(rp, inflight_bridge))
}
