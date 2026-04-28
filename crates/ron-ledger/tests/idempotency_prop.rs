//! RO:WHAT — Property tests for batch idempotency using the ledger's batch-level `idem_id` contract.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Replaying the same logical batch must not diverge state or roots.
//! RO:INTERACTS — ron_ledger::engine::Ledger, MemoryStorage, api::IngestRequest.
//! RO:INVARIANTS — same idem_id => same response/head root; no duplicate commits.
//! RO:METRICS — none.
//! RO:CONFIG — default LedgerConfig.
//! RO:SECURITY — IDs only; no secrets.
//! RO:TEST — property suite.

use proptest::prelude::*;
use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{Ledger, MemoryStorage},
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};

fn build_entry(amount: u64) -> Entry {
    Entry::new(
        format!("mint-{amount}"),
        amount,
        EntryKind::Mint,
        AccountId::new("acct_prop").unwrap(),
        amount.max(1),
        Nonce::from_base64("AAAAAAAAAAAAAAAAAAAAAA==").unwrap(),
        Kid::new("kid-prop").unwrap(),
        CapabilityRef::new("cap-prop").unwrap(),
        1,
    )
    .unwrap()
}

proptest! {
    #[test]
    fn repeated_idem_id_is_a_noop(amount in 1_u64..10_000) {
        let ledger = Ledger::new(MemoryStorage::default(), LedgerConfig::default()).unwrap();
        let entry = build_entry(amount);
        let req = IngestRequest { batch: vec![entry.clone()], idem_id: Some("same-batch".to_string()) };

        let first = ledger.ingest(req.clone()).unwrap();
        let second = ledger.ingest(req).unwrap();

        prop_assert_eq!(first, second);
        prop_assert_eq!(ledger.balance(&AccountId::new("acct_prop").unwrap()).unwrap(), amount as u128);
    }
}
