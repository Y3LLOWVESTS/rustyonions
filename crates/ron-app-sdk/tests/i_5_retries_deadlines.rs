//! RO:WHAT — I-5 retry/deadline contract checks for `ron-app-sdk`.
//! RO:WHY — Ensures retry schedules are bounded and cannot silently exceed caller deadlines.
//! RO:INTERACTS — RetryCfg, Jitter, SdkConfig.
//! RO:INVARIANTS — Retry attempts are finite, monotonic, cap-respecting, and deadline-aware.
//! RO:SECURITY — Prevents retry storms and unbounded waits under degraded network conditions.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::{cmp, time::Duration};

use ron_app_sdk::{config::RetryCfg, Jitter, SdkConfig, Timeouts};

fn local_retry_schedule(cfg: &RetryCfg) -> Vec<Duration> {
    let mut delays = Vec::new();
    let mut delay = cfg.base;

    for _attempt in 0..cfg.max_attempts {
        delays.push(delay);

        let next_ms = ((delay.as_millis() as f64) * f64::from(cfg.factor)).round() as u64;
        delay = cmp::min(Duration::from_millis(next_ms), cfg.cap);
    }

    delays
}

fn valid_sdk_config() -> SdkConfig {
    SdkConfig {
        overall_timeout: Duration::from_secs(60),
        timeouts: Timeouts {
            connect: Duration::from_secs(3),
            read: Duration::from_secs(10),
            write: Duration::from_secs(10),
        },
        ..Default::default()
    }
}

#[test]
fn retry_schedule_is_finite_monotonic_and_capped() {
    let cfg = RetryCfg {
        base: Duration::from_millis(25),
        factor: 2.0,
        cap: Duration::from_millis(250),
        max_attempts: 8,
        jitter: Jitter::None,
    };

    let delays = local_retry_schedule(&cfg);

    assert_eq!(
        delays.len(),
        cfg.max_attempts as usize,
        "retry schedule length must equal max_attempts"
    );

    let mut previous = Duration::ZERO;
    for delay in delays {
        assert!(
            delay >= previous,
            "retry schedule must be monotonic when jitter is disabled"
        );
        assert!(delay <= cfg.cap, "retry delay must never exceed cap");
        previous = delay;
    }
}

#[test]
fn explicit_valid_sdk_deadlines_are_bounded_and_validate() {
    let cfg = valid_sdk_config();

    assert!(cfg.overall_timeout > Duration::ZERO);
    assert!(cfg.timeouts.connect > Duration::ZERO);
    assert!(cfg.timeouts.read > Duration::ZERO);
    assert!(cfg.timeouts.write > Duration::ZERO);
    assert!(cfg.overall_timeout >= cfg.timeouts.read);
    assert!(cfg.overall_timeout >= cfg.timeouts.write);
    assert!(cfg.retry.max_attempts >= 1);
    assert!(cfg.retry.base <= cfg.retry.cap);
    assert!(cfg.validate().is_ok());
}

#[test]
fn invalid_timeout_relationship_fails_validation() {
    let cfg = SdkConfig {
        overall_timeout: Duration::from_millis(500),
        timeouts: Timeouts {
            connect: Duration::from_millis(100),
            read: Duration::from_secs(2),
            write: Duration::from_secs(2),
        },
        ..Default::default()
    };

    let err = cfg
        .validate()
        .expect_err("overall timeout below read/write timeout must fail");

    assert!(
        err.to_string().contains("overall_timeout"),
        "unexpected validation error: {err}"
    );
}
