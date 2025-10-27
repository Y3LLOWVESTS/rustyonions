//! RO:WHAT — Small runtime “tuning knobs” for svc-overlay.
//! RO:WHY  — Allow ops/tests to tune without rebuilds until full Config plumbing lands.
//!
//! Env vars (optional):
//! - RON_OVERLAY_TX_WATERMARK : i64   — default 96 (of 128-slot TX queue)
//! - RON_OVERLAY_HANDSHAKE_MS : u64   — default 2000 (ms)
//!
//! Invariants:
//! - Clamped to safe ranges; parsing failures fall back to defaults.
//! - Values are read on each call; cheap enough for infrequent reads in our usage.

use std::time::Duration;

const DEF_WATERMARK: i64 = 96;
const DEF_HSHAKE_MS: u64 = 2_000;

pub fn tx_queue_watermark() -> i64 {
    match std::env::var("RON_OVERLAY_TX_WATERMARK") {
        Ok(v) => v
            .parse::<i64>()
            .ok()
            .map(|n| n.clamp(1, 127))
            .unwrap_or(DEF_WATERMARK),
        Err(_) => DEF_WATERMARK,
    }
}

pub fn handshake_timeout() -> Duration {
    let ms = match std::env::var("RON_OVERLAY_HANDSHAKE_MS") {
        Ok(v) => v
            .parse::<u64>()
            .ok()
            .map(|ms| ms.clamp(100, 30_000))
            .unwrap_or(DEF_HSHAKE_MS),
        Err(_) => DEF_HSHAKE_MS,
    };
    Duration::from_millis(ms)
}
