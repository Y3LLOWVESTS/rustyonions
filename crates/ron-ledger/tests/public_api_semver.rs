//! RO:WHAT — Smoke test for the intended public API surface.
//! RO:WHY  — Pillar 12; Concerns: GOV/DX. Catch accidental drift in the crate's public imports.
//! RO:INTERACTS — ron_ledger public reexports and gated ron_ledger::quickchain preflight exports.
//! RO:INVARIANTS — expected types remain reachable; QuickChain smoke stays pre-root, pre-consensus, and non-authoritative.
//! RO:METRICS — none.
//! RO:CONFIG — QuickChain assertions compile only with the quickchain-preflight feature.
//! RO:SECURITY — smoke inputs are inert public fixtures; no wallet authority, receipt fabrication, roots, finality, or settlement.
//! RO:TEST — integration test.

use ron_ledger::{
    AccumulatorKind, EngineMode, LedgerConfig, LedgerError, PqMode, RejectReason, Root, Seq,
};

#[test]
fn public_surface_smoke() {
    let cfg = LedgerConfig::default();
    assert!(matches!(
        cfg.engine_mode,
        EngineMode::Amnesia | EngineMode::Persistent
    ));
    assert!(matches!(
        cfg.accumulator_kind,
        AccumulatorKind::Merkle | AccumulatorKind::Verkle
    ));
    assert!(matches!(cfg.pq_mode, PqMode::Off | PqMode::Hybrid));

    let reason = RejectReason::Invalid;
    assert_eq!(reason.as_str(), "invalid");

    let root = Root::zero();
    assert_eq!(root.to_hex().len(), 64);

    let seq = Seq(1);
    assert_eq!(seq.get(), 1);

    let err = LedgerError::reject(RejectReason::Conflict, "boom");
    assert_eq!(err.reject_reason(), Some(RejectReason::Conflict));
}

#[cfg(feature = "quickchain-preflight")]
mod quickchain_public_api {
    use ron_ledger::quickchain::{
        project_operation_hash_payload, project_receipt_hash_payload, QuickChainAcceptedOperation,
        QuickChainAcceptedReplayBoundary, QuickChainAccountLeafProjectionContext,
        QuickChainAccountSnapshot, QuickChainActiveHoldLeafProjectionContext,
        QuickChainActiveHoldSnapshot, QuickChainAtomicState, QuickChainBalanceExecutionOutcome,
        QuickChainBalanceState, QuickChainBalanceTransition, QuickChainCommittedOperationRecord,
        QuickChainEpochBinding, QuickChainExecutionDisposition, QuickChainExecutionError,
        QuickChainHashPayloadProjectionError, QuickChainHoldEpochInput, QuickChainHoldError,
        QuickChainHoldExecutionOutcome, QuickChainHoldState, QuickChainHoldTerminalStatus,
        QuickChainHoldTransition, QuickChainHoldTransitionKind, QuickChainLeafProjectionError,
        QuickChainOpenHoldRecord, QuickChainOperationHashProjectionContext,
        QuickChainReceiptHashProjectionContext, QuickChainReplayError, QuickChainReplayIndex,
        QuickChainStateSnapshot, QuickChainStateSnapshotError, QuickChainSubmissionDecision,
        QuickChainSupplyDecision, QuickChainTerminalHoldRecord, QuickChainTransitionError,
    };
    use ron_proto::{
        quickchain::{
            QuickChainOperationClassV1, QuickChainOperationHashPayloadV1,
            QuickChainOperationIntentV1, QuickChainReceiptHashPayloadV1, QUICKCHAIN_DTO_VERSION,
            QUICKCHAIN_OPERATION_INTENT_SCHEMA,
        },
        ContentId,
    };

    const CHAIN_ID: &str = "ron-devnet";

    fn touch_public_type<T>() {
        let _ = core::mem::size_of::<T>();
    }

    fn operation_id(hex_digit: char) -> String {
        format!("op_{}", hex_digit.to_string().repeat(32))
    }

    fn hold_id(hex_digit: char) -> String {
        format!("hold_{}", hex_digit.to_string().repeat(32))
    }

    /// Produce a real BLAKE3 content identifier for an inert smoke-test label.
    ///
    /// These identifiers prove public type plumbing only. They are not production
    /// roots, checkpoint hashes, policy hashes, receipt hashes, or golden vectors.
    fn test_content_id(label: &str) -> ContentId {
        let digest = blake3::hash(label.as_bytes()).to_hex().to_string();

        format!("b3:{digest}")
            .parse()
            .expect("BLAKE3 smoke-test content ID should parse")
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
    fn quickchain_preflight_public_surface_smoke() {
        touch_public_type::<QuickChainAcceptedReplayBoundary>();
        touch_public_type::<QuickChainAcceptedOperation>();
        touch_public_type::<QuickChainAccountSnapshot>();
        touch_public_type::<QuickChainActiveHoldSnapshot>();
        touch_public_type::<QuickChainBalanceState>();
        touch_public_type::<QuickChainBalanceTransition>();
        touch_public_type::<QuickChainHashPayloadProjectionError>();
        touch_public_type::<QuickChainHoldError>();
        touch_public_type::<QuickChainHoldState>();
        touch_public_type::<QuickChainHoldTerminalStatus>();
        touch_public_type::<QuickChainHoldTransition>();
        touch_public_type::<QuickChainHoldTransitionKind>();
        touch_public_type::<QuickChainLeafProjectionError>();
        touch_public_type::<QuickChainOpenHoldRecord>();
        touch_public_type::<QuickChainStateSnapshotError>();
        touch_public_type::<QuickChainTerminalHoldRecord>();
        touch_public_type::<QuickChainTransitionError>();

        assert_eq!(
            QuickChainAcceptedReplayBoundary::empty().operation_count(),
            0
        );
        assert_eq!(
            QuickChainAcceptedReplayBoundary::empty().next_ledger_sequence(),
            1
        );

        let issue = intent(
            '1',
            "idem:public-api:issue",
            QuickChainOperationClassV1::Issue,
            "account:alice",
            None,
            "250",
            None,
            1_000,
        );

        let mut replay = QuickChainReplayIndex::new();
        assert!(matches!(
            replay
                .classify_submission(&issue)
                .expect("fresh intent should classify"),
            QuickChainSubmissionDecision::Fresh
        ));

        let mut state = QuickChainAtomicState::new();

        let issue_outcome: QuickChainBalanceExecutionOutcome = state
            .execute_balance_operation(
                &issue,
                QuickChainSupplyDecision::IssueApproved,
                "tx:roc:public-api:issue",
            )
            .expect("issue should commit");

        assert_eq!(
            issue_outcome.disposition(),
            QuickChainExecutionDisposition::Committed
        );
        assert!(issue_outcome.is_committed());
        assert!(issue_outcome.transition().is_some());

        let issue_record: QuickChainCommittedOperationRecord = issue_outcome.record().clone();
        assert_eq!(issue_record.receipt_txid(), "tx:roc:public-api:issue");
        assert_eq!(issue_record.account_sequence(), 1);
        assert_eq!(issue_record.ledger_sequence_start(), 1);
        assert_eq!(issue_record.ledger_sequence_end(), 1);

        replay
            .record_committed(issue_record.clone())
            .expect("public replay index should accept committed issue record");

        let retry_decision = replay
            .classify_submission(&issue)
            .expect("accepted retry should classify");

        if let QuickChainSubmissionDecision::ReturnOriginal(record) = retry_decision {
            assert_eq!(record.receipt_txid(), issue_record.receipt_txid());
        } else {
            panic!("accepted retry should return original committed evidence");
        }

        let hold = hold_id('a');
        let open_hold = intent(
            '2',
            "idem:public-api:hold-open",
            QuickChainOperationClassV1::HoldOpen,
            "account:alice",
            Some("account:merchant"),
            "50",
            Some(&hold),
            2_000,
        );

        let open_epoch = QuickChainHoldEpochInput::Open {
            created_at_epoch: 10,
            expires_at_epoch: 20,
        };

        let hold_outcome: QuickChainHoldExecutionOutcome = state
            .execute_hold_operation(&open_hold, open_epoch, "tx:roc:public-api:hold-open")
            .expect("hold open should commit");

        assert_eq!(
            hold_outcome.disposition(),
            QuickChainExecutionDisposition::Committed
        );
        assert!(hold_outcome.is_committed());
        assert!(hold_outcome.transition().is_some());

        let hold_record: QuickChainCommittedOperationRecord = hold_outcome.record().clone();
        assert_eq!(hold_record.receipt_txid(), "tx:roc:public-api:hold-open");
        assert_eq!(hold_record.account_sequence(), 2);
        assert_eq!(hold_record.ledger_sequence_start(), 2);
        assert_eq!(hold_record.ledger_sequence_end(), 2);

        let accepted_issue = QuickChainAcceptedOperation::balance(
            issue_record.clone(),
            QuickChainSupplyDecision::IssueApproved,
        );
        let accepted_hold = QuickChainAcceptedOperation::hold(hold_record.clone(), open_epoch);

        assert_eq!(
            accepted_issue.trusted_receipt_txid(),
            issue_record.receipt_txid()
        );
        assert_eq!(
            accepted_hold.record().receipt_txid(),
            hold_record.receipt_txid()
        );

        let history = vec![accepted_issue.clone(), accepted_hold.clone()];
        let boundary = state.accepted_replay_boundary();

        assert_eq!(
            boundary,
            QuickChainAcceptedReplayBoundary::with_chain_id(2, 3, CHAIN_ID)
        );

        let rebuilt = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
            &history,
            boundary.clone(),
        )
        .expect("accepted history should rebuild to the supplied boundary");

        assert_eq!(rebuilt.balance_minor("account:alice"), 250);
        assert_eq!(rebuilt.held_minor("account:alice"), 50);
        assert_eq!(
            rebuilt
                .available_minor("account:alice")
                .expect("available balance should compute"),
            200
        );
        assert_eq!(rebuilt.accepted_replay_boundary(), boundary);

        let wrong_boundary = QuickChainAcceptedReplayBoundary::with_chain_id(99, 3, CHAIN_ID);
        let boundary_error = QuickChainAtomicState::rebuild_from_accepted_operations_with_boundary(
            &history,
            wrong_boundary,
        )
        .expect_err("wrong accepted boundary should reject");

        assert!(matches!(
            boundary_error,
            QuickChainExecutionError::Replay(
                QuickChainReplayError::AcceptedHistoryOperationCountMismatch {
                    expected: 99,
                    actual: 2
                }
            )
        ));

        let snapshot: QuickChainStateSnapshot = rebuilt
            .state_snapshot()
            .expect("rebuilt state should project a deterministic snapshot");

        assert_eq!(snapshot.chain_id(), Some(CHAIN_ID));
        assert_eq!(snapshot.operation_count(), 2);
        assert_eq!(snapshot.next_ledger_sequence(), 3);
        assert_eq!(snapshot.current_supply_minor(), 250);
        assert_eq!(snapshot.accounts().len(), 1);
        assert_eq!(snapshot.active_holds().len(), 1);

        let account: &QuickChainAccountSnapshot = snapshot
            .accounts()
            .first()
            .expect("snapshot should contain alice account row");

        assert_eq!(account.account_id(), "account:alice");
        assert_eq!(account.balance_minor(), 250);
        assert_eq!(account.held_minor(), 50);
        assert_eq!(account.available_minor(), 200);
        assert_eq!(account.account_sequence(), 2);

        let active_hold: &QuickChainActiveHoldSnapshot = snapshot
            .active_holds()
            .first()
            .expect("snapshot should contain active hold row");

        assert_eq!(active_hold.hold_id(), hold);
        assert_eq!(active_hold.account_id(), "account:alice");
        assert_eq!(
            active_hold.counterparty_account_id(),
            Some("account:merchant")
        );
        assert_eq!(active_hold.amount_minor(), 50);
        assert_eq!(active_hold.created_at_epoch_number(), 10);
        assert_eq!(active_hold.expires_at_epoch_number(), 20);
        assert_eq!(active_hold.opened_operation_id(), open_hold.operation_id);
        assert_eq!(
            active_hold.opened_idempotency_key(),
            "idem:public-api:hold-open"
        );

        let account_context = QuickChainAccountLeafProjectionContext::new(
            account.account_id(),
            test_content_id("public-api/account/receipt-root"),
            test_content_id("public-api/account/holds-root"),
            Some(test_content_id("public-api/account/permissions-root")),
            "epoch:20",
        );

        assert_eq!(account_context.account_id(), account.account_id());
        assert_eq!(account_context.updated_at_epoch(), "epoch:20");
        assert!(account_context.permissions_root().is_some());

        let active_hold_context = QuickChainActiveHoldLeafProjectionContext::new(
            active_hold.hold_id(),
            "paid-view",
            QuickChainEpochBinding::new(active_hold.created_at_epoch_number(), "epoch:10"),
            QuickChainEpochBinding::new(active_hold.expires_at_epoch_number(), "epoch:20"),
            test_content_id("public-api/hold/policy"),
        );

        assert_eq!(active_hold_context.hold_id(), active_hold.hold_id());
        assert_eq!(active_hold_context.purpose(), "paid-view");
        assert_eq!(
            active_hold_context.created_at_epoch().epoch_number(),
            active_hold.created_at_epoch_number()
        );
        assert_eq!(
            active_hold_context.expires_at_epoch().epoch_id(),
            "epoch:20"
        );

        let operation_context = QuickChainOperationHashProjectionContext::new(
            issue.operation_id.clone(),
            "public-api-smoke",
            None,
            test_content_id("public-api/operation/policy"),
            test_content_id("public-api/operation/chain-params"),
        );

        assert_eq!(operation_context.operation_id(), issue.operation_id);
        assert_eq!(operation_context.purpose(), "public-api-smoke");
        assert_eq!(operation_context.session_budget_id(), None);

        let operation_payload: QuickChainOperationHashPayloadV1 =
            project_operation_hash_payload(&issue, &operation_context)
                .expect("operation hash payload projection should validate");

        assert_eq!(operation_payload.operation_id, issue.operation_id);
        assert_eq!(operation_payload.asset, "roc");
        assert_eq!(operation_payload.amount_minor, "250");

        let operation_hash = test_content_id("public-api/operation/hash");
        let receipt_context = QuickChainReceiptHashProjectionContext::new(
            issue_record.intent().operation_id.clone(),
            operation_hash.clone(),
            "issue",
            None,
            test_content_id("public-api/receipt/previous-ledger-root"),
            test_content_id("public-api/receipt/new-ledger-root"),
            3_000,
        );

        assert_eq!(receipt_context.operation_hash(), &operation_hash);
        assert_eq!(receipt_context.op(), "issue");
        assert_eq!(receipt_context.session_budget_id(), None);
        assert_eq!(receipt_context.produced_at_ms(), 3_000);

        let receipt_payload: QuickChainReceiptHashPayloadV1 =
            project_receipt_hash_payload(&issue_record, &receipt_context)
                .expect("receipt hash payload projection should validate");

        assert_eq!(receipt_payload.txid, issue_record.receipt_txid());
        assert_eq!(
            receipt_payload.operation_id,
            issue_record.intent().operation_id
        );
        assert_eq!(receipt_payload.asset, "roc");
        assert_eq!(receipt_payload.amount_minor, "250");
    }
}
