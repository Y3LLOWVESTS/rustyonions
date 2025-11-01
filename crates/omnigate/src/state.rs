//! RO:WHAT   Shared application state container.
//! RO:WHY    Centralize config, policy evaluator, readiness policy, and helpers.

use std::sync::Arc;
use crate::readiness::policy::ReadyPolicy;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<crate::config::Config>,
    pub ready: ReadyPolicy,
    pub policy: Option<ron_policy::Evaluator>,
    pub tenant: Option<String>,
    pub region: Option<String>,
}

impl AppState {
    pub fn new(config: Arc<crate::config::Config>, ready: ReadyPolicy, policy: Option<ron_policy::Evaluator>) -> Arc<Self> {
        Arc::new(Self { config, ready, policy, tenant: None, region: None })
    }

    pub fn tags_for<B>(&self, _req: &axum::http::Request<B>) -> Vec<String> {
        // Future: pull auth claims or route-classifier tags.
        Vec::new()
    }
}
