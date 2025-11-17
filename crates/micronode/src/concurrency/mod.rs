//! RO:WHAT — Concurrency plane for Micronode (limits, registry, backpressure labels).
//! RO:WHY  — Central home for per-route concurrency caps and work queues.
//! RO:INTERACTS — `layers::concurrency::ConcurrencyLayer`, HTTP router, future worker pools.
//! RO:INVARIANTS — Bounded, non-blocking admission; prefer shed (429) over buffering;
//!                 no locks held across `.await`.
//! RO:TEST — Exercised indirectly by `tests/backpressure.rs`.

pub mod backpressure;
pub mod registry;
pub mod shutdown;

/// Named concurrency limit for a class of operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConcurrencyLimit {
    /// Stable name used for metrics labels and registry keys.
    pub name: &'static str,
    /// Maximum number of inflight operations allowed for this class.
    pub max_inflight: usize,
}

/// Static concurrency configuration for Micronode.
///
/// Today this is **in-code** and sized for a small, single-node Micronode.
/// Later we can source this from `Config` / env once the config plane grows.
#[derive(Debug, Clone)]
pub struct ConcurrencyConfig {
    pub http_admin: ConcurrencyLimit,
    pub http_dev_echo: ConcurrencyLimit,
    pub http_kv: ConcurrencyLimit,
    pub http_facets: ConcurrencyLimit,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            // Admin is cheap and should basically never be the bottleneck.
            http_admin: ConcurrencyLimit { name: HTTP_ADMIN_BUDGET, max_inflight: 64 },
            // Dev echo is purely DX; keep it modest so it cannot starve real traffic.
            http_dev_echo: ConcurrencyLimit { name: HTTP_DEV_ECHO_BUDGET, max_inflight: 32 },
            // KV is the main hot-path for Micronode; give it a healthier budget.
            http_kv: ConcurrencyLimit { name: HTTP_KV_BUDGET, max_inflight: 256 },
            // Facets will typically fan back into KV and CAS; sized conservatively for now.
            http_facets: ConcurrencyLimit { name: HTTP_FACETS_BUDGET, max_inflight: 128 },
        }
    }
}

impl ConcurrencyConfig {
    /// Lookup a limit by its logical name.
    ///
    /// This is primarily intended for tests and for any code that wants to
    /// introspect the static configuration.
    pub fn limit_for(&self, name: &str) -> Option<ConcurrencyLimit> {
        match name {
            HTTP_ADMIN_BUDGET => Some(self.http_admin),
            HTTP_DEV_ECHO_BUDGET => Some(self.http_dev_echo),
            HTTP_KV_BUDGET => Some(self.http_kv),
            HTTP_FACETS_BUDGET => Some(self.http_facets),
            _ => None,
        }
    }

    /// Build a `ConcurrencyRegistry` from this configuration.
    pub fn build_registry(&self) -> ConcurrencyRegistry {
        ConcurrencyRegistry::from_config(self)
    }
}

// Stable string constants for concurrency budget names. These are meant to be
// shared between the registry, metrics labels, and any future worker pools.

/// Budget name for admin-plane HTTP endpoints (`/healthz`, `/readyz`, `/metrics`, `/version`).
pub const HTTP_ADMIN_BUDGET: &str = "http_admin";

/// Budget name for dev DX endpoints such as `/dev/echo`.
pub const HTTP_DEV_ECHO_BUDGET: &str = "http_dev_echo";

/// Budget name for KV HTTP endpoints (`/v1/kv/...`).
pub const HTTP_KV_BUDGET: &str = "http_kv";

/// Budget name for facet-exposed HTTP endpoints (`/facets/...`).
pub const HTTP_FACETS_BUDGET: &str = "http_facets";

pub use registry::ConcurrencyRegistry;
