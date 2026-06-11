use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainEmptyTreeKindV1,
    QuickChainEmptyTreeV1, QuickChainTestVectorV1, QuickChainValidationError,
    QuickChainVectorCanonicalEncodingV1, QuickChainVectorHashAlgorithmV1,
    QuickChainVectorPreimageFramingV1, QuickChainVectorStatusV1, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_EMPTY_TREE_SCHEMA, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_TEST_VECTOR_SCHEMA,
};
use serde_json::json;

fn empty_tree(tree: QuickChainEmptyTreeKindV1) -> QuickChainEmptyTreeV1 {
    QuickChainEmptyTreeV1 {
        schema: QUICKCHAIN_EMPTY_TREE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        tree,
    }
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
fn empty_tree_kind_wire_names_are_exact_and_unknown_values_reject() {
    let cases = [
        (QuickChainEmptyTreeKindV1::State, "state"),
        (QuickChainEmptyTreeKindV1::Holds, "holds"),
        (QuickChainEmptyTreeKindV1::Receipts, "receipts"),
        (QuickChainEmptyTreeKindV1::Accounting, "accounting"),
        (QuickChainEmptyTreeKindV1::Rewards, "rewards"),
    ];

    for (variant, wire) in cases {
        assert_eq!(
            serde_json::to_string(&variant).unwrap(),
            format!("\"{wire}\"")
        );

        let decoded: QuickChainEmptyTreeKindV1 = serde_json::from_value(json!(wire)).unwrap();

        assert_eq!(decoded, variant);
    }

    serde_json::from_value::<QuickChainEmptyTreeKindV1>(json!("unknown"))
        .expect_err("unknown empty-tree kind must reject");
}

#[test]
fn empty_tree_variants_have_exact_canonical_json_bytes() {
    let cases = [
        (QuickChainEmptyTreeKindV1::State, "state"),
        (QuickChainEmptyTreeKindV1::Holds, "holds"),
        (QuickChainEmptyTreeKindV1::Receipts, "receipts"),
        (QuickChainEmptyTreeKindV1::Accounting, "accounting"),
        (QuickChainEmptyTreeKindV1::Rewards, "rewards"),
    ];

    for (tree_kind, wire) in cases {
        let payload = empty_tree(tree_kind);
        payload.validate().unwrap();

        let expected = format!(
            "{{\"schema\":\"quickchain.empty-tree.v1\",\"version\":1,\"tree\":\"{wire}\"}}"
        );

        let canonical = to_canonical_json_string(&payload).unwrap();
        assert_eq!(canonical, expected);

        let decoded: QuickChainEmptyTreeV1 =
            from_canonical_json_slice(expected.as_bytes()).unwrap();

        assert_eq!(decoded, payload);
    }
}

#[test]
fn state_empty_tree_locked_bytes_vector_validates_without_fake_hash() {
    let payload = empty_tree(QuickChainEmptyTreeKindV1::State);
    let canonical_payload_utf8 = to_canonical_json_string(&payload).unwrap();
    let canonical_payload_hex = lower_hex(canonical_payload_utf8.as_bytes());

    let vector = QuickChainTestVectorV1 {
        schema: QUICKCHAIN_TEST_VECTOR_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        vector_id: "canonical_empty_tree_state_vector_001".to_string(),
        status: QuickChainVectorStatusV1::LockedBytes,
        purpose: "empty_tree_canonical_bytes".to_string(),
        domain_separator: QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1.to_string(),
        canonical_encoding: QuickChainVectorCanonicalEncodingV1::CanonicalJsonV1,
        preimage_framing: QuickChainVectorPreimageFramingV1::DomainSeparatorNulPayload,
        hash_algorithm: QuickChainVectorHashAlgorithmV1::Blake3_256,
        human_readable_json: serde_json::to_value(&payload).unwrap(),
        canonical_payload_utf8: Some(canonical_payload_utf8),
        canonical_payload_hex: Some(canonical_payload_hex),
        preimage_hex: None,
        expected_b3: None,
        notes: vec!["locked bytes only; no preimage or expected hash".to_string()],
    };

    vector.validate().unwrap();

    assert!(vector.preimage_hex.is_none());
    assert!(vector.expected_b3.is_none());
}

#[test]
fn empty_tree_rejects_unknown_fields_wrong_schema_and_wrong_version() {
    let mut unknown = serde_json::to_value(empty_tree(QuickChainEmptyTreeKindV1::State)).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("empty_root".to_string(), json!("b3:not-authority"));

    serde_json::from_value::<QuickChainEmptyTreeV1>(unknown)
        .expect_err("unknown empty-tree fields must reject");

    let mut wrong_schema = empty_tree(QuickChainEmptyTreeKindV1::State);
    wrong_schema.schema = "quickchain.state-root.v1".to_string();

    assert!(matches!(
        wrong_schema.validate().unwrap_err(),
        QuickChainValidationError::InvalidSchema {
            field: "QuickChainEmptyTreeV1.schema",
            ..
        }
    ));

    let mut wrong_version = empty_tree(QuickChainEmptyTreeKindV1::State);
    wrong_version.version = QUICKCHAIN_DTO_VERSION + 1;

    assert!(matches!(
        wrong_version.validate().unwrap_err(),
        QuickChainValidationError::InvalidVersion {
            field: "QuickChainEmptyTreeV1.version",
            ..
        }
    ));
}

#[test]
fn missing_empty_tree_fields_reject_during_deserialization() {
    for field in ["schema", "version", "tree"] {
        let mut value = serde_json::to_value(empty_tree(QuickChainEmptyTreeKindV1::State)).unwrap();

        value.as_object_mut().unwrap().remove(field);

        serde_json::from_value::<QuickChainEmptyTreeV1>(value)
            .expect_err("required empty-tree field must reject when missing");
    }
}
