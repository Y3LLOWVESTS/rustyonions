// REPLACE ENTIRE FILE with this version
//! RO:WHAT — Mailbox plane helpers (send/recv/ack trio).
//! RO:WHY  — Provide a simple at-least-once-style interface.
//! RO:NOTE — JSON-on-OAP for this increment; retries via transport.

use std::time::{Duration, Instant};

use crate::errors::SdkError;
use crate::metrics::SdkMetrics;
use crate::transport::TransportHandle;
use crate::types::{Capability, IdemKey, Mail, MailInbox, Receipt};

const MAILBOX_SEND_ENDPOINT: &str = "mailbox_send";
const MAILBOX_RECV_ENDPOINT: &str = "mailbox_recv";
const MAILBOX_ACK_ENDPOINT: &str = "mailbox_ack";

// Wire endpoints (gateway-facing).
const EP_SEND: &str = "/mb/send";
const EP_RECV: &str = "/mb/recv";
const EP_ACK: &str = "/mb/ack";

#[derive(serde::Serialize)]
struct SendReq<'a> {
    cap: &'a Capability,
    mail: &'a Mail,
    #[serde(skip_serializing_if = "Option::is_none")]
    idem: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct SendResp {
    receipt: Receipt,
}

#[derive(serde::Serialize)]
struct RecvReq<'a> {
    cap: &'a Capability,
}

#[derive(serde::Deserialize)]
struct RecvResp {
    inbox: Vec<MailInbox>,
}

#[derive(serde::Serialize)]
struct AckReq<'a> {
    cap: &'a Capability,
    receipt: &'a Receipt,
}

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
    let payload = SendReq {
        cap: &cap,
        mail: &msg,
        // FIX: borrow the inner &str via IdemKey::as_str()
        idem: idem.as_ref().map(|k| k.as_str()),
    };

    let raw = transport.call_oap_json(EP_SEND, &payload, deadline).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            let parsed: SendResp = serde_json::from_slice(&bytes)
                .map_err(|e| SdkError::schema_violation("mailbox_send.body", e.to_string()))?;
            metrics.observe_latency(MAILBOX_SEND_ENDPOINT, true, elapsed_ms);
            Ok(parsed.receipt)
        }
        Err(err) => {
            metrics.observe_latency(MAILBOX_SEND_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(MAILBOX_SEND_ENDPOINT, classify(&err));
            Err(err)
        }
    }
}

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
    let payload = RecvReq { cap: &cap };

    let raw = transport.call_oap_json(EP_RECV, &payload, deadline).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match raw {
        Ok(bytes) => {
            let parsed: RecvResp = serde_json::from_slice(&bytes)
                .map_err(|e| SdkError::schema_violation("mailbox_recv.body", e.to_string()))?;
            metrics.observe_latency(MAILBOX_RECV_ENDPOINT, true, elapsed_ms);
            Ok(parsed.inbox)
        }
        Err(err) => {
            metrics.observe_latency(MAILBOX_RECV_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(MAILBOX_RECV_ENDPOINT, classify(&err));
            Err(err)
        }
    }
}

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
    let payload = AckReq {
        cap: &cap,
        receipt: &receipt,
    };

    let res = transport.call_oap_json(EP_ACK, &payload, deadline).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match res {
        Ok(_) => {
            metrics.observe_latency(MAILBOX_ACK_ENDPOINT, true, elapsed_ms);
            Ok(())
        }
        Err(err) => {
            metrics.observe_latency(MAILBOX_ACK_ENDPOINT, false, elapsed_ms);
            metrics.inc_failure(MAILBOX_ACK_ENDPOINT, classify(&err));
            Err(err)
        }
    }
}

fn classify(err: &SdkError) -> &'static str {
    use SdkError::*;
    match err {
        DeadlineExceeded => "deadline",
        Transport(_) => "transport",
        TorUnavailable => "tor",
        Tls => "tls",
        OapViolation { .. } => "oap",
        CapabilityExpired | CapabilityDenied => "capability",
        SchemaViolation { .. } => "schema",
        NotFound => "not_found",
        Conflict => "conflict",
        RateLimited { .. } => "rate_limited",
        Server(_) => "server",
        Unknown(_) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SdkConfig;
    use crate::metrics::NoopSdkMetrics;

    fn dummy_capability() -> Capability {
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
        let transport = TransportHandle::new(SdkConfig::default());
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
        .unwrap_err();
        match err {
            SdkError::SchemaViolation { path, .. } => assert_eq!(path, "mailbox_send.deadline"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn mailbox_recv_rejects_zero_deadline() {
        let transport = TransportHandle::new(SdkConfig::default());
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();

        let err = mailbox_recv(&transport, &metrics, cap, Duration::ZERO)
            .await
            .unwrap_err();
        match err {
            SdkError::SchemaViolation { path, .. } => assert_eq!(path, "mailbox_recv.deadline"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn mailbox_ack_rejects_zero_deadline() {
        let transport = TransportHandle::new(SdkConfig::default());
        let metrics = NoopSdkMetrics;
        let cap = dummy_capability();
        let receipt = dummy_receipt();

        let err = mailbox_ack(&transport, &metrics, cap, receipt, Duration::ZERO)
            .await
            .unwrap_err();
        match err {
            SdkError::SchemaViolation { path, .. } => assert_eq!(path, "mailbox_ack.deadline"),
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
