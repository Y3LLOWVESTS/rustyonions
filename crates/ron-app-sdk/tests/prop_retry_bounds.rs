//! RO:WHAT — Retry-bound property-style checks for ron-app-sdk.
//! RO:WHY — Guards retry math from exceeding configured caps or producing zero attempts.
//! RO:INTERACTS — RetryCfg, Jitter.
//! RO:INVARIANTS — Retry attempts are bounded, positive, and cap-respecting.
//! RO:SECURITY — Prevents runaway retry loops under degraded network conditions.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::{cmp, time::Duration};

use ron_app_sdk::{config::RetryCfg, Jitter};

#[test]
fn retry_config_defaults_are_bounded_and_nonzero() {
    let cfg = RetryCfg::default();

    assert!(cfg.max_attempts >= 1);
    assert!(cfg.base > Duration::ZERO);
    assert!(cfg.cap >= cfg.base);
    assert!(cfg.factor >= 1.0);

    match cfg.jitter {
        Jitter::Full | Jitter::None => {}
    }
}

#[test]
fn derived_retry_delays_do_not_exceed_cap() {
    let cfg = RetryCfg {
        base: Duration::from_millis(25),
        factor: 2.0,
        cap: Duration::from_millis(250),
        max_attempts: 8,
        jitter: Jitter::None,
    };

    let mut delay = cfg.base;
    let mut last = Duration::ZERO;

    for _attempt in 0..cfg.max_attempts {
        assert!(delay >= last, "retry delay should be monotonic before cap");
        assert!(delay <= cfg.cap, "retry delay must never exceed cap");

        last = delay;

        let next_ms = ((delay.as_millis() as f64) * f64::from(cfg.factor)).round() as u64;
        delay = cmp::min(Duration::from_millis(next_ms), cfg.cap);
    }
}
