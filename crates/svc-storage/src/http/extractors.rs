//! RO:WHAT — Shared app state (Storage handle) for route handlers.
//! RO:WHY  — Axum 0.7 needs state to be Send + Sync + 'static.

use crate::storage::DynStorage;

#[derive(Clone)]
pub struct AppState {
    pub store: DynStorage, // Arc<dyn Storage + Send + Sync + 'static>
}
