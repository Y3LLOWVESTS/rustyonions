//! RO:WHAT — Tracing init + HTTP trace layer.
//! RO:WHY  — Uniform logs + RED metrics; Concerns: OBS/PERF.
//! RO:INVARIANTS — Bounded labels; no PII in logs.

use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,axum=info"));
    fmt().with_env_filter(filter).compact().init();
}

pub fn http_trace_layer(
) -> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
}
