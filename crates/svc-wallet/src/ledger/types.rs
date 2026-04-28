//! RO:WHAT — Small internal ledger adapter types.
//! RO:WHY  — Pillar 12; Concerns: ECON/DX. Separates wallet receipts from primitive ron-ledger responses.
//! RO:INTERACTS — ledger::client, dto::responses, ron-ledger.
//! RO:INVARIANTS — ledger sequence/root are copied from ron-ledger response; no local truth replacement.
//! RO:METRICS — none directly.
//! RO:CONFIG — none.
//! RO:SECURITY — identifiers only.
//! RO:TEST — constructed by ledger client tests.

/// Common metadata for committing primitive ledger entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerIdentity {
    /// KMS key id reference stored in ledger entries.
    pub kid: String,
    /// Capability reference stored in ledger entries.
    pub capability_ref: String,
}

impl Default for LedgerIdentity {
    fn default() -> Self {
        Self {
            kid: "svc-wallet-dev-kid".to_string(),
            capability_ref: "svc-wallet-dev-cap".to_string(),
        }
    }
}
