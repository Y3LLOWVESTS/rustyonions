#![recursion_limit = "256"]

//! RO:WHAT — Phase 20A operator-status projection tests for svc-admin.
//! RO:WHY — The optional console must preserve real macronode status truth instead of
//!          inventing ROC, receipt, finality, reward, or authority state.
//! RO:INTERACTS — nodes::status::{RawStatus, from_raw}, macronode `/api/v1/status` JSON.
//! RO:INVARIANTS — nested status blocks pass through unchanged; non-authority flags stay false.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use svc_admin::config::NodeCfg;
use svc_admin::nodes::status::{from_raw, RawStatus};

fn service_node_cfg() -> NodeCfg {
    NodeCfg {
        base_url: "http://127.0.0.1:5310".to_string(),
        display_name: Some("Service Node".to_string()),
        environment: "dev".to_string(),
        insecure_http: true,
        forced_profile: Some("macronode".to_string()),
        macaroon_path: Option::<PathBuf>::None,
        default_timeout: Some(Duration::from_secs(2)),
    }
}

#[test]
fn operator_status_projection_preserves_backend_truth_and_non_authority() {
    let raw: RawStatus = serde_json::from_value(json!({
        "profile": "macronode",
        "node_role": "service_node",
        "node_profile": "macronode",
        "version": "0.1.0-test",
        "uptime_seconds": 77,
        "capabilities": [
            "oap_object_fetch_v1",
            "provider_status_v1",
            "policy_status_v1",
            "service_node_lifecycle_status_v1",
            "economic_pipeline_status_v1",
            "reward_binding_status_v1",
            "service_evidence_outbox_v1"
        ],
        "ready": true,
        "oap": {
            "protocol": "oap/1",
            "version": 1,
            "runtime_state": "active_local_http_oap",
            "max_frame_bytes": 1048576,
            "stream_chunk_bytes": 65536,
            "object_fetch_active": true,
            "full_digest_verification_active": true
        },
        "provider": {
            "state": "local_provider_store_active_not_advertising",
            "dht_worker_status": "active_embedded_router",
            "advertisement_active": false,
            "provider_records_published": 0,
            "public_node_uri_format": "crab://node/<node-id>",
            "residential_ip_publication": false
        },
        "policy": {
            "state": "all_object_read_policy_active",
            "serve_policy_enforced": true,
            "oap_serve_policy_enforced": true,
            "operator_moderation_active": true,
            "global_moderation_active": true,
            "moderation_configured": true,
            "moderation_state": "active",
            "moderation_source": "signed_global_plus_local",
            "moderation_load_failed": false,
            "signed_policy_verified": true,
            "signed_policy_epoch": 12,
            "signed_policy_expires_at_unix_s": 1900000000,
            "rollback_guard_persisted": true,
            "moderation_activation": "startup_snapshot",
            "moderation_hot_reload": false,
            "moderation_entries": {
                "total": 9,
                "global_deny": 2,
                "local_block": 1,
                "local_allow": 3,
                "owner_tombstone": 1,
                "quarantine": 2
            },
            "unvetted_persistence_posture": "amnesia_first",
            "serve_gate_phase": "phase_10_all_object_reads_active",
            "moderation_phase": "phase_10"
        },
        "economic_pipeline": {
            "stage": "epoch_transition_reviewed",
            "accounting_snapshot": {
                "chain_id": "rustyonions-dev",
                "snapshot_id": "snapshot:epoch:20",
                "snapshot_root": "b3:1111111111111111111111111111111111111111111111111111111111111111",
                "window_started_at_ms": 1000,
                "window_ended_at_ms": 2000,
                "sealed_at_ms": 3000,
                "source_event_count": 1,
                "economic_receipt_count": 0,
                "metering_count": 0,
                "proof_eligible_count": 1,
                "ad_budgeted_count": 0,
                "analytics_only_count": 0
            },
            "reward_plan": {
                "plan_id": "reward-plan:epoch:20",
                "plan_root": "b3:2222222222222222222222222222222222222222222222222222222222222222",
                "snapshot_id": "snapshot:epoch:20",
                "snapshot_root": "b3:1111111111111111111111111111111111111111111111111111111111111111",
                "source_event_class": "proof_eligible",
                "planned_total_minor": "1000",
                "payout_candidate_count": 1,
                "capped_by_policy": true,
                "verification_ref": "verification:epoch:20",
                "funding_budget_ref": null,
                "produced_at_ms": 4000
            },
            "epoch_transition": {
                "chain_id": "rustyonions-dev",
                "epoch_id": "epoch:20",
                "transition_hash": "b3:8888888888888888888888888888888888888888888888888888888888888888",
                "accounting_snapshot_hash": "b3:1111111111111111111111111111111111111111111111111111111111111111",
                "reward_plan_hash": "b3:2222222222222222222222222222222222222222222222222222222222222222",
                "policy_hash": "b3:3333333333333333333333333333333333333333333333333333333333333333",
                "economics_config_hash": "b3:4444444444444444444444444444444444444444444444444444444444444444",
                "registry_root": "b3:5555555555555555555555555555555555555555555555555555555555555555",
                "reward_binding_root": "b3:6666666666666666666666666666666666666666666666666666666666666666",
                "evidence_root": "b3:7777777777777777777777777777777777777777777777777777777777777777",
                "reward_cap_minor_units": "1000",
                "reward_total_minor_units": "1000",
                "allocation_count": 1,
                "eligible_service_node_count": 2,
                "required_signature_references": 2,
                "supplied_signature_references": 2,
                "quorum_reference_threshold_met": true,
                "cryptographic_signatures_verified": false,
                "recipient_accounts_resolved": false,
                "produced_at_ms": 5000
            },
            "epoch_payout_receipts": {
                "receipt_count": 1,
                "recipient_count": 1,
                "total_issued_minor": "1000",
                "first_ledger_seq": 41,
                "last_ledger_seq": 41,
                "ledger_root": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "first_receipt_hash": "b3:9999999999999999999999999999999999999999999999999999999999999999",
                "last_receipt_hash": "b3:9999999999999999999999999999999999999999999999999999999999999999",
                "accepted_at_ms": 5000,
                "wallet_source": "svc-wallet",
                "ledger_source": "ron-ledger",
                "settlement_status": "accepted",
                "finality_status": "not_reported"
            },
            "wallet_execution_reported": true,
            "ledger_receipt_reported": true,
            "confirmed_roc_reported": true,
            "finality_reported": false,
            "operator_projection_authorizes_economic_mutation": false
        },
        "service_node_lifecycle": {
            "lifecycle_state": "quarantined",
            "registered_at_epoch": 10,
            "state_effective_epoch": 20,
            "quorum_status": "ineligible",
            "counts_toward_quorum": false,
            "probation_reward_cap_required": false,
            "enforcement": {
                "status_id": "enforcement:operator_01",
                "state": "quarantined",
                "reason": "hash_mismatch",
                "evidence_root": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "effective_epoch": 20,
                "counts_toward_quorum": false,
                "permits_reward_planning": false,
                "authorizes_economic_mutation": false,
                "appeal": {
                    "state": "pending",
                    "appeal_id": "appeal:operator_01",
                    "submitted_epoch": 21,
                    "resolved_epoch": null,
                    "resolution_evidence_root": null,
                    "pending": true,
                    "authorizes_state_change": false
                }
            },
            "operator_projection_authorizes_state_change": false,
            "operator_projection_authorizes_economic_mutation": false
        },
        "persistence_review": {
            "state": "process_local_metadata_only",
            "candidates_total": 12,
            "awaiting_decision": 5,
            "pending_review": 3,
            "persistence_approvals": 4,
            "blocked_candidates": 2,
            "quarantined_candidates": 1,
            "completed_local_prunes": 7,
            "durable_bytes_written": false,
            "reward_finality": false,
            "wallet_mutation": false,
            "ledger_mutation": false
        },
        "reward_binding": {
            "state": "requested_runtime_local",
            "reward_recipient_display_address": "@operator",
            "pending_rotation_display_address": null,
            "updated_at_unix_s": 1800000000,
            "registry_finality": false,
            "wallet_mutation": false,
            "ledger_mutation": false,
            "confirmed_roc_minor_units": null
        },
        "service_evidence": {
            "state": "bounded_process_local_outbox",
            "queued_records": 4,
            "delivery_records": 2,
            "reward_evidence_records": 0,
            "signature_required": true,
            "replay_scope": "bounded_process_local",
            "durable": false,
            "accounting_accepted": false,
            "reward_eligible": false,
            "reward_truth": false,
            "payout_authority": false,
            "wallet_mutation": false,
            "ledger_mutation": false
        },
        "amnesia_mode": false,
        "privacy_mode": false,
        "public_inbound_enabled": false,
        "headless_mode": true,
        "admin_ui_enabled": true,
        "admin_ui_bind": "127.0.0.1:5300",
        "operator_ui_profile": "service_node_local",
        "admin_ui_runtime_required": false,
        "verification_enabled": false,
        "content_serving_enabled": true,
        "economic_replay_enabled": false,
        "service_quorum_enabled": false,
        "wallet_execution_participant": false,
        "ledger_replay_enabled": false,
        "user_ip_publication": "not_applicable_service_node",
        "peer_ip_display": "forbidden",
        "admin_bind_publication": false,
        "service_socket_publication": "service_node_only",
        "transport_routes_public": false,
        "raw_socket_publication": false,
        "planes": [{
            "name": "gateway",
            "health": "healthy",
            "ready": true,
            "restart_count": 0
        }]
    }))
    .expect("current macronode status JSON must deserialize");

    let view = from_raw("service-node", &service_node_cfg(), raw);

    assert_eq!(view.ready, Some(true));

    let oap = view.oap.expect("OAP status must be preserved");
    assert_eq!(oap.protocol, "oap/1");
    assert!(oap.object_fetch_active);
    assert!(oap.full_digest_verification_active);

    let provider = view.provider.expect("provider status must be preserved");
    assert!(!provider.advertisement_active);
    assert_eq!(provider.provider_records_published, 0);
    assert!(!provider.residential_ip_publication);

    let policy = view.policy.expect("policy status must be preserved");
    assert_eq!(policy.moderation_state, "active");
    assert!(policy.signed_policy_verified);
    assert_eq!(policy.signed_policy_epoch, Some(12));
    assert_eq!(policy.moderation_entries.quarantine, 2);

    let persistence = view
        .persistence_review
        .expect("persistence review status must be preserved");
    assert_eq!(persistence.candidates_total, 12);
    assert_eq!(persistence.awaiting_decision, 5);
    assert_eq!(persistence.pending_review, 3);
    assert_eq!(persistence.persistence_approvals, 4);
    assert_eq!(persistence.blocked_candidates, 2);
    assert_eq!(persistence.quarantined_candidates, 1);
    assert_eq!(persistence.completed_local_prunes, 7);
    assert!(!persistence.durable_bytes_written);
    assert!(!persistence.reward_finality);
    assert!(!persistence.wallet_mutation);
    assert!(!persistence.ledger_mutation);

    let economic = view
        .economic_pipeline
        .expect("economic pipeline status must be preserved");

    assert_eq!(economic.stage, "epoch_transition_reviewed");

    assert_eq!(economic.accounting_snapshot.chain_id, "rustyonions-dev");
    assert_eq!(
        economic.accounting_snapshot.snapshot_id,
        "snapshot:epoch:20"
    );
    assert_eq!(economic.accounting_snapshot.proof_eligible_count, 1);
    assert_eq!(economic.accounting_snapshot.economic_receipt_count, 0);

    let plan = economic
        .reward_plan
        .as_ref()
        .expect("reward-plan reference must be preserved");

    assert_eq!(plan.plan_id, "reward-plan:epoch:20");
    assert_eq!(plan.planned_total_minor, "1000");
    assert_eq!(plan.payout_candidate_count, 1);
    assert!(plan.capped_by_policy);

    let transition = economic
        .epoch_transition
        .as_ref()
        .expect("epoch transition must be preserved");

    assert_eq!(transition.epoch_id, "epoch:20");
    assert_eq!(transition.reward_total_minor_units, "1000");
    assert_eq!(transition.allocation_count, 1);
    assert_eq!(transition.required_signature_references, 2);
    assert_eq!(transition.supplied_signature_references, 2);
    assert!(transition.quorum_reference_threshold_met);

    // Signature references meeting a threshold are not equivalent to
    // cryptographic signature verification.
    assert!(!transition.cryptographic_signatures_verified);

    // Service Node allocation IDs are not wallet-recipient resolution.
    assert!(!transition.recipient_accounts_resolved);

    // The current slice ends before wallet execution and ledger receipt.
    let receipts = economic
        .epoch_payout_receipts
        .as_ref()
        .expect("accepted payout receipt summary must be preserved");

    assert_eq!(receipts.receipt_count, 1);
    assert_eq!(receipts.recipient_count, 1);
    assert_eq!(receipts.total_issued_minor, "1000");
    assert_eq!(receipts.first_ledger_seq, 41);
    assert_eq!(receipts.last_ledger_seq, 41);
    assert_eq!(
        receipts.ledger_root,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(receipts.wallet_source, "svc-wallet");
    assert_eq!(receipts.ledger_source, "ron-ledger");
    assert_eq!(receipts.settlement_status, "accepted");
    assert_eq!(receipts.finality_status, "not_reported");

    assert!(economic.wallet_execution_reported);
    assert!(economic.ledger_receipt_reported);
    assert!(economic.confirmed_roc_reported);
    assert!(!economic.finality_reported);
    assert!(!economic.operator_projection_authorizes_economic_mutation);

    let lifecycle = view
        .service_node_lifecycle
        .expect("lifecycle status must be preserved");

    assert_eq!(lifecycle.lifecycle_state, "quarantined");
    assert_eq!(lifecycle.quorum_status, "ineligible");
    assert_eq!(lifecycle.registered_at_epoch, 10);
    assert_eq!(lifecycle.state_effective_epoch, 20);
    assert!(!lifecycle.counts_toward_quorum);
    assert!(!lifecycle.probation_reward_cap_required);
    assert!(!lifecycle.operator_projection_authorizes_state_change);
    assert!(!lifecycle.operator_projection_authorizes_economic_mutation);

    let containment = lifecycle
        .enforcement
        .expect("containment status must be preserved");

    assert_eq!(containment.state, "quarantined");
    assert_eq!(containment.reason, "hash_mismatch");
    assert_eq!(containment.effective_epoch, 20);
    assert!(!containment.counts_toward_quorum);
    assert!(!containment.permits_reward_planning);
    assert!(!containment.authorizes_economic_mutation);

    assert_eq!(containment.appeal.state, "pending");
    assert!(containment.appeal.pending);
    assert_eq!(containment.appeal.submitted_epoch, Some(21));
    assert!(!containment.appeal.authorizes_state_change);

    let binding = view
        .reward_binding
        .expect("reward binding status must be preserved");
    assert_eq!(
        binding.reward_recipient_display_address.as_deref(),
        Some("@operator")
    );
    assert!(!binding.registry_finality);
    assert!(!binding.wallet_mutation);
    assert!(!binding.ledger_mutation);
    assert!(binding.confirmed_roc_minor_units.is_none());

    let evidence = view
        .service_evidence
        .expect("service evidence status must be preserved");
    assert_eq!(evidence.queued_records, 4);
    assert_eq!(evidence.delivery_records, 2);
    assert_eq!(evidence.reward_evidence_records, 0);
    assert!(!evidence.durable);
    assert!(!evidence.accounting_accepted);
    assert!(!evidence.reward_eligible);
    assert!(!evidence.reward_truth);
    assert!(!evidence.payout_authority);
    assert!(!evidence.wallet_mutation);
    assert!(!evidence.ledger_mutation);
}
