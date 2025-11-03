//! RO:WHAT  Rolling error-rate sampler: turns 429/503/drops into a pct-like signal.
//! RO:WHY   /readyz should trip on sustained errors even if inflight is modest.

use super::policy::ReadyPolicy;
use crate::metrics::gates::POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL;
use crate::metrics::{ADMISSION_QUOTA_EXHAUSTED_TOTAL, FAIR_Q_EVENTS_TOTAL};
use std::sync::Arc;
use std::time::Duration;

/// Spawns a background task; safe to call once at app boot.
pub fn spawn_err_rate_sampler(rp: Arc<ReadyPolicy>, window_secs: u64) {
    let window = window_secs.max(1);
    tokio::spawn(async move {
        let mut last_quota = {
            let g = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                .with_label_values(&["global"])
                .get();
            let i = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                .with_label_values(&["ip"])
                .get();
            g + i
        } as f64;
        let mut last_policy_503 = POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
            .with_label_values(&["503"])
            .get() as f64;
        let mut last_fair_drops = FAIR_Q_EVENTS_TOTAL.with_label_values(&["dropped"]).get() as f64;

        loop {
            tokio::time::sleep(Duration::from_secs(window)).await;

            let quota_now = {
                let g = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                    .with_label_values(&["global"])
                    .get();
                let i = ADMISSION_QUOTA_EXHAUSTED_TOTAL
                    .with_label_values(&["ip"])
                    .get();
                g + i
            } as f64;
            let policy_503_now = POLICY_MIDDLEWARE_SHORTCIRCUITS_TOTAL
                .with_label_values(&["503"])
                .get() as f64;
            let fair_drops_now = FAIR_Q_EVENTS_TOTAL.with_label_values(&["dropped"]).get() as f64;

            let d_quota = (quota_now - last_quota).max(0.0);
            let d_p503 = (policy_503_now - last_policy_503).max(0.0);
            let d_drops = (fair_drops_now - last_fair_drops).max(0.0);

            let err_events = d_quota + d_p503 + d_drops;
            let per_sec = err_events / (window as f64);
            let pct_like = (per_sec * 100.0).min(100.0);

            // Update the policy (truth) — it mirrors to the gauge internally.
            rp.update_err_rate(pct_like);

            last_quota = quota_now;
            last_policy_503 = policy_503_now;
            last_fair_drops = fair_drops_now;
        }
    });
}
