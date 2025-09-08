// crates/svc-omnigate/src/mailbox.rs
#![forbid(unsafe_code)]

use anyhow::{anyhow, Result};
use bytes::Bytes;
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::debug;

/// App protocol id for Mailbox.
pub const MAILBOX_APP_PROTO_ID: u16 = 0x0201;

/// Bronze semantics:
/// - At-least-once via visibility timeout.
/// - ULID message ids.
/// - Best-effort FIFO per topic.
/// - In-memory store (process lifetime). Good enough for the pilot; can swap to sled/file later.
pub struct Mailbox {
    inner: Mutex<Inner>,
    visibility: Duration,
}

struct Inner {
    /// topic -> queue of msg_ids
    queues: HashMap<String, VecDeque<String>>,
    /// msg_id -> message data
    messages: HashMap<String, Message>,
    /// idempotency_key -> msg_id
    idempotency: HashMap<String, String>,
}

struct Message {
    topic: String,
    body: Bytes,
    enqueued_at: Instant,
    leased_until: Option<Instant>,
}

impl Mailbox {
    pub fn new(visibility: Duration) -> Self {
        Self {
            inner: Mutex::new(Inner {
                queues: HashMap::new(),
                messages: HashMap::new(),
                idempotency: HashMap::new(),
            }),
            visibility,
        }
    }

    pub async fn send(&self, topic: &str, body: Bytes, idempotency_key: Option<String>) -> Result<String> {
        let mut g = self.inner.lock().await;

        if let Some(k) = idempotency_key.as_ref() {
            if let Some(existing) = g.idempotency.get(k) {
                // Return existing id for duplicate send.
                return Ok(existing.clone());
            }
        }

        let id = ulid::Ulid::new().to_string();

        let msg = Message {
            topic: topic.to_string(),
            body,
            enqueued_at: Instant::now(),
            leased_until: None,
        };

        // Borrow queues only for the push, then release before touching other fields.
        {
            let q = g.queues.entry(topic.to_string()).or_default();
            q.push_back(id.clone());
        }
        g.messages.insert(id.clone(), msg);

        if let Some(k) = idempotency_key {
            g.idempotency.insert(k, id.clone());
        }

        Ok(id)
    }

    /// Return up to `max` messages for topic, setting a visibility timeout.
    /// Redelivery: on each call we sweep expired leases back into the queue.
    pub async fn recv(&self, topic: &str, max: usize) -> Result<Vec<(String, Bytes)>> {
        let mut g = self.inner.lock().await;

        // Sweep expired leases back to queue (lazy redelivery).
        self.sweep_expired_locked(topic, &mut g);

        let mut out = Vec::with_capacity(max);

        for _ in 0..max {
            // Pop an id in a short scope so the mutable borrow of queues ends
            // before we mutably borrow g.messages.
            let id_opt = {
                let q = g.queues.entry(topic.to_string()).or_default();
                q.pop_front()
            };

            let Some(id) = id_opt else { break; };

            if let Some(m) = g.messages.get_mut(&id) {
                // Lease it
                m.leased_until = Some(Instant::now() + self.visibility);
                out.push((id, m.body.clone()));
            } else {
                // If message record vanished (shouldn't happen in normal flow), skip.
                continue;
            }
        }

        Ok(out)
    }

    /// ACK a message id, removing it from the store.
    pub async fn ack(&self, msg_id: &str) -> Result<()> {
        let mut g = self.inner.lock().await;

        if let Some(msg) = g.messages.remove(msg_id) {
            // Use enqueued_at for simple dwell-time telemetry to avoid dead_code on the field.
            let dwell = Instant::now().saturating_duration_since(msg.enqueued_at);
            debug!("ack {msg_id} dwell_ms={}", dwell.as_millis());

            // Remove any stray queued occurrences (best-effort) in a short scope.
            if let Some(q) = g.queues.get_mut(&msg.topic) {
                if let Some(pos) = q.iter().position(|x| x == msg_id) {
                    q.remove(pos);
                }
            }
            Ok(())
        } else {
            Err(anyhow!("not_found"))
        }
    }

    fn sweep_expired_locked(&self, topic: &str, g: &mut Inner) {
        let now = Instant::now();
        // Collect expired leases for this topic first.
        let expired: Vec<String> = g
            .messages
            .iter()
            .filter_map(|(id, m)| {
                if m.topic == topic {
                    if let Some(deadline) = m.leased_until {
                        if deadline <= now {
                            return Some(id.clone());
                        }
                    }
                }
                None
            })
            .collect();

        if expired.is_empty() {
            return;
        }

        // Then push them back into the queue.
        let q = g.queues.entry(topic.to_string()).or_default();
        for id in expired {
            if let Some(m) = g.messages.get_mut(&id) {
                m.leased_until = None;
                q.push_back(id.clone());
                debug!("redeliver {}", id);
            }
        }
    }
}
