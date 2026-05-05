//! RO:WHAT — I-1 profile parity checks for `ron-app-sdk`.
//! RO:WHY — Ensures one SDK config shape works for Micronode and Macronode profile targets.
//! RO:INTERACTS — SdkConfig, Transport, Timeouts, check_ready.
//! RO:INVARIANTS — Profile choice changes endpoint target, not SDK semantics or timeout policy.
//! RO:SECURITY — No ambient authority; profile parity does not bypass capability checks.
//! RO:TEST — cargo test -p ron-app-sdk --test i_1_profile_parity.

use std::time::Duration;

use ron_app_sdk::{check_ready, SdkConfig, Timeouts, Transport};

fn profile_cfg(gateway_addr: &str) -> SdkConfig {
    SdkConfig {
        transport: Transport::Tls,
        gateway_addr: gateway_addr.to_owned(),
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
fn micronode_and_macronode_configs_share_sdk_semantics() {
    let micronode = profile_cfg("https://micronode.local.invalid");
    let macronode = profile_cfg("https://macronode.local.invalid");

    let micro_ready = check_ready(&micronode);
    let macro_ready = check_ready(&macronode);

    assert!(
        micro_ready.is_ready(),
        "micronode-style SDK config should validate: {micro_ready:?}"
    );
    assert!(
        macro_ready.is_ready(),
        "macronode-style SDK config should validate: {macro_ready:?}"
    );

    assert_eq!(micronode.transport, macronode.transport);
    assert_eq!(micronode.overall_timeout, macronode.overall_timeout);
    assert_eq!(micronode.timeouts.connect, macronode.timeouts.connect);
    assert_eq!(micronode.timeouts.read, macronode.timeouts.read);
    assert_eq!(micronode.timeouts.write, macronode.timeouts.write);
    assert_ne!(micronode.gateway_addr, macronode.gateway_addr);
}
