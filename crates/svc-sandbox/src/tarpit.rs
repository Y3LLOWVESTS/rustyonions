use crate::metrics::SandboxMetrics;
use rand::{thread_rng, Rng};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Redirect, Mirror, Tarpit }

impl From<crate::Mode> for Mode {
    fn from(m: crate::Mode) -> Self {
        match m {
            crate::Mode::Redirect => Mode::Redirect,
            crate::Mode::Mirror => Mode::Mirror,
            crate::Mode::Tarpit => Mode::Tarpit,
        }
    }
}

pub async fn maybe_tarpit(mode: crate::Mode, min_ms: u64, max_ms: u64, metrics: &SandboxMetrics) {
    if matches!(Mode::from(mode), Mode::Tarpit) {
        let mut rng = thread_rng();
        let delay = rng.gen_range(min_ms..=max_ms) as f64;
        metrics.tarpit_ms_hist.observe(delay);
        sleep(Duration::from_millis(delay as u64)).await;
    }
}
