#![allow(clippy::doc_lazy_continuation, clippy::doc_overindented_list_items)]
#![forbid(unsafe_code)]
//! ron-app-sdk — Application SDK for RON-CORE.
//!
//! RO:WHAT — Tiny async client façade over Micronode/Macronode node
//!           surfaces (edge, storage, mailbox, index).
//! RO:WHY  — Give apps a boring, well-typed, capability-first client
//!           with consistent retries, deadlines, and DTO hygiene.
//! RO:INTERACTS —
//!   - `config` for `SdkConfig` + env loading/validation.
//!   - `transport` for OAP/1 calls (TLS/Tor).
//!   - `planes::*` for storage/edge/mailbox/index helpers.
//!   - `metrics`/`tracing` for observability hooks.
//! RO:INVARIANTS —
//!   - All outbound calls carry a capability (I-2).
//!   - No semantic branching on `NodeProfile` (I-1).
//!   - OAP frame cap is enforced in the transport layer (I-3).

pub mod cache;
pub mod config;
mod context;
pub mod errors;
mod idempotency;
pub mod metrics;
mod ready;
mod retry;
mod tracing;
pub mod transport;
mod types;

// Planes: defined as a nested module so we can keep each plane in a
// dedicated file under `src/planes/`.
pub mod planes {
    pub mod edge;
    pub mod index;
    pub mod mailbox;
    pub mod storage;
}

pub use context::{NodeProfile, SdkContext};
pub use errors::{RetryClass, SdkError};
pub use ready::{check_ready, ReadyReport};
pub use types::{Ack, AddrB3, ByteRange, Capability, IdemKey, IndexKey, Mail, MailInbox, Receipt};

pub use config::{
    CacheCfg, IdemCfg, Jitter, PqMode, Redaction, SdkConfig, Timeouts, TorCfg, TracingCfg,
    Transport,
};

pub use metrics::{NoopSdkMetrics, SdkMetrics};

use bytes::Bytes;
use std::time::Duration;

use context::NodeProfile as CtxProfile;
use transport::TransportHandle;

/// High-level async client façade for RON-CORE nodes.
///
/// Constructed from `SdkConfig` and a (future) handshake, and then
/// used to issue calls to the various planes (storage, edge, mailbox,
/// index) with consistent deadlines/retries/error handling.
pub struct RonAppSdk {
    transport: TransportHandle,
    ctx: SdkContext,
    metrics: Box<dyn SdkMetrics>,
}

impl RonAppSdk {
    /// Create a new SDK client from configuration.
    ///
    /// This validates the config and prepares internal handles. In
    /// future revisions it may perform a light handshake to fill in
    /// `SdkContext` with accurate profile/amnesia metadata.
    pub async fn new(cfg: SdkConfig) -> Result<RonAppSdk, SdkError> {
        // Fail-closed on invalid config.
        cfg.validate()
            .map_err(|err| SdkError::schema_violation("config", err.to_string()))?;

        let transport = TransportHandle::new(cfg);
        // Until a real handshake exists, assume Micronode + non-amnesia.
        let ctx = SdkContext::new(CtxProfile::Micronode, false);

        Ok(RonAppSdk {
            transport,
            ctx,
            metrics: Box::<NoopSdkMetrics>::default(),
        })
    }

    /// Expose the SDK context (profile + amnesia hint).
    pub fn context(&self) -> SdkContext {
        self.ctx
    }

    /// Get a reference to the metrics sink.
    pub fn metrics(&self) -> &dyn SdkMetrics {
        &*self.metrics
    }

    /// Mutably access the metrics sink.
    pub fn metrics_mut(&mut self) -> &mut dyn SdkMetrics {
        &mut *self.metrics
    }

    /// Replace the metrics sink with a custom implementation.
    pub fn set_metrics<M>(&mut self, metrics: M)
    where
        M: SdkMetrics + 'static,
    {
        self.metrics = Box::new(metrics);
    }

    // -------------- Mailbox plane --------------

    /// Send a message via the mailbox plane.
    pub async fn mailbox_send(
        &self,
        cap: Capability,
        msg: Mail,
        deadline: Duration,
        idem: Option<IdemKey>,
    ) -> Result<Receipt, SdkError> {
        planes::mailbox::mailbox_send(&self.transport, &*self.metrics, cap, msg, deadline, idem)
            .await
    }

    /// Receive messages from the mailbox plane.
    pub async fn mailbox_recv(
        &self,
        cap: Capability,
        deadline: Duration,
    ) -> Result<Vec<MailInbox>, SdkError> {
        planes::mailbox::mailbox_recv(&self.transport, &*self.metrics, cap, deadline).await
    }

    /// Acknowledge mailbox messages.
    pub async fn mailbox_ack(
        &self,
        cap: Capability,
        ack: Receipt,
        deadline: Duration,
    ) -> Result<(), SdkError> {
        planes::mailbox::mailbox_ack(&self.transport, &*self.metrics, cap, ack, deadline).await
    }

    // -------------- Edge plane --------------

    /// Fetch an edge resource with an optional byte range.
    pub async fn edge_get(
        &self,
        cap: Capability,
        path: &str,
        range: Option<ByteRange>,
        deadline: Duration,
    ) -> Result<Bytes, SdkError> {
        planes::edge::edge_get(&self.transport, &*self.metrics, cap, path, range, deadline).await
    }

    // -------------- Storage plane --------------

    /// Perform a content-addressed GET from the storage plane.
    ///
    /// `addr_b3_hex` must be a `"b3:<64 hex>"` string; invalid values
    /// are reported as `SdkError::SchemaViolation`.
    pub async fn storage_get(
        &self,
        cap: Capability,
        addr_b3_hex: &str,
        deadline: Duration,
    ) -> Result<Bytes, SdkError> {
        let addr = AddrB3::parse(addr_b3_hex)
            .map_err(|err| SdkError::schema_violation("addr_b3", err.to_string()))?;

        planes::storage::storage_get(&self.transport, &*self.metrics, cap, &addr, deadline).await
    }

    /// Perform a content-addressed PUT to the storage plane.
    pub async fn storage_put(
        &self,
        cap: Capability,
        blob: Bytes,
        deadline: Duration,
        idem: Option<IdemKey>,
    ) -> Result<AddrB3, SdkError> {
        planes::storage::storage_put(&self.transport, &*self.metrics, cap, blob, deadline, idem)
            .await
    }

    // -------------- Index plane --------------

    /// Resolve a logical index key into a content address.
    pub async fn index_resolve(
        &self,
        cap: Capability,
        key: &IndexKey,
        deadline: Duration,
    ) -> Result<AddrB3, SdkError> {
        planes::index::index_resolve(&self.transport, &*self.metrics, cap, key, deadline).await
    }
}
