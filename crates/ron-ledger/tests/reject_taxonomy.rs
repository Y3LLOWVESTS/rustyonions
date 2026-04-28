//! RO:WHAT — Reject taxonomy stability checks.
//! RO:WHY  — Pillar 12; Concerns: ECON/GOV. Library callers rely on stable reason categories.
//! RO:INTERACTS — ron_ledger::error::RejectReason and engine validation paths.
//! RO:INVARIANTS — invalid/too_large/conflict map consistently.
//! RO:METRICS — none.
//! RO:CONFIG — custom small limits for too_large path.
//! RO:SECURITY — none.
//! RO:TEST — integration test.

use ron_ledger::{
    api::IngestRequest,
    config::{LedgerConfig, Limits},
    engine::{Ledger, MemoryStorage},
    error::RejectReason,
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};

#[test]
fn invalid_nonce_maps_to_invalid() {
    let err = Nonce::from_base64("not-base64").unwrap_err();
    assert_eq!(err.reject_reason(), Some(RejectReason::Invalid));
}

#[test]
fn oversized_batch_maps_to_too_large() {
    let cfg = LedgerConfig {
        limits: Limits {
            batch_max_entries: 1,
            max_body_bytes: 1 << 20,
            queue_capacity: 8,
        },
        ..LedgerConfig::default()
    };
    let ledger = Ledger::new(MemoryStorage::default(), cfg).unwrap();
    let account = AccountId::new("acct_size").unwrap();

    let entry = |id: &str, nonce: &str| {
        Entry::new(
            id,
            1,
            EntryKind::Mint,
            account.clone(),
            1,
            Nonce::from_base64(nonce).unwrap(),
            Kid::new("kid-s").unwrap(),
            CapabilityRef::new("cap-s").unwrap(),
            1,
        )
        .unwrap()
    };

    let err = ledger
        .ingest(IngestRequest {
            batch: vec![
                entry("e1", "AAAAAAAAAAAAAAAAAAAAAA=="),
                entry("e2", "AQEBAQEBAQEBAQEBAQEBAQ=="),
            ],
            idem_id: None,
        })
        .unwrap_err();

    assert_eq!(err.reject_reason(), Some(RejectReason::TooLarge));
}

#[test]
fn duplicate_entry_id_in_batch_maps_to_conflict() {
    let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default()).unwrap();
    let account = AccountId::new("acct_conflict").unwrap();

    let mk = |nonce: &str| {
        Entry::new(
            "dup",
            1,
            EntryKind::Mint,
            account.clone(),
            1,
            Nonce::from_base64(nonce).unwrap(),
            Kid::new("kid-c").unwrap(),
            CapabilityRef::new("cap-c").unwrap(),
            1,
        )
        .unwrap()
    };

    let err = ledger
        .ingest(IngestRequest {
            batch: vec![
                mk("AAAAAAAAAAAAAAAAAAAAAA=="),
                mk("AQEBAQEBAQEBAQEBAQEBAQ=="),
            ],
            idem_id: None,
        })
        .unwrap_err();

    assert_eq!(err.reject_reason(), Some(RejectReason::Conflict));
}
