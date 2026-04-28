//! RO:WHAT — Capability authorization module tree.
//! RO:WHY  — Pillar 12; Concerns: SEC/ECON/GOV. token mutations require explicit scopes.
//! RO:INTERACTS — routes, policy, ron-auth integration seam.
//! RO:INVARIANTS — no ambient auth; issue/burn stricter than transfer; read separate from writes.
//! RO:METRICS — caller records auth rejects.
//! RO:CONFIG — future verifier config from svc-passport/ron-auth.
//! RO:SECURITY — never logs bearer tokens.
//! RO:TEST — scope checks in caps.rs.

pub mod caps;
