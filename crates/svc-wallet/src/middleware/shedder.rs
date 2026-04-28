//! RO:WHAT — Readiness-aware write shedding helper for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/ECON. Wallet writes must fail closed when readiness degrades.
//! RO:INTERACTS — readiness::ReadinessGate, routes/v1 write handlers, future tower layer.
//! RO:INVARIANTS — degraded state rejects writes before nonce/idempotency/ledger commit.
//! RO:METRICS — callers record RETRY_LATER rejects.
//! RO:CONFIG — none.
//! RO:SECURITY — no secret data.
//! RO:TEST — rejects_when_not_ready.

use crate::{
    errors::{WalletError, WalletResult},
    readiness::ReadinessGate,
};

/// Ensure write paths may proceed.
pub fn ensure_writes_ready(readiness: &ReadinessGate) -> WalletResult<()> {
    let snapshot = readiness.snapshot();
    if snapshot.ready && !snapshot.shed_writes {
        Ok(())
    } else {
        Err(WalletError::new(
            crate::errors::WalletErrorCode::RetryLater,
            "wallet is not ready for writes",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_when_not_ready() {
        let gate = ReadinessGate::new();
        assert!(ensure_writes_ready(&gate).is_err());
        gate.mark_ready();
        assert!(ensure_writes_ready(&gate).is_ok());
    }
}
