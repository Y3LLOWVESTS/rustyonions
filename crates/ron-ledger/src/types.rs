//! RO:WHAT — Primitive ledger identifiers, entry shapes, roots, checkpoints, and append records.
//! RO:WHY  — Pillar 12; Concerns: ECON/SEC/RES. Keep wire-ish primitives stable, explicit, and easily serializable.
//! RO:INTERACTS — crate::api, crate::engine, tests and future service wrappers.
//! RO:INVARIANTS — no floats; fixed integer amounts; account/KID/cap refs validated; nonce must decode to 16 bytes; nested DTOs reject unknown fields.
//! RO:METRICS — none directly.
//! RO:CONFIG — none.
//! RO:SECURITY — KID/cap refs are identifiers only; nonce validation prevents malformed idempotency data.
//! RO:TEST — interop_vectors.rs, reject_taxonomy.rs, idempotency_prop.rs.

use crate::error::{LedgerError, RejectReason};
use base64::Engine as _;
use serde::{Deserialize, Deserializer, Serialize};

/// Monotonic sequence assigned by the single writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seq(pub u64);

impl Seq {
    /// Get the raw sequence value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Accumulator root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Root([u8; 32]);

impl Root {
    /// The zero / genesis root.
    pub const fn zero() -> Self {
        Self([0; 32])
    }

    /// Construct from raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Encode as lower-hex.
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    /// Decode from lower/upper hex.
    pub fn from_hex(s: &str) -> Result<Self, LedgerError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "root hex must decode to 32 bytes",
            ));
        }
        let mut out = [0_u8; 32];
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

impl Default for Root {
    fn default() -> Self {
        Self::zero()
    }
}

/// SHA/BLAKE checksum bytes for file-backed artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Checksum([u8; 32]);

impl Checksum {
    /// Compute a checksum from bytes using BLAKE3.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Encode as hex.
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

/// Account identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct AccountId(String);

impl AccountId {
    /// Validate and construct an account identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, LedgerError> {
        let value = value.into();
        if value.is_empty() || value.len() > 256 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "account id must be 1..=256 bytes",
            ));
        }
        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '-' | '_'))
        {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "account id contains unsupported characters",
            ));
        }
        Ok(Self(value))
    }

    /// Borrow as string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// KMS key identifier reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Kid(String);

impl Kid {
    /// Validate and construct a KID.
    pub fn new(value: impl Into<String>) -> Result<Self, LedgerError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "kid must be 1..=128 bytes",
            ));
        }
        Ok(Self(value))
    }

    /// Borrow as string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Kid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Capability reference identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CapabilityRef(String);

impl CapabilityRef {
    /// Validate and construct a capability reference.
    pub fn new(value: impl Into<String>) -> Result<Self, LedgerError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "capability ref must be 1..=128 bytes",
            ));
        }
        Ok(Self(value))
    }

    /// Borrow as string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CapabilityRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// Base64-encoded, 16-byte nonce.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Nonce(String);

impl Nonce {
    /// Construct from a base64-encoded string that decodes to exactly 16 bytes.
    pub fn from_base64(value: impl Into<String>) -> Result<Self, LedgerError> {
        let value = value.into();
        let decoded = base64::engine::general_purpose::STANDARD.decode(value.as_bytes())?;
        if decoded.len() != 16 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "nonce must decode to exactly 16 bytes",
            ));
        }
        Ok(Self(value))
    }

    /// Borrow as string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Nonce {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_base64(value).map_err(serde::de::Error::custom)
    }
}

/// Primitive ledger entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryKind {
    /// Positive posting.
    Credit,
    /// Negative posting.
    Debit,
    /// Balanced movement posting.
    Transfer,
    /// Supply-creating exception.
    Mint,
    /// Supply-destroying exception.
    Burn,
    /// Reserve funds.
    Hold,
    /// Linked reversal / clawback style posting.
    Reverse,
}

impl EntryKind {
    /// Sign bucket used for balance simulation.
    pub const fn is_credit_like(self) -> bool {
        matches!(self, Self::Credit | Self::Mint | Self::Transfer)
    }

    /// Sign bucket used for balance simulation.
    pub const fn is_debit_like(self) -> bool {
        matches!(self, Self::Debit | Self::Burn | Self::Hold | Self::Reverse)
    }

    /// True when the kind should participate in strict conservation checks.
    pub const fn is_conservation_tracked(self) -> bool {
        matches!(
            self,
            Self::Credit | Self::Debit | Self::Transfer | Self::Hold | Self::Reverse
        )
    }
}

/// Primitive ingest entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Client-provided id.
    pub id: String,
    /// Source timestamp in unix millis.
    pub ts: u64,
    /// Posting kind.
    pub kind: EntryKind,
    /// Target account.
    pub account: AccountId,
    /// Amount in minor units.
    pub amount: u64,
    /// Idempotency component.
    pub nonce: Nonce,
    /// Key identifier reference.
    pub kid: Kid,
    /// Capability reference identifier.
    pub capability_ref: CapabilityRef,
    /// Entry schema version.
    pub v: u16,
}

impl Entry {
    /// Build and validate a primitive entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        ts: u64,
        kind: EntryKind,
        account: AccountId,
        amount: u64,
        nonce: Nonce,
        kid: Kid,
        capability_ref: CapabilityRef,
        v: u16,
    ) -> Result<Self, LedgerError> {
        let id = id.into();
        if id.is_empty() || id.len() > 128 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "entry id must be 1..=128 bytes",
            ));
        }
        if amount == 0 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "amount must be > 0",
            ));
        }
        if v == 0 {
            return Err(LedgerError::reject(
                RejectReason::Invalid,
                "entry version must be > 0",
            ));
        }
        Ok(Self {
            id,
            ts,
            kind,
            account,
            amount,
            nonce,
            kid,
            capability_ref,
            v,
        })
    }
}

/// Append-only record persisted after sequencing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryRecord {
    /// Monotonic sequence assigned by the writer.
    pub seq: Seq,
    /// Entry payload.
    pub entry: Entry,
    /// Previous root.
    pub prev_root: Root,
    /// New root after applying the entry.
    pub new_root: Root,
}

/// Durable checkpoint record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointRecord {
    /// Sequence covered by the checkpoint.
    pub seq: Seq,
    /// Root at that sequence.
    pub root: Root,
    /// Timestamp in unix millis when the checkpoint was emitted.
    pub ts: u64,
}
