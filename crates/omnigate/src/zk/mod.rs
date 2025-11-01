//! RO:WHAT — Zero-knowledge–ready envelope surface.
//! RO:WHY  — Carve clean seams (read-only vs mutate) + receipts without committing to a prover.
//! RO:INVARIANTS — No proof code here; only types and gating logic surfaces.

pub mod no_mutate;
pub mod receipts;

pub use no_mutate::{OpClass, OpGuard};
pub use receipts::{Receipt, ReceiptId, ReceiptStatus};
