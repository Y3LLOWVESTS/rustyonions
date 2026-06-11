//! RO:WHAT — QuickChain test-vector metadata DTOs for QC-0B planning.
//! RO:WHY — GOV/RES: vectors must progress from sketch to locked bytes/hash without placeholders.
//! RO:INTERACTS — canonical JSON helpers, domain separators, future locked vector files.
//! RO:INVARIANTS — metadata validation only; no hashing; no roots; no fake expected hashes.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — locked_hash vectors require explicit reviewed b3 data and exact preimage framing.
//! RO:TEST — tests/quickchain_vector_dto.rs.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::id::ContentId;

use super::{
    domain::{validate_domain_separator_v1, MAX_QUICKCHAIN_DOMAIN_SEPARATOR_BYTES},
    validate_schema, validate_version, QuickChainResult, QuickChainValidationError,
};

pub const QUICKCHAIN_TEST_VECTOR_SCHEMA: &str = "quickchain.test-vector.v1";

pub const QUICKCHAIN_CANONICAL_JSON_ENCODING_V1: &str = "quickchain.canonical-json.v1";
pub const QUICKCHAIN_PREIMAGE_FRAMING_DOMAIN_NUL_PAYLOAD_V1: &str =
    "domain_separator_bytes || 0x00 || canonical_payload_bytes";
pub const QUICKCHAIN_HASH_ALGORITHM_BLAKE3_256: &str = "blake3-256";

pub const MAX_QUICKCHAIN_VECTOR_ID_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_VECTOR_PURPOSE_BYTES: usize = 128;
pub const MAX_QUICKCHAIN_VECTOR_PAYLOAD_BYTES: usize = 64 * 1024;
pub const MAX_QUICKCHAIN_VECTOR_NOTES: usize = 16;
pub const MAX_QUICKCHAIN_VECTOR_NOTE_BYTES: usize = 256;
pub const MAX_QUICKCHAIN_VECTOR_PREIMAGE_HEX_BYTES: usize =
    (MAX_QUICKCHAIN_DOMAIN_SEPARATOR_BYTES + 1 + MAX_QUICKCHAIN_VECTOR_PAYLOAD_BYTES) * 2;

/// QC-0A vector lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QuickChainVectorStatusV1 {
    /// Human-readable scenario only. Must not drive root/hash tests.
    Sketch,

    /// Canonical bytes are locked, but no final hash expectation is attached.
    LockedBytes,

    /// Canonical preimage and expected b3 are locked.
    LockedHash,
}

/// Canonical encoding tag for reviewed QuickChain vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuickChainVectorCanonicalEncodingV1 {
    #[serde(rename = "quickchain.canonical-json.v1")]
    CanonicalJsonV1,
}

/// Preimage framing tag for reviewed QuickChain vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuickChainVectorPreimageFramingV1 {
    #[serde(rename = "domain_separator_bytes || 0x00 || canonical_payload_bytes")]
    DomainSeparatorNulPayload,
}

/// Hash algorithm tag for reviewed QuickChain vectors.
///
/// This is a label only. This module does not compute BLAKE3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuickChainVectorHashAlgorithmV1 {
    #[serde(rename = "blake3-256")]
    Blake3_256,
}

/// Test-vector metadata DTO.
///
/// This DTO validates vector metadata and byte-framing consistency. It does
/// not hash, build roots, create checkpoints, or claim settlement truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickChainTestVectorV1 {
    pub schema: String,
    pub version: u16,
    pub vector_id: String,
    pub status: QuickChainVectorStatusV1,
    pub purpose: String,
    pub domain_separator: String,
    pub canonical_encoding: QuickChainVectorCanonicalEncodingV1,
    pub preimage_framing: QuickChainVectorPreimageFramingV1,
    pub hash_algorithm: QuickChainVectorHashAlgorithmV1,
    pub human_readable_json: Value,

    #[serde(deserialize_with = "required_option")]
    pub canonical_payload_utf8: Option<String>,

    #[serde(deserialize_with = "required_option")]
    pub canonical_payload_hex: Option<String>,

    #[serde(deserialize_with = "required_option")]
    pub preimage_hex: Option<String>,

    #[serde(deserialize_with = "required_option")]
    pub expected_b3: Option<ContentId>,

    #[serde(default)]
    pub notes: Vec<String>,
}

impl QuickChainTestVectorV1 {
    /// Validate vector metadata only. This does not hash or construct roots.
    pub fn validate(&self) -> QuickChainResult<()> {
        validate_schema(
            "QuickChainTestVectorV1.schema",
            &self.schema,
            QUICKCHAIN_TEST_VECTOR_SCHEMA,
        )?;
        validate_version("QuickChainTestVectorV1.version", self.version)?;
        validate_vector_token("vector_id", &self.vector_id, MAX_QUICKCHAIN_VECTOR_ID_BYTES)?;
        validate_vector_token(
            "purpose",
            &self.purpose,
            MAX_QUICKCHAIN_VECTOR_PURPOSE_BYTES,
        )?;
        validate_domain_separator_v1(&self.domain_separator)?;
        validate_human_readable_json(&self.human_readable_json)?;
        validate_notes(&self.notes)?;

        match self.status {
            QuickChainVectorStatusV1::Sketch => self.validate_sketch_fields(),
            QuickChainVectorStatusV1::LockedBytes => self.validate_locked_bytes_fields(),
            QuickChainVectorStatusV1::LockedHash => {
                self.validate_canonical_payload_fields()?;
                self.validate_locked_hash_fields()
            }
        }
    }

    fn validate_sketch_fields(&self) -> QuickChainResult<()> {
        if self.canonical_payload_utf8.is_some()
            || self.canonical_payload_hex.is_some()
            || self.preimage_hex.is_some()
            || self.expected_b3.is_some()
        {
            return Err(QuickChainValidationError::InvalidField {
                field: "status",
                reason: "sketch vectors must not include canonical bytes, preimage bytes, or expected hashes",
            });
        }

        Ok(())
    }

    fn validate_locked_bytes_fields(&self) -> QuickChainResult<()> {
        if self.preimage_hex.is_some() || self.expected_b3.is_some() {
            return Err(QuickChainValidationError::InvalidField {
                field: "status",
                reason: "locked_bytes vectors must not include preimage_hex or expected_b3; use locked_hash",
            });
        }

        self.validate_canonical_payload_fields()
    }

    fn validate_locked_hash_fields(&self) -> QuickChainResult<()> {
        let preimage_hex =
            self.preimage_hex
                .as_ref()
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "preimage_hex",
                    reason: "required for locked_hash vectors",
                })?;

        if self.expected_b3.is_none() {
            return Err(QuickChainValidationError::InvalidField {
                field: "expected_b3",
                reason: "required for locked_hash vectors",
            });
        }

        validate_hex_field(
            "preimage_hex",
            preimage_hex,
            MAX_QUICKCHAIN_VECTOR_PREIMAGE_HEX_BYTES,
        )?;

        let payload_utf8 = self.canonical_payload_utf8.as_ref().ok_or(
            QuickChainValidationError::InvalidField {
                field: "canonical_payload_utf8",
                reason: "required for locked_hash vectors",
            },
        )?;
        let expected_preimage_hex =
            framed_preimage_hex(&self.domain_separator, payload_utf8.as_bytes());

        if *preimage_hex != expected_preimage_hex {
            return Err(QuickChainValidationError::InvalidField {
                field: "preimage_hex",
                reason: "must equal hex(domain_separator_bytes || 0x00 || canonical_payload_bytes)",
            });
        }

        Ok(())
    }

    fn validate_canonical_payload_fields(&self) -> QuickChainResult<()> {
        let payload_utf8 = self.canonical_payload_utf8.as_ref().ok_or(
            QuickChainValidationError::InvalidField {
                field: "canonical_payload_utf8",
                reason: "required for locked_bytes and locked_hash vectors",
            },
        )?;
        let payload_hex =
            self.canonical_payload_hex
                .as_ref()
                .ok_or(QuickChainValidationError::InvalidField {
                    field: "canonical_payload_hex",
                    reason: "required for locked_bytes and locked_hash vectors",
                })?;

        if payload_utf8.is_empty() {
            return Err(QuickChainValidationError::EmptyField {
                field: "canonical_payload_utf8",
            });
        }

        if payload_utf8.len() > MAX_QUICKCHAIN_VECTOR_PAYLOAD_BYTES {
            return Err(QuickChainValidationError::FieldTooLong {
                field: "canonical_payload_utf8",
                max: MAX_QUICKCHAIN_VECTOR_PAYLOAD_BYTES,
                actual: payload_utf8.len(),
            });
        }

        validate_hex_field(
            "canonical_payload_hex",
            payload_hex,
            MAX_QUICKCHAIN_VECTOR_PAYLOAD_BYTES * 2,
        )?;

        let expected_payload_hex = lower_hex(payload_utf8.as_bytes());
        if *payload_hex != expected_payload_hex {
            return Err(QuickChainValidationError::InvalidField {
                field: "canonical_payload_hex",
                reason: "must equal lowercase hex(canonical_payload_utf8 bytes)",
            });
        }

        Ok(())
    }
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn validate_human_readable_json(value: &Value) -> QuickChainResult<()> {
    if value.is_null() {
        return Err(QuickChainValidationError::InvalidField {
            field: "human_readable_json",
            reason: "must be present and non-null",
        });
    }

    Ok(())
}

fn validate_notes(notes: &[String]) -> QuickChainResult<()> {
    if notes.len() > MAX_QUICKCHAIN_VECTOR_NOTES {
        return Err(QuickChainValidationError::TooManyItems {
            field: "notes",
            max: MAX_QUICKCHAIN_VECTOR_NOTES,
            actual: notes.len(),
        });
    }

    for note in notes {
        if note.is_empty() {
            return Err(QuickChainValidationError::EmptyField { field: "notes" });
        }

        if note.len() > MAX_QUICKCHAIN_VECTOR_NOTE_BYTES {
            return Err(QuickChainValidationError::FieldTooLong {
                field: "notes",
                max: MAX_QUICKCHAIN_VECTOR_NOTE_BYTES,
                actual: note.len(),
            });
        }
    }

    Ok(())
}

fn validate_vector_token(field: &'static str, value: &str, max: usize) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    if value.len() > max {
        return Err(QuickChainValidationError::FieldTooLong {
            field,
            max,
            actual: value.len(),
        });
    }

    if !value.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
    }) {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    Ok(())
}

fn validate_hex_field(field: &'static str, value: &str, max: usize) -> QuickChainResult<()> {
    if value.is_empty() {
        return Err(QuickChainValidationError::EmptyField { field });
    }

    if value.len() > max {
        return Err(QuickChainValidationError::FieldTooLong {
            field,
            max,
            actual: value.len(),
        });
    }

    if value.len() % 2 != 0 {
        return Err(QuickChainValidationError::InvalidField {
            field,
            reason: "hex strings must have even length",
        });
    }

    if !value
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(QuickChainValidationError::InvalidToken { field });
    }

    Ok(())
}

fn framed_preimage_hex(domain_separator: &str, canonical_payload_bytes: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(domain_separator.len() + 1 + canonical_payload_bytes.len());
    bytes.extend_from_slice(domain_separator.as_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(canonical_payload_bytes);
    lower_hex(&bytes)
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }

    out
}
