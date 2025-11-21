//! RO:WHAT — Typed events emitted on Macronode’s internal bus.
//! RO:WHY  — Keep the event surface small and aligned with the kernel’s
//!           `KernelEvent` so services can reason about a single enum.
//! RO:INVARIANTS —
//!   - We do not invent new event types here; we alias the canonical
//!     `ron-kernel::KernelEvent` so the control plane stays coherent.
//!   - Higher-level “topic groups” (overlay, storage, etc.) are modeled
//!     as variants/fields on `KernelEvent` in `ron-kernel`, not here.
//!
//! RO:INTERACTS —
//!   - `crate::bus::NodeBus` publishes/subscribes these events.
//!   - Supervisor and services will eventually publish things like
//!     `ConfigUpdated`, `ServiceCrashed`, etc. onto this bus.

use ron_kernel::KernelEvent;

/// Node-level event type carried by the Macronode bus.
///
/// For now this is *exactly* the kernel’s `KernelEvent` so there is a
/// single, shared event taxonomy across the project.
pub type NodeEvent = KernelEvent;

// Re-export for convenience so callers can `use crate::bus::KernelEvent;`
/// Re-export of the canonical kernel event type.
pub use ron_kernel::KernelEvent;
