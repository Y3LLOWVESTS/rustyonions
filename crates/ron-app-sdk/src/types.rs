//! RO:WHAT — Public DTO surface for ron-app-sdk.
//! RO:WHY  — One stable place to import capability, content IDs,
//!           mailbox DTOs, and SDK-local aliases, without depending
//!           on `ron-proto` directly.
//! RO:INTERACTS — Re-exports canonical DTOs from `ron-proto` and
//!                aliases SDK types like `IdemKey`.
//! RO:INVARIANTS — DTOs remain pure data; no I/O or crypto.
//! RO:SECURITY — Capability is a header DTO only; verification lives
//!               in services like ron-auth.

use crate::idempotency::IdempotencyKey;

// Re-export canonical DTOs from `ron-proto`.
#[allow(unused_imports)] // Intentionally surfaced for callers even if not used here yet.
pub use ron_proto::{
    Ack as MailboxAck, // canonical mailbox receipt
    CapTokenHdr,       // capability header (macaroon-style)
    ContentId,         // BLAKE3 content address
    ManifestV1,        // exposed for future SDK callers
    NameRef,           // index key (logical name/alias)
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

/// Byte range used by the edge plane (inclusive start, inclusive end).
///
/// Defined locally so the SDK doesn’t depend on svc-edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    /// Start offset.
    pub start: u64,
    /// End offset (inclusive).
    pub end: u64,
}

impl ByteRange {
    /// Inclusive length; `{ start: 0, end: 9 }` → `10`.
    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::ByteRange;

    #[test]
    fn byte_range_len_is_inclusive() {
        let r = ByteRange { start: 0, end: 9 };
        assert_eq!(r.len(), 10);
    }
}
