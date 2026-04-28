//! RO:WHAT — Wallet issue client seam for turning reward settlements into svc-wallet issue requests.
//! RO:WHY — Pillar 12; Concerns: ECON/RES/DX. Rewarder must target wallet as the mutation boundary.
//! RO:INTERACTS — outputs::intents, http handlers, future svc-wallet HTTP adapter.
//! RO:INVARIANTS — rewarder never mutates ledger directly; dry-run emits nothing; idempotency stays run-key based.
//! RO:METRICS — handlers count wallet/ledger intent outcomes.
//! RO:CONFIG — wallet issue path comes from Config.ingress.wallet_issue_path.
//! RO:SECURITY — future egress macaroon scope is config-driven; this stub has no network side effects.
//! RO:TEST — tests/unit/wallet_client.rs and integration/http_compute.rs.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::outputs::intents::{
    IntentResult, IntentStore, SettlementBatch, WalletIssueBatch, WALLET_ISSUE_PATH,
};
use crate::Result;

/// Result returned by a wallet issue client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WalletIssueOutcome {
    /// Egress result.
    pub result: IntentResult,
    /// Wallet-compatible issue batch.
    pub batch: WalletIssueBatch,
}

/// Trait seam for future svc-wallet HTTP issue client.
///
/// Batch 4 keeps this synchronous and side-effect-free except for the dev in-memory
/// idempotency store. A later network client can implement the same trait shape or
/// graduate this to async once real HTTP dispatch is introduced.
pub trait WalletIssueClient: Send + Sync {
    /// Preview wallet issue requests without emitting anything.
    fn preview_issue_batch(&self, settlement: &SettlementBatch) -> Result<WalletIssueBatch>;

    /// Emit a wallet issue batch through the client.
    fn emit_issue_batch(
        &self,
        settlement: &SettlementBatch,
        dry_run: bool,
    ) -> Result<WalletIssueOutcome>;
}

/// Development wallet issue client.
///
/// It does not call the network. It converts settlement batches into wallet request DTOs and
/// uses the same in-memory idempotency store already proven by earlier batches.
#[derive(Debug, Clone)]
pub struct DevWalletIssueClient {
    store: Arc<IntentStore>,
    issue_path: String,
}

impl DevWalletIssueClient {
    /// Build a dev client from the shared intent store and configured wallet path.
    #[must_use]
    pub fn new(store: Arc<IntentStore>, issue_path: impl Into<String>) -> Self {
        Self {
            store,
            issue_path: normalize_issue_path(issue_path.into()),
        }
    }
}

impl WalletIssueClient for DevWalletIssueClient {
    fn preview_issue_batch(&self, settlement: &SettlementBatch) -> Result<WalletIssueBatch> {
        let mut batch = settlement.to_wallet_issue_batch();
        batch.wallet_path = self.issue_path.clone();
        Ok(batch)
    }

    fn emit_issue_batch(
        &self,
        settlement: &SettlementBatch,
        dry_run: bool,
    ) -> Result<WalletIssueOutcome> {
        let result = self.store.emit_batch_once(settlement, dry_run);
        let batch = self.preview_issue_batch(settlement)?;
        Ok(WalletIssueOutcome { result, batch })
    }
}

fn normalize_issue_path(path: String) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        WALLET_ISSUE_PATH.into()
    } else {
        trimmed.into()
    }
}
