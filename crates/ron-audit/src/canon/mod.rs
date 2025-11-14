//! Canonicalization for `AuditRecord`.
//!
//! The goal is to produce a **stable byte representation** of a record
//! *without* its `self_hash` field, suitable for hashing and dedupe.
//!
//! Rules (as per IDB):
//! - Struct fields appear in a fixed order.
//! - Strings are NFC-normalized.
//! - Floats are rejected (only ints/booleans/strings/objects/arrays allowed).
//! - Unknown top-level fields are rejected.

use serde_json::{Map, Value};
use unicode_normalization::UnicodeNormalization;

use crate::AuditRecord;

/// Errors produced during canonicalization.
#[derive(Debug, thiserror::Error)]
pub enum CanonError {
    /// The record could not be encoded as a JSON object.
    #[error("record was not encodable as a JSON object")]
    NonObject,

    /// The record was missing a required field.
    #[error("record missing required field `{0}`")]
    MissingField(&'static str),

    /// The record contained unexpected extra fields.
    #[error("record contained unexpected fields")]
    UnexpectedFields,

    /// A floating-point number was encountered in the payload.
    #[error("floats are not allowed in audit payloads")]
    FloatDisallowed,

    /// JSON encoding failed.
    #[error("failed to encode canonical JSON")]
    Encode,
}

/// Produce canonical bytes for an `AuditRecord` *without* its `self_hash`.
///
/// This function:
/// - Removes the `self_hash` field entirely.
/// - Re-orders top-level fields into a stable order.
/// - NFC-normalizes all strings recursively.
/// - Rejects floats anywhere in the payload.
pub fn canonicalize_without_self_hash(rec: &AuditRecord) -> Result<Vec<u8>, CanonError> {
    // Serialize the record into a generic JSON value first.
    let mut value = serde_json::to_value(rec).map_err(|_| CanonError::Encode)?;

    let obj = value.as_object_mut().ok_or(CanonError::NonObject)?;

    // Drop self_hash; we'll recompute it from the canonical bytes.
    obj.remove("self_hash");

    // Expected top-level order (must match AuditRecord field layout).
    const ORDER: [&str; 11] = [
        "v",
        "ts_ms",
        "writer_id",
        "seq",
        "stream",
        "kind",
        "actor",
        "subject",
        "reason",
        "attrs",
        "prev",
    ];

    let mut out = Map::new();

    for key in ORDER {
        let value = obj.remove(key).ok_or(CanonError::MissingField(key))?;
        let normalized = normalize_value(value)?;
        out.insert(key.to_string(), normalized);
    }

    // If anything is left, the record had extra fields we don't know about.
    if !obj.is_empty() {
        return Err(CanonError::UnexpectedFields);
    }

    let canonical = Value::Object(out);
    serde_json::to_vec(&canonical).map_err(|_| CanonError::Encode)
}

fn normalize_value(value: Value) -> Result<Value, CanonError> {
    match value {
        Value::Null | Value::Bool(_) => Ok(value),
        Value::Number(n) => {
            // Only integral numbers are allowed; floats are rejected.
            if n.as_i64().is_some() || n.as_u64().is_some() {
                Ok(Value::Number(n))
            } else {
                Err(CanonError::FloatDisallowed)
            }
        }
        Value::String(s) => {
            let normalized: String = s.nfc().collect();
            Ok(Value::String(normalized))
        }
        Value::Array(values) => {
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                out.push(normalize_value(v)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(map) => {
            // For nested maps we keep insertion order as provided by serde_json,
            // but still normalize all nested values and reject floats.
            let mut out = Map::new();
            for (k, v) in map {
                let normalized = normalize_value(v)?;
                out.insert(k, normalized);
            }
            Ok(Value::Object(out))
        }
    }
}
