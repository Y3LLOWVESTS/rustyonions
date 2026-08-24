//! RO:WHAT — Proves Omnigate defaults/config reserve canonical CrabNode port 9090.
//! RO:WHY — Mailbox owns 5305, so Omnigate must not retain that historical API bind.
//! RO:INTERACTS — Omnigate Config loader and checked-in configs/omnigate.toml.
//! RO:INVARIANTS — API default is 127.0.0.1:9090 and remains loopback.
//! RO:CONFIG — OMNIGATE_BIND may explicitly override this service-owned default.
//! RO:SECURITY — canonical default is not publicly exposed.
//! RO:TEST — cargo test -p omnigate --test cn2_canonical_defaults.

use omnigate::config::Config;

#[test]
fn canonical_crabnode_omnigate_bind_is_loopback_9090() {
    let cfg = Config::load().expect("canonical Omnigate config should load");

    assert_eq!(cfg.server.bind.to_string(), "127.0.0.1:9090");

    assert!(cfg.server.bind.ip().is_loopback());

    assert_ne!(
        cfg.server.bind.port(),
        5305,
        "mailbox owns canonical CrabNode port 5305"
    );
}
