//! RO:WHAT — Macronode managed services surface.
//! RO:WHY  — Single place to define which internal services (gateway, overlay,
//!           storage, index, mailbox, dht, etc.) this node composes.
//! RO:INVARIANTS —
//!   - Slice 1 only exposes `spawn_all()` and per-service stubs.
//!   - Future slices will add real service wiring and health reporting.

pub mod challenge_evidence;
pub mod delivery_evidence;
pub mod evidence_outbox;
pub mod moderation_policy;
pub mod policy_evidence;
pub mod ports;
pub mod prune;
pub mod range_repair_evidence;
pub mod spawn;
pub mod svc_dht;
pub mod svc_gateway;
pub mod svc_index;
pub mod svc_mailbox;
pub mod svc_overlay;
pub mod svc_storage;

pub use spawn::spawn_all;
