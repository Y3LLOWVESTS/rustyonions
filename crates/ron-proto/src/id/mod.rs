//! RO:WHAT — Newtypes and helpers for canonical IDs (content-addresses, names).
//! RO:WHY  — Strong typing for interop; prevent stringly-typed bugs.
//! RO:INTERACTS — Used across OAP envelopes, manifests, mailbox, governance.
//! RO:INVARIANTS — ContentId must be "b3:<64 lowercase hex>"; no hashing performed here.
//! RO:TEST — Round-trip serde tests and parser property tests live in this module.

mod content_id;
mod parse;

pub use content_id::{ContentId, CONTENT_ID_HEX_LEN, CONTENT_ID_PREFIX};
pub use parse::{ParseContentIdError, is_lower_hex64, validate_b3_str};
