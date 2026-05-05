//! RO:WHAT — I-7 transport-agnostic SDK semantics check.
//! RO:WHY — Ensures TLS vs Tor selection does not alter app-facing config semantics.
//! RO:INTERACTS — SdkConfig, Transport, TorCfg, Timeouts, check_ready.
//! RO:INVARIANTS — Transport changes routing posture, not API shape or timeout semantics.
//! RO:SECURITY — Tor config must be explicit and readiness-visible.
//! RO:TEST — cargo test -p ron-app-sdk --test i_7_transport_agnostic.

use std::time::Duration;

use ron_app_sdk::{check_ready, SdkConfig, Timeouts, TorCfg, Transport};

fn shared_valid_timeouts() -> (Duration, Timeouts) {
    (
        Duration::from_secs(60),
        Timeouts {
            connect: Duration::from_secs(3),
            read: Duration::from_secs(10),
            write: Duration::from_secs(10),
        },
    )
}

#[test]
fn tls_and_tor_configs_share_the_same_public_semantics() {
    let (overall_timeout, timeouts) = shared_valid_timeouts();

    let tls_cfg = SdkConfig {
        transport: Transport::Tls,
        gateway_addr: "https://example.invalid".to_owned(),
        overall_timeout,
        timeouts: timeouts.clone(),
        ..Default::default()
    };

    let tor_cfg = SdkConfig {
        transport: Transport::Tor,
        gateway_addr: "https://example.invalid".to_owned(),
        overall_timeout,
        timeouts,
        tor: TorCfg {
            socks5_addr: "127.0.0.1:9050".to_owned(),
        },
        ..Default::default()
    };

    let tls_ready = check_ready(&tls_cfg);
    let tor_ready = check_ready(&tor_cfg);

    assert!(
        tls_ready.config_ok,
        "TLS config should validate: {tls_ready:?}"
    );
    assert!(
        tls_ready.transport_ok,
        "TLS transport posture should be internally consistent: {tls_ready:?}"
    );

    assert!(
        tor_ready.config_ok,
        "Tor config should validate with explicit SOCKS address: {tor_ready:?}"
    );
    assert!(
        tor_ready.transport_ok,
        "Tor transport posture should be internally consistent: {tor_ready:?}"
    );
    assert_eq!(
        tor_ready.tor_ok,
        Some(true),
        "Tor readiness should surface Tor-specific status"
    );

    assert_eq!(tls_cfg.gateway_addr, tor_cfg.gateway_addr);
    assert_eq!(tls_cfg.overall_timeout, tor_cfg.overall_timeout);
    assert_eq!(tls_cfg.timeouts.connect, tor_cfg.timeouts.connect);
    assert_eq!(tls_cfg.timeouts.read, tor_cfg.timeouts.read);
    assert_eq!(tls_cfg.timeouts.write, tor_cfg.timeouts.write);
    assert_ne!(tls_cfg.transport, tor_cfg.transport);
}
