>Updated after Gemini & Grok critique - June 9 2026 - 22:55 cst

# QUICKCHAIN.MD — ROC Verifiable Settlement Spine / Future Checkpoint-Chain Blueprint

RO:WHAT — Defines the future ROC verifiable settlement spine, deterministic checkpoint-root system, and possible later distributed ledger / checkpoint-chain design after the internal value plane is proven.

RO:WHY — ROC needs a serious high-trust, fast, efficient, replayable, verifiable settlement layer without putting every site visit directly on a global chain, without treating fakeable Web2 engagement as money, and without prematurely committing to a proprietary public L1.

RO:INTERACTS — svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, svc-registry, svc-passport, ron-auth, svc-storage, svc-gateway, omnigate, svc-index, svc-dht, svc-edge, CrabLink, future validator committee, future carrier/archive layer, possible future external DA/L2/ROX/Solana anchor.

RO:INVARIANTS — ROC remains internal until proven; wallet remains mutation front-door; ledger remains truth; accounting is not balance truth; rewarder does not mutate ledger; integer minor units only; no fake receipts; no silent spend; bot-prone engagement metrics do not directly mint or allocate protocol ROC; Phase 1/2 must be validator-executable deterministic state-machine work, not a normal centralized database; QC-0A preflight artifacts are required before Phase 1 code; no pruning before DA/challenge/archive fallback is proven; no mandatory external settlement path during internal ROC MVP.

RO:METRICS — Future checkpoint, epoch, root, validator, carrier, archive, receipt, proof, reward, challenge, pruning, and anchor metrics.

RO:CONFIG — Future chain params, epoch duration, execution-spec version, validator set, carrier retention, checkpoint cadence, challenge windows, external anchor cadence, pruning horizons, DA mode, and settlement-finality thresholds.

RO:SECURITY — Feature-gated until internal ROC is proven; no external settlement during current WEB3/NEXT_LEVEL MVP; no privacy claims without real protocol; no public bridge until audited; no anonymous alt validators by default; no raw engagement reward loop without anti-abuse proof; no root-producing code without canonicalization and golden vectors.

RO:TEST — Future deterministic replay tests, canonical-state-transition tests, canonicalization vectors, state-root tests, receipt inclusion proofs, epoch sealing tests, validator quorum tests, anti-double-commit tests, nonce-hole/concurrent-hold tests, pruning tests, DA-missing challenge tests, carrier proof tests, external anchor verification tests, fork/challenge tests.

---

# 0. Status

`QUICKCHAIN.MD` is a **future settlement blueprint**.

It is **not** an instruction to implement blockchain logic immediately.

It is also **not** a commitment that RustyOnions must build a proprietary public Layer 1.

The first real QuickChain work, when allowed, should be:

```text
deterministic ROC receipt roots
deterministic account state roots
deterministic hold roots
deterministic epoch roots
replayable settlement artifacts
proof formats
strict DTOs
canonicalization rules
golden test vectors
```

not:

```text
public validators
public staking
public bridges
public DA markets
public liquidity
external settlement
custom BFT networking
```

Current active project order remains:

```text
1. Finish internal ROC closed loop.
2. Prove paid storage/pinning.
3. Prove paid image/site/post/comment/article flows.
4. Prove Visitor B pays Creator A for site visit.
5. Prove wallet receipts, ledger replay, and balance truth.
6. Harden ron-accounting → svc-rewarder → svc-wallet reward loop.
7. Complete QC-0A preflight artifacts.
8. Only then begin QuickChain Phase 1 research/prototype.
```

Current forbidden scope still applies:

```text
- No ROX in active ROC MVP.
- No Solana in active ROC MVP.
- No external settlement in active ROC MVP.
- No staking/liquidity/exchange-facing logic.
- No public bridge.
- No chain logic inside CrabLink.
- No gateway-side economic mutation.
- No omnigate direct ledger mutation.
- No protocol ROC payouts from raw fakeable engagement metrics.
- No pruning before DA/challenge/archive fallback is proven.
- No Phase 1 code before QC-0A preflight artifacts exist.
```

QuickChain is the **future ROC settlement spine** after RustyOnions proves the internal creator economy.

Possible future decentralization paths remain open:

```text
1. internal replicated checkpoint committee
2. external L2/rollup framework
3. external DA layer
4. anchor-only model
5. hybrid model
```

No future decentralization path is chosen until the internal ROC economy, deterministic replay, epoch roots, proof model, and QC-0A preflight artifacts are proven.

---

## 0.1 QC-0A preflight requirement

Before Phase 1 code begins, QuickChain requires a preflight documentation and test-vector package called **QC-0A**.

QC-0A exists because QuickChain’s hardest early risks are not validators or networking. The hardest risks are:

```text
canonicalization drift
root non-determinism
concurrent wallet operation ambiguity
event-classification mistakes
fake engagement sneaking into rewards
state bloat before DA exists
future Phase 2 → Phase 3 rewrite risk
```

QC-0A must produce the following documents before implementation:

```text
docs/quickchain/THREAT_MODEL.md
docs/quickchain/ROOTS_AND_CANONICALIZATION.md
docs/quickchain/STATE_TREE_DECISION.md
docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md
docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md
docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md
docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md
docs/quickchain/TEST_VECTORS.md
```

These documents are not optional research notes. They are preconditions.

Phase 1 may not begin until QC-0A answers:

```text
1. What canonical encoding is used for receipts, accounts, holds, windows, manifests, and checkpoints?
2. What exact bytes are hashed for each root?
3. What state tree is used?
4. How are concurrent holds represented without nonce holes or state bloat?
5. How does replay handle operation_id, idempotency_key, hold_id, and account_sequence?
6. Which event classes may influence balances, rewards, analytics, and fraud scoring?
7. Which reward formulas are forbidden?
8. What raw data is retained before DA exists?
9. What exact conditions block pruning?
10. How can a future validator replay Phase 1/2 artifacts without trusting the original operator?
```

QC-0A acceptance rule:

```text
If an independent verifier cannot reproduce the same receipt_hash,
state_root, receipt_root, accounting_root, reward_root, and checkpoint_hash
from the published test vectors, QuickChain Phase 1 must not start.
```

---

# 1. One-sentence thesis

QuickChain is a **high-trust, fast-finality, receipt-first ROC settlement spine**:

```text
hot actions produce backend-derived signed receipts immediately;
epochs settle netted deltas into deterministic verifiable state roots;
history is pruned only through signed checkpoint proofs after DA/challenge/archive safety is proven;
ROX/Solana or external L2/DA may later anchor compact checkpoint commitments, not raw ROC history.
```

Short form:

```text
Receipt-first.
Epoch-settled.
State-root verified.
Validator-executable.
Bot-aware.
Bounded before DA.
Chunk-carried later.
Passport-trusted.
Prunable only by proof.
Externally anchorable later.
Not guaranteed to become a proprietary L1.
```

---

# 2. Why QuickChain exists

RustyOnions has a different economic shape than generic blockchains.

ROC must support:

```text
paid site visits
paid content views
image access
article access
comment/post interactions
music plays
video access
storage byte-days
pinning
provider rewards
creator payouts
curator splits
ad/sponsor settlement
moderation/reputation effects
future validator rewards
future carrier/archive rewards
```

A naive blockchain design would do:

```text
every site visit = one global on-chain transaction
```

That is not viable.

A dangerous fake-economy design would do:

```text
every claimed site visit / watch second / ad impression = automatic ROC reward
```

That is also not viable, because fakeable Web2 engagement metrics can be botted.

QuickChain instead does:

```text
payer-authorized actions = wallet receipts / ledger truth
metering events = accounting snapshots, not direct balance truth
reward-eligible proofs = delayed, policy-gated, challengeable reward inputs
many receipts/proofs = accounting window
accounting window = Merkleized snapshot
snapshot + wallet deltas = epoch transition
epoch transition = deterministic checkpoint root
future validators may sign checkpoint
checkpoint = verifiable settlement state
optional external anchor = compact proof commitment
```

This lets RustyOnions be fast without becoming fake, centralized forever, or unverifiable.

---

# 3. Design goals

## 3.1 High trust

High trust means:

```text
cryptographically verifiable
wallet/ledger replayable
deterministic state transitions
passport/reputation aware where appropriate
validator accountable if validators exist
fraud-resistant
governance-gated during beta
challengeable before pruning
```

High trust does **not** mean:

```text
trust one server forever
trust CrabLink balances
trust gateway balances
trust unaudited validator claims
trust unprovable reward claims
trust raw engagement metrics as money
trust pruning before DA is proven
```

## 3.2 Blazing fast

Fast UX requires that common actions do not wait for slow global finality.

Examples:

```text
open paid site
view image
read article
play song
watch video
load app page
```

These should use:

```text
quote
hold/session budget
access ticket
backend wallet receipt
ledger-accepted receipt
later epoch settlement
later finality/anchor status
```

## 3.3 Efficient

Efficiency comes from:

```text
batching
netting
Merkle roots
state roots
receipt roots
accounting roots
reward roots
checkpoint roots
bounded pre-DA volume
future prunable checkpoint history
```

QuickChain should never require all nodes to carry all historical raw site visits forever.

Before DA/pruning is proven, QuickChain must not ingest unbounded raw web-scale event firehoses into settlement history.

## 3.4 Verifiable

Every balance and receipt must eventually be provable through:

```text
account proof
receipt inclusion proof
epoch transition proof
state root proof
checkpoint proof
future validator signature proof
future data availability proof
optional external anchor proof
```

Garbage collection is allowed only when proof roots remain and DA/challenge/archive fallback has been proven.

## 3.5 Validator-executable from the beginning

Phase 1/2 must not be built as a normal centralized Web2 database that later hopes to become a chain.

Phase 1/2 must be written as a deterministic execution specification:

```text
canonical input receipts
+ canonical accounting windows
+ canonical reward manifests
+ prior state root
+ policy hash
+ chain params hash
→ pure deterministic transition
→ new state root
→ receipt root
→ accounting root
→ reward root
→ checkpoint hash
```

Forbidden inside root-producing transition logic:

```text
wall-clock dependency
local DB iteration order
scheduler order
thread race order
floating point
randomness
hidden service-local side effects
shared-memory assumptions
network timing assumptions
```

Required:

```text
same inputs produce same roots
same replay produces same balances
future validator can replay without trusting original operator
```

## 3.5.1 Determinism is a release gate, not an implementation detail

The deterministic transition promise is the core of QuickChain.

The following must be treated as consensus-breaking hazards even before consensus exists:

```text
map iteration order
database scan order
wall-clock reads inside transition logic
local timezone behavior
locale-sensitive sorting
floating point
random IDs inside root-producing code
unordered JSON serialization
implicit default fields
unknown fields
thread scheduling
retry timing
network timing
service-local mutable caches
```

Root-producing code must use:

```text
canonical encoding
explicit field order
explicit default handling
strict unknown-field rejection
stable sorting
integer arithmetic
declared hash domains
versioned schemas
golden vectors
cross-platform replay tests
```

Every root must define its hash domain:

```text
receipt_hash_domain = "quickchain.receipt.v1"
account_leaf_hash_domain = "quickchain.account-state.v1"
hold_leaf_hash_domain = "quickchain.hold-state.v1"
receipt_root_domain = "quickchain.receipt-root.v1"
state_root_domain = "quickchain.state-root.v1"
accounting_root_domain = "quickchain.accounting-root.v1"
reward_root_domain = "quickchain.reward-root.v1"
checkpoint_hash_domain = "quickchain.checkpoint.v1"
```

Domain separation is mandatory so the same byte sequence cannot be confused across receipt, state, accounting, reward, or checkpoint contexts.

## 3.6 Anti-oracle / anti-bot posture

QuickChain must not confuse recorded events with truthful real-world events.

A Merkle root proves that an event was recorded.

It does not prove that:

```text
a human watched the video
an ad impression was valuable
a site visit was non-botted
a creator earned protocol subsidy
a storage proof was available to users
```

Therefore:

```text
bot-prone engagement metrics must never be a primary protocol payout denominator
```

Engagement metrics may inform:

```text
analytics
creator dashboards
fraud scoring
advertiser-funded settlement
policy scoring
capped experiments
```

but must not directly mint or allocate protocol ROC.

---

# 4. Relationship to current ROC architecture

QuickChain must evolve the existing architecture, not replace it.

Current and future boundaries:

```text
svc-wallet:
  normal mutation front-door
  issue, transfer, burn, hold, capture, release
  nonce/idempotency/operation uniqueness
  wallet receipts
  account balances
  concurrent holds/session budgets
  no fake receipts

ron-ledger:
  durable economic truth
  append-only/replayable log
  deterministic balance state
  future state-root engine
  future receipt-root engine
  future epoch transition engine

ron-accounting:
  usage events
  metering
  sealed snapshots
  reward-compatible exports
  not balance truth
  not direct money mutation

svc-rewarder:
  deterministic reward planning
  payout manifests
  wallet issue/transfer intents
  no direct ledger mutation
  no raw engagement auto-payouts

ron-policy:
  side-effect-free pricing/payout/rule authority
  basis points
  integer minor units
  feature gates
  reward eligibility rules
  anti-abuse gates

svc-gateway:
  public route surface
  quotas/admission
  proxy only for product routes
  no wallet/ledger mutation

omnigate:
  product coordinator / hydrator
  quote/access coordinator
  no durable economic truth
  no direct ledger mutation

CrabLink:
  Tauri-first primary client
  Chrome proof/companion
  gateway-compatible
  explicit user confirmation
  no private-key custody unless future vault gates are passed
  no fake receipts or balances
  no local entitlement truth
```

QuickChain adds, later:

```text
checkpoint DTOs
receipt roots
state roots
account proofs
epoch artifacts
future validator committee
future carrier/archive chunks
future pruning rules
possible external DA/L2/anchor path
```

It does **not** move economic mutation into gateway, omnigate, CrabLink, accounting, rewarder, storage, index, or carrier nodes.

---

# 5. Core architecture

QuickChain has two speeds, but both must be honest about finality.

## 5.1 Speed 1 — Hot user path

Used for immediate UX.

```text
CrabLink
→ svc-gateway
→ omnigate quote/access route
→ svc-wallet hold/transfer/access ticket
→ ron-ledger accepted receipt
→ CrabLink unlocks only after backend wallet/ledger truth
```

Examples:

```text
Visitor B opens crab://ron7.
Visitor B confirms 10 ROC site_visit.
svc-wallet commits accepted receipt through ron-ledger.
CrabLink unlocks site.
Creator A sees backend-derived balance/receipt status.
```

This is fast and user-visible.

It is not frontend credit.

CrabLink must never unlock paid content merely because the UI believes payment happened.

## 5.2 Speed 2 — Final settlement path

Used for epoch settlement, proof, distributed verification, and future pruning.

```text
wallet receipts
+ accounting windows
+ reward manifests
+ policy hash
+ chain params hash
→ deterministic epoch transition
→ state root
→ receipt root
→ accounting root
→ reward root
→ checkpoint hash
→ future validator signatures
→ future checkpoint block
```

This is the blockchain-grade settlement layer.

## 5.3 Receipt settlement states

Receipts must expose status clearly.

Suggested status levels:

```text
accepted:
  svc-wallet/ron-ledger accepted the mutation.
  sufficient for local paid unlock.

epoch_included:
  receipt is included in an epoch receipt root.

finalized:
  challenge window expired or finality threshold reached.

anchored:
  optional external anchor proves checkpoint commitment existed externally.
```

UX rules:

```text
paid content unlock:
  accepted backend receipt is enough.

creator visible pending earnings:
  accepted or epoch_included may be shown with status.

creator withdrawable/finalized rewards:
  require epoch_included/finalized according to policy.

external bridge/export/anchor claims:
  require finalized or anchored.

CrabLink display:
  must label status honestly.
```

---

# 6. Main data flow

```text
1. User action occurs
   paid_site_visit / content_view / storage_put / provider_serve / ad_view

2. svc-wallet produces receipt or hold/capture/release
   receipt has txid, account, amount, op, operation_id, nonce/sequence data,
   idempotency key, receipt hash, and accepted status

3. ron-accounting records usage/metering event when appropriate
   normalized labels, bounded fields, integer counters

4. accounting window seals
   canonical snapshot bytes
   b3:<snapshot_hash>
   accounting_snapshot_root

5. svc-rewarder computes deterministic eligible rewards
   reward_manifest_root
   settlement intents
   anti-abuse/policy gates included

6. svc-wallet commits payout/issue/transfer
   wallet receipts generated
   no direct rewarder ledger mutation

7. ron-ledger produces epoch transition
   previous_state_root → new_state_root

8. future validators may sign checkpoint
   quorum signatures over compact deterministic checkpoint header

9. future carrier/archive nodes store chunks
   receipt batches, accounting snapshots, reward manifests, ledger chunks

10. optional future external anchor
   compact checkpoint commitment only
```

---

# 7. QuickChain block/checkpoint model

A future checkpoint block should be compact.

```text
QuickChainCheckpointV1 {
  schema: "quickchain.checkpoint.v1"

  chain_id: "roc-main" | "roc-dev" | "roc-test"
  height: u64
  epoch_id: string
  execution_spec_version: string

  previous_checkpoint_hash: b3:<64hex>
  previous_state_root: b3:<64hex>
  new_state_root: b3:<64hex>

  receipt_root: b3:<64hex>
  accounting_snapshot_root: b3:<64hex>
  reward_manifest_root: b3:<64hex>
  data_availability_root: b3:<64hex> | null

  policy_hash: b3:<64hex>
  validator_set_hash: b3:<64hex> | null
  chain_params_hash: b3:<64hex>

  epoch_started_at_ms: u64
  epoch_ended_at_ms: u64
  produced_at_ms: u64

  supply_delta: {
    issued_minor: string
    burned_minor: string
    net_minor: string
  }

  conservation: {
    debits_minor: string
    credits_minor: string
    issue_exceptions_minor: string
    burn_exceptions_minor: string
    valid: bool
  }

  settlement_mode: "local_root" | "committee" | "external_da" | "external_l2" | "anchor_only" | "hybrid"

  signatures: Vec<ValidatorSignatureV1>
}
```

Checkpoint hash:

```text
checkpoint_hash = b3:<hash(canonical_checkpoint_bytes)>
```

Rules:

```text
- All money values are integer strings.
- No floating point.
- All hashes are b3:<64 lowercase hex>.
- Canonical JSON or canonical binary encoding must be specified.
- Same inputs must produce same checkpoint hash.
- Checkpoint root generation must be validator-executable.
- data_availability_root may be null only in early local-root phases where pruning is forbidden.
```

---

# 8. Account state model

QuickChain should use an account-based state model, not UTXO.

Reason:

```text
RustyOnions already has passports, wallet accounts, balances, holds, receipts, and creator payouts.
```

Proposed state:

```text
AccountStateV1 {
  account_id: string
  asset: "roc"

  balance_minor: string
  held_minor: string
  available_minor: string

  account_sequence: u64
  receipt_root: b3:<64hex>
  holds_root: b3:<64hex>
  permissions_root: b3:<64hex>

  pending_root: b3:<64hex> | null
  updated_at_epoch: string
}
```

Global state root:

```text
state_root = MerkleRoot(sorted(AccountStateV1 canonical bytes))
```

A light client should be able to verify:

```text
account_id acct_visitor_b
balance 1766 ROC
included in state_root X
checkpoint X signed by validator quorum if committee mode exists
optional anchor proves checkpoint X existed at external time Y
```

## 8.1 Concurrency rule

QuickChain must avoid the naive strict-single-nonce UX trap.

A single stuck request must not freeze all unrelated paid actions in CrabLink.

Preferred replay protection:

```text
durable operation_id
+ account_id
+ idempotency_key
+ hold_id/session_budget_id where applicable
+ canonical operation hash
+ ledger sequence bounds
```

Rules:

```text
- Client idempotency keys are hints, not authority.
- Server-side durable uniqueness is authority.
- Retries return the same receipt, not a second mutation.
- Concurrent holds/session budgets should support multi-tab paid UX.
- Strict sequential account nonce may exist internally, but must not create user-visible nonce holes.
- Ledger replay must still be deterministic.
```

## 8.2 Concurrent hold state must be root-safe

Concurrent paid actions must not rely on a fragile single global account nonce for UX ordering.

However, concurrency must also not create unbounded state bloat.

The future state model must define a bounded, replayable hold structure:

```text
HoldStateV1 {
  hold_id: string
  account_id: string
  counterparty_account_id: string | null
  amount_minor: string
  purpose: string
  created_at_ms: u64
  expires_at_ms: u64
  status: "open" | "captured" | "released" | "expired"
  operation_id: string
  idempotency_key: string
  policy_hash: b3:<64hex>
}
```

Rules:

```text
- Open holds are included in holds_root.
- Captured/released/expired holds are removed or compacted only through deterministic transition rules.
- Hold expiration must be based on epoch/ledger rules, not nondeterministic local timers.
- Multiple open holds per account are allowed within policy limits.
- Account balance math must distinguish balance_minor, held_minor, and available_minor.
- Capturing a hold must be idempotent.
- Releasing a hold must be idempotent.
- Retrying a hold/capture/release must return the original receipt when applicable.
```

The state root must commit to enough hold information to replay captures and releases deterministically without requiring all historical UI/session data.

---

# 9. Receipt model

Every economic mutation must have a receipt.

Examples:

```text
issue
transfer
burn
hold
capture
release
reward_issue
paid_site_visit
content_view
storage_put
storage_pin
ad_payout
carrier_reward
validator_reward
archive_reward
```

Receipt shape:

```text
RocReceiptV1 {
  schema: "quickchain.receipt.v1"

  txid: string
  operation_id: string
  op: string
  op_class: "payer_authorized" | "reward_commit" | "admin_issue" | "burn" | "hold" | "release"

  from: string | null
  to: string | null
  asset: "roc"
  amount_minor: string

  account_sequence: u64 | null
  hold_id: string | null
  session_budget_id: string | null
  idempotency_key: string
  operation_hash: b3:<64hex>

  memo: string | null
  produced_at_ms: u64

  ledger_seq_start: u64
  ledger_seq_end: u64
  previous_ledger_root: b3:<64hex>
  new_ledger_root: b3:<64hex>

  settlement_status: "accepted" | "epoch_included" | "finalized" | "anchored"
  receipt_hash: b3:<64hex>
}
```

Receipt hash:

```text
receipt_hash = b3:<hash(canonical_receipt_bytes_without_receipt_hash)>
```

Receipt inclusion proof:

```text
receipt_hash
→ receipt_batch_root
→ epoch_receipt_root
→ checkpoint_hash
→ future validator signatures
→ optional external anchor
```

Rules:

```text
- Every balance mutation must have exactly one durable receipt.
- Duplicate idempotency keys must not produce duplicate commits.
- Duplicate operation IDs must not produce duplicate commits.
- Accepted receipts may unlock paid content.
- Finality-sensitive actions must check settlement_status.
```

---

# 10. Usage/accounting model

High-volume actions should not all be direct global consensus transactions.

Examples of high-volume actions:

```text
site_visit
image_view
post_view
comment_view
article_view
music_play
video_watch_seconds
ad_impression
storage_byte_day
provider_egress_byte
proof_served
```

These become accounting or metering events, not automatic balance truth.

```text
UsageEventV1 {
  schema: "quickchain.usage-event.v1"
  event_id: string
  action: string
  event_class: "economic_receipt" | "metering" | "proof_eligible" | "ad_budgeted" | "analytics_only"

  account_id: string | null
  counterparty_account_id: string | null
  object_cid: b3:<64hex> | null
  site_name: string | null

  amount_minor: string | null
  units: u64
  labels: map<string,string>

  produced_at_ms: u64
  idempotency_key: string
}
```

A sealed accounting window produces:

```text
AccountingWindowV1 {
  schema: "quickchain.accounting-window.v1"
  window_id: string
  started_at_ms: u64
  ended_at_ms: u64
  event_count: u64
  event_root: b3:<64hex>
  aggregate_root: b3:<64hex>
  snapshot_cid: b3:<64hex>
}
```

Rules:

```text
- Open windows may change.
- Sealed windows are immutable.
- Sealed windows must be deterministic.
- Duplicate idempotency keys must not double-record events.
- Accounting never mutates balances directly.
- Bot-prone engagement metrics do not directly mint or allocate protocol ROC.
- Raw watch seconds, impressions, and visits may inform analytics/policy, but not automatic protocol payouts.
- Reward eligibility must be policy-gated, capped, delayed, replayable, and challengeable.
```

## 10.1 Event class rules

```text
economic_receipt:
  payer-authorized wallet event or ledger mutation.
  Can affect balances through svc-wallet/ron-ledger.

metering:
  usage measurement.
  Does not affect balances directly.

proof_eligible:
  storage/provider/carrier/archive proof candidate.
  Requires verification/challenge path before reward planning.

ad_budgeted:
  advertiser/sponsor-funded event.
  Paid from explicit budget, not raw protocol mint.

analytics_only:
  display/reporting signal.
  Never used for ROC reward allocation.
```

## 10.2 Oracle / Sybil rule

QuickChain must assume Web2 engagement can be faked.

Therefore:

```text
site_visit alone is not proof of human attention
video_watch_seconds alone is not proof of human attention
ad_impression alone is not proof of ad value
raw click/view counts alone are not reward entitlement
```

Any reward formula based on engagement must include:

```text
explicit funding source
anti-abuse policy
bounded budget
fraud scoring
delayed payout
challenge/review path
replayable inputs
```

## 10.3 Event classification must be enforceable, not just documented

Event class must be a strict enum in DTOs and policy.

Unsafe pattern:

```text
action = "video_watch_seconds"
rewarder decides later whether it is money
```

Safer pattern:

```text
event_class = "analytics_only" | "metering" | "proof_eligible" | "ad_budgeted" | "economic_receipt"
policy explicitly defines allowed downstream use
```

Rules:

```text
- analytics_only events cannot enter reward manifests.
- metering events cannot mutate balances.
- proof_eligible events require proof verification before reward planning.
- ad_budgeted events require explicit advertiser/sponsor budget.
- economic_receipt events must correspond to wallet/ledger receipts.
- rewarder must reject unknown or ambiguous event classes.
- policy must reject any reward formula using raw engagement as the primary denominator.
```

Forbidden reward basis examples:

```text
total_site_visits share
raw_watch_seconds share
raw_ad_impressions share
raw_click_count share
unverified active user share
```

Allowed or safer basis examples:

```text
payer-authorized purchases
advertiser-funded campaign budget with fraud scoring
storage proofs with challenge verification
provider egress proofs with bounded accounting
governance-approved grants with hard caps
creator-defined payout splits from explicit revenue
```

---

# 11. Reward model

Rewards are planned by `svc-rewarder`, then committed through `svc-wallet`.

Reward types:

```text
provider_reward
storage_reward
carrier_reward
archive_reward
validator_reward
curator_reward
moderation_reward
ad_share_reward
```

Reward manifest:

```text
RewardManifestV1 {
  schema: "quickchain.reward-manifest.v1"
  epoch_id: string
  input_accounting_root: b3:<64hex>
  policy_hash: b3:<64hex>
  reward_root: b3:<64hex>
  payout_count: u64
  total_payout_minor: string
  conservation: object
  generated_by: "svc-rewarder"
}
```

Rules:

```text
- Rewarder does not mutate ledger directly.
- Rewarder creates deterministic payout plans.
- Wallet commits payout/issue/transfer.
- Ledger records durable truth.
- Replaying the same epoch must not double-issue.
- Capped pools alone do not solve botting.
- Bot-prone engagement metrics must not be the primary denominator for protocol ROC rewards.
- Reward plans must identify funding source: protocol, advertiser, creator pool, sponsor, or governance budget.
```

Unsafe reward formula:

```text
daily_pool * user_watch_seconds / total_network_watch_seconds
```

because bots can dilute honest creators.

Safer reward posture:

```text
payer-authorized transfers first
advertiser-budgeted campaigns with fraud controls
storage/provider/carrier proofs with challenges
governance-approved experiments with hard caps
delayed payouts with review/challenge windows
```

---

# 12. Consensus / settlement model

QuickChain should not begin as a fully permissionless public chain.

It should also not pretend that a local deterministic ledger automatically becomes a distributed BFT chain.

The safe model is:

```text
Phase 1/2 define the canonical ROC execution specification and proof artifacts.
Phase 3+ may choose a decentralization path only after Phase 1/2 are proven.
```

Recommended phases:

```text
Phase 0: do not implement yet
Phase 0A: QC-0A preflight artifacts
Phase 1: local deterministic checkpoint roots
Phase 2: local deterministic epoch sealing
Phase 3: optional replicated validator committee
Phase 4: optional passport-gated validator set
Phase 5: optional bonded/slashable validators
Phase 6: optional external DA/L2/anchor integration
Phase 7: optional broader validator market
```

Initial consensus target, if committee mode is chosen later:

```text
BFT-style validator committee or simpler governance-approved beta committee
quorum signatures over checkpoint headers
deterministic state transition verification
fork detection
manual governance override during beta
```

Validator responsibilities:

```text
- verify previous checkpoint
- replay canonical inputs
- verify receipt roots
- verify accounting roots
- verify reward roots
- verify policy hash
- verify chain params hash
- verify conservation/non-negativity
- verify authorized issue/burn exceptions
- verify state transition
- sign checkpoint if valid
```

Validator non-responsibilities:

```text
- do not directly mint self-rewards
- do not bypass svc-wallet
- do not accept unknown money math
- do not validate arbitrary executable code
- do not treat raw engagement as reward truth
```

## 12.1 Phase 2 → Phase 3 chasm rule

The transition from local roots to a replicated committee is a major architectural boundary.

It is not a simple toggle.

Before Phase 3, the project must prove:

```text
root-producing execution is pure and deterministic
all inputs are canonical
all outputs have test vectors
state transition can be replayed outside the original service
no wall-clock or DB-order assumptions affect roots
operator tools can export/import epoch artifacts
validator simulator can independently reproduce checkpoint hash
```

If these are not true, Phase 3 must not begin.

## 12.2 Future decentralization options

Future decentralization may use:

```text
internal committee
existing L2/rollup framework
external DA layer
anchor-only model
hybrid model
```

QuickChain must not lock the project into a proprietary L1 before internal ROC and deterministic replay are proven.

---

# 13. Validator trust model

High-trust validator eligibility should require:

```text
main RON Passport
reputation threshold
uptime history
registered validator identity
capability token
governance admission during beta
future ROC bond if bonding exists
future slashing policy if slashing exists
```

Anonymous alt passports should not validate economic truth.

Allowed for alts:

```text
browse
comment
post if policy allows
hold/pay ROC
receive ROC if configured
```

Not allowed for alts by default:

```text
validator role
governance signing role
economy-wide checkpoint signing
public trust anchor role
```

Rules:

```text
- Validator role is not a normal user privilege.
- Validator admission remains governance-gated during beta.
- Validator set changes must be signed/versioned.
- Validator signatures must verify offline.
```

---

# 14. Carrier/archive model

QuickChain may later let normal users earn ROC by doing measurable useful work.

Node roles:

```text
Light client:
  keeps headers, own account proofs, recent proof cache

Pruned node:
  keeps recent N epochs, current state, assigned chunks

Carrier node:
  stores assigned ledger/receipt/accounting chunks and proves custody

Archive node:
  stores full historical ledger and receipt history

Validator node:
  validates and signs checkpoints

Provider node:
  stores/serves content and earns provider rewards
```

Carrier reward flow:

```text
1. Node receives assigned chunk.
2. Chunk is b3-addressed.
3. Network issues random challenge.
4. Node returns chunk bytes or segment + Merkle proof.
5. Proof enters accounting.
6. Rewarder computes payout.
7. Wallet commits reward.
8. Ledger records receipt.
```

Carrier proof types:

```text
full chunk proof
random segment proof
Merkle inclusion proof
erasure-coded shard proof
availability proof
historical archive proof
```

Rules:

```text
- Carrier nodes do not mint.
- Carrier nodes earn through rewarder/wallet path.
- Missing chunks reduce score.
- Fraudulent proofs are slashable later if slashing exists.
- Carrier/archive rewards must not be active until proof/challenge mechanics are tested.
```

---

# 15. Garbage collection / pruning

QuickChain must be prunable only by proof.

Unsafe garbage collection:

```text
delete old history and trust current balances
```

Safe garbage collection:

```text
raw receipts/events
→ Merkle batches
→ checkpoint roots
→ validator signatures or local root proof
→ challenge window expires
→ DA/archive fallback proven
→ raw history becomes optional/archive/carrier responsibility
→ current state remains provable
```

Core rule:

```text
Raw history may be pruned only after it is committed into signed checkpoint roots,
the challenge window has expired,
and data availability / archive fallback has been proven.
```

Before DA/pruning is proven:

```text
- Archive-first applies only at bounded scale.
- No web-scale event firehose is allowed into required settlement history.
- High-volume raw events must be sampled, aggregated, capped, summarized, or classified as analytics-only.
- Economic receipts take priority over analytics retention.
```

Suggested retention classes:

```text
Hot retention:
  recent receipts/events/blocks required by all full nodes

Warm retention:
  recent N epochs required by pruned nodes

Cold retention:
  older chunks assigned to carriers/archive nodes

Permanent retention:
  full archive nodes

Light retention:
  checkpoint headers, own proofs, trusted validator set changes
```

Normal node keeps:

```text
current state
recent epochs
checkpoint headers
own account proofs
recent receipt proofs
assigned carrier chunks if carrier role enabled
```

Archive node keeps:

```text
everything within configured archive scope
```

## 15.1 Pre-DA retention must be deliberately boring

Before DA is proven, QuickChain must prefer boring retention over clever pruning.

Rules:

```text
- No pruning in local-root mode.
- No pruning in epoch-sealing mode.
- No pruning merely because a checkpoint root exists.
- No pruning merely because a carrier assignment exists.
- No pruning until missing-data challenges work.
- No pruning until archive fallback is tested.
- No pruning until restore-from-proof is tested.
```

Pre-DA scale rule:

```text
Before DA is proven, the system must not promise web-scale raw event retention.
```

High-volume raw events must be handled as one of:

```text
analytics-only local retention
aggregated sealed summaries
sampled metering
bounded experiment data
explicitly funded campaign data
proof-relevant data with retention cap
```

Economic receipts are higher priority than raw engagement telemetry.

If storage pressure appears before DA is proven, the correct response is:

```text
reduce raw event scope
aggregate earlier
sample analytics
shorten non-economic telemetry retention
pause reward experiments
```

not:

```text
unsafe pruning
pretend DA exists
force normal nodes to become enterprise archive servers
```

---

# 16. Data availability

Checkpoint validity depends on data availability if pruning or distributed verification is active.

A checkpoint may include:

```text
receipt_root
accounting_snapshot_root
reward_manifest_root
data_availability_root
```

Data availability root should commit to:

```text
receipt batch chunks
accounting snapshot chunks
reward manifest chunks
ledger transition chunks
state proof chunks
carrier assignment map
erasure coding metadata
```

If data is unavailable during challenge window:

```text
checkpoint may be challenged
validator set may be penalized
carrier rewards may be withheld
archive requirement may be triggered
pruning must be blocked
```

Rules:

```text
- No pruning before DA is proven.
- No carrier reward market before proof/challenge mechanics are proven.
- No assumption that a small carrier network provides sufficient DA.
- Early phases may set data_availability_root = null only when pruning is forbidden.
- External DA remains a valid future option.
```

---

# 17. ROX / Solana / external anchor model

ROX/Solana/external anchors are future/deferred.

When allowed, external chains should not store ROC history directly.

Bad design:

```text
store raw ROC blocks, receipts, and site visits inside Solana transactions
```

Good design:

```text
store compact ROC checkpoint commitments
```

External anchor payload should contain only compact fields such as:

```text
quickchain_version
roc_chain_id
checkpoint_height
epoch_id
checkpoint_hash
previous_checkpoint_hash
state_root
receipt_root
accounting_root
reward_root
data_availability_root
validator_set_hash
policy_hash
produced_at_ms
```

Design rule:

```text
ROC stores the facts.
External anchors prove compact commitments existed.
```

Proof path:

```text
wallet receipt
→ receipt batch root
→ epoch receipt_root
→ QuickChain checkpoint_hash
→ future validator quorum signatures or local-root proof
→ optional external anchor transaction
```

This can give:

```text
fast internal ROC
cheap high-volume paid actions
verifiable settlement roots
future prunable history
external timestamp/finality anchor
future public audit bridge
```

External anchors must remain:

```text
optional
deferred
feature-gated
not required for internal ROC beta
not a hot-path dependency for page loads
not a replacement for wallet/ledger truth
not a commitment to build a public bridge
```

---

# 18. Forks, challenges, and fraud

QuickChain must handle:

```text
invalid state transition
double spend
duplicate idempotency commit
unauthorized issue
unauthorized burn
missing receipt batch
missing accounting snapshot
bad reward calculation
raw engagement reward abuse
validator equivocation
carrier unavailable chunk
archive proof failure
policy hash mismatch
chain params mismatch
```

Challenge object:

```text
QuickChainChallengeV1 {
  schema: "quickchain.challenge.v1"
  checkpoint_hash: b3:<64hex>
  challenger: passport/account/node id
  challenge_type: string
  evidence_cid: b3:<64hex>
  submitted_at_ms: u64
}
```

Challenge types:

```text
invalid_state_root
invalid_receipt_root
missing_data
invalid_reward_root
invalid_policy_hash
invalid_chain_params_hash
unauthorized_issue
unauthorized_burn
double_spend
duplicate_operation_commit
validator_equivocation
carrier_failure
raw_engagement_reward_abuse
```

Beta handling:

```text
manual governance quarantine
checkpoint pause
validator set review
operator alert
reward withholding
no automatic slashing until audited
```

Future handling:

```text
fraud proof
slashing
reward withholding
validator ejection
carrier reputation penalty
```

---

# 19. Chain parameter model

Future config:

```toml
[quickchain]
enabled = false
chain_id = "roc-dev"
epoch_duration = "5m"
checkpoint_cadence = "1"
challenge_window = "24h"
max_receipts_per_batch = 100000
max_accounting_snapshot_bytes = "16MiB"
max_reward_manifest_bytes = "16MiB"
canonical_encoding = "json-v1"
execution_spec_version = "quickchain-exec-v1"
settlement_mode = "local_root"

[quickchain.safety]
web_scale_ingest_enabled = false
raw_engagement_rewards_enabled = false
pruning_enabled = false
data_availability_required = false
carrier_rewards_enabled = false
external_settlement_enabled = false
phase_1_enabled = false
qc_0a_required = true

[quickchain.consensus]
mode = "none"
quorum_bps = 6700
min_validators = 0
max_validators = 0
passport_required = true
bond_required = false

[quickchain.retention]
hot_epochs = 288
warm_epochs = 8640
carrier_assignment = false
archive_required = true
pruning_requires_da = true

[quickchain.external_anchor]
enabled = false
kind = "none"
cadence_epochs = 288
payload = "checkpoint_commitment_v1"
```

Rules:

```text
- QuickChain disabled by default.
- Phase 1 disabled by default.
- Consensus disabled by default.
- Pruning disabled by default.
- Carrier rewards disabled by default.
- Raw engagement rewards disabled by default.
- External anchor disabled by default.
- Any external anchor requires explicit feature gate.
- Production chain params require signed governance.
```

---

# 20. Implementation phases

## Phase 0 — Do not implement yet

Precondition:

```text
internal ROC loop is not fully proven yet
```

Continue current work:

```text
paid site visit
paid content views
wallet receipts
ledger replay
accounting snapshots
rewarder payout loop
CrabLink truth display
```

## Phase 0A — QuickChain preflight artifacts

Goal:

```text
lock the deterministic settlement design before code exists
```

Required outputs:

```text
THREAT_MODEL.md
ROOTS_AND_CANONICALIZATION.md
STATE_TREE_DECISION.md
CONCURRENT_HOLDS_AND_OPERATION_IDS.md
EVENT_CLASSIFICATION_AND_REWARDS.md
RETENTION_AND_DA_PRECONDITIONS.md
PHASE_2_TO_PHASE_3_CHASM.md
TEST_VECTORS.md
```

Acceptance gates:

```text
canonical encoding chosen or explicitly versioned
state tree chosen or narrowed to testable candidates
receipt bytes defined
account state bytes defined
hold state bytes defined
checkpoint bytes defined
hash domains defined
event class rules defined
forbidden reward formulas listed
concurrent hold replay model defined
pre-DA retention limits defined
independent replay test-vector format defined
```

No Rust implementation should begin before Phase 0A is complete.

## Phase 1 — Local deterministic checkpoint roots

Goal:

```text
ron-ledger can produce state_root and receipt_root locally
```

Tasks:

```text
canonical receipt hashing
canonical account state bytes
canonical hold state bytes
account state tree
hold root generation
state root generation
receipt root generation
checkpoint DTO
local replay proves same roots
golden test vectors
validator-executable transition interface
```

No distributed consensus yet.

No pruning yet.

No external anchor yet.

## Phase 2 — Local deterministic epoch sealing

Goal:

```text
ledger/accounting/rewarder create deterministic epoch artifacts
```

Tasks:

```text
epoch_id convention
sealed accounting windows
reward manifest root
netted delta root
checkpoint root
conservation proof
non-negativity proof
policy hash inclusion
chain params hash inclusion
external replay tool can reproduce checkpoint hash
```

No distributed consensus yet.

No pruning yet.

No web-scale raw event firehose.

Phase 2 is not “a centralized database we will decentralize later.”

Phase 2 is the single-authority execution specification that future validators or external verifiers must be able to replay.

## Phase 3 — Optional replicated committee

Goal:

```text
multiple validators independently replay and sign the same checkpoint
```

Precondition:

```text
Phase 1/2 roots are validator-executable and boring.
```

Tasks:

```text
validator identity
validator set hash
checkpoint proposal
artifact export/import
independent replay
signature collection
quorum verification
fork detection
operator tools
```

This phase may be skipped or replaced by an external L2/DA/anchor strategy after research.

## Phase 4 — Carrier/archive chunks

Goal:

```text
nodes carry ledger/proof chunks and may later earn ROC for useful work
```

Tasks:

```text
chunk format
carrier assignment
challenge protocol
proof verification
accounting event for proof success
rewarder integration
archive role
```

No carrier reward market until proof/challenge logic is tested.

## Phase 5 — Pruning

Goal:

```text
normal nodes can prune old raw history safely
```

Precondition:

```text
DA/challenge/archive fallback proven.
```

Tasks:

```text
retention policy
challenge window
checkpoint proof verification
state snapshot restore
receipt proof lookup
archive fallback
DA-missing challenge
carrier chunk recovery
```

## Phase 6 — Optional external DA/L2/anchor

Goal:

```text
optional external system stores or verifies compact checkpoint commitments
```

Possible paths:

```text
external DA layer
L2/rollup framework
anchor-only transaction
hybrid model
```

Tasks:

```text
anchor payload DTO
anchor writer
anchor verifier
proof path docs
governance gate
strict no-hot-path dependency
```

## Phase 7 — Optional open validator market

Goal:

```text
broader validator set with bonding and slashing if chosen
```

Tasks:

```text
validator bond
slashing evidence
validator rewards
validator rotation
governance controls
anti-Sybil policy
formal review
```

---

# 21. Acceptance gates

QuickChain cannot be considered real until these are true.

## 21.0 Preflight correctness

```text
THREAT_MODEL.md exists
ROOTS_AND_CANONICALIZATION.md exists
STATE_TREE_DECISION.md exists
CONCURRENT_HOLDS_AND_OPERATION_IDS.md exists
EVENT_CLASSIFICATION_AND_REWARDS.md exists
RETENTION_AND_DA_PRECONDITIONS.md exists
PHASE_2_TO_PHASE_3_CHASM.md exists
TEST_VECTORS.md exists
canonical encoding is specified
hash domains are specified
state tree decision is specified or explicitly staged
concurrent hold model is specified
event class enforcement rules are specified
forbidden reward formulas are specified
pre-DA retention limits are specified
independent verifier can replay golden vectors
```

## 21.1 Local correctness

```text
ledger replay produces identical balances
ledger replay produces identical state roots
receipt replay produces identical receipt roots
idempotency prevents duplicate commits
operation_id uniqueness prevents duplicate commits
concurrent holds do not create nonce holes
nonce/sequence rules prevent replay
conservation holds except authorized issue/burn
```

## 21.2 Epoch correctness

```text
same input receipts → same checkpoint hash
same accounting snapshot → same accounting root
same reward input → same reward root
net deltas match account state transition
policy hash is included
chain params hash is included
execution_spec_version is included
external replay tool reproduces roots
```

## 21.3 Anti-oracle / reward correctness

```text
raw engagement metrics do not directly mint ROC
raw engagement metrics are not primary reward denominator
reward manifests identify funding source
reward formulas are deterministic
reward inputs are replayable
fraud/challenge status can withhold payout
duplicate reward epochs do not double issue
```

## 21.4 Consensus correctness

```text
validator quorum signs valid checkpoint
invalid checkpoint rejected
fork/equivocation detected
validator set changes are signed/versioned
checkpoint signatures verify offline
validators independently reproduce checkpoint hash
```

## 21.5 Pruning correctness

```text
old raw events are pruned only after challenge window
DA/challenge/archive fallback is proven
account proof still verifies
receipt proof still verifies
state snapshot restore works
archive fallback works
carrier chunks recover proof data
missing DA blocks pruning
```

## 21.6 External anchor correctness

```text
anchor contains compact checkpoint commitment only
anchor can be verified offline
anchor proof links checkpoint hash to external transaction
ROC can operate if anchor is unavailable
anchor is not required for site load
anchor does not become public bridge by accident
```

---

# 22. Testing plan

QC-0A preflight tests:

```text
quickchain-preflight:
  canonical_receipt_vector_001
  canonical_account_state_vector_001
  canonical_hold_state_vector_001
  receipt_hash_domain_separation
  state_root_determinism_vector_001
  receipt_root_determinism_vector_001
  checkpoint_hash_vector_001
  unordered_input_same_root
  unknown_field_rejected
  float_money_rejected
  raw_engagement_reward_formula_rejected
  concurrent_holds_replay_vector_001
  expired_hold_epoch_transition_vector_001
  duplicate_operation_id_rejected
  duplicate_idempotency_returns_original_receipt
```

Future tests:

```text
ron-ledger:
  state_root_determinism.rs
  receipt_root_determinism.rs
  epoch_transition_replay.rs
  idempotency_epoch.rs
  operation_id_uniqueness.rs
  concurrent_holds_no_nonce_hole.rs

ron-accounting:
  sealed_window_root.rs
  usage_event_batch_root.rs
  duplicate_ingest_no_double_count.rs
  raw_engagement_not_balance_truth.rs

svc-rewarder:
  reward_manifest_root.rs
  reward_epoch_replay_no_double_issue.rs
  carrier_reward_plan.rs
  raw_engagement_pool_rejected.rs
  reward_funding_source_required.rs

quickchain:
  checkpoint_dto_strict.rs
  checkpoint_hash_vectors.rs
  execution_spec_replay.rs
  validator_quorum.rs
  fork_detection.rs
  pruning_proof.rs
  da_missing_blocks_pruning.rs
  receipt_inclusion_proof.rs
  account_state_proof.rs
  external_anchor_payload.rs
```

Future smoke:

```text
Visitor B pays Creator A 10 ROC.
Receipt accepted by svc-wallet/ron-ledger.
CrabLink unlocks only after accepted receipt.
Receipt included in receipt batch.
Receipt batch included in checkpoint.
Checkpoint root reproduced by independent replay.
Future committee signs checkpoint if committee mode exists.
Old receipt raw batch pruned only after DA/challenge/archive gate.
Receipt still verifies through proof.
Optional anchor proves checkpoint existed externally.
```

---

# 23. Metrics

Future metrics:

```text
quickchain_checkpoints_total{result}
quickchain_checkpoint_height
quickchain_epoch_duration_seconds
quickchain_receipts_per_epoch
quickchain_state_root_changes_total
quickchain_execution_replay_total{result}
quickchain_validator_signatures_total{validator,result}
quickchain_quorum_failures_total{reason}
quickchain_forks_detected_total
quickchain_challenges_total{type,result}
quickchain_pruned_epochs_total
quickchain_pruning_blocked_total{reason}
quickchain_carrier_chunks_assigned_total
quickchain_carrier_proofs_total{result}
quickchain_archive_chunks_total
quickchain_anchor_submissions_total{chain,result}
quickchain_anchor_verify_total{chain,result}
quickchain_raw_engagement_reward_reject_total{reason}
quickchain_duplicate_operation_reject_total{reason}
```

Metrics rules:

```text
- Do not put account IDs in labels.
- Do not put wallet txids in labels.
- Do not put CIDs in high-cardinality labels.
- Use route/result/reason buckets.
- Use validator label only for bounded validator set metrics.
```

---

# 24. Security rules

```text
- No unsafe Rust unless separately justified and audited.
- No floating-point money math.
- No direct ledger mutation outside approved wallet/ledger boundary.
- No signature verification shortcuts.
- No unknown fields in chain DTOs.
- No pruning without signed checkpoint roots and DA/archive safety.
- No anchor without checkpoint verification.
- No external bridge until internal ROC is proven.
- No privacy claims without a real privacy protocol.
- No anonymous alt validators by default.
- No reward minting by carriers/providers/validators directly.
- No raw engagement auto-payouts.
- No client idempotency key as sole anti-double-spend mechanism.
- No DB-order or wall-clock dependency in root-producing logic.
- No Phase 1 code before QC-0A preflight artifacts exist.
- No root-producing code without golden vectors.
- No root-producing code that depends on DB iteration order.
- No root-producing code that depends on wall-clock reads.
- No root-producing code that accepts unknown fields.
- No rewarder path may accept ambiguous event_class.
- No raw engagement reward formula may be enabled by config alone.
- No concurrent hold model may ship without replay tests.
- No pruning may ship before DA-missing challenge tests.
```

---

# 25. Open research questions

Important unresolved decisions:

```text
1. Future decentralization path:
   internal committee?
   external L2/rollup?
   external DA?
   anchor-only?
   hybrid?

2. Consensus algorithm if internal committee is chosen:
   Tendermint-like BFT?
   HotStuff-like BFT?
   Raft-like beta committee only for non-Byzantine internal phase?
   no internal BFT at all?

3. Canonical encoding:
   canonical JSON first?
   CBOR later?
   fixed binary format?

4. State tree:
   simple Merkle map?
   sparse Merkle tree?
   account trie?

5. Challenge window:
   hours?
   days?
   longer for archive chunks?

6. Validator admission:
   governance-approved first?
   passport + reputation?
   ROC bond later?

7. Carrier assignment:
   random?
   reputation-weighted?
   geo-balanced?
   storage-capacity weighted?

8. Reward formula:
   uptime?
   proofs served?
   chunks carried?
   successful challenges?
   quality score?
   how to avoid fake engagement dilution?

9. External anchor cadence:
   every epoch?
   daily?
   high-value checkpoints only?

10. Privacy:
   what account data is public?
   how do alt accounts avoid accidental linkage?
   what is provable without exposing browsing history?

11. Migration:
   how does current ron-ledger local state become QuickChain genesis?

12. DA strategy:
   internal carrier/archive?
   external DA?
   full archive retention?
   hybrid?

13. Concurrency:
   how are holds/session budgets represented in account roots?
   how are retries made idempotent without nonce holes?

14. Preflight artifact order:
   Which QC-0A document must be written first?
   Suggested order:
   1. THREAT_MODEL.md
   2. ROOTS_AND_CANONICALIZATION.md
   3. STATE_TREE_DECISION.md
   4. CONCURRENT_HOLDS_AND_OPERATION_IDS.md
   5. EVENT_CLASSIFICATION_AND_REWARDS.md
   6. RETENTION_AND_DA_PRECONDITIONS.md
   7. PHASE_2_TO_PHASE_3_CHASM.md
   8. TEST_VECTORS.md

15. Canonicalization lock-in:
   Should canonical JSON-v1 be temporary only?
   Should CBOR or fixed binary be chosen before Phase 1?
   How painful is migration if JSON-v1 ships first?
   What test vectors prove cross-language compatibility?

16. Hold compaction:
   How long do closed holds remain in state?
   Are closed holds represented only by receipts after finalization?
   Can hold history be proven through receipt_root instead of holds_root?
   What prevents holds_root bloat?

17. Event taxonomy governance:
   Who may create new event_class values?
   Are event classes closed enums?
   How are event-class policy changes versioned?
   Can old events be reclassified?
   Should reclassification be forbidden after sealing?

18. Reward pressure:
   How do we prevent growth pressure from weakening anti-bot rules?
   Should raw engagement rewards require signed governance override?
   Should raw engagement rewards be impossible at schema level?
   Should ad-budgeted events be the only engagement-like payout path?
```

---

# 26. QuickChain genesis strategy

When ready, QuickChain should start from a signed genesis snapshot.

Genesis input:

```text
current ron-ledger replayed balances
current account sequences/nonces
current holds
current supply
current policy hash
current chain params hash
current validator set if any
genesis timestamp
execution_spec_version
```

Genesis checkpoint:

```text
height = 0
previous_checkpoint_hash = b3:000...000 or explicit genesis marker
previous_state_root = b3:000...000
new_state_root = replayed current state
receipt_root = empty root
accounting_snapshot_root = empty root
reward_manifest_root = empty root
validator_set_hash = genesis validator set or null
settlement_mode = "local_root"
signatures = genesis authority/committee signatures if applicable
```

Genesis must be:

```text
replayable
auditable
signed
documented
not silently mutable
validator-executable
```

---

# 27. Plain-English summary

QuickChain is the future ROC settlement design.

It should not put every site visit directly on a global chain.

It should also not pay protocol ROC directly from fakeable engagement metrics.

Instead:

```text
Users get backend-derived signed receipts.
Creators get fast visible receipt-backed payouts.
Usage events are classified carefully.
Payer-authorized actions mutate balances through wallet/ledger.
Bot-prone metrics stay metering/analytics unless strongly gated.
Wallet deltas settle into deterministic epoch state roots.
Future validators may sign compact checkpoints.
Carrier/archive nodes may later store proof chunks.
Normal nodes prune old raw history only after DA/challenge/archive safety.
External systems may later anchor checkpoint commitments.
```

The final design target:

```text
High trust.
Fast UX.
Efficient.
Verifiable.
Replayable.
Bot-aware.
Prunable only by proof.
Externally anchorable later.
Built specifically for RustyOnions.
Not prematurely committed to a proprietary public L1.
```

---

# 28. Do not implement until

Do not start QuickChain implementation until these are true:

```text
[ ] paid site visit proof works end-to-end
[ ] Visitor B balance decreases
[ ] Creator A balance increases
[ ] wallet receipt displays in CrabLink
[ ] receipt display distinguishes accepted/pending/final states
[ ] ron-ledger replay proves balances
[ ] ron-accounting has sealed usage snapshot proof
[ ] svc-rewarder can compute deterministic payout plan
[ ] wallet/rewarder replay does not double issue
[ ] product smoke scripts prove internal ROC loop
[ ] NEXT_LEVEL paid view patterns are stable enough to know real transaction volume
[ ] raw engagement events are classified as metering, not automatic rewards
[ ] QC-0A THREAT_MODEL.md exists
[ ] QC-0A ROOTS_AND_CANONICALIZATION.md exists
[ ] QC-0A STATE_TREE_DECISION.md exists
[ ] QC-0A CONCURRENT_HOLDS_AND_OPERATION_IDS.md exists
[ ] QC-0A EVENT_CLASSIFICATION_AND_REWARDS.md exists
[ ] QC-0A RETENTION_AND_DA_PRECONDITIONS.md exists
[ ] QC-0A PHASE_2_TO_PHASE_3_CHASM.md exists
[ ] QC-0A TEST_VECTORS.md exists
```

Until then, keep QuickChain as a design blueprint only.

---

# 29. Final north-star phrase

```text
QuickChain is not “a blockchain bolted onto RustyOnions.”

QuickChain is the future settlement spine of ROC:
a receipt-first, epoch-settled, state-root verified, validator-executable,
bot-aware, proof-prunable ledger design for the content-addressed creator web.

Its first job is not to be a public L1.

Its first job is to make ROC truth replayable, root-verifiable, and impossible to fake.
```
