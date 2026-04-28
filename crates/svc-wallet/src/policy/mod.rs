//! RO:WHAT — Policy enforcement module tree.
//! RO:WHY  — Pillar 12; Concerns: SEC/ECON/GOV. Policy must deny unsafe economic actions before ledger commit.
//! RO:INTERACTS — auth::caps, dto requests, ledger client, future ron-policy adapter.
//! RO:INVARIANTS — deny by default for unsupported ops; amount ceilings enforced before ledger IO.
//! RO:METRICS — caller records policy denies.
//! RO:CONFIG — WalletConfig supplies local ceilings until ron-policy is wired.
//! RO:SECURITY — policy uses claims and identifiers only.
//! RO:TEST — policy_allows_configured_amounts.

pub mod enforce;
