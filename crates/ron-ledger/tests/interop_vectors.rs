//! RO:WHAT — Canonical interop vector tests using JSON payloads shaped like future service requests.
//! RO:WHY  — Pillar 12; Concerns: ECON/DX/GOV. Keep DTO compatibility and deterministic outcomes visible.
//! RO:INTERACTS — ron_ledger::api DTOs, engine Ledger.
//! RO:INVARIANTS — valid vectors commit; malformed vectors reject; roots advance deterministically.
//! RO:METRICS — none.
//! RO:CONFIG — default config.
//! RO:SECURITY — IDs only.
//! RO:TEST — integration test.

use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{Ledger, MemoryStorage},
    types::{AccountId, Entry},
};

#[test]
fn happy_vector_round_trips_and_commits() {
    let raw = r#"{
        "batch": [{
            "id": "mint-happy",
            "ts": 1,
            "kind": "Mint",
            "account": "acct_happy",
            "amount": 42,
            "nonce": "AAAAAAAAAAAAAAAAAAAAAA==",
            "kid": "kid-happy",
            "capability_ref": "cap-happy",
            "v": 1
        }],
        "idem_id": "happy-1"
    }"#;
    let req: IngestRequest = serde_json::from_str(raw).unwrap();
    let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default()).unwrap();
    let resp = ledger.ingest(req).unwrap();
    assert!(resp.accepted);
    assert_eq!(
        ledger
            .balance(&AccountId::new("acct_happy").unwrap())
            .unwrap(),
        42
    );
}

#[test]
fn malformed_vector_rejects_unknown_field() {
    let raw = r#"{
        "batch": [{
            "id": "bad",
            "ts": 1,
            "kind": "Mint",
            "account": "acct_bad",
            "amount": 1,
            "nonce": "AAAAAAAAAAAAAAAAAAAAAA==",
            "kid": "kid-bad",
            "capability_ref": "cap-bad",
            "v": 1,
            "extra": 7
        }]
    }"#;
    assert!(serde_json::from_str::<IngestRequest>(raw).is_err());
}

#[test]
fn conservation_vector_needs_balanced_credit_and_debit() {
    let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default()).unwrap();
    let credit: Entry = serde_json::from_str(
        r#"{
        "id": "credit-1", "ts": 1, "kind": "Credit", "account": "acct_a", "amount": 5,
        "nonce": "AAAAAAAAAAAAAAAAAAAAAA==", "kid": "kid-a", "capability_ref": "cap-a", "v": 1
    }"#,
    )
    .unwrap();
    let debit: Entry = serde_json::from_str(
        r#"{
        "id": "debit-1", "ts": 1, "kind": "Debit", "account": "acct_b", "amount": 5,
        "nonce": "AQEBAQEBAQEBAQEBAQEBAQ==", "kid": "kid-b", "capability_ref": "cap-b", "v": 1
    }"#,
    )
    .unwrap();
    let err = ledger
        .ingest(IngestRequest {
            batch: vec![credit, debit],
            idem_id: None,
        })
        .unwrap_err();
    assert!(err.to_string().contains("insufficient balance"));
}
