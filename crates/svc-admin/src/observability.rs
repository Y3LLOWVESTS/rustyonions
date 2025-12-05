use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,svc_admin=info,axum=warn,tower_http=warn"));

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .finish()
        .init();
}
