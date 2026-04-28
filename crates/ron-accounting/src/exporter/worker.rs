//! RO:WHAT — Async helper functions for ordered exporter drain operations.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/PERF. Keeps single-writer exporter boundary explicit.
//! RO:INTERACTS — Exporter trait, ExporterRouter, SealedSlice, metrics export latency.
//! RO:INVARIANTS — ACK advances order; failed put NACKs so slices can retry; Duplicate is success.
//! RO:METRICS — caller observes ExportReport.latency and status.
//! RO:CONFIG — retry policy mirrors config::ExporterConfig without owning a runtime.
//! RO:SECURITY — concrete exporter enforces capabilities.
//! RO:TEST — examples/export_to_mock.rs and exporter tests.

use std::time::{Duration, Instant};

use crate::{
    accounting::SealedSlice,
    config::schema::ExporterConfig,
    errors::Result,
    exporter::{Ack, Exporter, ExporterRouter, StreamKey},
};

/// Export attempt report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReport {
    /// Downstream ACK state.
    pub status: Ack,
    /// Elapsed wall-clock latency.
    pub latency: Duration,
}

/// Pure retry/backoff policy for hosts that drive exporter loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportRetryPolicy {
    /// Maximum total attempts, including the first attempt.
    pub max_attempts: u32,
    /// Base backoff in milliseconds.
    pub backoff_base_ms: u32,
    /// Maximum backoff in milliseconds.
    pub backoff_cap_ms: u32,
    /// Whether the host should apply full jitter around the returned cap.
    pub jitter: bool,
}

impl ExportRetryPolicy {
    /// Build a retry policy from exporter config and an explicit attempt cap.
    pub fn from_config(cfg: &ExporterConfig, max_attempts: u32) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            backoff_base_ms: cfg.backoff_base_ms,
            backoff_cap_ms: cfg.backoff_cap_ms.max(cfg.backoff_base_ms),
            jitter: cfg.jitter,
        }
    }

    /// Return the delay cap after a failed attempt.
    ///
    /// `failed_attempt` is 1-based. If this returns `None`, the caller should stop
    /// retrying and surface the failure.
    pub fn retry_delay_ms(&self, failed_attempt: u32) -> Option<u64> {
        if failed_attempt == 0 || failed_attempt >= self.max_attempts {
            return None;
        }

        let shift = failed_attempt.saturating_sub(1).min(31);
        let factor = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
        let base = u64::from(self.backoff_base_ms.max(1));
        let cap = u64::from(self.backoff_cap_ms.max(self.backoff_base_ms.max(1)));

        Some(base.saturating_mul(factor).min(cap))
    }
}

impl Default for ExportRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_base_ms: 50,
            backoff_cap_ms: 5_000,
            jitter: true,
        }
    }
}

/// Export one sealed slice through a concrete exporter.
pub async fn export_one<E>(exporter: &E, slice: &SealedSlice) -> Result<ExportReport>
where
    E: Exporter + ?Sized,
{
    let start = Instant::now();
    let status = exporter.put(slice).await?;
    Ok(ExportReport {
        status,
        latency: start.elapsed(),
    })
}

/// Lease, export, and ACK/NACK one slice from a router.
///
/// On success, including `Ack::Duplicate`, the stream sequence is ACKed. On
/// failure, the slice is NACKed so a caller can retry later.
pub async fn export_next<E>(
    router: &mut ExporterRouter,
    exporter: &E,
) -> Result<Option<ExportReport>>
where
    E: Exporter + ?Sized,
{
    let Some(slice) = router.lease_next() else {
        return Ok(None);
    };

    let key = StreamKey::from(&slice);
    let seq = slice.id.seq;

    match export_one(exporter, &slice).await {
        Ok(report) => {
            router.ack(key, seq)?;
            Ok(Some(report))
        }
        Err(err) => {
            router.nack(key, seq)?;
            Err(err)
        }
    }
}

/// Export up to `max_items` currently queued slices.
///
/// This helper does not sleep, spawn, or own a runtime. Hosts/services decide
/// retry timing, cancellation, readiness, and metrics around this primitive.
pub async fn export_until_blocked<E>(
    router: &mut ExporterRouter,
    exporter: &E,
    max_items: usize,
) -> Result<Vec<ExportReport>>
where
    E: Exporter + ?Sized,
{
    let mut reports = Vec::new();

    for _ in 0..max_items {
        let Some(report) = export_next(router, exporter).await? else {
            break;
        };
        reports.push(report);
    }

    Ok(reports)
}
