# QuickChain Review Bundle

Generated at UTC: `2026-06-10T17:30:55Z`

Includes:

- `QUICKCHAIN.MD`
- `QUICKCHAIN_QC0A_COMBINED.MD`

---

<!-- BEGIN QUICKCHAIN.MD -->

# File: `QUICKCHAIN.MD`

>Updated after Gemini, Grok, and QC-0A-R1 review - June 10 2026

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


## 0.2 QC-0A-R1 current status

As of June 10, 2026, the QC-0A-R1 documentation package exists as the active QuickChain Phase 0 safety/preflight reference.

Current QC-0A-R1 package:

```text
docs/quickchain/README.md
docs/quickchain/PHASE_0_PRE_FLIGHT.md
docs/quickchain/THREAT_MODEL.md
docs/quickchain/DOMAIN_SEPARATION.md
docs/quickchain/ROOTS_AND_CANONICALIZATION.md
docs/quickchain/STATE_TREE_DECISION.md
docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md
docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md
docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md
docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md
docs/quickchain/TEST_VECTORS.md
docs/quickchain/NO_GO_LIVE_CHECKLIST.md
```

The combined upload/review artifact is:

```text
QUICKCHAIN_QC0A_COMBINED.MD
```

QC-0A-R1 has been reviewed as a strong safety-first preflight package.

It is accepted as the authoritative QuickChain Phase 0 reference for:

```text
threat model
canonicalization posture
domain separation
state tree candidate
concurrent holds and operation IDs
closed-hold compaction
event classification
forbidden reward bases
retention and DA preconditions
Phase 2 to Phase 3 chasm
test-vector format
no-go-live checklist
```

This status does **not** authorize Phase 1 root-producing implementation.

The remaining blocker before Phase 1 is locked, independently reproducible test vectors.

Required vector progression:

```text
sketch -> locked_bytes -> locked_hash
```

Required locked vector areas:

```text
canonical receipt bytes
canonical account-state bytes
canonical hold-state bytes
canonical operation-intent bytes
canonical empty-tree bytes
preimage framing vectors
receipt hash vectors
account leaf vectors
hold leaf vectors
state root vectors
receipt root vectors
concurrent-hold replay vectors
closed-hold compaction vectors
duplicate operation/idempotency vectors
checkpoint hash vectors
```

Until the relevant vectors are locked and independently reproducible, QuickChain remains a future blueprint.

The next allowed implementation slice, after project priorities permit, is **DTO-only** work inside:

```text
ron-proto::quickchain
```

Allowed DTO-only scope:

```text
strict DTO shapes
schema constants
validation helpers
unknown-field rejection tests
integer minor-unit validation tests
b3 lowercase validation tests
event-class enum tests
operation/idempotency shape tests
```

Forbidden in the DTO-only slice:

```text
no roots
no hashes over live ledger state
no checkpoint production
no validators
no pruning
no settlement
no external anchors
no service calls
no sockets
no ledger mutation
no CrabLink chain authority
```

QuickChain remains disabled by default until the internal ROC value plane and QC-0A vector gates are green.


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


<!-- END QUICKCHAIN.MD -->

---

<!-- BEGIN QUICKCHAIN_QC0A_COMBINED.MD -->

# File: `QUICKCHAIN_QC0A_COMBINED.MD`

# QuickChain QC-0A Combined Docs

Generated by: `scripts/combine_quickchain_docs.sh`

Generated at UTC: `2026-06-10T17:30:54Z`

Source directory:

```text
docs/quickchain
```

## Included files

- `docs/quickchain/README.md`
- `docs/quickchain/PHASE_0_PRE_FLIGHT.md`
- `docs/quickchain/THREAT_MODEL.md`
- `docs/quickchain/DOMAIN_SEPARATION.md`
- `docs/quickchain/ROOTS_AND_CANONICALIZATION.md`
- `docs/quickchain/STATE_TREE_DECISION.md`
- `docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md`
- `docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md`
- `docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md`
- `docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md`
- `docs/quickchain/TEST_VECTORS.md`
- `docs/quickchain/NO_GO_LIVE_CHECKLIST.md`

---

<!-- BEGIN docs/quickchain/README.md -->

# File: `docs/quickchain/README.md`

# QuickChain Preflight

RO:WHAT — Preflight documentation for QuickChain, the future ROC checkpoint/settlement spine.
RO:WHY — QuickChain must begin as safety-first, future-gated work before any chain runtime exists.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-registry, ron-kms, ron-audit.
RO:INVARIANTS — no live chain; no external settlement; no fake receipts; no ledger mutation outside svc-wallet/ron-ledger.
RO:METRICS — none yet; future metrics are checkpoint/proof/challenge/reward/carrier counters.
RO:CONFIG — QuickChain remains disabled by default.
RO:SECURITY — no ROX/Solana bridge, staking, liquidity, public validator market, or private-key custody.
RO:TEST — docs reviewed with ron-proto DTO strictness tests.

## Status

QuickChain is a future-chain blueprint.

This directory exists to make QuickChain harder to implement incorrectly.

Current scope:

Allowed:

- threat model
- preflight gates
- crate ownership map
- DTO boundaries
- canonicalization notes
- strict ron-proto DTO scaffolding
- tests that prove DTOs reject drift

Forbidden:

- live consensus
- validators
- public chain state
- ROX
- Solana
- bridges
- staking
- liquidity
- exchange-facing logic
- runtime ledger mutation from QuickChain
- chain authority in CrabLink

## Phase 0 rule

QuickChain Phase 0 is not a blockchain implementation.

Phase 0 is:

- determinism before distribution
- DTOs before roots
- roots before validators
- proofs before pruning
- internal ROC before external anchors

## First implementation slice

The first safe code slice is ron-proto::quickchain.

ron-proto::quickchain may define strict DTO shapes and validation helpers.

It must not:

- hash live ledger state
- produce roots
- verify validator signatures
- mutate balances
- call services
- write storage
- open sockets
- spawn tasks
- anchor externally

## Acceptance

After applying the first slice:

    cargo fmt -p ron-proto
    cargo clippy -p ron-proto --all-targets -- -D warnings
    cargo test -p ron-proto --all-targets

Recommended wider gate before the next QuickChain slice:

    cargo check --workspace
    cargo test -p ron-ledger
    cargo test -p svc-wallet
    cargo test -p ron-accounting
    cargo test -p svc-rewarder
    cargo test -p ron-policy
    cargo test -p svc-registry
    cargo test -p ron-kms
    cargo test -p ron-audit


<!-- END docs/quickchain/README.md -->

---

<!-- BEGIN docs/quickchain/PHASE_0_PRE_FLIGHT.md -->

# File: `docs/quickchain/PHASE_0_PRE_FLIGHT.md`

# QuickChain Phase 0 Preflight

RO:WHAT — Phase 0 checklist for beginning QuickChain without creating live settlement behavior.
RO:WHY — Prevent consensus, bridge, validator, or token-launch creep before internal ROC is proven.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-registry, ron-audit.
RO:INVARIANTS — ROC internal first; wallet remains mutation front-door; ledger remains truth; accounting/rewarder do not mutate balances.
RO:METRICS — none yet.
RO:CONFIG — QuickChain and external anchors remain disabled.
RO:SECURITY — no fake proofs, no fake receipts, no public bridge, no privacy claims.
RO:TEST — enforced by review plus ron-proto DTO strictness tests.

## 1. Phase 0 objective

The objective is not to build a chain.

The objective is to freeze the safety model and create strict wire contracts that future root/proof code can use.

## 2. Non-negotiable boundaries

svc-wallet:
  mutation front-door

ron-ledger:
  durable economic truth

ron-accounting:
  usage snapshots, not balance truth

svc-rewarder:
  deterministic payout planning, no direct mutation

ron-proto:
  DTOs only

CrabLink:
  display/user intent only

## 3. Forbidden in Phase 0

- consensus
- validator network
- checkpoint signing runtime
- fork choice
- pruning
- carrier rewards
- archive rewards
- ROX
- Solana
- public bridge
- staking
- liquidity
- exchange-facing logic
- CrabLink chain authority
- gateway ledger mutation
- omnigate ledger mutation
- accounting ledger mutation
- rewarder ledger mutation
- fake balances
- fake receipts
- fake proofs

## 4. Allowed in Phase 0

- strict DTOs
- validation helpers
- schema constants
- readiness docs
- threat model docs
- canonicalization notes
- golden-vector planning
- serde drift tests
- unknown-field rejection tests
- invalid b3 rejection tests
- integer-money rejection tests

## 5. Canonicalization posture

Phase 0 does not decide final consensus encoding.

Until the canonicalizer is implemented, DTO tests must not pretend to prove checkpoint hashes.

The next safe step after DTOs is:

- canonical JSON v1 experiment
- exact byte vectors
- BLAKE3 vector tests
- field order tests
- unknown field rejection
- float rejection
- map ordering tests

## 6. Go / no-go gate for Phase 1

Do not begin local checkpoint roots until:

- [ ] ron-proto QuickChain DTOs are strict and green
- [ ] canonical bytes are specified
- [ ] root domain separation strings are specified
- [ ] receipt-root ordering rule is specified
- [ ] account-state ordering rule is specified
- [ ] no live service path consumes QuickChain DTOs as authority


<!-- END docs/quickchain/PHASE_0_PRE_FLIGHT.md -->

---

<!-- BEGIN docs/quickchain/THREAT_MODEL.md -->

# File: `docs/quickchain/THREAT_MODEL.md`

# QuickChain Threat Model

RO:WHAT — Defines the threat model for QuickChain before any root-producing, validator, pruning, DA, bridge, or settlement implementation begins.
RO:WHY — QuickChain touches economic truth. Its hardest failures are not syntax bugs; they are fake receipts, replay ambiguity, canonicalization drift, unsafe pruning, fake engagement rewards, and future decentralization mistakes.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-registry, svc-passport, ron-auth, ron-kms, ron-audit, svc-storage, svc-gateway, omnigate, CrabLink, future validators, future carriers, future archives, future external anchors.
RO:INVARIANTS — ROC remains internal until proven; wallet remains mutation front-door; ledger remains truth; accounting is not balance truth; rewarder does not mutate balances; no raw engagement auto-payouts; no pruning before DA/challenge/archive fallback; no Phase 1 code before QC-0A is complete.
RO:METRICS — Future challenge, replay, duplicate operation, raw engagement rejection, pruning blocked, DA missing, validator equivocation, and checkpoint verification metrics.
RO:CONFIG — Future chain params, challenge windows, retention limits, safety gates, validator set, DA mode, pruning flags, reward flags, external anchor flags.
RO:SECURITY — No fake receipts, no silent spend, no floating money, no DB-order roots, no wall-clock roots, no unknown fields in consensus DTOs, no public bridge, no external settlement during internal ROC MVP.
RO:TEST — Future canonicalization vectors, replay vectors, duplicate-operation tests, concurrent-hold tests, reward-formula rejection tests, DA-missing challenge tests, pruning restore tests, validator replay tests.

---

## 0. Status

This document is part of the QuickChain QC-0A preflight package.

It is not implementation.

It does not enable QuickChain.

It does not create roots, validators, checkpoints, settlement behavior, pruning, bridges, or external anchors.

Its job is to make the dangerous parts explicit before code exists.

QuickChain must remain blocked from Phase 1 until the QC-0A package answers the required preflight questions and produces independent replayable test-vector expectations.

---

## 1. Security objective

QuickChain exists to make ROC economic truth:

- replayable
- deterministic
- root-verifiable
- receipt-first
- epoch-settled
- validator-executable later
- bot-aware
- prunable only by proof later
- externally anchorable later, without making external systems the source of ROC truth

The primary security objective is:

Make it impossible for UI, gateway, omnigate, accounting, rewarder, cache, storage, carriers, validators, or external anchors to fake balances, fake receipts, fake entitlement, fake rewards, or fake finality.

---

## 2. Primary assets

QuickChain must protect the following assets.

### 2.1 Economic assets

- ROC balances
- held ROC
- available ROC
- issue/burn exceptions
- transfer/capture/release receipts
- reward payout receipts
- account sequences
- operation IDs
- idempotency records
- hold IDs
- session budget IDs

### 2.2 Verifiability assets

- canonical receipt bytes
- receipt hashes
- account state bytes
- hold state bytes
- state roots
- receipt roots
- accounting roots
- reward roots
- checkpoint hashes
- checkpoint headers
- proof formats
- future validator signatures
- future external anchor commitments

### 2.3 Availability assets

- receipt batches
- accounting windows
- reward manifests
- ledger transition chunks
- state proof chunks
- carrier/archive chunks
- checkpoint headers
- challenge evidence

### 2.4 Privacy and identity assets

- passport identities
- alt passport separation
- wallet account linkage
- capability tokens
- private keys
- private main-to-alt relationships
- browsing/payment history where policy requires privacy
- creator payout metadata where policy restricts exposure

---

## 3. Trust boundaries

### 3.1 Trusted for economic mutation

Only these layers may participate in durable economic mutation:

- svc-wallet
- ron-ledger

Rules:

- svc-wallet is the mutation front-door.
- ron-ledger is durable replayable truth.
- Every balance mutation must produce a durable receipt.
- Every retry must be idempotent.
- Every duplicate operation must return the original receipt or reject safely.

### 3.2 Not trusted for economic mutation

The following must never become direct economic authority:

- CrabLink
- React UI
- Tauri display state
- Chrome extension state
- gateway
- omnigate
- ron-accounting
- svc-rewarder
- svc-storage
- svc-index
- svc-dht
- svc-edge
- carriers
- archive nodes
- external anchors
- external L1/L2/DA systems

These may display, proxy, hydrate, store, meter, plan, or prove data, but they must not invent balances or receipts.

### 3.3 Future conditional trust

Future validators may become checkpoint-signing participants only after:

- deterministic replay is proven
- canonical bytes are frozen
- golden vectors exist
- exported epoch artifacts are replayable outside the original operator
- validator keys are controlled through approved KMS/registry paths
- validator equivocation detection exists
- governance admission rules exist

Future carriers/archive nodes may become availability participants only after:

- chunk formats exist
- challenge protocols exist
- missing-data challenges work
- archive fallback works
- reward withholding works
- pruning restore tests pass

External anchors may become read-only commitment witnesses only after:

- checkpoint verification exists
- anchor payload DTO is frozen
- anchor verifier is read-only
- ROC remains internally authoritative
- no public bridge mutates ROC balances

---

## 4. Adversary model

QuickChain must assume these adversaries exist.

### 4.1 Malicious user

A user may try to:

- replay an old receipt
- duplicate an operation ID
- reuse an idempotency key with a changed payload
- submit conflicting hold/capture/release operations
- exploit multi-tab concurrency
- claim paid access without backend receipt truth
- forge a receipt display object
- exploit UI cache to unlock paid content

### 4.2 Malicious creator

A creator may try to:

- fake site visits
- bot watch seconds
- inflate ad impressions
- create fake engagement loops
- route fake reward claims through accounting
- bypass payout splits
- claim ownership of someone else’s b3 asset
- re-mint duplicate stolen content
- exploit raw metrics as payout basis

### 4.3 Malicious service operator

An operator may try to:

- reorder ledger events
- hide failed writes
- create fake receipts
- mutate state outside svc-wallet/ron-ledger
- generate roots from DB iteration order
- prune history too early
- publish a checkpoint without available data
- present centralized DB state as decentralized verification
- make Phase 2 artifacts impossible for future validators to replay

### 4.4 Malicious validator, future only

A future validator may try to:

- sign conflicting checkpoints
- sign without replaying artifacts
- accept invalid reward roots
- accept invalid state transitions
- ignore missing data
- collude to finalize bad checkpoints
- sign over ambiguous canonical bytes
- sign unknown schema fields

### 4.5 Malicious carrier/archive node, future only

A future carrier/archive node may try to:

- claim custody of missing chunks
- serve corrupted chunks
- withhold data during challenge windows
- provide stale chunks
- exploit erasure coding metadata
- farm carrier rewards without availability

### 4.6 External anchor or bridge adversary, future only

A future external system may:

- be unavailable
- reorganize
- censor anchor writes
- report stale finality
- be mistaken for ROC economic truth
- encourage bridge logic before ROC is proven
- become a hot-path dependency by accident

### 4.7 Bot/Sybil adversary

Bots may try to:

- create fake accounts
- generate fake visits
- generate fake watch seconds
- generate fake ad impressions
- dilute reward pools
- farm analytics-only events
- exploit capped reward experiments
- create fake storage/provider activity without proof

---

## 5. Non-goals and forbidden assumptions

QuickChain must not assume:

- recorded event equals real-world truth
- view count equals human attention
- watch seconds equal value
- ad impression equals value
- a checkpoint root alone proves data availability
- a local deterministic database is already a distributed chain
- a future external anchor makes ROC safe
- a cached receipt display is receipt truth
- client idempotency keys alone prevent double spends
- DB iteration order is stable
- map iteration order is stable
- wall-clock reads are safe inside root-producing logic
- floating point is acceptable for money
- validators can be added as a toggle
- pruning is safe because roots exist
- raw engagement can directly allocate protocol ROC

---

## 6. Threats and required defenses

### 6.1 Canonicalization split-brain

Threat:

Same logical input produces different bytes on different machines.

Impact:

- different receipt hashes
- different state roots
- different checkpoint hashes
- invalid validator signatures
- impossible independent replay

Required defenses:

- strict DTOs
- deny unknown fields
- explicit field order
- explicit default handling
- no unordered maps in root-producing payloads unless canonicalized
- no floats
- lowercase b3 only
- versioned schemas
- exact byte vectors
- cross-platform replay tests

No-go:

Do not implement root-producing code until canonical byte vectors exist.

---

### 6.2 Root non-determinism

Threat:

State roots depend on local DB order, scheduler timing, map order, wall-clock reads, timezone, locale, or hidden mutable caches.

Impact:

- validators cannot reproduce roots
- Phase 2 cannot become Phase 3
- honest nodes disagree

Required defenses:

- stable sorting
- deterministic transition interface
- no wall-clock inside transition logic
- no DB-order dependency
- no random IDs inside root-producing code
- no shared mutable cache dependency
- explicit epoch inputs
- golden root vectors

No-go:

If an external verifier cannot reproduce a root from exported artifacts, that root design is invalid.

---

### 6.3 Duplicate economic commit

Threat:

A retry, race, or malicious request commits the same operation twice.

Impact:

- double spend
- duplicate reward issue
- duplicate capture
- duplicate payout
- broken supply conservation

Required defenses:

- durable operation_id uniqueness
- durable idempotency handling
- account_id scoping
- canonical operation hash
- conflict detection for same key/different payload
- retry returns original receipt
- replay tests for duplicate operation IDs
- replay tests for duplicate idempotency keys

No-go:

Client-provided idempotency keys must never be the only anti-double-spend mechanism.

---

### 6.4 Concurrent hold ambiguity

Threat:

Multiple paid actions, tabs, sessions, or retries create nonce holes, stuck holds, conflicting captures, or unbounded hold state.

Impact:

- paid UX breaks
- funds become stuck
- state root grows without bound
- replay becomes ambiguous

Required defenses:

- hold_id
- session_budget_id where applicable
- operation_id
- idempotency_key
- account_sequence
- deterministic expiration rule
- maximum open holds per account
- hold root model
- closed hold compaction rule
- replay vectors for concurrent holds

No-go:

Do not implement hold roots until concurrent hold replay vectors exist.

---

### 6.5 Fake receipt or fake entitlement

Threat:

A UI, cache, gateway, omnigate route, or local display object claims paid access without wallet/ledger truth.

Impact:

- creators are robbed
- paid content unlocks for free
- users see fake balances
- receipts become untrustworthy

Required defenses:

- paid unlock requires backend accepted receipt
- receipt display cache is display-only
- CrabLink labels receipt status honestly
- no local entitlement truth
- no fake receipt construction in UI
- gateway/omnigate proxy/hydrate but do not mutate ledger
- wallet/ledger receipt source shown where possible

No-go:

No paid unlock from cache alone.

---

### 6.6 Fake engagement reward abuse

Threat:

Bots generate fake visits, watch seconds, clicks, impressions, or activity and convert them into protocol ROC.

Impact:

- reward pool dilution
- creator economy corruption
- protocol insolvency
- honest creators lose value

Required defenses:

- closed event_class enum
- analytics_only events cannot enter reward manifests
- metering cannot mutate balances
- proof_eligible requires verification/challenge
- ad_budgeted requires explicit advertiser/sponsor budget
- economic_receipt must correspond to wallet/ledger receipt
- forbidden reward formulas listed
- policy rejects ambiguous event classes
- rewarder rejects raw engagement primary denominators

No-go:

Raw engagement metrics must not directly mint or allocate protocol ROC.

---

### 6.7 Reward double issue

Threat:

A reward epoch is replayed, retried, or reprocessed and issues payouts twice.

Impact:

- supply inflation
- duplicate payouts
- ledger conservation failure

Required defenses:

- deterministic reward manifest ID
- epoch ID
- funding source
- policy hash
- payout intent IDs
- wallet idempotency
- rewarder does not mutate ledger
- replay same epoch cannot double issue
- duplicate reward commit test

No-go:

Rewarder must never directly mutate ron-ledger.

---

### 6.8 Unauthorized issue or burn

Threat:

An operator, validator, rewarder, admin route, or config path issues or burns ROC outside approved policy.

Impact:

- broken supply
- trust failure
- hidden inflation or destruction

Required defenses:

- issue/burn exception classes
- policy hash in checkpoint
- chain params hash in checkpoint
- ledger conservation checks
- governance approval where required
- audit events
- receipt for every mutation

No-go:

No issue/burn without durable receipt and replayable authorization.

---

### 6.9 Data unavailable after checkpoint

Threat:

A checkpoint commits receipt/accounting/reward roots but raw chunks are missing during challenge or verification.

Impact:

- cannot challenge
- cannot replay
- unsafe pruning
- validators signed unverifiable state

Required defenses:

- data availability root later
- receipt batch chunks
- accounting snapshot chunks
- reward manifest chunks
- missing-data challenge
- archive fallback
- pruning blocked when DA is missing
- carrier rewards withheld when proofs fail

No-go:

A checkpoint root alone does not prove data availability.

---

### 6.10 Unsafe pruning

Threat:

Raw receipts, events, or ledger chunks are deleted before proofs, DA, challenge windows, and archive fallback are proven.

Impact:

- cannot verify old receipts
- cannot restore state
- cannot challenge fraud
- balances become trust-me state

Required defenses:

- pruning disabled in local-root mode
- pruning disabled in epoch-sealing mode
- challenge windows
- receipt inclusion proofs
- account proofs
- archive fallback
- restore-from-proof tests
- missing DA blocks pruning

No-go:

No pruning before DA/challenge/archive fallback is proven.

---

### 6.11 Validator equivocation, future only

Threat:

A validator signs conflicting checkpoints for the same height or epoch.

Impact:

- forked settlement truth
- double finality claims
- external anchor ambiguity

Required defenses:

- validator identity
- key ID
- signed checkpoint preimage
- validator-set hash
- equivocation evidence
- audit trail
- quarantine path
- no automatic slashing until audited

No-go:

No validator signing runtime until equivocation evidence format exists.

---

### 6.12 Phase 2 to Phase 3 rewrite trap

Threat:

Phase 1/2 are implemented as centralized database jobs that cannot be replayed by future validators.

Impact:

- “decentralization later” becomes impossible
- committee mode requires rewrite
- early roots are not credible

Required defenses:

- validator-executable transition interface from the start
- exported epoch artifacts
- independent replay tool
- no service-local hidden assumptions
- no wall-clock/DB-order dependency
- external verifier reproduces checkpoint hash

No-go:

If validators cannot replay Phase 2 artifacts without trusting the original operator, Phase 3 must not begin.

---

### 6.13 External anchor confusion, future only

Threat:

External anchor or bridge becomes perceived as ROC truth or required for hot-path UX.

Impact:

- bridge risk
- external outage risk
- wrong finality claims
- premature public chain dependency

Required defenses:

- anchors optional
- anchors read-only
- anchors compact checkpoint commitments only
- anchors not required for site load
- anchor verifier cannot mutate ROC balances
- no public bridge until audit
- no external settlement in internal ROC MVP

No-go:

External anchors must never replace svc-wallet/ron-ledger truth.

---

### 6.14 Privacy and alt-linkage leakage

Threat:

Receipts, metrics, account proofs, validator artifacts, or public profile data accidentally reveal private alt/main passport relationships or browsing/payment history.

Impact:

- user privacy harm
- deanonymization
- trust failure

Required defenses:

- no public main-alt linkage by default
- minimize public proof fields
- do not put account IDs in high-cardinality metrics labels
- capability-scoped proof views
- no privacy claims without a real protocol
- redaction in logs/errors
- no private keys/capabilities in UI state

No-go:

Do not claim privacy properties that are not implemented.

---

## 7. Event classification security model

Event classes must be closed and enforceable.

Allowed event classes:

- economic_receipt
- metering
- proof_eligible
- ad_budgeted
- analytics_only

Required downstream rules:

- economic_receipt may affect balances only through svc-wallet/ron-ledger.
- metering cannot mutate balances.
- proof_eligible cannot become reward without verification/challenge.
- ad_budgeted requires explicit external budget.
- analytics_only cannot enter protocol ROC reward manifests.

Forbidden:

- unknown event_class
- ambiguous event_class
- action-only reward routing
- config-only enablement of raw engagement rewards
- raw visits/watch seconds/clicks/impressions as primary protocol reward denominator

---

## 8. Settlement status threat model

Receipt status must be honest.

Status levels:

- accepted
- epoch_included
- finalized
- anchored

Threat:

A UI or service presents accepted as finalized, or anchored as economic truth.

Rules:

- accepted is enough for normal paid unlock.
- epoch_included means included in a receipt root.
- finalized means challenge/finality conditions are met.
- anchored means an optional external anchor proves a compact commitment existed.
- anchored does not replace internal ROC truth.
- CrabLink must label status honestly.

No-go:

Finality-sensitive actions must not treat accepted receipts as finalized.

---

## 9. Metrics and observability threats

Metrics can leak sensitive data or create unstable cardinality.

Forbidden labels:

- account IDs
- wallet txids
- CIDs in high-cardinality metrics
- raw receipt IDs
- idempotency keys
- passport handles where unbounded
- alt linkage

Allowed labels:

- route
- result
- reason
- bounded event_class
- bounded challenge_type
- bounded settlement_status
- bounded service name
- bounded validator ID only when validator set is bounded

Future metrics should include:

- quickchain_duplicate_operation_reject_total{reason}
- quickchain_raw_engagement_reward_reject_total{reason}
- quickchain_challenges_total{type,result}
- quickchain_pruning_blocked_total{reason}
- quickchain_da_missing_total{result}
- quickchain_replay_total{result}
- quickchain_checkpoint_verify_total{result}
- quickchain_validator_equivocation_total{result}

---

## 10. Required challenge types

QuickChain challenge types must include at least:

- invalid_state_root
- invalid_receipt_root
- missing_data
- invalid_reward_root
- invalid_policy_hash
- invalid_chain_params_hash
- unauthorized_issue
- unauthorized_burn
- double_spend
- duplicate_operation_commit
- validator_equivocation
- carrier_failure
- raw_engagement_reward_abuse

Beta handling may use:

- manual governance quarantine
- checkpoint pause
- validator set review
- operator alert
- reward withholding

Future handling may add:

- fraud proof
- slashing
- validator ejection
- carrier reputation penalty

No-go:

Do not add automatic slashing before evidence formats and audits exist.

---

## 11. Security invariants by component

### 11.1 svc-wallet

Must:

- own mutation front-door
- enforce idempotency
- enforce operation uniqueness
- distinguish hold/capture/release
- produce durable receipts
- call ron-ledger for truth

Must not:

- silently spend
- double commit retries
- trust client idempotency alone
- expose uncapped spend authority

### 11.2 ron-ledger

Must:

- be durable truth
- be append/replay oriented
- prove conservation
- reject duplicate operations
- support deterministic replay

Must not:

- depend on wall-clock reads inside replay
- depend on DB iteration order for roots
- accept floating money

### 11.3 ron-accounting

Must:

- record usage/metering
- seal windows deterministically later
- classify event_class strictly
- export reward-compatible snapshots

Must not:

- mutate balances
- treat analytics as money
- reclassify sealed events casually

### 11.4 svc-rewarder

Must:

- produce deterministic payout plans
- identify funding source
- include policy hash
- route payout intents through svc-wallet

Must not:

- mutate ledger directly
- use raw engagement as primary reward denominator
- double issue same epoch

### 11.5 ron-policy

Must:

- reject ambiguous event classes
- reject forbidden reward formulas
- use integer minor units
- be side-effect-free

Must not:

- enable raw engagement rewards by config alone

### 11.6 svc-gateway

Must:

- proxy/admit/shape requests
- enforce quotas
- preserve backend truth boundary

Must not:

- mutate ledger
- invent receipts
- unlock paid content itself

### 11.7 omnigate

Must:

- hydrate/coordinator views
- enforce policy where appropriate
- request wallet-backed access through approved routes

Must not:

- own durable economic truth
- mutate ledger directly
- treat cached entitlement as truth

### 11.8 CrabLink

Must:

- display backend truth
- request explicit confirmation for paid actions
- label receipt status honestly

Must not:

- invent balances
- invent receipts
- unlock paid content from cache alone
- hold validator authority
- hold raw spend authority in React state

### 11.9 Future validators

Must:

- replay artifacts
- verify roots
- verify policy hash
- verify chain params hash
- sign only valid checkpoint headers

Must not:

- directly mint rewards
- trust original operator DB state
- sign ambiguous bytes
- bypass wallet/ledger truth

### 11.10 Future carriers/archive nodes

Must:

- store assigned chunks
- answer challenges
- earn only through rewarder/wallet path later

Must not:

- mint
- become DA by assertion
- justify pruning before challenge/restore tests

---

## 12. QC-0A acceptance checklist for this document

This threat model is acceptable only if it clearly answers:

- [ ] What assets are protected?
- [ ] Who is trusted to mutate balances?
- [ ] Who is not trusted to mutate balances?
- [ ] What adversaries are assumed?
- [ ] What canonicalization threats exist?
- [ ] What replay/idempotency threats exist?
- [ ] What concurrent hold threats exist?
- [ ] What fake receipt threats exist?
- [ ] What raw engagement reward threats exist?
- [ ] What reward double-issue threats exist?
- [ ] What DA/pruning threats exist?
- [ ] What validator threats exist?
- [ ] What external anchor threats exist?
- [ ] What privacy/linkage threats exist?
- [ ] What no-go conditions block Phase 1?

---

## 13. No-go conditions

QuickChain Phase 1 must not begin if any of these are true:

- THREAT_MODEL.md is incomplete.
- ROOTS_AND_CANONICALIZATION.md is incomplete.
- STATE_TREE_DECISION.md is incomplete.
- CONCURRENT_HOLDS_AND_OPERATION_IDS.md is incomplete.
- EVENT_CLASSIFICATION_AND_REWARDS.md is incomplete.
- RETENTION_AND_DA_PRECONDITIONS.md is incomplete.
- PHASE_2_TO_PHASE_3_CHASM.md is incomplete.
- TEST_VECTORS.md is incomplete.
- Canonical bytes are not specified.
- Domain separators are not specified.
- Preimage framing is not specified.
- State tree is not decided or narrowed to testable candidates.
- Concurrent hold replay model is missing.
- Event class rules are not enforceable.
- Forbidden reward formulas are not listed.
- Pre-DA retention limits are missing.
- Independent verifier cannot replay vectors.
- Any root-producing code depends on DB iteration order.
- Any root-producing code depends on wall-clock reads.
- Any reward path can use raw engagement as primary protocol payout basis.
- Any pruning path can run before DA/challenge/archive fallback.
- Any service outside svc-wallet/ron-ledger can mutate balances.
- Any UI can unlock paid content from cache alone.
- Any external anchor can mutate ROC truth.

---

## 14. Reviewer questions

Ask reviewers and critique tools to rate this document against these questions:

1. Does it clearly identify the highest-risk QuickChain failure modes?
2. Does it prevent fake receipts and fake balances?
3. Does it prevent raw engagement from becoming money?
4. Does it block unsafe pruning?
5. Does it prevent Phase 2 from becoming an unreplayable centralized database?
6. Does it make canonicalization drift a first-class threat?
7. Does it address concurrent holds and idempotency deeply enough?
8. Does it keep validators future-gated?
9. Does it keep external anchors read-only and optional?
10. Does it define clear no-go conditions for Phase 1?
11. Is anything too vague to convert into tests?
12. Are any dangerous assumptions still implicit?

---

## 15. Plain-English summary

QuickChain’s threat model is simple:

Do not let anything fake ROC truth.

Not the UI.
Not gateway.
Not omnigate.
Not accounting.
Not rewarder.
Not storage.
Not carriers.
Not future validators.
Not external anchors.

Only backend wallet/ledger truth counts for accepted economic mutation.

Everything else must be replayable, canonical, explicit, policy-gated, challengeable where needed, and honest about what it proves.

QuickChain Phase 1 should not start until this threat model and the rest of QC-0A make those rules testable.


<!-- END docs/quickchain/THREAT_MODEL.md -->

---

<!-- BEGIN docs/quickchain/DOMAIN_SEPARATION.md -->

# File: `docs/quickchain/DOMAIN_SEPARATION.md`

# QuickChain Domain Separation

RO:WHAT — Versioned domain names for future QuickChain hash/signature preimages.
RO:WHY — Prevent cross-context hash/signature reuse before roots, validators, pruning, or anchors exist.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto quickchain DTOs, future ron-ledger roots, future receipt/state/accounting/reward/checkpoint hashes.
RO:INVARIANTS — constants only; no hashing; no roots; no signatures; no ledger mutation; no settlement authority.
RO:METRICS — none.
RO:CONFIG — none; QuickChain remains disabled.
RO:SECURITY — every future root/hash context must use a distinct, versioned, test-gated domain.
RO:TEST — crates/ron-proto/tests/quickchain_domain_separation.rs.

## Status

This file is part of QC-0A preflight.

It does not implement QuickChain.

It does not compute hashes.

It does not produce roots.

It only freezes names that future root-producing code must use after canonical bytes and golden vectors are ready.

## Required root/hash domains

The current QuickChain blueprint defines these mandatory domains:

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

These names are intentionally short, versioned, human-auditable, and context-specific.

## Extra preflight domains

The preflight package also reserves domains for future compact artifacts:

```text
quickchain.chain-params.v1
quickchain.validator-set.v1
quickchain.policy.v1
quickchain.data-availability-root.v1
quickchain.receipt-batch.v1
quickchain.accounting-window.v1
quickchain.reward-manifest.v1
quickchain.challenge-evidence.v1
quickchain.anchor-payload.v1
```

These remain constants only.

They do not enable anchors, validators, DA, rewards, pruning, or settlement.

## Future preimage rule

Future root-producing code must use explicit framing.

Candidate framing:

```text
domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Do not implement the framing until TEST_VECTORS.md contains exact byte and hash vectors.

## Rules

Every domain separator must:

- start with `quickchain.`
- end with `.v1`
- use lowercase ASCII only
- use only lowercase letters, digits, dots, underscores, and hyphens
- be unique
- be test-gated
- never be silently renamed after vectors exist

## Go / no-go

If these domain names change, all future vectors must be regenerated.

If there are no golden vectors, no root-producing code may ship.


<!-- END docs/quickchain/DOMAIN_SEPARATION.md -->

---

<!-- BEGIN docs/quickchain/ROOTS_AND_CANONICALIZATION.md -->

# File: `docs/quickchain/ROOTS_AND_CANONICALIZATION.md`

# QuickChain Roots and Canonicalization

RO:WHAT — QC-0A canonicalization and future root-design preflight.
RO:WHY — Consensus, proofs, pruning, and checkpoints require identical bytes, explicit preimage framing, and exact hash domains.
RO:INTERACTS — QUICKCHAIN.MD, ron-proto quickchain DTOs, future ron-ledger roots, svc-wallet receipts, ron-accounting windows, svc-rewarder manifests.
RO:INVARIANTS — no roots before canonical bytes; no fake checkpoint hashes; no floats; no DB-order dependency; no wall-clock dependency; no unknown fields.
RO:METRICS — none.
RO:CONFIG — none; QuickChain remains disabled.
RO:SECURITY — canonicalization drift is a consensus failure.
RO:TEST — ron-proto quickchain canonical JSON tests now; future golden byte/hash/root vectors later.

---

## 0. Status

This is a QC-0A-R1 canonicalization decision record.

It is still pre-implementation.

It does not implement hashes, roots, receipts, validators, pruning, settlement, or anchors.

It freezes enough byte/preimage policy for the next safe DTO/vector work.

---

## 1. Current scope

The current ron-proto canonical JSON helpers are a Phase 0A byte experiment.

They prove:

```text
strict DTO deserialization
unknown-field rejection
minified typed JSON serialization
exact bytes for current DTO experiments
shuffled input normalizes to struct order
```

They do not prove:

```text
receipt hashes
state roots
receipt roots
accounting roots
reward roots
checkpoint hashes
validator signatures
pruning safety
settlement finality
```

---

## 2. Canonical JSON v1 decision

Decision:

```text
First QuickChain vectors use canonical JSON v1.
```

Canonical JSON v1 is not general-purpose JSON canonicalization.

It is a restricted RustyOnions vector encoding profile.

Rules:

```text
- UTF-8 only
- minified JSON only
- root-producing payloads are typed structs
- no unordered maps in root-producing payloads unless explicitly represented as sorted arrays
- field order is schema-defined and test-locked
- all root-producing DTOs reject unknown fields
- money is integer minor-unit string
- no floating point
- b3 values are lowercase b3:<64hex>
- optional fields serialize as explicit null when part of the schema
- no implicit default fields in root-producing vectors
- no skip_serializing_if for root-producing vector payloads
- timestamps are u64 milliseconds only when explicitly required
- epoch IDs are strings when epoch identity, not wall-clock behavior, is required
```

No-go:

```text
Do not hash arbitrary serde_json::Value.
Do not hash BTreeMap payloads unless the map-to-array canonicalization rule is vectorized.
Do not allow unknown fields in root-producing DTOs.
```

---

## 3. Preimage framing decision

Decision:

```text
QuickChain v1 hash preimage framing is:

domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Where:

```text
domain_separator_bytes:
  ASCII bytes of the exact domain separator string

0x00:
  single zero delimiter byte

canonical_payload_bytes:
  canonical JSON v1 UTF-8 bytes
```

Example shape:

```text
quickchain.account-state.v1 || 0x00 || {"schema":"quickchain.account-state.v1",...}
```

Rules:

```text
- delimiter is exactly one byte: 0x00
- domain separator must validate before use
- canonical payload must validate before use
- preimage framing must appear in TEST_VECTORS.md
- no hash code until preimage_hex vectors exist
```

No-go:

```text
No root-producing code may use raw canonical payload bytes without domain separation.
No root-producing code may use a different delimiter without a version bump.
No root-producing code may use implicit domain context.
```

---

## 4. Required future root/hash domains

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

Reserved supporting domains:

```text
quickchain.chain-params.v1
quickchain.validator-set.v1
quickchain.policy.v1
quickchain.data-availability-root.v1
quickchain.receipt-batch.v1
quickchain.accounting-window.v1
quickchain.reward-manifest.v1
quickchain.challenge-evidence.v1
quickchain.anchor-payload.v1
```

---

## 5. Canonicalization hazards

Root-producing code must not depend on:

```text
map iteration order
database scan order
wall-clock reads
local timezone
locale sorting
floating point
random IDs generated during replay
unordered JSON
implicit default fields
unknown fields
thread scheduling
retry timing
network timing
service-local mutable caches
```

---

## 6. Sorting rules

All root-producing sorted collections must define:

```text
sort key
duplicate-key behavior
byte encoding
ascending/descending order
tie rejection
```

Default rule:

```text
bytewise ascending over explicit UTF-8 sort keys
```

Duplicate key rule:

```text
duplicate keys are invalid
```

No-go:

```text
Do not use DB iteration order.
Do not use hash map iteration order.
Do not use locale-aware string comparison.
```

---

## 7. Money rules

All ROC money values must be:

```text
integer minor-unit strings
```

Allowed examples:

```text
"0"
"1"
"1000000"
```

Forbidden examples:

```text
1
1.0
"1.0"
"+1"
"-1"
"01"
"1_000"
"1 ROC"
```

Rules:

```text
- no floats
- no signed strings
- no leading zero unless exactly "0"
- no separators
- no units inside value
```

---

## 8. Null and optional fields

Root-producing DTOs must avoid optional ambiguity.

Rules:

```text
- optional fields must be explicitly declared
- optional fields serialize as null when absent
- absent optional fields are not allowed in canonical payloads unless the schema says a field may be omitted
- root-producing DTOs should prefer explicit null over omission
```

Reason:

```text
Omitted-vs-null drift creates cross-language vector hazards.
```

---

## 9. Hash algorithm

Decision for QuickChain v1 vectors:

```text
hash_algorithm = "blake3-256"
content ID encoding = "b3:<64 lowercase hex>"
```

Rules:

```text
- expected hashes in production vectors must be b3:<64 lowercase hex>
- all vectors must name hash_algorithm
- all vectors must name domain_separator
```

No-go:

```text
No fake hashes.
No placeholder hashes in production vectors.
```

---

## 10. Root-producing code no-go

No Phase 1 roots until:

```text
[ ] canonical receipt bytes are defined
[ ] canonical account-state bytes are defined
[ ] canonical hold-state bytes are defined
[ ] canonical checkpoint bytes are defined
[ ] hash domains are exact
[ ] preimage framing is exact
[ ] money string rules are tested
[ ] null/optional rules are tested
[ ] sorted collection rules are tested
[ ] golden vectors exist
[ ] independent verifier reproduces expected b3 hashes
```


<!-- END docs/quickchain/ROOTS_AND_CANONICALIZATION.md -->

---

<!-- BEGIN docs/quickchain/STATE_TREE_DECISION.md -->

# File: `docs/quickchain/STATE_TREE_DECISION.md`

# QuickChain State Tree Decision

RO:WHAT — QC-0A decision record for the future QuickChain state tree.
RO:WHY — State roots must be deterministic, validator-executable, and replayable before Phase 1 begins.
RO:INTERACTS — QUICKCHAIN.MD, ron-ledger, ron-proto quickchain DTOs, future account/hold/state proofs, future receipt roots.
RO:INVARIANTS — no root-producing code yet; no pruning; no validator runtime; no DB-order dependency; no wall-clock dependency.
RO:METRICS — none.
RO:CONFIG — none.
RO:SECURITY — state tree choice must avoid nondeterministic ordering, hidden service-local assumptions, and unbounded active hold bloat.
RO:TEST — future state tree golden vectors, unordered_input_same_root, account_state_vector_001, hold_compaction_vector_001.

---

## 0. Status

This is a QC-0A-R1 decision record.

It is still pre-implementation.

It does not implement state roots, Merkle roots, account proofs, hold proofs, validators, pruning, or settlement.

It narrows the first Phase 1 candidate so DTOs and vectors have a concrete target.

---

## 1. Phase 1 decision

Decision:

```text
Phase 1 state tree candidate = sorted binary Merkle map over canonical account-state leaves.
```

This means:

```text
- account-based state model
- canonical account-state leaf bytes
- bytewise deterministic sort keys
- duplicate keys rejected
- binary Merkle reduction over sorted leaf hashes
- no DB iteration order
- no map iteration order
- no sparse tree in Phase 1
```

Sparse Merkle tree is deferred.

Account trie is deferred.

Reason:

```text
A sorted Merkle map is the smallest auditable state-root design for Phase 1.
It is easier to vectorize, easier to replay independently, and less likely to hide consensus-breaking path rules.
```

---

## 2. Non-decision: final forever tree

This is not a forever commitment.

Future designs may migrate to:

```text
sparse Merkle tree
account trie
verkle-like structure
external DA-native proof tree
```

But any migration must be:

```text
versioned
governance-gated
test-vector-backed
replayable from old checkpoint to new checkpoint
explicit about proof compatibility
```

No silent tree migration is allowed.

---

## 3. Account-based model

QuickChain uses an account-based state model, not UTXO.

Reason:

```text
RustyOnions already has passports, wallet accounts, balances, holds, receipts, creator payouts, and paid access flows.
```

Future account leaf target:

```text
AccountStateV1 {
  schema: "quickchain.account-state.v1"
  account_id: string
  asset: "roc"

  balance_minor: string
  held_minor: string
  available_minor: string

  account_sequence: u64
  receipt_root: b3:<64hex>
  holds_root: b3:<64hex>
  permissions_root: b3:<64hex> | null

  updated_at_epoch: string
}
```

Rules:

```text
- all money values are integer minor-unit strings
- available_minor = balance_minor - held_minor
- no floats
- no negative values
- account_sequence assigned by ledger commit path
- receipt_root commits account-related receipts according to future receipt rules
- holds_root commits currently open holds only
```

---

## 4. Hold subtree decision

Decision:

```text
Each account has a holds_root over currently open holds only.
```

Open hold leaf target:

```text
HoldStateV1 {
  schema: "quickchain.hold-state.v1"
  hold_id: string
  account_id: string
  counterparty_account_id: string | null
  amount_minor: string
  purpose: string
  created_at_epoch: string
  expires_at_epoch: string
  status: "open"
  operation_id: string
  idempotency_key: string
  policy_hash: b3:<64hex>
}
```

Rules:

```text
- open holds sorted by hold_id
- duplicate hold_id rejected
- terminal holds are removed from active holds_root after terminal receipt inclusion
- terminal lifecycle is proven by receipt_root
- no closed-hold bloat in active state
```

---

## 5. Sort keys

### 5.1 Account leaf sort key

Decision:

```text
account_leaf_sort_key = utf8(account_id) || 0x00 || utf8(asset)
```

Rules:

```text
- account_id must be normalized before reaching root code
- asset for current ROC is exactly "roc"
- bytewise ascending order
- duplicate sort key is invalid
```

### 5.2 Hold leaf sort key

Decision:

```text
hold_leaf_sort_key = utf8(hold_id)
```

Rules:

```text
- hold_id must be normalized before reaching root code
- bytewise ascending order
- duplicate hold_id is invalid
```

### 5.3 Receipt sort key, deferred

Receipt tree ordering is not finalized in this file.

Candidate:

```text
receipt_sort_key = ledger_seq_start || txid
```

Receipt ordering must be finalized in ROOTS_AND_CANONICALIZATION.md and TEST_VECTORS.md before receipt roots.

---

## 6. Node model

Phase 1 candidate:

```text
leaf_hash = b3(domain("quickchain.account-state.v1") || 0x00 || canonical_account_state_bytes)

node_hash = b3(domain("quickchain.state-root.v1") || 0x00 || left_hash || right_hash)
```

Status:

```text
This is a target shape, not implemented code.
```

Before implementation:

```text
- exact preimage bytes must be in TEST_VECTORS.md
- exact odd-leaf promotion rule must be chosen
- exact empty tree rule must be chosen
- exact account leaf bytes must be frozen
```

---

## 7. Empty tree rule

Decision:

```text
No all-zero root is used as a normal empty tree root.
```

Reason:

```text
All-zero values are ambiguous and should be reserved only for explicit genesis sentinels where the schema says so.
```

Future empty tree root must be computed from a canonical empty-tree payload.

Target payload shape:

```text
EmptyTreeV1 {
  schema: "quickchain.empty-tree.v1"
  tree: "state" | "holds" | "receipts" | "accounting" | "rewards"
}
```

Status:

```text
Not implemented.
No production empty-root hash exists yet.
```

No-go:

```text
No root code until empty tree vectors exist.
```

---

## 8. Odd leaf rule

Decision for first vectors:

```text
Odd leaf/node promotion duplicates the final child at that level.
```

Example:

```text
[A, B, C] -> [hash(A,B), hash(C,C)]
```

Reason:

```text
This is simple, deterministic, and easy to vectorize.
```

No-go:

```text
If this rule changes, all vectors must be regenerated.
```

---

## 9. Proof format, deferred but bounded

Future account proof must include:

```text
schema
version
checkpoint_hash
state_root
account_leaf
account_leaf_hash
sort_key
sibling_path
path_directions
domain_separator
hash_algorithm
```

Future hold proof must include:

```text
schema
version
account_id
holds_root
hold_leaf
hold_leaf_hash
sort_key
sibling_path
path_directions
domain_separator
hash_algorithm
```

Rules:

```text
- no proof may omit domain separator
- no proof may omit hash algorithm
- no proof may rely on implicit tree rules
```

---

## 10. Why not sparse Merkle tree first?

Sparse Merkle tree is attractive later because:

```text
stable proof size
non-inclusion proofs
large keyspace support
```

But it is deferred because it requires early decisions on:

```text
key hashing
path length
empty subtree constants
non-inclusion proofs
path compression if any
update semantics
migration story
```

Those are too many consensus-critical choices before internal ROC is proven.

Phase 1 should optimize for:

```text
boring replay
small surface
clear vectors
independent verification
```

---

## 11. Phase 1 state-root no-go list

Do not implement state roots until:

```text
[ ] AccountStateV1 canonical bytes exist
[ ] HoldStateV1 canonical bytes exist
[ ] EmptyTreeV1 canonical bytes exist
[ ] account leaf sort vector exists
[ ] hold leaf sort vector exists
[ ] duplicate key rejection vector exists
[ ] unordered input same root vector exists
[ ] odd leaf duplication vector exists
[ ] empty tree root vector exists
[ ] closed-hold compaction vector exists
[ ] independent verifier can reproduce vectors
```

---

## 12. Remaining open decisions

These remain open but narrowed:

```text
[ ] exact AccountStateV1 DTO field set
[ ] exact HoldStateV1 DTO field set
[ ] exact receipt tree order
[ ] exact account proof DTO
[ ] exact hold proof DTO
[ ] exact permissions_root model
```

These do not block this document as QC-0A-R1 guidance.

They block root-producing code.


<!-- END docs/quickchain/STATE_TREE_DECISION.md -->

---

<!-- BEGIN docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md -->

# File: `docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md`

# Concurrent Holds and Operation IDs

RO:WHAT — QC-0A model for idempotency, operation IDs, account sequences, concurrent holds, and closed-hold compaction.
RO:WHY — QuickChain must support multi-tab paid UX without nonce holes, duplicate commits, ambiguous replay, or unbounded hold bloat.
RO:INTERACTS — svc-wallet, ron-ledger, ron-proto quickchain DTOs, future account/hold roots, future receipt roots, future checkpoint replay.
RO:INVARIANTS — wallet remains mutation front-door; client idempotency keys are hints, not authority; replay must be deterministic; closed holds compact only through deterministic receipt-root rules.
RO:METRICS — future duplicate operation/idempotency reject counters, hold-expiry counters, hold-compaction counters.
RO:CONFIG — future maximum open holds, session budget limits, expiration epoch windows, idempotency retention windows.
RO:SECURITY — duplicate operation_id/idempotency_key must not create duplicate economic commits; hold compaction must not allow hold resurrection.
RO:TEST — future concurrent_holds_replay_vector_001, expired_hold_epoch_transition_vector_001, duplicate_operation_id_rejected, duplicate_idempotency_returns_original_receipt.

---

## 0. Status

This is a QC-0A-R1 decision record.

It is still pre-implementation.

It does not implement hold roots, receipts, account roots, hashes, validators, pruning, or settlement.

It narrows the model enough that future DTOs and golden vectors have a concrete target.

---

## 1. Rule

QuickChain must not rely on a fragile strict-single-nonce UX model.

A stuck paid action must not freeze unrelated CrabLink paid actions.

The system needs both:

```text
good UX:
  multiple tabs / views / streams can prepare paid actions concurrently

economic safety:
  retries do not double commit
  captures do not double spend
  releases do not resurrect funds
  holds do not bloat state forever
  replay is deterministic
```

---

## 2. Scope decisions

### 2.1 operation_id

Decision:

```text
operation_id is a backend-assigned durable ledger-operation identifier.
```

Properties:

```text
- unique across ledger mutation attempts
- treated as immutable replay input
- generated before ledger append
- never generated during root replay
- bound to canonical_operation_hash
- conflict checked if reused with different payload
- appears in every economic receipt
```

Phase 1 candidate string form:

```text
op_<32 lowercase hex>
```

Example:

```text
op_0123456789abcdef0123456789abcdef
```

Rules:

```text
- Client may send a suggested operation ID only in dev/testing flows.
- Production backend may ignore client suggestion and mint its own.
- Once accepted, operation_id is durable.
- Reusing operation_id with identical canonical_operation_hash returns the original receipt.
- Reusing operation_id with different canonical_operation_hash rejects as conflict.
```

No-go:

```text
operation_id must not be a UI-only nonce.
operation_id must not be generated during deterministic replay.
operation_id must not be scoped only to a browser session.
```

### 2.2 idempotency_key

Decision:

```text
idempotency_key is a retry key scoped by account_id + operation_family + canonical_operation_hash.
```

Purpose:

```text
- lets clients retry after timeout
- maps repeat requests to the original operation/receipt
- does not itself authorize spend
- does not replace operation_id uniqueness
```

Suggested scope tuple:

```text
(account_id, operation_family, idempotency_key)
```

Conflict rule:

```text
same scope + same canonical_operation_hash:
  return original receipt

same scope + different canonical_operation_hash:
  reject as idempotency conflict
```

Suggested string limits:

```text
- UTF-8 visible ASCII
- max 128 bytes
- no secrets
- no account IDs embedded by the client
```

No-go:

```text
Client idempotency keys are hints, not economic authority.
```

### 2.3 canonical_operation_hash

Decision:

```text
canonical_operation_hash commits to the operation intent before receipt fields exist.
```

It should eventually cover:

```text
operation_family
account_id
counterparty_account_id where applicable
asset
amount_minor
purpose
hold_id/session_budget_id where applicable
policy_hash
chain_params_hash where applicable
idempotency scope
```

It must not cover:

```text
receipt_hash
settlement_status
validator signatures
checkpoint fields
wall-clock data not part of the accepted operation
local UI state
```

Status:

```text
No hash is implemented in QC-0A.
The hash input must be frozen in TEST_VECTORS.md before code.
```

### 2.4 account_sequence

Decision:

```text
account_sequence is ledger-assigned at commit time.
```

Purpose:

```text
- deterministic account replay
- replay ordering inside one account
- audit/proof clarity
```

Rules:

```text
- account_sequence is not a preflight UX nonce.
- account_sequence is assigned by wallet/ledger commit path.
- account_sequence may be sequential internally without forcing UI to pre-reserve nonces.
- failed prepare attempts do not create user-visible nonce holes.
- each committed account-affecting receipt records the resulting account_sequence.
```

No-go:

```text
Do not require CrabLink to know the next account_sequence before a paid action.
```

### 2.5 hold_id

Decision:

```text
hold_id identifies one hold lifecycle.
```

Lifecycle:

```text
open -> captured
open -> released
open -> expired
```

Rules:

```text
- hold_id is assigned by backend wallet path.
- hold_id is unique within ledger replay history.
- hold_id is included in hold/capture/release receipts.
- hold_id cannot be reused to create a new hold after terminal state.
```

Suggested Phase 1 candidate string form:

```text
hold_<32 lowercase hex>
```

### 2.6 session_budget_id

Decision:

```text
session_budget_id groups multiple small paid captures under one user-confirmed budget.
```

Status:

```text
future optional model
not required for first paid site visit proof
not required for first hold root vector
```

Rules when introduced:

```text
- explicit user confirmation required
- maximum budget required
- expiration epoch required
- capture rules must be deterministic
- no uncapped spend authority
```

---

## 3. Replay identity

Future replay protection must include:

```text
account_id
operation_id
idempotency_key
operation_family
canonical_operation_hash
hold_id or session_budget_id when applicable
ledger sequence bounds
account_sequence
```

Authority:

```text
client idempotency key:
  retry hint only

server-side durable uniqueness:
  authority

svc-wallet:
  mutation front-door

ron-ledger:
  durable replay truth
```

---

## 4. Required behavior

```text
same idempotency_key + same operation:
  returns original receipt

same idempotency_key + different operation:
  rejects as idempotency conflict

same operation_id + same canonical_operation_hash:
  returns original receipt

same operation_id + different canonical_operation_hash:
  rejects as operation conflict

hold capture retry:
  returns original capture receipt

hold release retry:
  returns original release receipt

expired hold:
  handled by deterministic epoch/ledger rule, not local timer race
```

---

## 5. Concurrent hold model

### 5.1 Account balance fields

Account state must distinguish:

```text
balance_minor:
  total account balance including held funds

held_minor:
  sum of currently open holds

available_minor:
  balance_minor - held_minor
```

Rules:

```text
- opening a hold increases held_minor and decreases available_minor.
- capturing a hold decreases balance_minor and held_minor.
- releasing a hold decreases held_minor and increases available_minor.
- expiring a hold behaves like deterministic release.
- no field may become negative.
```

### 5.2 Open holds

Open holds are active state.

They belong in the future holds_root.

Minimum open hold fields:

```text
hold_id
account_id
counterparty_account_id
amount_minor
purpose
created_at_epoch
expires_at_epoch
status = "open"
operation_id
idempotency_key
policy_hash
```

Rules:

```text
- multiple open holds per account are allowed within policy limits.
- open holds are sorted by hold_id for root purposes.
- maximum open holds per account must be policy-limited.
- maximum total held_minor must not exceed balance_minor.
```

### 5.3 Terminal holds

Terminal states:

```text
captured
released
expired
```

Terminal hold receipts must exist.

Terminal holds must not remain in active state forever.

---

## 6. Closed-hold compaction rule

Decision:

```text
The active holds_root commits only to currently open holds.
```

Terminal hold history is proven through receipt_root, not through indefinite hold-state retention.

Compaction rule:

```text
1. A hold opens and appears in active holds_root.
2. A terminal receipt is committed:
   - capture
   - release
   - expiry release
3. The terminal receipt enters the receipt_root for the epoch.
4. At the deterministic epoch transition that includes the terminal receipt,
   the hold is removed from the next active holds_root.
5. Future proof of that hold lifecycle is through receipt inclusion proofs,
   not active hold state.
```

Safety requirements:

```text
- hold_id uniqueness remains durable.
- terminal receipt must be included before removal.
- replay must reject capture/release against a missing or already-terminal active hold.
- before pruning is proven, full receipt history remains available.
- after future pruning, receipt inclusion proof plus checkpoint proof must prove closure.
```

No-go:

```text
Do not compact a hold merely because local time passed.
Do not compact a hold before terminal receipt inclusion.
Do not keep closed holds forever in active state.
Do not allow hold_id reuse after compaction.
```

---

## 7. Expiration rule

Decision:

```text
Hold expiration is epoch-based, not wall-clock based.
```

A hold may include:

```text
created_at_epoch
expires_at_epoch
```

At epoch transition:

```text
if current_epoch >= expires_at_epoch and hold is still open:
  deterministic expiry release is eligible
```

Rules:

```text
- expiry order is stable sorted by hold_id.
- expiry effect must produce a durable receipt or equivalent replay artifact.
- expiry must not depend on local timers.
- expiry must not depend on scheduler order.
- expiry cannot spend funds; it releases held funds.
```

No-go:

```text
No root-producing transition may read system wall-clock to decide expiry.
```

---

## 8. Concrete replay vector sketch

This is a sketch, not a production vector.

It becomes a production vector only after canonical bytes, domains, and preimage framing are frozen in TEST_VECTORS.md.

### 8.1 Initial state

```text
account_id = "acct_visitor_b"
asset = "roc"
balance_minor = "1000"
held_minor = "0"
available_minor = "1000"
account_sequence = 0
open_holds = []
```

### 8.2 Operations

```text
1. open hold H1
   hold_id = "hold_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
   operation_id = "op_11111111111111111111111111111111"
   amount_minor = "100"
   expires_at_epoch = "epoch_0002"

2. open hold H2
   hold_id = "hold_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
   operation_id = "op_22222222222222222222222222222222"
   amount_minor = "250"
   expires_at_epoch = "epoch_0002"

3. capture H2
   operation_id = "op_33333333333333333333333333333333"

4. retry capture H2 with same idempotency_key
   expected: return original capture receipt, no balance change

5. release H1
   operation_id = "op_44444444444444444444444444444444"

6. retry release H1 with same idempotency_key
   expected: return original release receipt, no balance change
```

### 8.3 Expected human-readable state transitions

```text
after open H1:
  balance_minor = "1000"
  held_minor = "100"
  available_minor = "900"
  open_holds = [H1]

after open H2:
  balance_minor = "1000"
  held_minor = "350"
  available_minor = "650"
  open_holds = [H1, H2]

after capture H2:
  balance_minor = "750"
  held_minor = "100"
  available_minor = "650"
  open_holds = [H1]

after retry capture H2:
  no state change

after release H1:
  balance_minor = "750"
  held_minor = "0"
  available_minor = "750"
  open_holds = []

after retry release H1:
  no state change
```

### 8.4 Expected proof implications

```text
H1 open receipt is in receipt_root.
H2 open receipt is in receipt_root.
H2 capture receipt is in receipt_root.
H1 release receipt is in receipt_root.
After terminal receipts are included, active holds_root is empty.
Closed hold lifecycle remains provable by receipt inclusion.
```

---

## 9. Open decisions remaining

These are deliberately narrowed.

```text
[ ] exact canonical OperationIntentV1 DTO fields
[ ] exact canonical HoldStateV1 DTO fields
[ ] exact idempotency conflict error wire name
[ ] exact expiry receipt op_class
[ ] exact session_budget_id model, deferred until needed
```

These do not block this document from serving as QC-0A-R1 guidance.

They do block root-producing code.

---

## 10. No-go

Do not implement hold roots until:

```text
[ ] canonical HoldStateV1 bytes exist
[ ] canonical OperationIntentV1 bytes exist
[ ] concurrent_holds_replay_vector_001 exists
[ ] expired_hold_epoch_transition_vector_001 exists
[ ] duplicate_operation_id_rejected exists
[ ] duplicate_idempotency_returns_original_receipt exists
[ ] closed-hold compaction vector exists
```


<!-- END docs/quickchain/CONCURRENT_HOLDS_AND_OPERATION_IDS.md -->

---

<!-- BEGIN docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md -->

# File: `docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md`

# Event Classification and Rewards

RO:WHAT — QC-0A event-class and reward-safety rules.
RO:WHY — Prevent fakeable engagement metrics from becoming protocol ROC payout truth.
RO:INTERACTS — ron-accounting, svc-rewarder, svc-wallet, ron-policy, future QuickChain accounting/reward roots.
RO:INVARIANTS — accounting is not balance truth; rewarder does not mutate ledger; raw engagement does not directly mint or allocate protocol ROC.
RO:METRICS — future raw engagement reward reject counters.
RO:CONFIG — future reward feature gates; raw engagement reward remains disabled.
RO:SECURITY — reward formulas must identify funding source and reject ambiguous event classes.
RO:TEST — future raw_engagement_reward_formula_rejected.

## Event classes

```text
economic_receipt:
  payer-authorized wallet event or ledger mutation.
  Can affect balances only through svc-wallet/ron-ledger.

metering:
  usage measurement.
  Cannot mutate balances.

proof_eligible:
  storage/provider/carrier/archive proof candidate.
  Requires verification/challenge before reward planning.

ad_budgeted:
  advertiser/sponsor-funded event.
  Paid from explicit budget, not raw protocol mint.

analytics_only:
  display/reporting signal.
  Never enters protocol ROC reward allocation.
```

## Forbidden reward bases

```text
total_site_visits share
raw_watch_seconds share
raw_ad_impressions share
raw_click_count share
unverified active user share
```

## Safer reward bases

```text
payer-authorized purchases
advertiser-funded campaign budget with fraud scoring
storage proofs with challenge verification
provider egress proofs with bounded accounting
governance-approved grants with hard caps
creator-defined payout splits from explicit revenue
```

## Required reward manifest rules

```text
funding source required
policy hash required
input accounting root required
deterministic payout plan required
wallet commits payout
rewarder does not mutate balances
same epoch cannot double issue
```

## No-go

Do not implement reward roots until event-class enforcement and forbidden reward formula tests exist.


<!-- END docs/quickchain/EVENT_CLASSIFICATION_AND_REWARDS.md -->

---

<!-- BEGIN docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md -->

# File: `docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md`

# Retention and Data Availability Preconditions

RO:WHAT — QC-0A retention, pruning, archive, carrier, and DA safety rules.
RO:WHY — Prevent unsafe pruning and fake DA before proof/challenge/archive fallback is proven.
RO:INTERACTS — ron-ledger, ron-accounting, svc-storage, future carrier/archive layer, future QuickChain checkpoints.
RO:INVARIANTS — no pruning before DA/challenge/archive fallback; economic receipts outrank raw engagement telemetry.
RO:METRICS — future pruning blocked and missing DA challenge counters.
RO:CONFIG — future hot/warm/cold/archive retention windows.
RO:SECURITY — checkpoint roots alone do not justify deletion of raw proof data.
RO:TEST — future da_missing_blocks_pruning and restore-from-proof tests.

## Pre-DA rule

Before DA is proven, retention must be boring.

```text
No pruning in local-root mode.
No pruning in epoch-sealing mode.
No pruning merely because a checkpoint root exists.
No pruning merely because a carrier assignment exists.
No pruning until missing-data challenges work.
No pruning until archive fallback is tested.
No pruning until restore-from-proof is tested.
```

## Pre-DA scale rule

Before DA is proven, QuickChain must not promise web-scale raw event retention.

High-volume raw events must be:

```text
analytics-only local retention
aggregated sealed summaries
sampled metering
bounded experiment data
explicitly funded campaign data
proof-relevant data with retention cap
```

Economic receipts take priority over engagement telemetry.

## Pruning preconditions

```text
[ ] checkpoint roots exist
[ ] challenge window is defined
[ ] receipt inclusion proofs exist
[ ] account proofs exist
[ ] missing data challenge exists
[ ] archive fallback exists
[ ] restore-from-proof test passes
[ ] DA root/proof format exists if pruning depends on DA
```

## No-go

If missing DA cannot block pruning, pruning must remain disabled.


<!-- END docs/quickchain/RETENTION_AND_DA_PRECONDITIONS.md -->

---

<!-- BEGIN docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md -->

# File: `docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md`

# Phase 2 to Phase 3 Chasm

RO:WHAT — QC-0A warning document for the local-root to replicated-validator transition.
RO:WHY — Moving from deterministic local roots to a validator committee is a major architectural boundary, not a toggle.
RO:INTERACTS — ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-registry, ron-kms, future validators.
RO:INVARIANTS — no committee until artifacts are independently replayable; no validator signing until checkpoint hashes reproduce.
RO:METRICS — future replay and quorum verification counters.
RO:CONFIG — future settlement_mode and consensus mode.
RO:SECURITY — validators must not trust original operator state; they must replay canonical artifacts.
RO:TEST — future validator simulator and independent replay tests.

## Chasm rule

Phase 2 local epoch sealing must be validator-executable before Phase 3 begins.

Phase 3 is blocked until:

```text
root-producing execution is pure and deterministic
all inputs are canonical
all outputs have test vectors
state transition can be replayed outside the original service
no wall-clock reads affect roots
no DB-order assumptions affect roots
operator tools can export/import epoch artifacts
validator simulator reproduces checkpoint hash
```

## Required artifact export

Future Phase 2 must export:

```text
checkpoint header bytes
receipt batches
account state leaves
hold state leaves
accounting windows
reward manifests
policy hash input
chain params hash input
validator set hash input if used
```

## No-go

If a validator cannot independently reproduce the checkpoint hash, Phase 3 must not begin.


<!-- END docs/quickchain/PHASE_2_TO_PHASE_3_CHASM.md -->

---

<!-- BEGIN docs/quickchain/TEST_VECTORS.md -->

# File: `docs/quickchain/TEST_VECTORS.md`

# QuickChain Test Vectors

RO:WHAT — QC-0A format and acceptance rules for future QuickChain canonical byte/hash/root vectors.
RO:WHY — No root-producing code may ship without independent replayable vectors.
RO:INTERACTS — ron-proto canonical JSON helpers, future ron-ledger roots, future receipt/account/hold/checkpoint hashes.
RO:INVARIANTS — no fake hashes; no unstable golden vectors; no Phase 1 roots until vectors are frozen and independently reproducible.
RO:METRICS — none.
RO:CONFIG — none.
RO:SECURITY — fake vectors create false confidence and are forbidden.
RO:TEST — future canonical_receipt_vector_001, canonical_account_state_vector_001, canonical_hold_state_vector_001, checkpoint_hash_vector_001.

---

## 0. Status

This is a QC-0A-R1 vector-format decision record.

No production hash vectors exist yet.

This is intentional.

Canonical JSON byte tests may exist now, but hash/root vectors must wait until:

```text
canonical byte rules are frozen
domain separator strings are frozen
preimage framing is frozen
receipt/account/hold/checkpoint schemas are frozen
BLAKE3 call site is placed in the correct crate/layer
independent replay expectations are documented
```

---

## 1. Vector status levels

Vectors use explicit status.

```text
sketch:
  human-readable scenario only
  may contain placeholders
  cannot drive root-producing tests

locked_bytes:
  canonical_payload_utf8 and canonical_payload_hex are frozen
  no expected_b3 required yet
  may drive canonical byte tests

locked_hash:
  preimage_hex and expected_b3 are frozen
  may drive hash/root tests
```

No-go:

```text
Do not use sketch vectors as production tests.
Do not use placeholder expected_b3 values.
Do not call a vector golden until it is locked_hash.
```

---

## 2. Vector schema

Future vectors must use this outer shape.

```json
{
  "schema": "quickchain.test-vector.v1",
  "vector_id": "canonical_account_state_vector_001",
  "status": "locked_bytes",
  "purpose": "account_state_canonical_bytes",
  "domain_separator": "quickchain.account-state.v1",
  "canonical_encoding": "quickchain.canonical-json.v1",
  "preimage_framing": "domain_separator_bytes || 0x00 || canonical_payload_bytes",
  "hash_algorithm": "blake3-256",
  "human_readable_json": {},
  "canonical_payload_utf8": "",
  "canonical_payload_hex": "",
  "preimage_hex": null,
  "expected_b3": null,
  "notes": []
}
```

Required fields:

```text
schema
vector_id
status
purpose
domain_separator
canonical_encoding
preimage_framing
hash_algorithm
human_readable_json
canonical_payload_utf8
canonical_payload_hex
preimage_hex
expected_b3
notes
```

Rules:

```text
- status must be sketch, locked_bytes, or locked_hash.
- locked_bytes requires canonical_payload_utf8 and canonical_payload_hex.
- locked_hash requires preimage_hex and expected_b3.
- expected_b3 must be b3:<64 lowercase hex> for locked_hash.
- preimage_hex must equal hex(domain || 0x00 || payload) for locked_hash.
- human_readable_json is explanatory and not used as authority.
- canonical_payload_utf8 and canonical_payload_hex are the byte authority.
```

---

## 3. Preimage framing

Decision:

```text
preimage = domain_separator_bytes || 0x00 || canonical_payload_bytes
```

Example:

```text
domain_separator:
  quickchain.account-state.v1

delimiter:
  00

canonical_payload_utf8:
  {"schema":"quickchain.account-state.v1","account_id":"acct_visitor_b"}
```

The preimage bytes are:

```text
utf8("quickchain.account-state.v1")
+
00
+
utf8(canonical_payload_utf8)
```

No-go:

```text
No hash/root test may use a different preimage frame.
```

---

## 4. Vector file layout

Recommended future directory:

```text
docs/quickchain/vectors/
  canonical_account_state_vector_001.json
  canonical_hold_state_vector_001.json
  canonical_receipt_vector_001.json
  receipt_hash_domain_separation_001.json
  state_root_determinism_vector_001.json
  receipt_root_determinism_vector_001.json
  checkpoint_hash_vector_001.json
  concurrent_holds_replay_vector_001.json
  expired_hold_epoch_transition_vector_001.json
  duplicate_operation_id_rejected_001.json
  duplicate_idempotency_returns_original_receipt_001.json
```

This directory does not need to exist until vectors are actually frozen.

---

## 5. First required vectors

### 5.1 Canonical bytes

```text
canonical_receipt_vector_001
canonical_account_state_vector_001
canonical_hold_state_vector_001
canonical_operation_intent_vector_001
canonical_empty_tree_vector_001
```

### 5.2 Domain separation

```text
receipt_hash_domain_separation_001
account_leaf_domain_separation_001
hold_leaf_domain_separation_001
checkpoint_hash_domain_separation_001
```

### 5.3 Roots

```text
state_root_determinism_vector_001
receipt_root_determinism_vector_001
hold_root_determinism_vector_001
checkpoint_hash_vector_001
unordered_input_same_root_001
odd_leaf_duplication_vector_001
empty_tree_root_vector_001
duplicate_sort_key_rejected_001
```

### 5.4 Wallet/replay behavior

```text
concurrent_holds_replay_vector_001
expired_hold_epoch_transition_vector_001
closed_hold_compaction_vector_001
duplicate_operation_id_rejected_001
duplicate_idempotency_returns_original_receipt_001
idempotency_conflict_rejected_001
```

### 5.5 Reward/event safety

```text
unknown_field_rejected_001
float_money_rejected_001
invalid_money_string_rejected_001
ambiguous_event_class_rejected_001
raw_engagement_reward_formula_rejected_001
reward_epoch_replay_no_double_issue_001
```

---

## 6. Draft vector sketch: account state

Status:

```text
sketch
```

Human-readable payload target:

```json
{
  "schema": "quickchain.account-state.v1",
  "account_id": "acct_visitor_b",
  "asset": "roc",
  "balance_minor": "750",
  "held_minor": "0",
  "available_minor": "750",
  "account_sequence": 4,
  "receipt_root": "b3:1111111111111111111111111111111111111111111111111111111111111111",
  "holds_root": "b3:2222222222222222222222222222222222222222222222222222222222222222",
  "permissions_root": null,
  "updated_at_epoch": "epoch_0001"
}
```

Required future locked_bytes vector:

```text
canonical_payload_utf8 must be exact minified JSON.
canonical_payload_hex must be exact UTF-8 hex.
```

Required future locked_hash vector:

```text
domain_separator = "quickchain.account-state.v1"
preimage_hex = hex(domain || 00 || canonical_payload)
expected_b3 = b3:<64 lowercase hex>
```

---

## 7. Draft vector sketch: concurrent holds

Status:

```text
sketch
```

Initial state:

```text
account_id = "acct_visitor_b"
balance_minor = "1000"
held_minor = "0"
available_minor = "1000"
account_sequence = 0
open_holds = []
```

Operations:

```text
open H1 amount 100
open H2 amount 250
capture H2
retry capture H2
release H1
retry release H1
```

Expected final state:

```text
balance_minor = "750"
held_minor = "0"
available_minor = "750"
open_holds = []
```

Expected safety properties:

```text
retry capture does not double spend
retry release does not change balance
closed holds are absent from active holds_root
terminal hold lifecycle is proven through receipt_root
```

Required future locked vectors:

```text
concurrent_holds_replay_vector_001
closed_hold_compaction_vector_001
duplicate_idempotency_returns_original_receipt_001
```

---

## 8. Production vector acceptance rule

A vector is production-ready only if:

```text
[ ] schema is quickchain.test-vector.v1
[ ] status is locked_hash
[ ] canonical_payload_utf8 is exact
[ ] canonical_payload_hex matches canonical_payload_utf8 bytes
[ ] domain_separator is exact and validated
[ ] preimage_framing is exact
[ ] preimage_hex matches domain || 00 || payload
[ ] hash_algorithm is blake3-256
[ ] expected_b3 is b3:<64 lowercase hex>
[ ] independent verifier reproduces expected_b3
[ ] vector is stable across macOS/Linux
[ ] vector does not depend on map/DB iteration order
```

---

## 9. No-go

If an independent verifier cannot reproduce the same:

```text
receipt_hash
state_root
receipt_root
accounting_root
reward_root
checkpoint_hash
```

from the published vectors, QuickChain Phase 1 must not start.


<!-- END docs/quickchain/TEST_VECTORS.md -->

---

<!-- BEGIN docs/quickchain/NO_GO_LIVE_CHECKLIST.md -->

# File: `docs/quickchain/NO_GO_LIVE_CHECKLIST.md`

# QuickChain No-Go-Live Checklist

RO:WHAT — Explicit checklist of conditions that block QuickChain from live settlement use.
RO:WHY — Prevent accidental launch of chain behavior before proofs, governance, replay, and internal ROC are ready.
RO:INTERACTS — QUICKCHAIN.MD, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-registry, ron-kms, CrabLink.
RO:INVARIANTS — no live chain; no fake balances; no fake receipts; no public bridge; no external settlement.
RO:METRICS — future readiness and no-go counters only.
RO:CONFIG — QuickChain must remain disabled until every required gate is green.
RO:SECURITY — any unchecked item blocks live QuickChain.
RO:TEST — review checklist plus future automated readiness gates.

## Status

QuickChain is blocked from live settlement.

This is intentional.

## Hard blockers

QuickChain must not go live while any item below is unchecked.

### Internal ROC economy

- [ ] Visitor B paid action decreases Visitor B balance through svc-wallet
- [ ] Creator A balance increases through backend wallet/ledger truth
- [ ] CrabLink displays backend-derived receipt
- [ ] No fake receipt path exists in CrabLink
- [ ] Paid unlock requires backend truth
- [ ] ron-ledger replay proves balances
- [ ] svc-wallet idempotency prevents double commit
- [ ] hold/capture/release is replay-safe

### Accounting and rewards

- [ ] ron-accounting sealed windows are deterministic
- [ ] sealed accounting snapshots have stable roots
- [ ] svc-rewarder produces deterministic manifests
- [ ] rewarder does not mutate balances directly
- [ ] wallet commits reward intents and returns receipts
- [ ] reward replay cannot double issue

### Canonicalization and roots

- [ ] canonical byte encoding is frozen
- [ ] checkpoint byte vectors are golden
- [ ] account-state byte vectors are golden
- [ ] receipt byte vectors are golden
- [ ] state root vectors are golden
- [ ] receipt root vectors are golden
- [ ] domain separation strings are frozen
- [ ] no map-order ambiguity remains

### Validators and governance

- [ ] validator set comes from svc-registry
- [ ] validator set changes are quorum-gated
- [ ] validator signature preimage is domain-separated
- [ ] validator double-sign guard exists
- [ ] equivocation evidence exists
- [ ] emergency freeze path exists
- [ ] audit records exist for governance actions

### Data availability and pruning

- [ ] receipt batch chunks are stored and retrievable
- [ ] accounting snapshot chunks are stored and retrievable
- [ ] reward manifest chunks are stored and retrievable
- [ ] data availability challenges exist
- [ ] archive fallback exists
- [ ] pruning remains disabled until proof restore tests pass
- [ ] challenge window is enforced before pruning

### External anchors

- [ ] no ROX/Solana anchor in active MVP
- [ ] no bridge mutates ROC balances
- [ ] anchor verification is read-only
- [ ] external anchor is not required for hot-path UX
- [ ] public bridge has independent audit before launch

### CrabLink

- [ ] CrabLink remains display/user intent only
- [ ] no validator private keys in CrabLink
- [ ] no chain authority in React
- [ ] no paid unlock from cached proof alone
- [ ] proof viewers clearly label unverified/dev data

## Rule

If any checkbox is unchecked, QuickChain is not live.


<!-- END docs/quickchain/NO_GO_LIVE_CHECKLIST.md -->

---



<!-- END QUICKCHAIN_QC0A_COMBINED.MD -->

