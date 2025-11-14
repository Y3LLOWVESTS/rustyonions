//! Small DTO helpers for ron-audit.
//!
//! For now, `AuditRecord` and its helper types live here. The long-term plan
//! (per the blueprints) is to host these DTOs in `ron-proto` and have
//! `ron-audit` re-export them, but that module doesn't exist yet.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Canonical audit record shape.
///
/// This matches the IDB specification:
///
/// ```text
/// #[serde(deny_unknown_fields)]
/// pub struct AuditRecord {
///   pub v: u16,             // schema major
///   pub ts_ms: u64,         // advisory wall-clock millis
///   pub writer_id: String,  // "svc-gateway@inst-123" (lex order stable)
///   pub seq: u64,           // strictly monotone per (writer_id, stream)
///   pub stream: String,     // "ingress" | "policy" | ...
///   pub kind: AuditKind,    // CapIssued | PolicyChanged | IndexWrite | ...
///   pub actor: ActorRef,    // {cap_id?, key_fpr?, passport_id?, anon?:bool}
///   pub subject: SubjectRef,// {content_id? "b3:<hex>", ledger_txid?, name?}
///   pub reason: ReasonCode, // normalized taxonomy
///   pub attrs: serde_json::Value, // ≤ 1 KiB canonicalized
///   pub prev: String,       // "b3:<hex>" previous or "b3:0" for genesis
///   pub self_hash: String,  // "b3:<hex>" over canonicalized record excl. self_hash
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRecord {
    /// Schema major version.
    pub v: u16,
    /// Advisory wall-clock timestamp (milliseconds since Unix epoch).
    pub ts_ms: u64,
    /// Logical writer identifier (e.g. "svc-edge@inst-123").
    pub writer_id: String,
    /// Strictly monotone per `(writer_id, stream)`.
    pub seq: u64,
    /// Logical stream (e.g. "ingress", "policy", "index").
    pub stream: String,
    /// High-level category of the event.
    pub kind: AuditKind,
    /// Actor performing the action.
    pub actor: ActorRef,
    /// Subject of the action.
    pub subject: SubjectRef,
    /// Normalized reason taxonomy.
    pub reason: ReasonCode,
    /// Free-form, canonicalized attributes (bounded in size).
    pub attrs: Value,
    /// Previous record's `self_hash` ("b3:<hex>" or "b3:0" for genesis).
    pub prev: String,
    /// Canonical BLAKE3 hash of the record excluding `self_hash`.
    pub self_hash: String,
}

/// High-level category of an audit event.
///
/// This is intentionally minimal for now; more variants can be added as the
/// taxonomy hardens.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    /// Fallback for events that haven't been fully classified yet.
    #[default]
    Unknown,
    /// Capability issued (e.g. macaroon/passport/cap token).
    CapIssued,
    /// Capability revoked or invalidated.
    CapRevoked,
    /// Policy document changed.
    PolicyChanged,
    /// Storage/index write operation.
    IndexWrite,
    /// A read / get operation was served.
    GetServed,
    /// Request was rejected due to quotas/limits.
    QuotaReject,
}

/// Reference to the actor performing the audited action.
///
/// Fields are optional; different layers may populate different subsets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActorRef {
    /// Capability identifier (e.g. passport/cap token id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_id: Option<String>,

    /// Key fingerprint (e.g. Ed25519 public key hash).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_fpr: Option<String>,

    /// Passport identifier (from svc-passport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_id: Option<String>,

    /// Whether the actor is anonymous.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anon: Option<bool>,
}

/// Reference to the subject of the audited action.
///
/// Again, all fields are optional so hosts can fill as much as they know.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubjectRef {
    /// ContentId / object hash ("b3:<hex>").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,

    /// Ledger transaction id (when present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ledger_txid: Option<String>,

    /// Human-readable name or label, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Normalized reason taxonomy.
///
/// For now this is a thin newtype over `String` so we can evolve the
/// vocabulary without breaking the wire schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReasonCode(pub String);

/// Fixed-size dedupe key derived from the canonical bytes of an `AuditRecord`.
///
/// This is the raw BLAKE3 output used for indexing / dedupe structures.
pub type DedupeKey = [u8; 32];

/// Export-friendly representation of a chain head.
///
/// Host crates can use this when exposing heads over admin/diagnostic APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainHeadDto {
    /// Stream identifier (e.g. logical stream or partition key).
    pub stream: String,
    /// Last known sequence number within the stream.
    pub seq: u64,
    /// Last known `self_hash` at the head of the stream.
    pub head: String,
}
