//! RO:WHAT — Public DTO surface for ron-app-sdk.
//! RO:WHY  — Give applications a single place to import capability,
//!           content-addressed IDs, mailbox types, etc., without having
//!           to depend on `ron-proto` directly.
//! RO:INTERACTS — Re-exports canonical DTOs from `ron-proto` and
//!                aliases the SDK’s own idempotency key type.
//! RO:INVARIANTS —
//!   - DTOs remain pure data (no I/O, no crypto).
//!   - Types here are stable and SemVer-protected via api-history.
//!   - We avoid stringly-typed IDs where possible.
//! RO:SECURITY — Capability types here are header-DTOs only; actual
//!               verification lives in auth services (ron-auth, etc.).

use crate::idempotency::IdempotencyKey;

// Re-export canonical DTOs from `ron-proto`.
pub use ron_proto::{
    Ack as MailboxAck,
    CapTokenHdr,
    ContentId,
    ManifestV1,
    NameRef,
    Recv as MailboxRecv,
    Send as MailboxSend,
};

/// Capability token header (macaroon-style claims, no signature).
pub type Capability = CapTokenHdr;

/// Canonical BLAKE3 content address used by storage/index planes.
pub type AddrB3 = ContentId;

/// Logical index key used by the index plane.
pub type IndexKey = NameRef;

/// SDK-level alias for idempotency keys (SDK-owned type).
pub type IdemKey = IdempotencyKey;

/// Mail payload used when *sending* via the mailbox plane.
pub type Mail = MailboxSend;

/// Mail payload delivered when *receiving* from the mailbox plane.
pub type MailInbox = MailboxRecv;

/// Receipt/acknowledgement from mailbox operations (canonical name).
pub type Receipt = MailboxAck;

/// Short alias matching the docs (`Ack`).
pub type Ack = Receipt;

/// Single byte range (inclusive start, inclusive end).
///
/// This mirrors the shape used in `svc-edge` but is defined locally so
/// the SDK does not have to depend on that crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// Start offset.
    pub start: u64,
    /// End offset (inclusive).
    pub end: u64,
}

impl ByteRange {
    /// Length of the range in bytes, saturating.
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start) + 1
    }
}
