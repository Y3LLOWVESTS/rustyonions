//! RO:WHAT — Replay recovery test for durable file storage.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Restarting from WAL/checkpoints must yield the same head root and balances.
//! RO:INTERACTS — ron_ledger::engine::{Ledger, FileStorage}, api::IngestRequest.
//! RO:INVARIANTS — replay is deterministic; no seq gaps; durable backend restores balances.
//! RO:METRICS — none.
//! RO:CONFIG — default config with amnesia storage replaced by FileStorage.
//! RO:SECURITY — tempdir only; no secrets.
//! RO:TEST — integration test.

use ron_ledger::{
    api::IngestRequest,
    config::LedgerConfig,
    engine::{FileStorage, Ledger},
    types::{AccountId, CapabilityRef, Entry, EntryKind, Kid, Nonce},
};
use tempfile::tempdir;

#[test]
fn file_backed_replay_restores_root_and_balance() {
    let dir = tempdir().unwrap();
    let storage = FileStorage::open(dir.path()).unwrap();
    let account = AccountId::new("acct_replay").unwrap();

    let ledger = Ledger::new(storage.clone(), LedgerConfig::default()).unwrap();
    let req = IngestRequest {
        batch: vec![
            Entry::new(
                "mint-1",
                1,
                EntryKind::Mint,
                account.clone(),
                50,
                Nonce::from_base64("AAAAAAAAAAAAAAAAAAAAAA==").unwrap(),
                Kid::new("kid-r").unwrap(),
                CapabilityRef::new("cap-r").unwrap(),
                1,
            )
            .unwrap(),
            Entry::new(
                "mint-2",
                2,
                EntryKind::Mint,
                account.clone(),
                25,
                Nonce::from_base64("AQEBAQEBAQEBAQEBAQEBAQ==").unwrap(),
                Kid::new("kid-r").unwrap(),
                CapabilityRef::new("cap-r").unwrap(),
                1,
            )
            .unwrap(),
        ],
        idem_id: Some("replay-batch".into()),
    };
    let before = ledger.ingest(req).unwrap();
    let before_balance = ledger.balance(&account).unwrap();
    drop(ledger);

    let reopened = Ledger::new(storage, LedgerConfig::default()).unwrap();
    let roots = reopened.roots_since(0).unwrap();
    let last = roots.roots.last().unwrap();

    assert_eq!(before_balance, reopened.balance(&account).unwrap());
    assert_eq!(before.new_root, last.root);
}
