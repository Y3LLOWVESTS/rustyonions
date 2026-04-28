//! RO:WHAT — Small rate-limit decision types for svc-wallet.
//! RO:WHY  — Pillar 12; Concerns: RES/PERF. Keeps 429 behavior explicit before real token buckets are wired.
//! RO:INTERACTS — future gateway/admission middleware.
//! RO:INVARIANTS — rejected decisions carry Retry-After seconds.
//! RO:METRICS — callers record BUSY rejects.
//! RO:CONFIG — future policy/rate config.
//! RO:SECURITY — no account ids or tokens in decisions.
//! RO:TEST — allow_and_reject_decisions_are_stable.

/// Admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request is admitted.
    Allow,
    /// Request is rejected with Retry-After seconds.
    Reject { retry_after_secs: u64 },
}

impl RateLimitDecision {
    /// True when request may proceed.
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_and_reject_decisions_are_stable() {
        assert!(RateLimitDecision::Allow.is_allowed());
        assert!(!RateLimitDecision::Reject {
            retry_after_secs: 1
        }
        .is_allowed());
    }
}
