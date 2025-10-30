//! RO:WHAT — Structural validation for `PolicyBundle`.
//!
//! # Errors
//!
//! Returns `Error::Validation` if the bundle violates invariants (e.g., duplicate rule IDs,
//! body caps exceeding 1 MiB).

use crate::{
    errors::Error,
    model::{PolicyBundle, Rule},
};
use std::collections::BTreeSet;

pub fn validate(b: &PolicyBundle) -> Result<(), Error> {
    if b.version == 0 {
        return Err(Error::Validation("version must be ≥ 1".into()));
    }

    let mut ids = BTreeSet::<&str>::new();
    for r in &b.rules {
        if r.id.trim().is_empty() {
            return Err(Error::Validation("rule.id must be non-empty".into()));
        }
        if !ids.insert(&r.id) {
            return Err(Error::Validation(format!("duplicate rule id: {}", r.id)));
        }
        if let Some(n) = r.when.max_body_bytes {
            if n > 1_048_576 {
                // 1 MiB guard per Hardening blueprint
                return Err(Error::Validation(format!(
                    "rule {} max_body_bytes > 1MiB",
                    r.id
                )));
            }
        }
    }
    if let Some(n) = b.defaults.max_body_bytes {
        if n > 1_048_576 {
            return Err(Error::Validation("defaults.max_body_bytes > 1MiB".into()));
        }
    }
    Ok(())
}
