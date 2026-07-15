//! RO:WHAT — Deterministic bounded User Node object-verification queue.
//! RO:WHY — Phase 22 requires micronode to verify or challenge exact B3 material and emit canonical evidence.
//! RO:INTERACTS — ron-proto UserVerificationAccountingInputV1, AppState, local HTTP verification routes.
//! RO:INVARIANTS — replay rejected; queue bounded; full BLAKE3 review; evidence only; no economic authority.
//! RO:SECURITY — no IP/socket fields; opaque privacy-route ID only; no payout target, reward amount, wallet, or ledger mutation.
//! RO:TEST — tests/object_verification.rs and passive/admin status regressions.

#![forbid(unsafe_code)]

use parking_lot::Mutex;
use ron_proto::{
    ContentId, UserVerificationAccountingInputV1, UserVerificationEvidenceKindV1,
    UserVerificationFailureReasonV1, UserVerificationResultV1,
    USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA, USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use thiserror::Error;

pub const OBJECT_VERIFICATION_REQUEST_SCHEMA: &str = "micronode.object_verification_request.v1";

pub const OBJECT_VERIFICATION_REQUEST_VERSION: u16 = 1;

pub const OBJECT_VERIFICATION_RESULT_SCHEMA: &str = "micronode.object_verification_result.v1";

pub const OBJECT_VERIFICATION_PENDING_SCHEMA: &str = "micronode.pending_object_verifications.v1";

pub const MAX_VERIFICATION_OBJECT_BYTES: usize = 4 * 1024 * 1024;

pub const MAX_PENDING_VERIFICATION_READ_ITEMS: usize = 64;

const MAX_TOKEN_BYTES: usize = 512;
const USER_NODE_ID: &str = "micronode_local";
const CAPABILITY_ID: &str = "b3_integrity_challenge_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectVerificationRequest {
    pub schema: String,
    pub version: u16,

    pub object: String,
    pub bytes: Vec<u8>,

    pub observed_at_ms: u64,
    pub nonce: String,
    pub idempotency_key: String,

    /// Opaque route identity. It must not contain an IP,
    /// socket, URL, hostname, or raw transport address.
    pub privacy_route_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectVerificationResult {
    pub schema: &'static str,
    pub version: u16,

    pub queue_state: &'static str,
    pub queue_depth: usize,

    pub expected_object: String,
    pub calculated_object: String,

    pub full_digest_verified: bool,
    pub challenge_raised: bool,
    pub pending_evidence: bool,

    pub evidence: UserVerificationAccountingInputV1,

    pub privacy_safe: bool,
    pub raw_peer_ip_recorded: bool,

    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,

    pub confirmed_roc_minor_units: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingObjectVerifications {
    pub schema: &'static str,
    pub version: u16,
    pub count: usize,
    pub limit: usize,
    pub items: Vec<UserVerificationAccountingInputV1>,

    pub evidence_only: bool,
    pub accounting_accepted: bool,
    pub reward_eligible: bool,
    pub reward_truth: bool,
    pub payout_authority: bool,
    pub wallet_mutation: bool,
    pub ledger_mutation: bool,

    pub confirmed_roc_minor_units: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ObjectVerificationQueueError {
    #[error("invalid verification request: {0}")]
    InvalidRequest(String),

    #[error("duplicate verification idempotency key")]
    DuplicateIdempotencyKey,

    #[error("pending verification queue is full")]
    QueueFull,

    #[error("canonical verification evidence failed validation: {0}")]
    InvalidEvidence(String),

    #[error("verification sequence exhausted")]
    SequenceExhausted,
}

#[derive(Debug)]
struct QueueState {
    next_sequence: u64,
    seen_idempotency_keys: HashSet<String>,
    pending: VecDeque<UserVerificationAccountingInputV1>,
}

#[derive(Debug)]
pub struct ObjectVerificationQueue {
    capacity: usize,
    inner: Mutex<QueueState>,
}

impl ObjectVerificationQueue {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Mutex::new(QueueState {
                next_sequence: 1,
                seen_idempotency_keys: HashSet::new(),
                pending: VecDeque::new(),
            }),
        }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.inner.lock().pending.len()
    }

    #[must_use]
    pub fn pending(&self, limit: usize) -> PendingObjectVerifications {
        let guard = self.inner.lock();

        let items = guard.pending.iter().take(limit).cloned().collect::<Vec<_>>();

        PendingObjectVerifications {
            schema: OBJECT_VERIFICATION_PENDING_SCHEMA,
            version: 1,
            count: items.len(),
            limit,
            items,
            evidence_only: true,
            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
            confirmed_roc_minor_units: None,
        }
    }

    pub fn review_and_enqueue(
        &self,
        request: ObjectVerificationRequest,
    ) -> Result<ObjectVerificationResult, ObjectVerificationQueueError> {
        request.validate()?;

        let expected = ContentId::parse(&request.object).map_err(|error| {
            ObjectVerificationQueueError::InvalidRequest(format!(
                "object must be canonical B3: {error}"
            ))
        })?;

        let calculated_object = format!("b3:{}", blake3::hash(&request.bytes).to_hex(),);

        let calculated = ContentId::parse(&calculated_object).map_err(|error| {
            ObjectVerificationQueueError::InvalidEvidence(format!(
                "generated digest was invalid: {error}"
            ))
        })?;

        let full_digest_verified = calculated == expected;

        let (result, failure_reason, challenge_raised) = if full_digest_verified {
            (UserVerificationResultV1::VerifiedValid, None, false)
        } else {
            (
                UserVerificationResultV1::ChallengeRaised,
                Some(UserVerificationFailureReasonV1::DigestMismatch),
                true,
            )
        };

        let mut guard = self.inner.lock();

        if guard.seen_idempotency_keys.contains(&request.idempotency_key) {
            return Err(ObjectVerificationQueueError::DuplicateIdempotencyKey);
        }

        if guard.pending.len() >= self.capacity {
            return Err(ObjectVerificationQueueError::QueueFull);
        }

        let sequence = guard.next_sequence;

        guard.next_sequence = guard
            .next_sequence
            .checked_add(1)
            .ok_or(ObjectVerificationQueueError::SequenceExhausted)?;

        let identity_material = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            expected.as_str(),
            calculated.as_str(),
            request.observed_at_ms,
            request.nonce,
            request.idempotency_key,
            request.privacy_route_id,
        );

        let identity_digest = blake3::hash(identity_material.as_bytes()).to_hex().to_string();

        let expected_hex = expected.as_str().strip_prefix("b3:").ok_or_else(|| {
            ObjectVerificationQueueError::InvalidEvidence(
                "canonical expected object lost B3 prefix".to_string(),
            )
        })?;

        let evidence = UserVerificationAccountingInputV1 {
            schema: USER_VERIFICATION_ACCOUNTING_INPUT_SCHEMA.to_string(),

            version: USER_VERIFICATION_ACCOUNTING_INPUT_VERSION,

            sequence,

            evidence_id: format!("verification:{identity_digest}"),

            user_node_id: USER_NODE_ID.to_string(),

            subject_ref: format!("oap-object:{expected_hex}"),

            verification_kind: UserVerificationEvidenceKindV1::B3IntegrityChallenge,

            observed_at_ms: request.observed_at_ms,

            input_digest: calculated,

            result,
            failure_reason,

            nonce: request.nonce.clone(),

            idempotency_key: request.idempotency_key.clone(),

            capability_id: CAPABILITY_ID.to_string(),

            privacy_route_id: Some(request.privacy_route_id.clone()),

            attestation_ref: format!("local-attestation:{identity_digest}"),

            local_attestation_verified: true,

            evidence_only: true,

            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,
        };

        evidence
            .validate()
            .map_err(|error| ObjectVerificationQueueError::InvalidEvidence(error.to_string()))?;

        guard.seen_idempotency_keys.insert(request.idempotency_key);

        guard.pending.push_back(evidence.clone());

        let queue_depth = guard.pending.len();

        Ok(ObjectVerificationResult {
            schema: OBJECT_VERIFICATION_RESULT_SCHEMA,
            version: 1,

            queue_state: "pending",
            queue_depth,

            expected_object: expected.to_string(),

            calculated_object,

            full_digest_verified,
            challenge_raised,
            pending_evidence: true,

            evidence,

            privacy_safe: true,
            raw_peer_ip_recorded: false,

            accounting_accepted: false,
            reward_eligible: false,
            reward_truth: false,
            payout_authority: false,
            wallet_mutation: false,
            ledger_mutation: false,

            confirmed_roc_minor_units: None,
        })
    }
}

impl ObjectVerificationRequest {
    fn validate(&self) -> Result<(), ObjectVerificationQueueError> {
        if self.schema != OBJECT_VERIFICATION_REQUEST_SCHEMA {
            return Err(ObjectVerificationQueueError::InvalidRequest(
                "invalid request schema".to_string(),
            ));
        }

        if self.version != OBJECT_VERIFICATION_REQUEST_VERSION {
            return Err(ObjectVerificationQueueError::InvalidRequest(
                "invalid request version".to_string(),
            ));
        }

        ContentId::parse(&self.object).map_err(|error| {
            ObjectVerificationQueueError::InvalidRequest(format!(
                "object must be canonical B3: {error}"
            ))
        })?;

        if self.bytes.is_empty() {
            return Err(ObjectVerificationQueueError::InvalidRequest(
                "object bytes must not be empty".to_string(),
            ));
        }

        if self.bytes.len() > MAX_VERIFICATION_OBJECT_BYTES {
            return Err(ObjectVerificationQueueError::InvalidRequest(format!(
                "object bytes exceed maximum: {} > {}",
                self.bytes.len(),
                MAX_VERIFICATION_OBJECT_BYTES,
            )));
        }

        if self.observed_at_ms == 0 {
            return Err(ObjectVerificationQueueError::InvalidRequest(
                "observedAtMs must be greater than zero".to_string(),
            ));
        }

        validate_token("nonce", &self.nonce)?;

        validate_token("idempotencyKey", &self.idempotency_key)?;

        validate_privacy_route_id(&self.privacy_route_id)
    }
}

fn validate_token(field: &str, value: &str) -> Result<(), ObjectVerificationQueueError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b':' | b'.' | b'/')
        });

    if !valid {
        return Err(ObjectVerificationQueueError::InvalidRequest(format!(
            "{field} must be a bounded lowercase identifier token"
        )));
    }

    Ok(())
}

fn validate_privacy_route_id(value: &str) -> Result<(), ObjectVerificationQueueError> {
    let colon_count = value.bytes().filter(|byte| *byte == b':').count();

    let valid = !value.is_empty()
        && value.len() <= MAX_TOKEN_BYTES
        && colon_count <= 1
        && !value.starts_with(':')
        && !value.ends_with(':')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b':')
        });

    if !valid {
        return Err(ObjectVerificationQueueError::InvalidRequest(
            "privacyRouteId must be an opaque route identifier".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

    fn request(bytes: &[u8], idempotency_key: &str) -> ObjectVerificationRequest {
        ObjectVerificationRequest {
            schema: OBJECT_VERIFICATION_REQUEST_SCHEMA.to_string(),

            version: OBJECT_VERIFICATION_REQUEST_VERSION,

            object: ABC_CID.to_string(),
            bytes: bytes.to_vec(),
            observed_at_ms: 1_700_000_000_000,
            nonce: format!("{idempotency_key}-nonce"),

            idempotency_key: idempotency_key.to_string(),

            privacy_route_id: "relay:phase22e".to_string(),
        }
    }

    #[test]
    fn valid_and_corrupt_bytes_create_distinct_evidence_results() {
        let queue = ObjectVerificationQueue::new(4);

        let valid = queue
            .review_and_enqueue(request(b"abc", "phase22e-valid"))
            .expect("valid verification");

        assert!(valid.full_digest_verified);
        assert!(!valid.challenge_raised);

        assert_eq!(valid.evidence.result, UserVerificationResultV1::VerifiedValid,);

        assert!(valid.evidence.failure_reason.is_none());

        let corrupt = queue
            .review_and_enqueue(request(b"abd", "phase22e-corrupt"))
            .expect("corrupt challenge");

        assert!(!corrupt.full_digest_verified);
        assert!(corrupt.challenge_raised);

        assert_eq!(corrupt.evidence.result, UserVerificationResultV1::ChallengeRaised,);

        assert_eq!(
            corrupt.evidence.failure_reason,
            Some(UserVerificationFailureReasonV1::DigestMismatch,),
        );

        assert_eq!(queue.pending_count(), 2);
    }

    #[test]
    fn replay_and_queue_overflow_are_rejected() {
        let queue = ObjectVerificationQueue::new(1);

        queue.review_and_enqueue(request(b"abc", "phase22e-one")).expect("first verification");

        let duplicate = queue
            .review_and_enqueue(request(b"abc", "phase22e-one"))
            .expect_err("duplicate must reject");

        assert_eq!(duplicate, ObjectVerificationQueueError::DuplicateIdempotencyKey,);

        let full = queue
            .review_and_enqueue(request(b"abd", "phase22e-two"))
            .expect_err("full queue must reject");

        assert_eq!(full, ObjectVerificationQueueError::QueueFull,);
    }
}
