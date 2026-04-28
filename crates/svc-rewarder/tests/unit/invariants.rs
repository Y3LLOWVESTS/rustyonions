use svc_rewarder::core::algebra::AmountMinor;
use svc_rewarder::core::invariants::validate_payouts;
use svc_rewarder::outputs::RewardPayout;

#[test]
fn conservation_residual_is_pool_minus_payouts() {
    let payouts = vec![
        RewardPayout {
            account: "acct_a".into(),
            amount_minor_units: AmountMinor(40),
            score: 4,
        },
        RewardPayout {
            account: "acct_b".into(),
            amount_minor_units: AmountMinor(50),
            score: 5,
        },
    ];
    let residual = validate_payouts(AmountMinor(100), &payouts).unwrap();
    assert_eq!(residual.get(), 10);
}

#[test]
fn payouts_cannot_exceed_pool() {
    let payouts = vec![RewardPayout {
        account: "acct_a".into(),
        amount_minor_units: AmountMinor(101),
        score: 1,
    }];
    let err = validate_payouts(AmountMinor(100), &payouts).unwrap_err();
    assert_eq!(err.reason(), "invariant");
}
