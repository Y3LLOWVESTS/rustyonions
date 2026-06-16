use svc_rewarder::core::algebra::AmountMinor;
use svc_rewarder::inputs::{
    canonical_snapshot_cid, resolve_accounting_snapshot, resolve_reward_policy,
    AccountContribution, AccountingSnapshot, ContentCid, RewardFundingSource, RewardPolicy,
};

fn policy_hash() -> String {
    format!("b3:{}", "b".repeat(64))
}

fn mismatched_inputs_cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap()
}

fn cid_for_snapshot(snapshot: AccountingSnapshot) -> ContentCid {
    let cid = canonical_snapshot_cid(snapshot).expect("canonical snapshot cid");
    ContentCid::parse(cid).expect("canonical cid parses")
}

fn canonicalization_sample() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(100),
        contributions: vec![
            AccountContribution {
                account: " acct_b ".into(),
                bytes_stored: 20,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 10,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    }
}

#[test]
fn accounting_snapshot_canonicalizes_account_order_and_whitespace() {
    let snapshot = canonicalization_sample();
    let cid = cid_for_snapshot(snapshot.clone());

    let resolved = resolve_accounting_snapshot(&cid, Some(snapshot)).unwrap();

    assert_eq!(resolved.contributions[0].account, "acct_a");
    assert_eq!(resolved.contributions[1].account, "acct_b");
}

#[test]
fn accounting_snapshot_rejects_duplicate_accounts() {
    let snapshot = AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(100),
        contributions: vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 20,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: " acct_a ".into(),
                bytes_stored: 10,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    };

    let err = resolve_accounting_snapshot(&mismatched_inputs_cid(), Some(snapshot)).unwrap_err();

    assert_eq!(err.reason(), "bad_request");
}

#[test]
fn accounting_snapshot_rejects_mismatched_inputs_cid() {
    let snapshot = canonicalization_sample();

    let err = resolve_accounting_snapshot(&mismatched_inputs_cid(), Some(snapshot)).unwrap_err();

    assert_eq!(err.reason(), "bad_request");
    assert!(err.to_string().contains("inputs_cid mismatch"));
}

#[test]
fn accounting_snapshot_accepts_matching_inputs_cid() {
    let snapshot = canonicalization_sample();
    let cid = cid_for_snapshot(snapshot.clone());

    let resolved = resolve_accounting_snapshot(&cid, Some(snapshot)).unwrap();

    assert_eq!(canonical_snapshot_cid(resolved).unwrap(), cid.as_str());
}

#[test]
fn canonical_snapshot_cid_is_deterministic_after_sorting() {
    let one = AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(100),
        contributions: vec![
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 20,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 10,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    };

    let two = AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(100),
        contributions: vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 10,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 20,
                bytes_served: 0,
                uptime_seconds: 0,
            },
        ],
    };

    assert_eq!(
        canonical_snapshot_cid(one).unwrap(),
        canonical_snapshot_cid(two).unwrap()
    );
}

#[test]
fn policy_resolver_rejects_mismatched_hash() {
    let policy = RewardPolicy::dev_default("policy:v1", policy_hash());
    let err = resolve_reward_policy(Some(policy), "policy:v1", &format!("b3:{}", "c".repeat(64)))
        .unwrap_err();

    assert_eq!(err.reason(), "bad_request");
}

#[test]
fn policy_resolver_rejects_uppercase_hash() {
    let hash = format!("b3:{}", "B".repeat(64));
    let policy = RewardPolicy::dev_default("policy:v1", hash.clone());
    let err = resolve_reward_policy(Some(policy), "policy:v1", &hash).unwrap_err();

    assert_eq!(err.reason(), "bad_request");
}

#[test]
fn policy_resolver_accepts_default_inline_absence() {
    let hash = policy_hash();
    let policy = resolve_reward_policy(None, "policy:v1", &hash).unwrap();

    assert_eq!(policy.id, "policy:v1");
    assert_eq!(policy.hash, hash);
    assert_eq!(policy.weight_bps, 10_000);
    assert_eq!(policy.funding_source, RewardFundingSource::ProtocolPool);
}
