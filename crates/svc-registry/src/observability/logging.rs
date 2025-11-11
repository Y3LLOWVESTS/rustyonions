//! Tracing/logging init (JSON by default).
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing() {
    let env = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,axum=warn,tower_http=warn,svc_registry=info"));
    let fmt = fmt::layer().json().flatten_event(true).with_target(true);
    tracing_subscriber::registry().with(env).with(fmt).init();
}
