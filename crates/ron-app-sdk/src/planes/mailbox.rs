//! RO:WHAT — Mailbox plane helpers (send/recv/ack trio).
//! RO:WHY  — Provide a simple at-least-once-style interface for
//!           application messaging, while leaving ordering and
//!           de-duplication policies to the host and higher layers.
//! RO:INTERACTS — Will eventually call into `TransportHandle::call_oap`
//!                once wired; uses `SdkMetrics` for latency/failure
//!                tracking.
//! RO:INVARIANTS —
//!   - Idempotency keys (if any) are attached at the SDK level.
//!   - No panics: all failures surfaced as `SdkError`.
//!   - No local persistence; all state is remote (svc-mailbox).
//!
//! NOTE: In this increment the mailbox plane is *shape-complete* but
//! not yet wired to the network. All functions currently return a
//! typed `SdkError::Unknown` instead of `todo!()` panics. This keeps
//! the SDK link-safe and honest until we connect to the real service.

use std::time::{Duration, Instant};

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{Capability, IdemKey, Mail, MailInbox, Receipt};

/// Metric endpoint labels for this plane.
/// Keep these low-cardinality.
const MAILBOX_SEND_ENDPOINT: &str = "mailbox_send";
const MAILBOX_RECV_ENDPOINT: &str = "mailbox_recv";
const MAILBOX_ACK_ENDPOINT: &str = "mailbox_ack";

/// Send a message via the mailbox plane.
///
/// This is intentionally a *thin* façade. In this increment it:
/// - validates the deadline is non-zero,
/// - records a failure metric,
/// - returns a typed `SdkError::Unknown` indicating that the mailbox
///   surface is not wired yet.
///
/// Once the transport and svc-mailbox interop are ready, this function
/// will:
/// - build the appropriate OAP/1 envelope,
/// - attach the capability and (optionally) an idempotency key,
/// - call `TransportHandle::call_oap`,
/// - parse a `Receipt` (at-least-once semantics).
pub async fn mailbox_send(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    msg: Mail,
    deadline: Duration,
    idem: Option<IdemKey>,
) -> Result<Receipt, SdkError> {
    if deadline == Duration::from_millis(0) {
        return Err(SdkError::schema_violation(
            "mailbox_send.deadline",
            "deadline must be > 0",
        ));
    }

    let start = Instant::now();

    // For now, we do not perform any network I/O. We simply record a
    // failure metric and return a typed error stating that the mailbox
    // plane has not been wired up yet.
    let _ = (transport, cap, msg, idem);

    let err = SdkError::Unknown("mailbox_send not wired to transport yet".to_string());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    metrics.observe_latency(MAILBOX_SEND_ENDPOINT, false, elapsed_ms);
    metrics.inc_failure(MAILBOX_SEND_ENDPOINT, "not_wired");

    Err(err)
}

/// Receive messages from the mailbox plane.
///
/// Concrete semantics (poll vs. long-poll vs. stream) are handled by
/// the underlying service; the SDK just exposes a batch-friendly DTO.
///
/// In this increment, this is a non-panicking stub:
/// - validates the deadline,
/// - emits a failure metric,
/// - returns a typed `SdkError::Unknown`.
pub async fn mailbox_recv(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    deadline: Duration,
) -> Result<Vec<MailInbox>, SdkError> {
    if deadline == Duration::from_millis(0) {
        return Err(SdkError::schema_violation(
            "mailbox_recv.deadline",
            "deadline must be > 0",
        ));
    }

    let start = Instant::now();

    let _ = (transport, cap);

    let err = SdkError::Unknown("mailbox_recv not wired to transport yet".to_string());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    metrics.observe_latency(MAILBOX_RECV_ENDPOINT, false, elapsed_ms);
    metrics.inc_failure(MAILBOX_RECV_ENDPOINT, "not_wired");

    Err(err)
}

/// Acknowledge mailbox messages via their `Receipt`.
///
/// In this increment, this is also a non-panicking stub:
/// - validates the deadline,
/// - emits a failure metric,
/// - returns a typed `SdkError::Unknown`.
///
/// Once wired, this will:
/// - build an ACK request from the `Receipt`,
/// - call `TransportHandle::call_oap`,
/// - interpret success/failure coherently with the rest of the plane.
pub async fn mailbox_ack(
    transport: &TransportHandle,
    metrics: &dyn SdkMetrics,
    cap: Capability,
    receipt: Receipt,
    deadline: Duration,
) -> Result<(), SdkError> {
    if deadline == Duration::from_millis(0) {
        return Err(SdkError::schema_violation(
            "mailbox_ack.deadline",
            "deadline must be > 0",
        ));
    }

    let start = Instant::now();

    let _ = (transport, cap, receipt);

    let err = SdkError::Unknown("mailbox_ack not wired to transport yet".to_string());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    metrics.observe_latency(MAILBOX_ACK_ENDPOINT, false, elapsed_ms);
    metrics.inc_failure(MAILBOX_ACK_ENDPOINT, "not_wired");

    Err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::config::SdkConfig;
    use crate::metrics::NoopSdkMetrics;
    use crate::transport::TransportHandle;
    use crate::types::Capability;

    fn dummy_capability() -> Capability {
        // Minimal stand-in capability for exercising client-side
        // invariants. The real semantics are enforced by auth services.
        Capability {
            subject: "test-subject".to_string(),
            scope: "test-scope".to_string(),
            issued_at: 0,
            expires_at: u64::MAX,
            caveats: Vec::new(),
        }
    }

    fn dummy_mail() -> Mail {
        Mail {
            msg_id: "msg-1".to_string(),
            to: "dest".to_string(),
            kind: "test-kind".to_string(),
            payload: Vec::new(),
            idempotency_key: None,
        }
    }

    fn dummy_receipt() -> Receipt {
        Receipt {
            msg_id: "ack-1".to_string(),
            ok: true,
            error: None,
        }
    }

    #[tokio::test]
    async fn mailbox_send_rejects_zero_deadline() {
        let cfg = SdkConfig::default();
        let transport = TransportHandle::new(cfg);
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();
        let mail = dummy_mail();

        let err = mailbox_send(
            &transport,
            &metrics,
            cap,
            mail,
            Duration::from_millis(0),
            None,
        )
        .await
        .expect_err("expected schema_violation for zero deadline");

        match err {
            SdkError::SchemaViolation { path, .. } => {
                assert_eq!(path, "mailbox_send.deadline");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn mailbox_recv_rejects_zero_deadline() {
        let cfg = SdkConfig::default();
        let transport = TransportHandle::new(cfg);
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();

        let err =
            mailbox_recv(&transport, &metrics, cap, Duration::from_millis(0))
                .await
                .expect_err("expected schema_violation for zero deadline");

        match err {
            SdkError::SchemaViolation { path, .. } => {
                assert_eq!(path, "mailbox_recv.deadline");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn mailbox_ack_rejects_zero_deadline() {
        let cfg = SdkConfig::default();
        let transport = TransportHandle::new(cfg);
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();
        let receipt = dummy_receipt();

        let err = mailbox_ack(
            &transport,
            &metrics,
            cap,
            receipt,
            Duration::from_millis(0),
        )
        .await
        .expect_err("expected schema_violation for zero deadline");

        match err {
            SdkError::SchemaViolation { path, .. } => {
                assert_eq!(path, "mailbox_ack.deadline");
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }
}
