//! RO:WHAT — Lightweight readiness probe for ron-app-sdk.
//! RO:WHY  — Give hosts a simple, synchronous way to ask “is this SDK
//!           configuration plausibly usable?” without performing any
//!           network I/O.
//! RO:INTERACTS — Uses `SdkConfig::validate()` plus some direct checks
//!                on transport profile (e.g., Tor SOCKS addr).
//! RO:INVARIANTS —
//!   - Purely in-process; no sockets, no DNS.
//!   - Never panics; all issues are reflected in `missing` + flags.
//! RO:SECURITY — Does not log or expose secrets; only high-level flags.

use crate::config::{SdkConfig, Transport};

/// Readiness summary for the SDK.
///
/// This is intentionally small and boring so host applications can
/// translate it into whatever readiness surface they prefer
/// (`/readyz`, health widgets, etc.).
#[derive(Debug, Clone)]
pub struct ReadyReport {
    /// `true` if `SdkConfig::validate()` succeeded.
    pub config_ok: bool,
    /// `true` if the selected transport profile is internally consistent.
    pub transport_ok: bool,
    /// For Tor profiles, whether the SOCKS endpoint is usable at the
    /// *config* level (non-empty); `None` when transport != Tor.
    pub tor_ok: Option<bool>,
    /// High-level reasons why readiness is not yet achieved.
    ///
    /// This is intentionally coarse (e.g., "config", "tor_socks5_addr")
    /// so it can be safely surfaced in logs and UIs.
    pub missing: Vec<&'static str>,
}

impl ReadyReport {
    /// Convenience getter: overall readiness.
    ///
    /// Hosts are free to apply stricter policies, but this is a
    /// sensible default: config + transport OK and Tor (if used) OK.
    pub fn is_ready(&self) -> bool {
        self.config_ok && self.transport_ok && self.tor_ok.unwrap_or(true)
    }
}

/// Evaluate SDK readiness based on configuration alone.
///
/// This does **not** attempt any network calls, TLS handshakes, or Tor
/// reachability tests. Those belong in higher-level smoke checks. Here
/// we only ask “would it even make sense to construct a client?”
pub fn check_ready(cfg: &SdkConfig) -> ReadyReport {
    let mut missing = Vec::new();

    // 1) Config validation (semantic invariants from CONFIG.md).
    let config_ok = cfg.validate().is_ok();
    if !config_ok {
        missing.push("config");
    }

    // 2) Transport profile.
    let mut transport_ok = true;
    let mut tor_ok = None;

    match cfg.transport {
        Transport::Tls => {
            // Nothing extra to check here for now — TLS reachability lives
            // in the underlying transport (ron-transport) and smoke tests.
        }
        Transport::Tor => {
            if cfg.tor.socks5_addr.trim().is_empty() {
                transport_ok = false;
                tor_ok = Some(false);
                missing.push("tor_socks5_addr");
            } else {
                tor_ok = Some(true);
            }
        }
    }

    ReadyReport {
        config_ok,
        transport_ok,
        tor_ok,
        missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CacheCfg, IdemCfg, Jitter, PqMode, Redaction, RetryCfg, Timeouts, TorCfg, TracingCfg,
    };
    use std::time::Duration;

    fn baseline_cfg() -> SdkConfig {
        SdkConfig {
            transport: Transport::Tls,
            gateway_addr: "https://example.invalid".to_string(),
            overall_timeout: Duration::from_secs(30),
            timeouts: Timeouts {
                connect: Duration::from_secs(5),
                read: Duration::from_secs(10),
                write: Duration::from_secs(10),
            },
            retry: RetryCfg {
                base: Duration::from_millis(100),
                factor: 2.0,
                cap: Duration::from_secs(5),
                max_attempts: 3,
                jitter: Jitter::Full,
            },
            idempotency: IdemCfg {
                enabled: true,
                key_prefix: Some("test".to_string()),
            },
            cache: CacheCfg {
                enabled: false,
                max_entries: 0,
                ttl: Duration::from_secs(0),
            },
            tracing: TracingCfg {
                spans: true,
                metrics: true,
                redaction: Redaction::Safe,
            },
            pq_mode: PqMode::Off,
            tor: TorCfg {
                socks5_addr: String::new(),
            },
        }
    }

    #[test]
    fn tls_baseline_is_ready() {
        let cfg = baseline_cfg();
        let report = check_ready(&cfg);
        assert!(report.config_ok);
        assert!(report.transport_ok);
        assert_eq!(report.tor_ok, None);
        assert!(report.is_ready());
        assert!(report.missing.is_empty());
    }

    #[test]
    fn tor_without_socks_addr_is_not_ready() {
        let mut cfg = baseline_cfg();
        cfg.transport = Transport::Tor;
        cfg.tor.socks5_addr.clear();

        let report = check_ready(&cfg);

        // This is an invalid config (Tor + empty SOCKS addr),
        // so `config_ok` should be false.
        assert!(!report.config_ok);
        assert!(!report.transport_ok);
        assert_eq!(report.tor_ok, Some(false));
        assert!(!report.is_ready());
        assert!(report.missing.contains(&"config"));
        assert!(report.missing.contains(&"tor_socks5_addr"));
    }

    #[test]
    fn tor_with_socks_addr_becomes_ready() {
        let mut cfg = baseline_cfg();
        cfg.transport = Transport::Tor;
        cfg.tor.socks5_addr = "127.0.0.1:9050".to_string();

        let report = check_ready(&cfg);
        assert!(report.config_ok);
        assert!(report.transport_ok);
        assert_eq!(report.tor_ok, Some(true));
        assert!(report.is_ready());
    }
}
