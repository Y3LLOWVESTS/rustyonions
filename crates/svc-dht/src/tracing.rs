//! RO:WHAT — Tracing initialization (env-filter aware)
//! RO:WHY — Uniform logs; Concerns: GOV/DX
//! RO:INTERACTS — all modules via tracing
//! RO:INVARIANTS — JSON optional; defaults to INFO

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,svc_dht=info"));
    tracing_subscriber::registry().with(filter).with(fmt::layer().with_target(false)).init();
}
