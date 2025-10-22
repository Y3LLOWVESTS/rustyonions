//! RO:WHAT — KernelEvent enum shared across the kernel bus.
//! RO:WHY  — Central, stable event vocabulary for kernel interactions.
//! RO:INVARIANTS — Backward-compatible additions only; no breaking renames.

/// Events published on the kernel bus.
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// Health probe from a service (declarative signal).
    Health {
        /// Service name emitting the health status.
        service: String,
        /// Whether the service currently reports healthy.
        ok: bool,
    },
    /// Configuration updated to a monotonic version.
    ConfigUpdated {
        /// Version that became active.
        version: u64,
    },
    /// A supervised service crashed (supervisor should record+restart).
    ServiceCrashed {
        /// Service name that crashed.
        service: String,
    },
    /// Request orderly shutdown of the kernel.
    Shutdown,
}
