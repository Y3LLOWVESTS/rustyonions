//! RO:WHAT — Wallet HTTP DTO module tree.
//! RO:WHY  — Pillar 12; Concerns: DX/SEC/ECON. Keeps strict request/response/error wire shapes centralized.
//! RO:INTERACTS — routes/v1, errors, util::blake3_receipt, ledger::client.
//! RO:INVARIANTS — serde deny_unknown_fields on request structs; amount strings; no floats.
//! RO:METRICS — none directly.
//! RO:CONFIG — request validation receives WalletConfig.
//! RO:SECURITY — DTOs carry identifiers only; tokens remain in headers.
//! RO:TEST — dto_hygiene and receipt_hash vectors.

pub mod errors;
pub mod requests;
pub mod responses;
