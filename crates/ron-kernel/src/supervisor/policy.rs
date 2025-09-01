#![forbid(unsafe_code)]

use std::time::Duration;

#[derive(Clone, Copy)]
pub struct RestartPolicy {
    pub base: Duration,
    pub max: Duration,
    pub factor: f64,
    pub jitter: f64, // +/- percentage of delay (0.0..1.0)
}
impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(300),
            max: Duration::from_secs(30),
            factor: 2.0,
            jitter: 0.2,
        }
    }
}

pub fn mul_duration(d: Duration, f: f64) -> Duration {
    let secs = d.as_secs_f64() * f;
    if secs <= 0.0 { Duration::from_millis(0) } else { Duration::from_secs_f64(secs) }
}

pub fn compute_backoff(policy: &RestartPolicy, gen: u64) -> Duration {
    let mut delay = mul_duration(policy.base, policy.factor.powf(gen as f64));
    if delay > policy.max { delay = policy.max; }
    if policy.jitter > 0.0 {
        let j = ((gen as u64).wrapping_mul(1103515245).wrapping_add(12345) % 1000) as f64 / 1000.0;
        let scale = 1.0 + policy.jitter * (j * 2.0 - 1.0);
        delay = mul_duration(delay, scale);
    }
    delay
}
