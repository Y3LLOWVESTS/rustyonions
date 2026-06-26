//! RO:WHAT — Phase 4 Round 1 internal bond accounting model for QuickChain preflight.
//! RO:WHY — ECON/GOV: model conservative internal bond math before any live slash, public market, or wallet route exists.
//! RO:INTERACTS — ron-proto bond DTOs, future svc-wallet explicit confirmation, and ledger conservation tests.
//! RO:INVARIANTS — deterministic; COW updates; ROC-only; no IO/clocks; no public staking/liquidity; slash evidence is no-op.
//! RO:METRICS — none.
//! RO:CONFIG — quickchain-preflight feature only.
//! RO:SECURITY — this model grants no spend authority, no paid unlock, no finality, no bridge, and no automatic slashing.
//! RO:TEST — tests/quickchain_phase4_bond_accounting.rs.

use std::collections::BTreeMap;

use ron_proto::quickchain::{
    QuickChainBondAccountStatusV1, QuickChainBondIntentKindV1,
    QuickChainBondLifecycleDecisionStatusV1, QuickChainBondLifecycleDecisionV1,
    QuickChainBondLifecycleOperationV1, QuickChainBondLifecycleRejectionCodeV1,
    QuickChainSlashEvidenceV1, QuickChainValidatorBondAccountV1, QuickChainValidatorBondIntentV1,
    QUICKCHAIN_BOND_ASSET_ROC, QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA, QUICKCHAIN_DTO_VERSION,
    QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA,
};
use thiserror::Error;

/// Deterministic rejection taxonomy for the Phase 4 Round 1 bond model.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum QuickChainBondLedgerError {
    /// The submitted bond intent failed the ron-proto DTO contract.
    #[error("invalid bond intent: {0}")]
    InvalidBondIntent(String),

    /// The supplied slash evidence failed the ron-proto DTO contract.
    #[error("invalid slash evidence: {0}")]
    InvalidSlashEvidence(String),

    /// A bond account already exists for this identifier.
    #[error("bond account already exists: {bond_account_id}")]
    BondAccountAlreadyExists {
        /// Bond account identifier that already exists.
        bond_account_id: String,
    },

    /// A bond account does not exist for this identifier.
    #[error("unknown bond account: {bond_account_id}")]
    UnknownBondAccount {
        /// Bond account identifier that could not be found.
        bond_account_id: String,
    },

    /// The intent does not match the existing bond account owner.
    #[error("bond owner account mismatch")]
    OwnerAccountMismatch,

    /// The intent does not match the existing validator binding.
    #[error("bond validator mismatch")]
    ValidatorMismatch,

    /// The intent asset does not match the existing account asset.
    #[error("bond asset mismatch")]
    AssetMismatch,

    /// The explicit owner balance boundary cannot cover the requested lock.
    #[error("insufficient owner available amount: available={available_minor} required={required_minor}")]
    InsufficientOwnerAvailable {
        /// Explicit owner-available boundary supplied by the caller.
        available_minor: u128,
        /// Amount required by the requested bond operation.
        required_minor: u128,
    },

    /// The account does not have enough available bonded ROC for an unlock request.
    #[error("insufficient available bonded amount: available={available_minor} required={required_minor}")]
    InsufficientBondAvailable {
        /// Amount currently available to move into pending unlock.
        available_minor: u128,
        /// Amount requested for the unlock operation.
        required_minor: u128,
    },

    /// The account does not have enough pending unlock amount for a cancel request.
    #[error(
        "insufficient pending unlock amount: pending={pending_minor} required={required_minor}"
    )]
    InsufficientPendingUnlock {
        /// Amount currently pending unlock.
        pending_minor: u128,
        /// Amount requested for the cancel-unlock operation.
        required_minor: u128,
    },

    /// Checked arithmetic overflowed.
    #[error("bond accounting arithmetic overflow")]
    ArithmeticOverflow,

    /// Bond component accounting did not conserve.
    #[error("bond accounting invariant violation")]
    StateInvariantViolation,
}

/// Deterministic internal bond account record.
///
/// This is model state only. It is not wallet authority and not a public staking
/// account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainBondAccountRecord {
    validator_id: String,
    bond_account_id: String,
    owner_account_id: String,
    asset: String,
    locked_minor: u128,
    available_to_unlock_minor: u128,
    pending_unlock_minor: u128,
    slash_reserved_minor: u128,
    status: QuickChainBondAccountStatusV1,
    account_sequence: u64,
}

impl QuickChainBondAccountRecord {
    /// Validator identity bound to this bond account.
    #[must_use]
    pub fn validator_id(&self) -> &str {
        &self.validator_id
    }

    /// Bond account identifier.
    #[must_use]
    pub fn bond_account_id(&self) -> &str {
        &self.bond_account_id
    }

    /// Owner wallet account identifier.
    #[must_use]
    pub fn owner_account_id(&self) -> &str {
        &self.owner_account_id
    }

    /// Asset identifier. Phase 4 Round 1 only accepts `roc`.
    #[must_use]
    pub fn asset(&self) -> &str {
        &self.asset
    }

    /// Total locked amount represented by this account.
    #[must_use]
    pub const fn locked_minor(&self) -> u128 {
        self.locked_minor
    }

    /// Locked amount not currently pending unlock or evidence reserve.
    #[must_use]
    pub const fn available_to_unlock_minor(&self) -> u128 {
        self.available_to_unlock_minor
    }

    /// Amount in a pending unlock window.
    #[must_use]
    pub const fn pending_unlock_minor(&self) -> u128 {
        self.pending_unlock_minor
    }

    /// Evidence-only reserve amount. Phase 4 Round 1 never increments this automatically.
    #[must_use]
    pub const fn slash_reserved_minor(&self) -> u128 {
        self.slash_reserved_minor
    }

    /// Descriptive lifecycle status.
    #[must_use]
    pub const fn status(&self) -> QuickChainBondAccountStatusV1 {
        self.status
    }

    /// Ledger-assigned model sequence for this bond account.
    #[must_use]
    pub const fn account_sequence(&self) -> u64 {
        self.account_sequence
    }

    fn ensure_matches_intent(
        &self,
        intent: &QuickChainValidatorBondIntentV1,
    ) -> Result<(), QuickChainBondLedgerError> {
        if self.validator_id != intent.validator_id {
            return Err(QuickChainBondLedgerError::ValidatorMismatch);
        }

        if self.owner_account_id != intent.actor_account_id {
            return Err(QuickChainBondLedgerError::OwnerAccountMismatch);
        }

        if self.asset != intent.asset {
            return Err(QuickChainBondLedgerError::AssetMismatch);
        }

        Ok(())
    }

    fn validate_components(&self) -> Result<(), QuickChainBondLedgerError> {
        let components = self
            .available_to_unlock_minor
            .checked_add(self.pending_unlock_minor)
            .and_then(|value| value.checked_add(self.slash_reserved_minor))
            .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;

        if components != self.locked_minor || self.account_sequence == 0 {
            return Err(QuickChainBondLedgerError::StateInvariantViolation);
        }

        Ok(())
    }

    fn to_dto(&self, chain_id: &str, epoch_id: &str) -> QuickChainValidatorBondAccountV1 {
        QuickChainValidatorBondAccountV1 {
            schema: QUICKCHAIN_VALIDATOR_BOND_ACCOUNT_SCHEMA.to_owned(),
            version: QUICKCHAIN_DTO_VERSION,
            chain_id: chain_id.to_owned(),
            epoch_id: epoch_id.to_owned(),
            validator_id: self.validator_id.clone(),
            bond_account_id: self.bond_account_id.clone(),
            owner_account_id: self.owner_account_id.clone(),
            asset: self.asset.clone(),
            locked_minor: self.locked_minor.to_string(),
            available_to_unlock_minor: self.available_to_unlock_minor.to_string(),
            pending_unlock_minor: self.pending_unlock_minor.to_string(),
            slash_reserved_minor: self.slash_reserved_minor.to_string(),
            status: self.status,
            account_sequence: self.account_sequence,
        }
    }
}

/// Outcome of one explicit bond intent model application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainBondLedgerOutcome {
    account: QuickChainValidatorBondAccountV1,
    owner_debit_minor: u128,
}

impl QuickChainBondLedgerOutcome {
    /// Bond account snapshot after the accepted model operation.
    #[must_use]
    pub const fn account(&self) -> &QuickChainValidatorBondAccountV1 {
        &self.account
    }

    /// Explicit owner debit implied by this model operation.
    ///
    /// Only open/increase bond intents debit the owner in Phase 4 Round 1.
    #[must_use]
    pub const fn owner_debit_minor(&self) -> u128 {
        self.owner_debit_minor
    }
}

/// Pure in-memory Phase 4 Round 1 bond accounting model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainBondAccountingState {
    accounts: BTreeMap<String, QuickChainBondAccountRecord>,
    next_account_sequence: u64,
}

impl Default for QuickChainBondAccountingState {
    fn default() -> Self {
        Self {
            accounts: BTreeMap::new(),
            next_account_sequence: 1,
        }
    }
}

impl QuickChainBondAccountingState {
    /// Create an empty deterministic bond accounting model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return one internal bond account by id.
    #[must_use]
    pub fn account(&self, bond_account_id: &str) -> Option<&QuickChainBondAccountRecord> {
        self.accounts.get(bond_account_id)
    }

    /// Return all accounts in deterministic bond-account-id order.
    pub fn ordered_accounts(&self) -> impl Iterator<Item = &QuickChainBondAccountRecord> {
        self.accounts.values()
    }

    /// Number of bond accounts in the model.
    #[must_use]
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Next ledger-assigned model sequence.
    #[must_use]
    pub const fn next_account_sequence(&self) -> u64 {
        self.next_account_sequence
    }

    /// Total locked ROC across all internal bond accounts.
    #[must_use]
    pub fn total_locked_minor(&self) -> u128 {
        self.accounts
            .values()
            .map(QuickChainBondAccountRecord::locked_minor)
            .sum()
    }

    /// Total pending unlock ROC across all internal bond accounts.
    #[must_use]
    pub fn total_pending_unlock_minor(&self) -> u128 {
        self.accounts
            .values()
            .map(QuickChainBondAccountRecord::pending_unlock_minor)
            .sum()
    }

    /// Total evidence-reserved ROC across all internal bond accounts.
    #[must_use]
    pub fn total_slash_reserved_minor(&self) -> u128 {
        self.accounts
            .values()
            .map(QuickChainBondAccountRecord::slash_reserved_minor)
            .sum()
    }

    /// Total available-to-unlock ROC across all internal bond accounts.
    #[must_use]
    pub fn total_available_to_unlock_minor(&self) -> u128 {
        self.accounts
            .values()
            .map(QuickChainBondAccountRecord::available_to_unlock_minor)
            .sum()
    }

    /// Apply one explicit internal bond intent to the model.
    ///
    /// This is not a wallet route and does not move live balances. Callers must
    /// supply an explicit owner-available boundary for open/increase operations;
    /// otherwise the model rejects. The method uses copy-on-write so rejected
    /// intents never partially mutate state.
    pub fn apply_explicit_bond_intent(
        &mut self,
        intent: &QuickChainValidatorBondIntentV1,
        owner_available_minor: u128,
    ) -> Result<QuickChainBondLedgerOutcome, QuickChainBondLedgerError> {
        intent
            .validate()
            .map_err(|error| QuickChainBondLedgerError::InvalidBondIntent(error.to_string()))?;

        if intent.asset != QUICKCHAIN_BOND_ASSET_ROC {
            return Err(QuickChainBondLedgerError::AssetMismatch);
        }

        let amount_minor = required_amount_minor(intent)?;
        let mut candidate = self.clone();
        let owner_debit_minor = match intent.kind {
            QuickChainBondIntentKindV1::OpenBond => {
                ensure_owner_available(owner_available_minor, amount_minor)?;

                if candidate.accounts.contains_key(&intent.bond_account_id) {
                    return Err(QuickChainBondLedgerError::BondAccountAlreadyExists {
                        bond_account_id: intent.bond_account_id.clone(),
                    });
                }

                let account_sequence = candidate.allocate_account_sequence()?;
                let record = QuickChainBondAccountRecord {
                    validator_id: intent.validator_id.clone(),
                    bond_account_id: intent.bond_account_id.clone(),
                    owner_account_id: intent.actor_account_id.clone(),
                    asset: intent.asset.clone(),
                    locked_minor: amount_minor,
                    available_to_unlock_minor: amount_minor,
                    pending_unlock_minor: 0,
                    slash_reserved_minor: 0,
                    status: QuickChainBondAccountStatusV1::Active,
                    account_sequence,
                };

                candidate
                    .accounts
                    .insert(intent.bond_account_id.clone(), record);

                amount_minor
            }
            QuickChainBondIntentKindV1::IncreaseBond => {
                ensure_owner_available(owner_available_minor, amount_minor)?;
                candidate.ensure_existing_account_matches(intent)?;

                let account_sequence = candidate.allocate_account_sequence()?;
                let account = candidate
                    .accounts
                    .get_mut(&intent.bond_account_id)
                    .ok_or_else(|| QuickChainBondLedgerError::UnknownBondAccount {
                        bond_account_id: intent.bond_account_id.clone(),
                    })?;

                account.locked_minor = account
                    .locked_minor
                    .checked_add(amount_minor)
                    .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;
                account.available_to_unlock_minor = account
                    .available_to_unlock_minor
                    .checked_add(amount_minor)
                    .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;
                account.status = QuickChainBondAccountStatusV1::Active;
                account.account_sequence = account_sequence;

                amount_minor
            }
            QuickChainBondIntentKindV1::RequestUnlock => {
                candidate.ensure_existing_account_matches(intent)?;
                candidate.ensure_available_to_unlock(intent, amount_minor)?;

                let account_sequence = candidate.allocate_account_sequence()?;
                let account = candidate
                    .accounts
                    .get_mut(&intent.bond_account_id)
                    .ok_or_else(|| QuickChainBondLedgerError::UnknownBondAccount {
                        bond_account_id: intent.bond_account_id.clone(),
                    })?;

                account.available_to_unlock_minor = account
                    .available_to_unlock_minor
                    .checked_sub(amount_minor)
                    .ok_or(QuickChainBondLedgerError::StateInvariantViolation)?;
                account.pending_unlock_minor = account
                    .pending_unlock_minor
                    .checked_add(amount_minor)
                    .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;
                account.status = QuickChainBondAccountStatusV1::UnlockPending;
                account.account_sequence = account_sequence;

                0
            }
            QuickChainBondIntentKindV1::CancelUnlockRequest => {
                candidate.ensure_existing_account_matches(intent)?;
                candidate.ensure_pending_unlock(intent, amount_minor)?;

                let account_sequence = candidate.allocate_account_sequence()?;
                let account = candidate
                    .accounts
                    .get_mut(&intent.bond_account_id)
                    .ok_or_else(|| QuickChainBondLedgerError::UnknownBondAccount {
                        bond_account_id: intent.bond_account_id.clone(),
                    })?;

                account.pending_unlock_minor = account
                    .pending_unlock_minor
                    .checked_sub(amount_minor)
                    .ok_or(QuickChainBondLedgerError::StateInvariantViolation)?;
                account.available_to_unlock_minor = account
                    .available_to_unlock_minor
                    .checked_add(amount_minor)
                    .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;
                account.status = if account.pending_unlock_minor == 0 {
                    QuickChainBondAccountStatusV1::Active
                } else {
                    QuickChainBondAccountStatusV1::UnlockPending
                };
                account.account_sequence = account_sequence;

                0
            }
            _ => {
                return Err(QuickChainBondLedgerError::InvalidBondIntent(
                    "unsupported Phase 4 Round 1 bond intent kind".to_owned(),
                ));
            }
        };

        candidate.validate_invariants()?;

        let account = candidate
            .accounts
            .get(&intent.bond_account_id)
            .ok_or_else(|| QuickChainBondLedgerError::UnknownBondAccount {
                bond_account_id: intent.bond_account_id.clone(),
            })?
            .to_dto(&intent.chain_id, &intent.epoch_id);

        account
            .validate()
            .map_err(|error| QuickChainBondLedgerError::InvalidBondIntent(error.to_string()))?;

        *self = candidate;

        Ok(QuickChainBondLedgerOutcome {
            account,
            owner_debit_minor,
        })
    }

    fn ensure_existing_account_matches(
        &self,
        intent: &QuickChainValidatorBondIntentV1,
    ) -> Result<(), QuickChainBondLedgerError> {
        self.accounts
            .get(&intent.bond_account_id)
            .ok_or_else(|| QuickChainBondLedgerError::UnknownBondAccount {
                bond_account_id: intent.bond_account_id.clone(),
            })?
            .ensure_matches_intent(intent)
    }

    fn ensure_available_to_unlock(
        &self,
        intent: &QuickChainValidatorBondIntentV1,
        amount_minor: u128,
    ) -> Result<(), QuickChainBondLedgerError> {
        let account = self.accounts.get(&intent.bond_account_id).ok_or_else(|| {
            QuickChainBondLedgerError::UnknownBondAccount {
                bond_account_id: intent.bond_account_id.clone(),
            }
        })?;

        if amount_minor > account.available_to_unlock_minor {
            return Err(QuickChainBondLedgerError::InsufficientBondAvailable {
                available_minor: account.available_to_unlock_minor,
                required_minor: amount_minor,
            });
        }

        Ok(())
    }

    fn ensure_pending_unlock(
        &self,
        intent: &QuickChainValidatorBondIntentV1,
        amount_minor: u128,
    ) -> Result<(), QuickChainBondLedgerError> {
        let account = self.accounts.get(&intent.bond_account_id).ok_or_else(|| {
            QuickChainBondLedgerError::UnknownBondAccount {
                bond_account_id: intent.bond_account_id.clone(),
            }
        })?;

        if amount_minor > account.pending_unlock_minor {
            return Err(QuickChainBondLedgerError::InsufficientPendingUnlock {
                pending_minor: account.pending_unlock_minor,
                required_minor: amount_minor,
            });
        }

        Ok(())
    }

    fn allocate_account_sequence(&mut self) -> Result<u64, QuickChainBondLedgerError> {
        let sequence = self.next_account_sequence;
        self.next_account_sequence = self
            .next_account_sequence
            .checked_add(1)
            .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;

        Ok(sequence)
    }

    fn validate_invariants(&self) -> Result<(), QuickChainBondLedgerError> {
        for account in self.accounts.values() {
            account.validate_components()?;
        }

        let total_components = self
            .total_available_to_unlock_minor()
            .checked_add(self.total_pending_unlock_minor())
            .and_then(|value| value.checked_add(self.total_slash_reserved_minor()))
            .ok_or(QuickChainBondLedgerError::ArithmeticOverflow)?;

        if total_components != self.total_locked_minor() {
            return Err(QuickChainBondLedgerError::StateInvariantViolation);
        }

        Ok(())
    }
}

/// Evaluate slash evidence as a deterministic no-op rejection.
///
/// Phase 4 Round 1 may validate and carry slash evidence as data, but it must
/// not automatically slash, reserve, burn, capture, release, transfer, bridge,
/// or settle ROC.
pub fn evaluate_slash_evidence_noop(
    evidence: &QuickChainSlashEvidenceV1,
) -> Result<QuickChainBondLifecycleDecisionV1, QuickChainBondLedgerError> {
    evidence
        .validate()
        .map_err(|error| QuickChainBondLedgerError::InvalidSlashEvidence(error.to_string()))?;

    let decision = QuickChainBondLifecycleDecisionV1 {
        schema: QUICKCHAIN_BOND_LIFECYCLE_DECISION_SCHEMA.to_owned(),
        version: QUICKCHAIN_DTO_VERSION,
        chain_id: evidence.chain_id.clone(),
        epoch_id: evidence.epoch_id.clone(),
        validator_id: evidence.validator_id.clone(),
        bond_account_id: "bond:evidence-only".to_owned(),
        operation: QuickChainBondLifecycleOperationV1::EvaluateSlashEvidenceNoop,
        status: QuickChainBondLifecycleDecisionStatusV1::Rejected,
        rejection_code: Some(QuickChainBondLifecycleRejectionCodeV1::AutomaticSlashingForbidden),
        amount_minor: evidence.recommended_amount_minor.clone(),
    };

    decision
        .validate()
        .map_err(|error| QuickChainBondLedgerError::InvalidSlashEvidence(error.to_string()))?;

    Ok(decision)
}

fn required_amount_minor(
    intent: &QuickChainValidatorBondIntentV1,
) -> Result<u128, QuickChainBondLedgerError> {
    let value = intent.amount_minor.as_deref().ok_or_else(|| {
        QuickChainBondLedgerError::InvalidBondIntent("amount is required".to_owned())
    })?;

    value
        .parse::<u128>()
        .map_err(|error| QuickChainBondLedgerError::InvalidBondIntent(error.to_string()))
}

fn ensure_owner_available(
    owner_available_minor: u128,
    required_minor: u128,
) -> Result<(), QuickChainBondLedgerError> {
    if owner_available_minor < required_minor {
        return Err(QuickChainBondLedgerError::InsufficientOwnerAvailable {
            available_minor: owner_available_minor,
            required_minor,
        });
    }

    Ok(())
}
