use ron_proto::{
    ContentId, QuickChainChallengeTypeV1, QuickChainChallengeV1, QUICKCHAIN_CHALLENGE_SCHEMA,
    QUICKCHAIN_DTO_VERSION,
};
use serde_json::json;

fn cid(ch: char) -> ContentId {
    let hex = ch.to_string().repeat(64);
    format!("b3:{hex}").parse().unwrap()
}

#[test]
fn revised_challenge_types_have_expected_wire_names() {
    let cases = [
        (
            QuickChainChallengeTypeV1::InvalidChainParamsHash,
            "invalid_chain_params_hash",
        ),
        (
            QuickChainChallengeTypeV1::DuplicateOperationCommit,
            "duplicate_operation_commit",
        ),
        (
            QuickChainChallengeTypeV1::InvalidCheckpointHash,
            "invalid_checkpoint_hash",
        ),
        (
            QuickChainChallengeTypeV1::InvalidValidatorSet,
            "invalid_validator_set",
        ),
        (
            QuickChainChallengeTypeV1::InvalidValidatorSignature,
            "invalid_validator_signature",
        ),
        (
            QuickChainChallengeTypeV1::InvalidCheckpointEvidence,
            "invalid_checkpoint_evidence",
        ),
        (
            QuickChainChallengeTypeV1::RawEngagementRewardAbuse,
            "raw_engagement_reward_abuse",
        ),
    ];

    for (variant, wire) in cases {
        let encoded = serde_json::to_string(&variant).unwrap();
        assert_eq!(encoded, format!("\"{wire}\""));

        let decoded: QuickChainChallengeTypeV1 = serde_json::from_value(json!(wire)).unwrap();
        assert_eq!(decoded, variant);
    }
}

#[test]
fn duplicate_operation_commit_challenge_validates() {
    let challenge = QuickChainChallengeV1 {
        schema: QUICKCHAIN_CHALLENGE_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: "roc-dev".to_string(),
        checkpoint_hash: cid('a'),
        challenger_id: "account:creator-a".to_string(),
        challenge_type: QuickChainChallengeTypeV1::DuplicateOperationCommit,
        evidence_cid: cid('b'),
        submitted_at_ms: 1_800_000_000_000,
    };

    challenge.validate().unwrap();

    let json = serde_json::to_string(&challenge).unwrap();
    assert!(json.contains("\"challenge_type\":\"duplicate_operation_commit\""));
}

#[test]
fn checkpoint_integrity_challenge_types_validate() {
    let cases = [
        QuickChainChallengeTypeV1::InvalidCheckpointHash,
        QuickChainChallengeTypeV1::InvalidValidatorSet,
        QuickChainChallengeTypeV1::InvalidValidatorSignature,
        QuickChainChallengeTypeV1::InvalidCheckpointEvidence,
    ];

    for challenge_type in cases {
        let challenge = QuickChainChallengeV1 {
            schema: QUICKCHAIN_CHALLENGE_SCHEMA.to_string(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: "ron-devnet".to_string(),

            checkpoint_hash: cid('c'),

            challenger_id: "user_node:phase19-challenger".to_string(),

            challenge_type,

            evidence_cid: cid('d'),

            submitted_at_ms: 1_800_000_000_000,
        };

        challenge
            .validate()
            .expect("checkpoint-integrity challenge must remain a valid descriptive DTO");
    }
}
