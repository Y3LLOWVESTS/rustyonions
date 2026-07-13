//! RO:WHAT — Readiness gate smoke test.
//! RO:WHY — Keep the bootstrap/readiness regression meaningful without fake assert!(true).

use svc_dht::readiness::ReadyGate;

#[test]
fn ready_gate_starts_closed_and_opens_explicitly() {
    let ready = ReadyGate::new();

    assert!(!ready.is_ready(), "new readiness gate must start closed");

    ready.set_ready();

    assert!(ready.is_ready(), "readiness gate must open only after set_ready()");
}
