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
