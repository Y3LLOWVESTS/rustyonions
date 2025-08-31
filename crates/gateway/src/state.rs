#![allow(dead_code)]
#![forbid(unsafe_code)]

use crate::index_client::IndexClient;
use crate::overlay_client::OverlayClient;

#[derive(Clone)]
pub struct AppState {
    pub index: IndexClient,
    pub overlay: OverlayClient,
    pub enforce_payments: bool,
}

impl AppState {
    pub fn new(index: IndexClient, overlay: OverlayClient, enforce_payments: bool) -> Self {
        Self {
            index,
            overlay,
            enforce_payments,
        }
    }
}
