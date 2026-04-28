//! RO:WHAT — Concurrency-model tests for svc-wallet nonce and idempotency gates.
//! RO:WHY  — Pillar 12; Concerns: ECON/RES. Concurrent retries must not double-spend or fork decisions.
//! RO:INTERACTS — seq::nonce::NonceTable, idem::store::IdempotencyStore, receipt hashing.
//! RO:INVARIANTS — exactly one strict nonce reservation wins; same idem fingerprint replays; different fingerprint conflicts.
//! RO:METRICS — none directly; route layer records conflicts/replays.
//! RO:CONFIG — uses short test TTLs.
//! RO:SECURITY — account ids and request fingerprints only.
//! RO:TEST — cargo test -p svc-wallet --test loom_invariants.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    },
    thread,
    time::Duration,
};

use svc_wallet::{
    dto::{
        requests::AmountMinor,
        responses::{Receipt, WalletOp},
    },
    idem::store::IdempotencyStore,
    seq::nonce::NonceTable,
    util::blake3_receipt::finalize_receipt,
};

fn dummy_receipt(txid: &str, idem: &str) -> Receipt {
    finalize_receipt(Receipt {
        txid: txid.to_string(),
        op: WalletOp::Transfer,
        from: Some("acct_a".to_string()),
        to: Some("acct_b".to_string()),
        asset: "roc".to_string(),
        amount_minor: AmountMinor(1),
        nonce: Some(1),
        idem: idem.to_string(),
        ts: 1_777_309_851_000,
        ledger_seq_start: Some(1),
        ledger_seq_end: Some(2),
        ledger_root: "00".repeat(32),
        receipt_hash: String::new(),
    })
    .expect("dummy receipt should hash")
}

#[test]
fn concurrent_same_nonce_has_exactly_one_winner() {
    let table = Arc::new(NonceTable::default());
    let start = Arc::new(Barrier::new(12));
    let wins = Arc::new(AtomicUsize::new(0));
    let losses = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..12 {
        let table = Arc::clone(&table);
        let start = Arc::clone(&start);
        let wins = Arc::clone(&wins);
        let losses = Arc::clone(&losses);

        handles.push(thread::spawn(move || {
            start.wait();

            match table.reserve_strict("acct_a", 1) {
                Ok(reservation) => {
                    reservation.commit();
                    wins.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    losses.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(wins.load(Ordering::Relaxed), 1);
    assert_eq!(losses.load(Ordering::Relaxed), 11);
    assert_eq!(table.last_nonce("acct_a"), Some(1));
}

#[test]
fn rollback_reopens_first_nonce_after_failed_commit_path() {
    let table = NonceTable::default();

    let reservation = table
        .reserve_strict("acct_a", 1)
        .expect("first reservation should succeed");
    assert_eq!(table.last_nonce("acct_a"), Some(1));

    table.rollback(reservation);
    assert_eq!(table.last_nonce("acct_a"), None);

    let second = table
        .reserve_strict("acct_a", 1)
        .expect("rollback should reopen nonce 1");
    second.commit();

    assert_eq!(table.last_nonce("acct_a"), Some(1));
}

#[test]
fn concurrent_same_idempotency_fingerprint_replays_same_receipt() {
    let store = Arc::new(IdempotencyStore::new(Duration::from_secs(60)));
    let receipt = dummy_receipt("tx_concurrent_idem", "idem_concurrent");
    store.insert(
        "idem_concurrent".to_string(),
        "fingerprint_a".to_string(),
        receipt.clone(),
        1_000,
    );

    let start = Arc::new(Barrier::new(10));
    let replays = Arc::new(AtomicUsize::new(0));
    let conflicts = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..10 {
        let store = Arc::clone(&store);
        let start = Arc::clone(&start);
        let replays = Arc::clone(&replays);
        let conflicts = Arc::clone(&conflicts);
        let expected = receipt.clone();

        handles.push(thread::spawn(move || {
            start.wait();

            match store.lookup("idem_concurrent", "fingerprint_a", 1_001) {
                Ok(Some(actual)) if actual == expected => {
                    replays.fetch_add(1, Ordering::Relaxed);
                }
                Ok(_) => {
                    panic!("same fingerprint should replay the stored receipt");
                }
                Err(_) => {
                    conflicts.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(replays.load(Ordering::Relaxed), 10);
    assert_eq!(conflicts.load(Ordering::Relaxed), 0);
}

#[test]
fn concurrent_different_idempotency_fingerprint_conflicts() {
    let store = Arc::new(IdempotencyStore::new(Duration::from_secs(60)));
    let receipt = dummy_receipt("tx_concurrent_conflict", "idem_conflict");
    store.insert(
        "idem_conflict".to_string(),
        "fingerprint_a".to_string(),
        receipt,
        1_000,
    );

    let start = Arc::new(Barrier::new(8));
    let conflicts = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for idx in 0..8 {
        let store = Arc::clone(&store);
        let start = Arc::clone(&start);
        let conflicts = Arc::clone(&conflicts);
        let fingerprint = format!("fingerprint_b_{idx}");

        handles.push(thread::spawn(move || {
            start.wait();

            if store.lookup("idem_conflict", &fingerprint, 1_001).is_err() {
                conflicts.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("worker thread should not panic");
    }

    assert_eq!(conflicts.load(Ordering::Relaxed), 8);
}
