//! RO:WHAT — Internal ROC Beta Phase 2 DTO proof for paid-flow operation histories.
//! RO:WHY — ECON/RES: replay proof should reuse existing strict operation DTOs before adding any new shape.
//! RO:INTERACTS — ron_proto::quickchain operation/replay DTOs and future ron-ledger accepted histories.
//! RO:INVARIANTS — DTO-only; account_sequence remains ledger-assigned; no receipt/balance/finality authority.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — idempotency keys are retry metadata only; no bridge, staking, or external settlement fields.
//! RO:TEST — this file and crates/ron-proto/scripts/dev-internal-roc-beta-phase2-preflight.sh.

use ron_proto::{
    from_canonical_json_slice, to_canonical_json_string, QuickChainOperationClassV1,
    QuickChainOperationIntentV1, QUICKCHAIN_DTO_VERSION, QUICKCHAIN_OPERATION_INTENT_SCHEMA,
};

const CHAIN_ID: &str = "ron-devnet";

fn operation_id(hex_digit: char) -> String {
    format!("op_{}", hex_digit.to_string().repeat(32))
}

fn hold_id(hex_digit: char) -> String {
    format!("hold_{}", hex_digit.to_string().repeat(32))
}

#[allow(clippy::too_many_arguments)]
fn intent(
    operation_hex_digit: char,
    idempotency_key: &str,
    op_class: QuickChainOperationClassV1,
    actor: &str,
    counterparty: Option<&str>,
    amount_minor: &str,
    hold_id: Option<&str>,
    produced_at_ms: u64,
) -> QuickChainOperationIntentV1 {
    QuickChainOperationIntentV1 {
        schema: QUICKCHAIN_OPERATION_INTENT_SCHEMA.to_string(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: CHAIN_ID.to_string(),
        operation_id: operation_id(operation_hex_digit),
        idempotency_key: idempotency_key.to_string(),
        op_class,
        actor_account_id: actor.to_string(),
        counterparty_account_id: counterparty.map(str::to_string),
        amount_minor: Some(amount_minor.to_string()),
        hold_id: hold_id.map(str::to_string),
        account_sequence: None,
        produced_at_ms,
    }
}

#[test]
fn paid_content_histories_use_existing_transfer_intent_contract() {
    let paid_actions = [
        ("paid_image", '1'),
        ("paid_site", '2'),
        ("paid_site_visit", '3'),
        ("paid_post", '4'),
        ("paid_comment", '5'),
        ("paid_article", '6'),
        ("paid_content_view", '7'),
    ];

    for (action, operation_hex_digit) in paid_actions {
        let submitted = intent(
            operation_hex_digit,
            &format!("idem:internal-roc-beta-phase2:{action}"),
            QuickChainOperationClassV1::Transfer,
            "account:viewer-a",
            Some("account:creator-a"),
            "10",
            None,
            1_800_000_000_000 + u64::from(operation_hex_digit.to_digit(16).unwrap()),
        );

        submitted
            .validate()
            .unwrap_or_else(|error| panic!("{action} transfer intent should validate: {error}"));

        assert_eq!(submitted.op_class, QuickChainOperationClassV1::Transfer);
        assert!(submitted.account_sequence.is_none());
        assert!(submitted.hold_id.is_none());

        let canonical = to_canonical_json_string(&submitted).expect("canonical JSON should encode");
        assert!(canonical.contains("\"account_sequence\":null"));
        assert!(canonical.contains("\"op_class\":\"transfer\""));

        let decoded: QuickChainOperationIntentV1 =
            from_canonical_json_slice(canonical.as_bytes()).expect("canonical JSON should decode");

        assert_eq!(decoded, submitted);
        decoded
            .validate()
            .expect("decoded transfer intent should validate");
    }
}

#[test]
fn hold_lifecycle_histories_use_existing_hold_intent_contract() {
    let hold = hold_id('a');

    let operations = [
        intent(
            '8',
            "idem:internal-roc-beta-phase2:hold-open",
            QuickChainOperationClassV1::HoldOpen,
            "account:viewer-a",
            Some("account:creator-a"),
            "50",
            Some(&hold),
            1_800_000_000_100,
        ),
        intent(
            '9',
            "idem:internal-roc-beta-phase2:hold-capture",
            QuickChainOperationClassV1::HoldCapture,
            "account:viewer-a",
            Some("account:creator-a"),
            "50",
            Some(&hold),
            1_800_000_000_101,
        ),
        intent(
            'b',
            "idem:internal-roc-beta-phase2:hold-release",
            QuickChainOperationClassV1::HoldRelease,
            "account:viewer-a",
            None,
            "50",
            Some(&hold),
            1_800_000_000_102,
        ),
        intent(
            'c',
            "idem:internal-roc-beta-phase2:hold-expire",
            QuickChainOperationClassV1::HoldExpire,
            "account:viewer-a",
            None,
            "50",
            Some(&hold),
            1_800_000_000_103,
        ),
    ];

    for submitted in operations {
        submitted
            .validate()
            .expect("hold lifecycle intent should validate");
        assert!(submitted.account_sequence.is_none());
        assert!(submitted.hold_id.is_some());

        let canonical = to_canonical_json_string(&submitted).expect("canonical JSON should encode");
        let decoded: QuickChainOperationIntentV1 =
            from_canonical_json_slice(canonical.as_bytes()).expect("canonical JSON should decode");

        assert_eq!(decoded, submitted);
    }
}

#[test]
fn client_assigned_account_sequence_stays_rejected_before_replay() {
    let mut submitted = intent(
        'd',
        "idem:internal-roc-beta-phase2:bad-client-sequence",
        QuickChainOperationClassV1::Transfer,
        "account:viewer-a",
        Some("account:creator-a"),
        "10",
        None,
        1_800_000_000_200,
    );

    submitted.account_sequence = Some(7);

    let error = submitted
        .validate()
        .expect_err("client-assigned account_sequence must reject");

    assert!(
        error.to_string().contains("account_sequence"),
        "unexpected validation error: {error}"
    );
}
