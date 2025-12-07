// crates/svc-admin/src/observability.rs

use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::util::SubscriberInitExt;

/// Initialize tracing for svc-admin.
///
/// This is intentionally tolerant of being called multiple times (eg. in
/// tests that spin up more than one server in a single process). If a global
/// subscriber is already installed, we just ignore the error.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Default to something sensible for local dev/tests
            EnvFilter::new("info,svc_admin=info,axum=warn,tower_http=warn")
        });

    // Use `try_init()` so a second call does *not* panic with
    // `SetGlobalDefaultError("a global default trace dispatcher has already been set")`.
    let _ = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init();
}
