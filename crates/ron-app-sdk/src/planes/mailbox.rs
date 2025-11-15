//! RO:WHAT — Mailbox plane helpers (send/recv/ack trio).
//! RO:WHY  — Provide a simple at-least-once-style interface for
//!           application messaging, while leaving ordering and
//!           de-duplication policies to the host and higher layers.
//! RO:INTERACTS — Will call into `TransportHandle::call_oap` once
//!                wired; uses `SdkMetrics` for latency/retry metrics.
//! RO:INVARIANTS —
//!   - Idempotency keys (if any) are attached at the SDK level.
//!   - No persistence beyond what the remote mailbox service provides.

use std::time::Duration;

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{Capability, IdemKey, Mail, MailInbox, Receipt};

/// Send a message via the mailbox plane.
pub async fn mailbox_send(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    msg: Mail,
    deadline: Duration,
    idem: Option<IdemKey>,
) -> Result<Receipt, SdkError> {
    let _ = (transport, metrics, cap, msg, deadline, idem);
    todo!("mailbox_send not implemented yet");
}

/// Receive messages from the mailbox plane.
///
/// Concrete semantics (poll vs. long-poll vs. stream) are handled by
/// the underlying service; the SDK just exposes a batch-friendly DTO.
pub async fn mailbox_recv(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    deadline: Duration,
) -> Result<Vec<MailInbox>, SdkError> {
    let _ = (transport, metrics, cap, deadline);
    todo!("mailbox_recv not implemented yet");
}

/// Acknowledge mailbox messages by ID.
pub async fn mailbox_ack(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    receipt: Receipt,
    deadline: Duration,
) -> Result<(), SdkError> {
    let _ = (transport, metrics, cap, receipt, deadline);
    todo!("mailbox_ack not implemented yet");
}
