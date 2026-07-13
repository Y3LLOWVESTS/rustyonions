//! RO:WHAT — Thin wrapper around RustyOnions-native transport clients.
//! RO:WHY — Keep svc-dht transport-agnostic; Concerns: SEC/RES.
//! RO:INVARIANTS — provider records expose crab://node identities, not transport schemes.
//! RO:SECURITY — no Tor/Arti/onion route is part of the current BUILD_PLAN_Z architecture.

pub mod clients; // TODO phase 2
