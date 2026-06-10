use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, validate_canonical_json_roundtrip,
    ContentId, QuickChainAccountStateV1, QuickChainCanonicalEncodingV1,
    QuickChainCheckpointHeaderV1, QuickChainReceiptRootSchemeV1, QuickChainStateRootSchemeV1,
    QUICKCHAIN_ACCOUNT_STATE_SCHEMA, QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA, QUICKCHAIN_DTO_VERSION,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

fn account_state() -> QuickChainAccountStateV1 {
    QuickChainAccountStateV1 {
        schema: QUICKCHAIN_ACCOUNT_STATE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        account_id: "account:creator-a".to_string(),
        available_minor_units: "1000".to_string(),
        held_minor_units: "0".to_string(),
        nonce: 7,
        last_ledger_seq: 42,
    }
}

fn checkpoint_header_without_signatures() -> QuickChainCheckpointHeaderV1 {
    QuickChainCheckpointHeaderV1 {
        schema: QUICKCHAIN_CHECKPOINT_HEADER_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        height: 1,
        epoch_id: "roc-dev:height:000000000001".to_string(),
        previous_checkpoint_hash: cid('0'),
        previous_state_root: cid('1'),
        new_state_root: cid('2'),
        receipt_root: cid('3'),
        accounting_snapshot_root: cid('4'),
        reward_manifest_root: cid('5'),
        data_availability_root: cid('6'),
        policy_hash: cid('7'),
        validator_set_hash: cid('8'),
        chain_params_hash: cid('9'),
        canonical_encoding: QuickChainCanonicalEncodingV1::JsonV1,
        state_root_scheme: QuickChainStateRootSchemeV1::SortedMerkleMapV1,
        receipt_root_scheme: QuickChainReceiptRootSchemeV1::LedgerSequenceMerkleV1,
        supply_delta_minor_units: "0".to_string(),
        started_at_ms: 1_800_000_000_000,
        ended_at_ms: 1_800_000_060_000,
        produced_at_ms: 1_800_000_061_000,
        signatures: Vec::new(),
    }
}

#[test]
fn account_state_canonical_json_has_exact_bytes() {
    let canonical = to_canonical_json_string(&account_state()).unwrap();

    assert_eq!(
        canonical,
        concat!(
            r#"{"schema":"quickchain.account_state.v1","#,
            r#""version":1,"#,
            r#""chain_id":"roc-dev","#,
            r#""account_id":"account:creator-a","#,
            r#""available_minor_units":"1000","#,
            r#""held_minor_units":"0","#,
            r#""nonce":7,"#,
            r#""last_ledger_seq":42}"#
        )
    );
}

#[test]
fn checkpoint_header_canonical_json_has_exact_bytes() {
    let canonical = to_canonical_json_string(&checkpoint_header_without_signatures()).unwrap();

    assert_eq!(
        canonical,
        concat!(
            r#"{"schema":"quickchain.checkpoint_header.v1","#,
            r#""version":1,"#,
            r#""chain_id":"roc-dev","#,
            r#""height":1,"#,
            r#""epoch_id":"roc-dev:height:000000000001","#,
            r#""previous_checkpoint_hash":"b3:0000000000000000000000000000000000000000000000000000000000000000","#,
            r#""previous_state_root":"b3:1111111111111111111111111111111111111111111111111111111111111111","#,
            r#""new_state_root":"b3:2222222222222222222222222222222222222222222222222222222222222222","#,
            r#""receipt_root":"b3:3333333333333333333333333333333333333333333333333333333333333333","#,
            r#""accounting_snapshot_root":"b3:4444444444444444444444444444444444444444444444444444444444444444","#,
            r#""reward_manifest_root":"b3:5555555555555555555555555555555555555555555555555555555555555555","#,
            r#""data_availability_root":"b3:6666666666666666666666666666666666666666666666666666666666666666","#,
            r#""policy_hash":"b3:7777777777777777777777777777777777777777777777777777777777777777","#,
            r#""validator_set_hash":"b3:8888888888888888888888888888888888888888888888888888888888888888","#,
            r#""chain_params_hash":"b3:9999999999999999999999999999999999999999999999999999999999999999","#,
            r#""canonical_encoding":"json-v1","#,
            r#""state_root_scheme":"sorted_merkle_map_v1","#,
            r#""receipt_root_scheme":"ledger_sequence_merkle_v1","#,
            r#""supply_delta_minor_units":"0","#,
            r#""started_at_ms":1800000000000,"#,
            r#""ended_at_ms":1800000060000,"#,
            r#""produced_at_ms":1800000061000,"#,
            r#""signatures":[]}"#
        )
    );
}

#[test]
fn shuffled_input_fields_canonicalize_to_struct_order() {
    let shuffled = json!({
        "last_ledger_seq": 42,
        "nonce": 7,
        "held_minor_units": "0",
        "available_minor_units": "1000",
        "account_id": "account:creator-a",
        "chain_id": "roc-dev",
        "version": QUICKCHAIN_DTO_VERSION,
        "schema": QUICKCHAIN_ACCOUNT_STATE_SCHEMA
    });

    let decoded: QuickChainAccountStateV1 =
        from_canonical_json_slice(shuffled.to_string().as_bytes()).unwrap();

    assert_eq!(
        to_canonical_json_string(&decoded).unwrap(),
        to_canonical_json_string(&account_state()).unwrap()
    );
}

#[test]
fn unknown_fields_reject_before_canonicalization() {
    let value = json!({
        "schema": QUICKCHAIN_ACCOUNT_STATE_SCHEMA,
        "version": QUICKCHAIN_DTO_VERSION,
        "chain_id": "roc-dev",
        "account_id": "account:creator-a",
        "available_minor_units": "1000",
        "held_minor_units": "0",
        "nonce": 7,
        "last_ledger_seq": 42,
        "extra_field": "must-reject"
    });

    let err = from_canonical_json_slice::<QuickChainAccountStateV1>(value.to_string().as_bytes())
        .unwrap_err();

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn canonical_roundtrip_preserves_checkpoint_value() {
    let checkpoint = checkpoint_header_without_signatures();

    validate_canonical_json_roundtrip(&checkpoint).unwrap();
}
