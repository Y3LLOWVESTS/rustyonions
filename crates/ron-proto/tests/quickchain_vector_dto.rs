use ron_proto::{
    ContentId, QuickChainTestVectorV1, QuickChainVectorCanonicalEncodingV1,
    QuickChainVectorHashAlgorithmV1, QuickChainVectorPreimageFramingV1, QuickChainVectorStatusV1,
    QUICKCHAIN_DTO_VERSION, QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, QUICKCHAIN_TEST_VECTOR_SCHEMA,
};
use serde_json::json;

const PAYLOAD_UTF8: &str = "{\"schema\":\"demo\"}";
const PAYLOAD_HEX: &str = "7b22736368656d61223a2264656d6f227d";
const RECEIPT_PREIMAGE_HEX: &str =
    "717569636b636861696e2e726563656970742e7631007b22736368656d61223a2264656d6f227d";

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn vector(status: QuickChainVectorStatusV1) -> QuickChainTestVectorV1 {
    QuickChainTestVectorV1 {
        schema: QUICKCHAIN_TEST_VECTOR_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        vector_id: "canonical_receipt_vector_001".to_string(),
        status,
        purpose: "receipt_canonical_bytes".to_string(),
        domain_separator: QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1.to_string(),
        canonical_encoding: QuickChainVectorCanonicalEncodingV1::CanonicalJsonV1,
        preimage_framing: QuickChainVectorPreimageFramingV1::DomainSeparatorNulPayload,
        hash_algorithm: QuickChainVectorHashAlgorithmV1::Blake3_256,
        human_readable_json: json!({ "schema": "demo" }),
        canonical_payload_utf8: None,
        canonical_payload_hex: None,
        preimage_hex: None,
        expected_b3: None,
        notes: Vec::new(),
    }
}

#[test]
fn sketch_vector_requires_no_fake_bytes_or_hash() {
    vector(QuickChainVectorStatusV1::Sketch).validate().unwrap();

    let mut sketch = vector(QuickChainVectorStatusV1::Sketch);
    sketch.canonical_payload_utf8 = Some(PAYLOAD_UTF8.to_string());
    sketch
        .validate()
        .expect_err("sketch vectors must not carry canonical bytes");
}

#[test]
fn locked_bytes_requires_matching_canonical_payload_fields() {
    let mut missing = vector(QuickChainVectorStatusV1::LockedBytes);
    missing
        .validate()
        .expect_err("locked_bytes must require payload fields");

    missing.canonical_payload_utf8 = Some(PAYLOAD_UTF8.to_string());
    missing.canonical_payload_hex = Some(PAYLOAD_HEX.to_string());
    missing.validate().unwrap();

    let mut bad_hex = missing.clone();
    bad_hex.canonical_payload_hex = Some("64656d6f".to_string());
    bad_hex
        .validate()
        .expect_err("canonical_payload_hex must match canonical_payload_utf8 bytes");

    let mut fake_hash = missing;
    fake_hash.expected_b3 = Some(cid('b'));
    fake_hash
        .validate()
        .expect_err("locked_bytes must not carry expected_b3");
}

#[test]
fn locked_hash_requires_exact_preimage_and_expected_b3() {
    let mut locked = vector(QuickChainVectorStatusV1::LockedHash);
    locked.canonical_payload_utf8 = Some(PAYLOAD_UTF8.to_string());
    locked.canonical_payload_hex = Some(PAYLOAD_HEX.to_string());
    locked
        .validate()
        .expect_err("locked_hash must require preimage_hex and expected_b3");

    locked.preimage_hex = Some(RECEIPT_PREIMAGE_HEX.to_string());
    locked.expected_b3 = Some(cid('c'));
    locked.validate().unwrap();

    let mut wrong_preimage = locked;
    wrong_preimage.preimage_hex = Some("00".to_string());
    wrong_preimage
        .validate()
        .expect_err("preimage_hex must equal domain || nul || payload");
}

#[test]
fn vector_required_wire_fields_are_present_and_unknown_fields_reject() {
    let mut value = serde_json::to_value(vector(QuickChainVectorStatusV1::Sketch)).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("surprise".to_string(), json!(true));
    serde_json::from_value::<QuickChainTestVectorV1>(value)
        .expect_err("unknown vector fields must reject");

    let mut missing = serde_json::to_value(vector(QuickChainVectorStatusV1::Sketch)).unwrap();
    missing.as_object_mut().unwrap().remove("preimage_hex");
    serde_json::from_value::<QuickChainTestVectorV1>(missing)
        .expect_err("required nullable vector fields must be present");

    let mut missing = serde_json::to_value(vector(QuickChainVectorStatusV1::Sketch)).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("human_readable_json");
    serde_json::from_value::<QuickChainTestVectorV1>(missing)
        .expect_err("human_readable_json must be present");
}

#[test]
fn vector_tags_have_exact_wire_values_and_unknown_values_reject() {
    assert_eq!(
        serde_json::to_string(&QuickChainVectorCanonicalEncodingV1::CanonicalJsonV1).unwrap(),
        "\"quickchain.canonical-json.v1\""
    );
    assert_eq!(
        serde_json::to_string(&QuickChainVectorPreimageFramingV1::DomainSeparatorNulPayload)
            .unwrap(),
        "\"domain_separator_bytes || 0x00 || canonical_payload_bytes\""
    );
    assert_eq!(
        serde_json::to_string(&QuickChainVectorHashAlgorithmV1::Blake3_256).unwrap(),
        "\"blake3-256\""
    );

    serde_json::from_value::<QuickChainVectorCanonicalEncodingV1>(json!("canonical-json"))
        .expect_err("unknown canonical encoding must reject");
    serde_json::from_value::<QuickChainVectorPreimageFramingV1>(json!("payload_only"))
        .expect_err("unknown preimage framing must reject");
    serde_json::from_value::<QuickChainVectorHashAlgorithmV1>(json!("sha256"))
        .expect_err("unknown hash algorithm must reject");
}

#[test]
fn vector_rejects_bad_tokens_and_null_human_json() {
    let mut bad = vector(QuickChainVectorStatusV1::Sketch);
    bad.vector_id = "Canonical Receipt Vector".to_string();
    bad.validate().expect_err("vector id must be token-shaped");

    let mut bad = vector(QuickChainVectorStatusV1::Sketch);
    bad.purpose = "receipt canonical bytes".to_string();
    bad.validate().expect_err("purpose must be token-shaped");

    let mut bad = vector(QuickChainVectorStatusV1::Sketch);
    bad.human_readable_json = serde_json::Value::Null;
    bad.validate()
        .expect_err("human_readable_json must be non-null");
}
