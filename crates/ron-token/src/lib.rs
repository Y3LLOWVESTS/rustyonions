//! ron-token: domain facade for token economy atop `ron-ledger`.
//!
//! Today this crate re-exports the ledger surface so services depend on a
//! stable domain name (`ron-token`). Grow this with higher-level helpers:
//! - idempotency keys & per-account sequences
//! - policy-aware wrappers
//! - receipt signing & epoch-root emission

pub use ron_ledger::{
    AccountId, Amount, InMemoryLedger, LedgerEntry, Op, Receipt, TokenError, TokenLedger,
};
