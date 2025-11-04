use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

pub fn init() {
    let filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,axum=warn,tower_http=warn".into());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_span_events(FmtSpan::CLOSE)
        .json()
        .flatten_event(true)
        .init();
}
