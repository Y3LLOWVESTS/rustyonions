//! RO:WHAT    Shared imports/types for crate-internal modules.
//! RO:WHY     Keep lib files concise and consistent (CODECOMMENTS.MD).
//! RO:INTERACTS  Used by most modules (types, errors, tools).
//! RO:INVARIANTS No I/O, no async, no SHA; BLAKE3 only.

pub use crate::errors::AuthError;
pub use serde::{Deserialize, Serialize};
