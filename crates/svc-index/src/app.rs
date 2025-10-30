//! App wiring helpers (bootstrap)

use crate::state::AppState;
use std::sync::Arc;
use tracing::info;

impl AppState {
    pub async fn bootstrap(state: Arc<AppState>) -> Arc<AppState> {
        // Verify deps here later; for MVP, set ready immediately.
        state.health.mark_ready();
        info!("svc-index ready");
        state
    }
}
