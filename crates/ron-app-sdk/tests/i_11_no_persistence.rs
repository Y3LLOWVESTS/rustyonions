//! RO:WHAT — I-11 no-persistence contract for ron-app-sdk.
//! RO:WHY — Proves the SDK default posture is client-side, bounded, and memory-only.
//! RO:INTERACTS — SdkConfig, CacheCfg.
//! RO:INVARIANTS — No disk paths, no file creation, no durable state in SDK config.
//! RO:SECURITY — Cache is convenience state only; backend remains truth.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

use std::time::Duration;

use ron_app_sdk::{CacheCfg, SdkConfig};

#[test]
fn sdk_default_cache_is_disabled_and_ephemeral() {
    let cfg = SdkConfig::default();

    assert!(
        !cfg.cache.enabled,
        "SDK cache should be disabled by default so there is no implicit persistence posture"
    );
    assert!(
        cfg.cache.max_entries > 0,
        "cache capacity default should still be sane if a caller enables it"
    );
    assert!(
        cfg.cache.ttl >= Duration::from_secs(1),
        "cache TTL default should be bounded and meaningful"
    );
}

#[test]
fn enabled_cache_config_has_no_disk_path_or_persistence_knob() {
    let cfg = SdkConfig {
        cache: CacheCfg {
            enabled: true,
            max_entries: 32,
            ttl: Duration::from_secs(30),
        },
        ..Default::default()
    };

    assert!(cfg.cache.enabled);
    assert_eq!(cfg.cache.max_entries, 32);
    assert_eq!(cfg.cache.ttl, Duration::from_secs(30));
}
