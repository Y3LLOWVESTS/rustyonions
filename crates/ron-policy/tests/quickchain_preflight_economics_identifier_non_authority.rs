//! RO:WHAT — Economics identifier non-authority tests for ron-policy QuickChain preflight.
//! RO:WHY — Prevent account/role/split/action names from becoming receipt, balance, root, settlement, or token authority.
//! RO:INTERACTS — `EconomicsPolicy`, `validate_economics_policy`, checked-in ROC economics config.
//! RO:INVARIANTS — economics config identifiers are declarative aliases only, never proof or mutation authority.

use ron_policy::economics::{load_economics_toml_str, validate_economics_policy, EconomicsPolicy};

const CHECKED_IN_POLICY: &str = include_str!("../../../configs/roc-economics.toml");

#[test]
fn checked_in_economics_identifiers_remain_non_authority() {
    let policy = load_checked_in();

    validate_economics_policy(&policy).expect("checked-in economics identifiers should validate");
}

#[test]
fn declarative_burn_account_alias_is_allowed_without_granting_mutation_authority() {
    let mut policy = load_checked_in();

    policy
        .accounts
        .entry("burn".to_string())
        .or_insert_with(|| "t:default/burn/sink".to_string());

    validate_economics_policy(&policy)
        .expect("burn account alias is declarative config, not a burn operation");
}

#[test]
fn account_alias_named_like_receipt_authority_rejects() {
    let mut policy = load_checked_in();
    policy.accounts.insert(
        "receipt_root".to_string(),
        "t:default/receipt/root".to_string(),
    );

    let err =
        validate_economics_policy(&policy).expect_err("receipt-shaped account alias must reject");

    assert_authority_identifier_error(&err.to_string());
}

#[test]
fn role_alias_named_like_balance_authority_rejects() {
    let mut policy = load_checked_in();
    policy.roles.insert(
        "balance_minor".to_string(),
        "dynamic balance-looking recipient".to_string(),
    );

    let err =
        validate_economics_policy(&policy).expect_err("balance-shaped role alias must reject");

    assert_authority_identifier_error(&err.to_string());
}

#[test]
fn remainder_sink_named_like_settlement_authority_rejects() {
    let mut policy = load_checked_in();
    policy.accounts.insert(
        "settlement_status".to_string(),
        "t:default/settlement/status".to_string(),
    );
    policy.remainder_sink = "settlement_status".to_string();

    let err = validate_economics_policy(&policy)
        .expect_err("settlement-shaped remainder sink must reject");

    assert_authority_identifier_error(&err.to_string());
}

#[test]
fn split_destination_named_like_validator_authority_rejects() {
    let mut policy = load_checked_in();
    policy.roles.insert(
        "validator_signature".to_string(),
        "dynamic validator-looking recipient".to_string(),
    );

    let action = policy
        .actions
        .get_mut("paid_content_view")
        .expect("checked-in policy should contain paid_content_view");
    let split = action
        .splits
        .first_mut()
        .expect("paid_content_view should have at least one split");
    split.to = "validator_signature".to_string();

    let err = validate_economics_policy(&policy)
        .expect_err("validator-shaped split destination must reject");

    assert_authority_identifier_error(&err.to_string());
}

#[test]
fn action_id_named_like_token_mutation_rejects_before_use() {
    let mut policy = load_checked_in();
    let sample = policy
        .actions
        .get("site_visit")
        .expect("checked-in policy should contain site_visit")
        .clone();
    policy.actions.insert("mint_roc".to_string(), sample);

    let err = validate_economics_policy(&policy).expect_err("mint-shaped action id must reject");

    assert!(
        err.to_string().contains("unknown economics action")
            || err.to_string().contains("looks like economic authority"),
        "unexpected error: {err}"
    );
}

fn load_checked_in() -> EconomicsPolicy {
    load_economics_toml_str(CHECKED_IN_POLICY).expect("checked-in economics config should load")
}

fn assert_authority_identifier_error(message: &str) {
    assert!(
        message.contains("looks like economic authority"),
        "expected authority-shaped identifier validation error, got: {message}"
    );
}
