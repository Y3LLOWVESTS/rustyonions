//! RO:WHAT — Pure copy-on-write QuickChain balance transitions for issue, transfer, and burn.
//! RO:WHY — ECON/RES: establish checked deterministic arithmetic before replay composition, persistence, receipts, holds, or roots.
//! RO:INTERACTS — balance_state.rs, transition_error.rs, ron-proto QuickChain operation intents.
//! RO:INVARIANTS — ron-proto validation first; positive u128 execution amounts; atomic commit; transfer conserves; issue/burn require approval.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — supply decisions are pre-evaluated inputs, not capabilities; callers must enforce real wallet/policy authority.
//! RO:TEST — tests/quickchain_balance_transition.rs.

use ron_proto::quickchain::{QuickChainOperationClassV1, QuickChainOperationIntentV1};

use super::{balance_state::QuickChainBalanceState, transition_error::QuickChainTransitionError};

/// Pre-evaluated supply-policy decision supplied to the pure transition layer.
///
/// This value is not a capability, signature, or authorization token. A future
/// `svc-wallet` integration must establish real authority before selecting an
/// approved issue or burn decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum QuickChainSupplyDecision {
    /// No supply-changing operation has been approved.
    #[default]
    NoSupplyChange,

    /// The surrounding trusted wallet/policy path approved one issue operation.
    IssueApproved,

    /// The surrounding trusted wallet/policy path approved one burn operation.
    BurnApproved,
}

/// Non-receipt summary of one successfully applied pure balance transition.
///
/// This summary proves only what changed in this in-memory transition. It is not
/// a transaction receipt, accepted-status claim, entitlement, root, or finality
/// artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct QuickChainBalanceTransition {
    /// Durable operation identity copied from the validated intent.
    pub operation_id: String,

    /// Operation class executed by this transition.
    pub op_class: QuickChainOperationClassV1,

    /// Positive amount applied in ROC minor units.
    pub amount_minor: u128,

    /// Primary actor account.
    pub actor_account_id: String,

    /// Optional destination account for transfers.
    pub counterparty_account_id: Option<String>,

    /// Actor balance before the transition.
    pub actor_balance_before: u128,

    /// Actor balance after the transition.
    pub actor_balance_after: u128,

    /// Counterparty balance before the transition, when applicable.
    pub counterparty_balance_before: Option<u128>,

    /// Counterparty balance after the transition, when applicable.
    pub counterparty_balance_after: Option<u128>,

    /// Circulating supply before the transition.
    pub supply_before: u128,

    /// Circulating supply after the transition.
    pub supply_after: u128,
}

impl QuickChainBalanceState {
    /// Apply one validated issue, transfer, or burn using copy-on-write atomicity.
    ///
    /// The original state changes only after all DTO validation, amount parsing,
    /// checked arithmetic, authorization checks, and conservation checks succeed.
    pub fn apply_balance_operation(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        supply_decision: QuickChainSupplyDecision,
    ) -> Result<QuickChainBalanceTransition, QuickChainTransitionError> {
        // ron-proto owns the complete operation-intent shape and canonical
        // money-width contract. Ledger-specific execution begins only after this
        // validation succeeds.
        intent
            .validate()
            .map_err(|error| QuickChainTransitionError::InvalidIntent(error.to_string()))?;

        let amount_minor = parse_validated_positive_amount(intent)?;
        let mut candidate = self.clone();

        let transition =
            candidate.apply_validated_operation(intent, amount_minor, supply_decision)?;

        candidate.validate_invariants()?;
        *self = candidate;

        Ok(transition)
    }

    fn apply_validated_operation(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        amount_minor: u128,
        supply_decision: QuickChainSupplyDecision,
    ) -> Result<QuickChainBalanceTransition, QuickChainTransitionError> {
        match intent.op_class {
            QuickChainOperationClassV1::Issue => {
                self.apply_issue(intent, amount_minor, supply_decision)
            }
            QuickChainOperationClassV1::Transfer => self.apply_transfer(intent, amount_minor),
            QuickChainOperationClassV1::Burn => {
                self.apply_burn(intent, amount_minor, supply_decision)
            }
            QuickChainOperationClassV1::HoldOpen
            | QuickChainOperationClassV1::HoldCapture
            | QuickChainOperationClassV1::HoldRelease
            | QuickChainOperationClassV1::HoldExpire => {
                Err(QuickChainTransitionError::UnsupportedOperationClass)
            }
            _ => Err(QuickChainTransitionError::UnsupportedOperationClass),
        }
    }

    fn apply_issue(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        amount_minor: u128,
        supply_decision: QuickChainSupplyDecision,
    ) -> Result<QuickChainBalanceTransition, QuickChainTransitionError> {
        if supply_decision != QuickChainSupplyDecision::IssueApproved {
            return Err(QuickChainTransitionError::UnauthorizedIssue);
        }

        let actor_balance_before = self.balance_minor(&intent.actor_account_id);
        let actor_balance_after =
            actor_balance_before
                .checked_add(amount_minor)
                .ok_or_else(|| QuickChainTransitionError::BalanceOverflow {
                    account_id: intent.actor_account_id.clone(),
                })?;

        let supply_before = self.current_supply_minor();
        let supply_after = supply_before
            .checked_add(amount_minor)
            .ok_or(QuickChainTransitionError::SupplyOverflow)?;

        let total_issued_after = self
            .total_issued_minor()
            .checked_add(amount_minor)
            .ok_or(QuickChainTransitionError::SupplyOverflow)?;

        self.set_balance(intent.actor_account_id.clone(), actor_balance_after);
        self.set_supply_counters(total_issued_after, self.total_burned_minor(), supply_after);

        Ok(QuickChainBalanceTransition {
            operation_id: intent.operation_id.clone(),
            op_class: intent.op_class,
            amount_minor,
            actor_account_id: intent.actor_account_id.clone(),
            counterparty_account_id: None,
            actor_balance_before,
            actor_balance_after,
            counterparty_balance_before: None,
            counterparty_balance_after: None,
            supply_before,
            supply_after,
        })
    }

    fn apply_transfer(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        amount_minor: u128,
    ) -> Result<QuickChainBalanceTransition, QuickChainTransitionError> {
        // A validated transfer must contain its counterparty. Failure here means
        // ron-proto validation and ledger execution have diverged.
        let counterparty_account_id = intent
            .counterparty_account_id
            .as_ref()
            .ok_or(QuickChainTransitionError::StateInvariantViolation)?;

        let actor_balance_before = self.balance_minor(&intent.actor_account_id);
        if actor_balance_before < amount_minor {
            return Err(QuickChainTransitionError::InsufficientFunds {
                account_id: intent.actor_account_id.clone(),
                available_minor: actor_balance_before,
                required_minor: amount_minor,
            });
        }

        let supply_before = self.current_supply_minor();

        if counterparty_account_id == &intent.actor_account_id {
            return Ok(QuickChainBalanceTransition {
                operation_id: intent.operation_id.clone(),
                op_class: intent.op_class,
                amount_minor,
                actor_account_id: intent.actor_account_id.clone(),
                counterparty_account_id: Some(counterparty_account_id.clone()),
                actor_balance_before,
                actor_balance_after: actor_balance_before,
                counterparty_balance_before: Some(actor_balance_before),
                counterparty_balance_after: Some(actor_balance_before),
                supply_before,
                supply_after: supply_before,
            });
        }

        let actor_balance_after =
            actor_balance_before
                .checked_sub(amount_minor)
                .ok_or_else(|| QuickChainTransitionError::InsufficientFunds {
                    account_id: intent.actor_account_id.clone(),
                    available_minor: actor_balance_before,
                    required_minor: amount_minor,
                })?;

        let counterparty_balance_before = self.balance_minor(counterparty_account_id);
        let counterparty_balance_after = counterparty_balance_before
            .checked_add(amount_minor)
            .ok_or_else(|| QuickChainTransitionError::BalanceOverflow {
                account_id: counterparty_account_id.clone(),
            })?;

        self.set_balance(intent.actor_account_id.clone(), actor_balance_after);
        self.set_balance(counterparty_account_id.clone(), counterparty_balance_after);

        Ok(QuickChainBalanceTransition {
            operation_id: intent.operation_id.clone(),
            op_class: intent.op_class,
            amount_minor,
            actor_account_id: intent.actor_account_id.clone(),
            counterparty_account_id: Some(counterparty_account_id.clone()),
            actor_balance_before,
            actor_balance_after,
            counterparty_balance_before: Some(counterparty_balance_before),
            counterparty_balance_after: Some(counterparty_balance_after),
            supply_before,
            supply_after: supply_before,
        })
    }

    fn apply_burn(
        &mut self,
        intent: &QuickChainOperationIntentV1,
        amount_minor: u128,
        supply_decision: QuickChainSupplyDecision,
    ) -> Result<QuickChainBalanceTransition, QuickChainTransitionError> {
        if supply_decision != QuickChainSupplyDecision::BurnApproved {
            return Err(QuickChainTransitionError::UnauthorizedBurn);
        }

        let actor_balance_before = self.balance_minor(&intent.actor_account_id);
        let actor_balance_after =
            actor_balance_before
                .checked_sub(amount_minor)
                .ok_or_else(|| QuickChainTransitionError::InsufficientFunds {
                    account_id: intent.actor_account_id.clone(),
                    available_minor: actor_balance_before,
                    required_minor: amount_minor,
                })?;

        let supply_before = self.current_supply_minor();
        let supply_after = supply_before
            .checked_sub(amount_minor)
            .ok_or(QuickChainTransitionError::SupplyUnderflow)?;

        let total_burned_after = self
            .total_burned_minor()
            .checked_add(amount_minor)
            .ok_or(QuickChainTransitionError::SupplyOverflow)?;

        self.set_balance(intent.actor_account_id.clone(), actor_balance_after);
        self.set_supply_counters(self.total_issued_minor(), total_burned_after, supply_after);

        Ok(QuickChainBalanceTransition {
            operation_id: intent.operation_id.clone(),
            op_class: intent.op_class,
            amount_minor,
            actor_account_id: intent.actor_account_id.clone(),
            counterparty_account_id: None,
            actor_balance_before,
            actor_balance_after,
            counterparty_balance_before: None,
            counterparty_balance_after: None,
            supply_before,
            supply_after,
        })
    }
}

fn parse_validated_positive_amount(
    intent: &QuickChainOperationIntentV1,
) -> Result<u128, QuickChainTransitionError> {
    // Every currently defined operation class requires amount_minor, and
    // ron-proto validation guarantees canonical decimal text fitting u128.
    let amount_minor = intent
        .amount_minor
        .as_deref()
        .ok_or(QuickChainTransitionError::StateInvariantViolation)?;

    let amount = amount_minor
        .parse::<u128>()
        .map_err(|_| QuickChainTransitionError::StateInvariantViolation)?;

    if amount == 0 {
        return Err(QuickChainTransitionError::ZeroAmount);
    }

    Ok(amount)
}
