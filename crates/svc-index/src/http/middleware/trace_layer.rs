//! Tower HTTP TraceLayer (Axum + tower-http 0.6.x).
//! Minimal sane defaults using DefaultMakeSpan at INFO level.

use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest,
    DefaultOnResponse, TraceLayer,
};
use tracing::Level;

/// Return a concrete, cloneable TraceLayer with the default classifier
/// (treat 5xx as failures) and the default span builder at INFO.
pub fn layer() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    DefaultOnFailure,
> {
    TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::new().level(Level::INFO))
}
