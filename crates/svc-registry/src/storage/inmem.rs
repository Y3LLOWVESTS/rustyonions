//! RO:WHAT — In-memory RegistryStore implementation with broadcasted commit events.
//! RO:WHY  — Fast foundation path; persistence can plug in later behind feature flags.
//! RO:INTERACTS — storage::RegistryStore trait; http::{routes,sse}.
//! RO:INVARIANTS — Single-writer discipline by &mut on commit path via Mutex gate.

use super::{Head, RegistryEvent, RegistryStore};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_stream::wrappers::BroadcastStream;

const BROADCAST_CAPACITY: usize = 1024; // bounded, drops oldest when overrun

#[derive(Clone)]
pub struct InMemoryStore {
    head: Arc<RwLock<Head>>,
    tx: broadcast::Sender<RegistryEvent>,
    // Single-writer commit gate; cheap because commit is tiny.
    writer: Arc<Mutex<()>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let head = Head {
            version: 0,
            payload_b3: "b3:0".to_string(),
            committed_at: None,
        };
        Self {
            head: Arc::new(RwLock::new(head)),
            tx,
            writer: Arc::new(Mutex::new(())),
        }
    }
}

#[async_trait::async_trait]
impl RegistryStore for InMemoryStore {
    async fn head(&self) -> Head {
        self.head.read().await.clone()
    }

    async fn commit(&self, payload_b3: String) -> anyhow::Result<Head> {
        anyhow::ensure!(
            payload_b3.starts_with("b3:"),
            "payload must be base64-with-prefix (b3:..)"
        );

        // Single writer section
        let _guard = self.writer.lock().await;

        // Bump version/commit time
        let mut w = self.head.write().await;
        let new_head = Head {
            version: w.version.saturating_add(1),
            payload_b3,
            committed_at: Some(Utc::now()),
        };
        *w = new_head.clone();

        // Broadcast (best-effort; drop if no one is listening)
        let _ = self.tx.send(RegistryEvent::Commit {
            head: new_head.clone(),
        });

        Ok(new_head)
    }

    fn subscribe(&self) -> BroadcastStream<RegistryEvent> {
        BroadcastStream::new(self.tx.subscribe())
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
