//! RO:WHAT — Amnesia-mode tests for RAM-only wallet-local state.
//! RO:WHY  — Pillar 12; Concerns: SEC/RES/ECON. Wallet-local caches are derivative and must be discardable.
//! RO:INTERACTS — WalletConfig, IdempotencyStore, LocalLedgerClient.
//! RO:INVARIANTS — default config is amnesia; idempotency cache is RAM-only and TTL-purged; ledger client can run in memory.
//! RO:METRICS — none.
//! RO:CONFIG — WalletConfig::default.
//! RO:SECURITY — no filesystem or token persistence.
//! RO:TEST — cargo test -p svc-wallet --test i_9_amnesia.

mod harness;

use svc_wallet::idem::store::IdempotencyStore;

#[test]
fn default_config_is_amnesia_first() {
    let cfg = harness::cfg();

    assert!(cfg.amnesia);
    cfg.validate()
        .expect("default amnesia config should validate");
}

#[test]
fn idempotency_store_expires_ram_entries_without_persistence() {
    let ttl = std::time::Duration::from_millis(10);
    let store = IdempotencyStore::new(ttl);
    let receipt = harness::dummy_receipt("tx_amnesia", "idem_amnesia");

    store.insert(
        "idem_amnesia".to_string(),
        "fingerprint".to_string(),
        receipt,
        1_000,
    );
    assert_eq!(store.len(), 1);

    store.purge_expired(1_011);
    assert!(store.is_empty());
}

#[test]
fn in_memory_ledger_state_is_rebuildable_for_tests_and_dev() {
    let cfg = harness::cfg();
    let client = harness::client();

    harness::issue_to(&client, &cfg, "acct_a", 5, "idem_amnesia_issue");
    assert_eq!(harness::balance_of(&client, &cfg, "acct_a"), 5);

    let fresh_client = harness::client();
    assert_eq!(harness::balance_of(&fresh_client, &cfg, "acct_a"), 0);
}
