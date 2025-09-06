// crates/gateway/src/state.rs
#![forbid(unsafe_code)]

use std::sync::Arc;

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;

/// Shared application state carried through the gateway.
///
/// Wrap clients in Arc so clones are cheap.
#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)]
    pub index: Arc<IndexClient>,
    pub overlay: Arc<OverlayClient>,
    pub enforce_payments: bool,
}

impl AppState {
    pub fn new(index: IndexClient, overlay: OverlayClient, enforce_payments: bool) -> Self {
        Self {
            index: Arc::new(index),
            overlay: Arc::new(overlay),
            enforce_payments,
        }
    }
}
