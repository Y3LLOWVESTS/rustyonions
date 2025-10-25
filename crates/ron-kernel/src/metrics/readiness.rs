use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use axum::{response::IntoResponse, Json};

use crate::metrics::health::HealthState;

#[derive(Clone)]
pub struct Readiness {
    health: HealthState,
    config_loaded: Arc<AtomicBool>,
}

impl Readiness {
    pub fn new(health: HealthState) -> Self {
        Self {
            health,
            config_loaded: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn set_config_loaded(&self, yes: bool) {
        self.config_loaded.store(yes, Ordering::Relaxed);
    }

    pub fn ready(&self) -> bool {
        self.config_loaded.load(Ordering::Relaxed) && self.health.all_ready()
    }
}

pub async fn readyz_handler(state: Readiness) -> axum::response::Response {
    if state.ready() {
        (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "ready": true })),
        )
            .into_response()
    } else {
        let mut missing = vec![];
        if !state
            .config_loaded
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            missing.push("config");
        }
        if !state.health.all_ready() {
            missing.push("services");
        }
        let mut resp = axum::response::Response::new(
            serde_json::to_vec(&serde_json::json!({ "missing": missing }))
                .unwrap()
                .into(),
        );
        *resp.status_mut() = axum::http::StatusCode::SERVICE_UNAVAILABLE;
        resp.headers_mut()
            .insert("Retry-After", axum::http::HeaderValue::from_static("3"));
        resp
    }
}
