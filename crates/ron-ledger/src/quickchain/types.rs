//! RO:WHAT — Non-wire value types for QuickChain replay classification and committed-operation evidence.
//! RO:WHY — ECON/DX: preserve frozen ron-proto intents while keeping ledger-assigned results separate from client input.
//! RO:INTERACTS — replay_index.rs, error.rs, ron_proto::quickchain::QuickChainOperationIntentV1.
//! RO:INVARIANTS — no serde; no receipt fabrication; operation intents keep account_sequence absent; committed evidence carries positive sequences.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — these types carry public identifiers only and grant no mutation authority.
//! RO:TEST — tests/quickchain_replay_index.rs.

use ron_proto::quickchain::{QuickChainOperationClassV1, QuickChainOperationIntentV1};

use super::error::QuickChainReplayError;

const MAX_RECEIPT_TXID_BYTES: usize = 256;

/// Stable operation-family vocabulary used to scope idempotency keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub(crate) enum QuickChainOperationFamily {
    /// ROC supply issue.
    Issue,

    /// Account-to-account transfer.
    Transfer,

    /// ROC supply burn.
    Burn,

    /// Open a hold lifecycle.
    HoldOpen,

    /// Capture one hold lifecycle.
    HoldCapture,

    /// Release one hold lifecycle.
    HoldRelease,

    /// Expire one hold lifecycle.
    HoldExpire,
}

impl QuickChainOperationFamily {
    pub(crate) fn try_from_class(
        op_class: QuickChainOperationClassV1,
    ) -> Result<Self, QuickChainReplayError> {
        match op_class {
            QuickChainOperationClassV1::Issue => Ok(Self::Issue),
            QuickChainOperationClassV1::Transfer => Ok(Self::Transfer),
            QuickChainOperationClassV1::Burn => Ok(Self::Burn),
            QuickChainOperationClassV1::HoldOpen => Ok(Self::HoldOpen),
            QuickChainOperationClassV1::HoldCapture => Ok(Self::HoldCapture),
            QuickChainOperationClassV1::HoldRelease => Ok(Self::HoldRelease),
            QuickChainOperationClassV1::HoldExpire => Ok(Self::HoldExpire),
            _ => Err(QuickChainReplayError::UnsupportedOperationClass),
        }
    }
}

/// Backend-only evidence for one operation that has already committed economically.
///
/// This is not a wallet receipt and is not serializable. Future live integration
/// must construct it only after the atomic wallet/ledger transition succeeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickChainCommittedOperationRecord {
    intent: QuickChainOperationIntentV1,
    receipt_txid: String,
    account_sequence: u64,
    ledger_sequence_start: u64,
    ledger_sequence_end: u64,
}

impl QuickChainCommittedOperationRecord {
    /// Build and validate committed replay evidence.
    pub fn new(
        intent: QuickChainOperationIntentV1,
        receipt_txid: impl Into<String>,
        account_sequence: u64,
        ledger_sequence_start: u64,
        ledger_sequence_end: u64,
    ) -> Result<Self, QuickChainReplayError> {
        if intent.account_sequence.is_some() {
            return Err(QuickChainReplayError::ClientAssignedAccountSequence);
        }

        intent
            .validate()
            .map_err(|error| QuickChainReplayError::InvalidIntent(error.to_string()))?;

        let receipt_txid = receipt_txid.into();
        if !valid_token(&receipt_txid, MAX_RECEIPT_TXID_BYTES) {
            return Err(QuickChainReplayError::InvalidReceiptTxid);
        }

        if account_sequence == 0 {
            return Err(QuickChainReplayError::InvalidCommittedAccountSequence);
        }

        if ledger_sequence_start == 0
            || ledger_sequence_end == 0
            || ledger_sequence_start > ledger_sequence_end
        {
            return Err(QuickChainReplayError::InvalidLedgerSequenceRange);
        }

        Ok(Self {
            intent,
            receipt_txid,
            account_sequence,
            ledger_sequence_start,
            ledger_sequence_end,
        })
    }

    /// Original validated operation intent.
    #[must_use]
    pub const fn intent(&self) -> &QuickChainOperationIntentV1 {
        &self.intent
    }

    /// Original backend receipt transaction reference.
    #[must_use]
    pub fn receipt_txid(&self) -> &str {
        &self.receipt_txid
    }

    /// Ledger-assigned sequence for the primary actor account.
    #[must_use]
    pub const fn account_sequence(&self) -> u64 {
        self.account_sequence
    }

    /// First primitive ledger sequence committed by this operation.
    #[must_use]
    pub const fn ledger_sequence_start(&self) -> u64 {
        self.ledger_sequence_start
    }

    /// Last primitive ledger sequence committed by this operation.
    #[must_use]
    pub const fn ledger_sequence_end(&self) -> u64 {
        self.ledger_sequence_end
    }
}

/// Classification of a validated submission against accepted replay history.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuickChainSubmissionDecision {
    /// No accepted operation occupies this durable identity or idempotency scope.
    Fresh,

    /// The exact accepted intent was retried; return the original evidence.
    ///
    /// The record is boxed so this enum remains compact when the overwhelmingly
    /// common `Fresh` variant carries no committed evidence.
    ReturnOriginal(Box<QuickChainCommittedOperationRecord>),
}

fn valid_token(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b':' | b'@' | b'/')
        })
}
