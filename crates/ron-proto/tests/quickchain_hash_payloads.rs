//! RO:WHAT — Verifies immutable QuickChain hash-payload boundaries and five genuine locked_hash vectors.
//! RO:WHY — ECON/RES: independently reviewed bytes and BLAKE3 expectations must exist before any root-producing code.
//! RO:INVARIANTS — test-only hashing over published vector bytes; no live ledger state, roots, checkpoints, signatures, or mutation.
//! RO:TEST — paired with verify_quickchain_hash_payloads.py.

use std::fmt::Debug;

use ron_proto::{
    QuickChainAccountLeafPayloadV1, QuickChainActiveHoldLeafPayloadV1,
    QuickChainOperationHashPayloadV1, QuickChainReceiptHashPayloadV1, QuickChainResult,
    QuickChainTestVectorV1, QuickChainUnsignedCheckpointPayloadV1, QuickChainVectorStatusV1,
    QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1, QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1, QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

const OPERATION_VECTOR: &str =
    include_str!("vectors/quickchain/hash_payloads/operation_hash_payload_locked_hash_v1.json");
const RECEIPT_VECTOR: &str =
    include_str!("vectors/quickchain/hash_payloads/receipt_hash_payload_locked_hash_v1.json");
const ACCOUNT_VECTOR: &str =
    include_str!("vectors/quickchain/hash_payloads/account_leaf_payload_locked_hash_v1.json");
const HOLD_VECTOR: &str =
    include_str!("vectors/quickchain/hash_payloads/active_hold_leaf_payload_locked_hash_v1.json");
const CHECKPOINT_VECTOR: &str = include_str!(
    "vectors/quickchain/hash_payloads/unsigned_checkpoint_payload_locked_hash_v1.json"
);

fn assert_locked_hash_vector<T>(
    raw: &str,
    expected_domain: &str,
    validate: impl Fn(&T) -> QuickChainResult<()>,
) -> T
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    let vector: QuickChainTestVectorV1 = serde_json::from_str(raw).unwrap();
    vector.validate().unwrap();

    assert_eq!(vector.status, QuickChainVectorStatusV1::LockedHash);
    assert_eq!(vector.domain_separator, expected_domain);
    assert!(!vector.notes.is_empty());

    let canonical_payload = vector.canonical_payload_utf8.as_ref().unwrap();
    let payload: T = serde_json::from_str(canonical_payload).unwrap();
    validate(&payload).unwrap();

    let typed_canonical = serde_json::to_string(&payload).unwrap();
    assert_eq!(typed_canonical, canonical_payload.as_str());

    let human_payload: T = serde_json::from_value(vector.human_readable_json.clone()).unwrap();
    assert_eq!(human_payload, payload);

    let mut preimage = Vec::with_capacity(expected_domain.len() + 1 + canonical_payload.len());
    preimage.extend_from_slice(expected_domain.as_bytes());
    preimage.push(0);
    preimage.extend_from_slice(canonical_payload.as_bytes());

    assert_eq!(
        lower_hex(&preimage),
        vector.preimage_hex.as_deref().unwrap()
    );

    let actual_b3 = format!("b3:{}", blake3::hash(&preimage).to_hex());
    assert_eq!(actual_b3, vector.expected_b3.as_ref().unwrap().as_str());

    payload
}

fn vector_payload_value(raw: &str) -> Value {
    let vector: QuickChainTestVectorV1 = serde_json::from_str(raw).unwrap();
    serde_json::from_str(vector.canonical_payload_utf8.as_ref().unwrap()).unwrap()
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }

    output
}

#[test]
fn operation_hash_payload_has_exact_bytes_preimage_and_b3() {
    assert_locked_hash_vector::<QuickChainOperationHashPayloadV1>(
        OPERATION_VECTOR,
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        QuickChainOperationHashPayloadV1::validate,
    );
}

#[test]
fn receipt_hash_payload_has_exact_bytes_preimage_and_b3() {
    assert_locked_hash_vector::<QuickChainReceiptHashPayloadV1>(
        RECEIPT_VECTOR,
        QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QuickChainReceiptHashPayloadV1::validate,
    );
}

#[test]
fn account_leaf_payload_has_exact_bytes_preimage_and_b3() {
    assert_locked_hash_vector::<QuickChainAccountLeafPayloadV1>(
        ACCOUNT_VECTOR,
        QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
        QuickChainAccountLeafPayloadV1::validate,
    );
}

#[test]
fn active_hold_leaf_payload_has_exact_bytes_preimage_and_b3() {
    assert_locked_hash_vector::<QuickChainActiveHoldLeafPayloadV1>(
        HOLD_VECTOR,
        QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
        QuickChainActiveHoldLeafPayloadV1::validate,
    );
}

#[test]
fn unsigned_checkpoint_payload_has_exact_bytes_preimage_and_b3() {
    assert_locked_hash_vector::<QuickChainUnsignedCheckpointPayloadV1>(
        CHECKPOINT_VECTOR,
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        QuickChainUnsignedCheckpointPayloadV1::validate,
    );
}

#[test]
fn operation_hash_boundary_rejects_runtime_and_receipt_fields() {
    for forbidden in ["produced_at_ms", "receipt_hash", "settlement_status"] {
        let mut value = vector_payload_value(OPERATION_VECTOR);
        value
            .as_object_mut()
            .unwrap()
            .insert(forbidden.to_string(), json!(true));

        serde_json::from_value::<QuickChainOperationHashPayloadV1>(value)
            .expect_err("operation hash payload must reject non-preimage fields");
    }

    let mut payload = assert_locked_hash_vector::<QuickChainOperationHashPayloadV1>(
        OPERATION_VECTOR,
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        QuickChainOperationHashPayloadV1::validate,
    );
    payload.idempotency_scope_operation_family = "transfer".to_string();
    payload
        .validate()
        .expect_err("idempotency scope family must match operation class");
}

#[test]
fn receipt_hash_boundary_rejects_self_reference_and_mutable_evidence() {
    for forbidden in [
        "receipt_hash",
        "receipt_root",
        "checkpoint_hash",
        "status",
        "memo",
        "signatures",
    ] {
        let mut value = vector_payload_value(RECEIPT_VECTOR);
        value
            .as_object_mut()
            .unwrap()
            .insert(forbidden.to_string(), json!(null));

        serde_json::from_value::<QuickChainReceiptHashPayloadV1>(value)
            .expect_err("receipt hash payload must reject self-reference and later evidence");
    }

    let mut payload = assert_locked_hash_vector::<QuickChainReceiptHashPayloadV1>(
        RECEIPT_VECTOR,
        QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QuickChainReceiptHashPayloadV1::validate,
    );
    payload.hold_id = None;
    payload
        .validate()
        .expect_err("hold-open receipt hash payload must require hold_id");
}

#[test]
fn receipt_payload_binds_the_reviewed_operation_hash_vector() {
    let operation_vector: QuickChainTestVectorV1 = serde_json::from_str(OPERATION_VECTOR).unwrap();
    let receipt = assert_locked_hash_vector::<QuickChainReceiptHashPayloadV1>(
        RECEIPT_VECTOR,
        QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QuickChainReceiptHashPayloadV1::validate,
    );

    assert_eq!(
        receipt.operation_hash,
        operation_vector.expected_b3.unwrap()
    );

    let operation = assert_locked_hash_vector::<QuickChainOperationHashPayloadV1>(
        OPERATION_VECTOR,
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        QuickChainOperationHashPayloadV1::validate,
    );

    assert_eq!(receipt.operation_id, operation.operation_id);
    assert_eq!(receipt.op_class, operation.op_class);
    assert_eq!(receipt.idempotency_key, operation.idempotency_key);
}

#[test]
fn account_and_active_hold_payloads_reject_semantic_drift() {
    let mut account = assert_locked_hash_vector::<QuickChainAccountLeafPayloadV1>(
        ACCOUNT_VECTOR,
        QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
        QuickChainAccountLeafPayloadV1::validate,
    );
    account.available_minor = "751".to_string();
    account
        .validate()
        .expect_err("account leaf arithmetic drift must reject");

    let mut terminal_status = vector_payload_value(HOLD_VECTOR);
    terminal_status
        .as_object_mut()
        .unwrap()
        .insert("status".to_string(), json!("captured"));
    serde_json::from_value::<QuickChainActiveHoldLeafPayloadV1>(terminal_status)
        .expect_err("active hold leaf must represent open holds only");

    let mut terminal_field = vector_payload_value(HOLD_VECTOR);
    terminal_field.as_object_mut().unwrap().insert(
        "terminal_operation_id".to_string(),
        json!("op_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    serde_json::from_value::<QuickChainActiveHoldLeafPayloadV1>(terminal_field)
        .expect_err("active hold leaf must reject terminal lifecycle fields");
}

#[test]
fn unsigned_checkpoint_boundary_rejects_signatures_and_bad_conservation() {
    let mut signed = vector_payload_value(CHECKPOINT_VECTOR);
    signed
        .as_object_mut()
        .unwrap()
        .insert("signatures".to_string(), json!([]));
    serde_json::from_value::<QuickChainUnsignedCheckpointPayloadV1>(signed)
        .expect_err("checkpoint hash payload must structurally exclude signatures");

    let mut checkpoint = assert_locked_hash_vector::<QuickChainUnsignedCheckpointPayloadV1>(
        CHECKPOINT_VECTOR,
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        QuickChainUnsignedCheckpointPayloadV1::validate,
    );
    checkpoint.conservation.credits_minor = "576".to_string();
    checkpoint
        .validate()
        .expect_err("checkpoint conservation drift must reject");

    let mut supply_drift = assert_locked_hash_vector::<QuickChainUnsignedCheckpointPayloadV1>(
        CHECKPOINT_VECTOR,
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        QuickChainUnsignedCheckpointPayloadV1::validate,
    );
    supply_drift.supply_delta.net_minor = "76".to_string();
    supply_drift
        .validate()
        .expect_err("checkpoint supply delta drift must reject");
}
