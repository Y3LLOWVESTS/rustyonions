//! RO:WHAT — Phase 14 service-evidence accounting handoff tests.
//! RO:WHY — Lock privacy, actor, signature, and non-authority boundaries.

use ron_proto::{
    ContentId, ServiceEvidenceAccountingInputV1, ServiceEvidenceAccountingInputValidationError,
    ServiceEvidenceAccountingKindV1, SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA,
    SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
};
use serde_json::json;

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn input(kind: ServiceEvidenceAccountingKindV1) -> ServiceEvidenceAccountingInputV1 {
    ServiceEvidenceAccountingInputV1 {
        schema: SERVICE_EVIDENCE_ACCOUNTING_INPUT_SCHEMA.to_owned(),
        version: SERVICE_EVIDENCE_ACCOUNTING_INPUT_VERSION,
        sequence: 1,
        kind,
        proof_id: "evidence:proof:0001".to_owned(),
        service_node_id: "service_node:alpha".to_owned(),
        witness_node_id: "user_node:bravo".to_owned(),
        related_actor_ids: Vec::new(),
        content_id: CID.parse::<ContentId>().expect("canonical content ID"),
        observed_at_ms: 1_900_000_000_000,
        signature_verified: true,
        evidence_only: true,
        accounting_accepted: false,
        reward_eligible: false,
        reward_truth: false,
        payout_authority: false,
        wallet_mutation: false,
        ledger_mutation: false,
    }
}

#[test]
fn all_service_evidence_kinds_validate_and_roundtrip() {
    for kind in [
        ServiceEvidenceAccountingKindV1::Delivery,
        ServiceEvidenceAccountingKindV1::Availability,
        ServiceEvidenceAccountingKindV1::RangeRequest,
        ServiceEvidenceAccountingKindV1::Repair,
        ServiceEvidenceAccountingKindV1::HotCache,
        ServiceEvidenceAccountingKindV1::PolicyRefusal,
        ServiceEvidenceAccountingKindV1::ModerationAction,
    ] {
        let input = input(kind);
        input.validate().expect("valid accounting input");

        let encoded = serde_json::to_value(&input).expect("accounting-input JSON");

        let decoded: ServiceEvidenceAccountingInputV1 =
            serde_json::from_value(encoded).expect("accounting-input decode");

        assert_eq!(decoded, input);
    }
}

#[test]
fn signature_and_evidence_only_are_required() {
    let mut unsigned = input(ServiceEvidenceAccountingKindV1::Delivery);
    unsigned.signature_verified = false;

    assert_eq!(
        unsigned.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::SignatureNotVerified)
    );

    let mut authoritative = input(ServiceEvidenceAccountingKindV1::Delivery);
    authoritative.evidence_only = false;

    assert_eq!(
        authoritative.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::NotEvidenceOnly)
    );
}

#[test]
fn economic_authority_flags_reject() {
    for field in [
        "accounting_accepted",
        "reward_eligible",
        "reward_truth",
        "payout_authority",
        "wallet_mutation",
        "ledger_mutation",
    ] {
        let mut value = serde_json::to_value(input(ServiceEvidenceAccountingKindV1::Delivery))
            .expect("accounting-input JSON");

        value
            .as_object_mut()
            .expect("accounting-input object")
            .insert(field.to_owned(), json!(true));

        let poisoned: ServiceEvidenceAccountingInputV1 =
            serde_json::from_value(value).expect("known field must decode");

        assert_eq!(
            poisoned.validate(),
            Err(ServiceEvidenceAccountingInputValidationError::AuthorityBoundary { field })
        );
    }
}

#[test]
fn self_traffic_and_actor_collisions_reject() {
    let mut self_traffic = input(ServiceEvidenceAccountingKindV1::Availability);
    self_traffic.witness_node_id = self_traffic.service_node_id.clone();

    assert_eq!(
        self_traffic.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::SelfTraffic)
    );

    let mut collision = input(ServiceEvidenceAccountingKindV1::Repair);
    collision.related_actor_ids = vec![collision.service_node_id.clone()];

    assert_eq!(
        collision.validate(),
        Err(
            ServiceEvidenceAccountingInputValidationError::ActorCollision {
                field: "service_node_id",
            }
        )
    );
}

#[test]
fn related_actors_must_be_sorted_and_unique() {
    let mut unsorted = input(ServiceEvidenceAccountingKindV1::ModerationAction);

    unsorted.related_actor_ids = vec!["operator:zulu".to_owned(), "operator:alpha".to_owned()];

    assert_eq!(
        unsorted.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::RelatedActorsNotCanonical)
    );

    let mut duplicate = input(ServiceEvidenceAccountingKindV1::ModerationAction);

    duplicate.related_actor_ids = vec!["operator:alpha".to_owned(), "operator:alpha".to_owned()];

    assert_eq!(
        duplicate.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::RelatedActorsNotCanonical)
    );
}

#[test]
fn zero_sequence_and_timestamp_reject() {
    let mut zero_sequence = input(ServiceEvidenceAccountingKindV1::Delivery);
    zero_sequence.sequence = 0;

    assert_eq!(
        zero_sequence.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::ZeroValue { field: "sequence" })
    );

    let mut zero_timestamp = input(ServiceEvidenceAccountingKindV1::Delivery);
    zero_timestamp.observed_at_ms = 0;

    assert_eq!(
        zero_timestamp.validate(),
        Err(ServiceEvidenceAccountingInputValidationError::ZeroValue {
            field: "observed_at_ms",
        })
    );
}

#[test]
fn wire_shape_rejects_ip_and_amount_smuggling() {
    for forbidden in [
        ("requester_ip", json!("192.0.2.10")),
        ("provider_ip", json!("198.51.100.10")),
        ("reward_amount_minor", json!("100")),
        ("payout_account", json!("account:attacker")),
    ] {
        let mut value = serde_json::to_value(input(ServiceEvidenceAccountingKindV1::Delivery))
            .expect("accounting-input JSON");

        value
            .as_object_mut()
            .expect("accounting-input object")
            .insert(forbidden.0.to_owned(), forbidden.1);

        assert!(
            serde_json::from_value::<ServiceEvidenceAccountingInputV1>(value).is_err(),
            "unknown field must reject: {}",
            forbidden.0
        );
    }
}
