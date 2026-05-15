# QUICKCHAIN.MD — ROC Fast-Finality, Receipt-First, Prunable Ledger Blueprint

RO:WHAT — Defines the future distributed ledger / checkpoint-chain design for ROC after the internal value plane is proven.
RO:WHY — ROC needs a serious high-trust, blazing-fast, efficient, verifiable settlement layer without putting every site visit directly on a global chain.
RO:INTERACTS — svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, svc-registry, svc-passport, ron-auth, svc-storage, svc-gateway, omnigate, svc-index, svc-dht, svc-edge, CrabLink, future ROX/Solana anchor.
RO:INVARIANTS — ROC remains internal until proven; wallet remains mutation front-door; ledger remains truth; accounting is not balance truth; rewarder does not mutate ledger; integer minor units only; no fake receipts; no silent spend.
RO:METRICS — Future checkpoint, validator, carrier, archive, receipt, proof, reward, pruning, and anchor metrics.
RO:CONFIG — Future chain params, epoch duration, validator set, carrier retention, checkpoint cadence, challenge windows, ROX anchor cadence, and pruning horizons.
RO:SECURITY — Feature-gated until internal ROC is proven; no external settlement during current WEB3/NEXT_LEVEL MVP; no privacy claims without real protocol; no public bridge until audited.
RO:TEST — Future deterministic replay tests, state-root tests, receipt inclusion proofs, validator quorum tests, pruning tests, carrier proof tests, ROX anchor verification tests, fork/challenge tests.

---

# 0. Status

`QUICKCHAIN.MD` is a **future-chain blueprint**.

It is **not** an instruction to implement blockchain logic immediately.

Current active project order remains:

```text
1. Finish internal ROC closed loop.
2. Prove paid storage/pinning.
3. Prove paid image/site/post/comment/article flows.
4. Prove Visitor B pays Creator A for site visit.
5. Prove wallet receipts, ledger replay, and balance truth.
6. Harden ron-accounting → svc-rewarder → svc-wallet reward loop.
7. Only then begin QuickChain research/prototype.
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
```

QuickChain is the **future distributed settlement design** for ROC after RustyOnions proves the internal creator economy.

---

# 1. One-sentence thesis

QuickChain is a **high-trust, fast-finality, receipt-first ROC ledger**:

```text
hot actions produce signed receipts immediately;
epochs settle netted deltas into verifiable state roots;
history is pruned only through signed checkpoint proofs;
ROX/Solana may later anchor compact checkpoint commitments, not raw ROC history.
```

Short form:

```text
Receipt-first.
Epoch-settled.
State-root verified.
Chunk-carried.
Passport-trusted.
Prunable by proof.
Externally anchorable.
```

---

# 2. Why QuickChain exists

RustyOnions has a different economic shape than generic blockchains.

ROC must support:

```text
site visits
paid content views
image views
article views
comment/post interactions
music plays
video watch seconds
storage byte-days
pinning
provider rewards
creator payouts
curator splits
ad impressions
moderation/reputation effects
validator rewards
carrier/archive rewards
```

A naive blockchain design would do:

```text
every site visit = one global on-chain transaction
```

That is probably not viable.

QuickChain instead does:

```text
every site visit = signed receipt / usage event / wallet access proof
many receipts = accounting window
accounting window = Merkleized snapshot
snapshot + wallet deltas = epoch transition
epoch transition = validator-signed checkpoint
checkpoint = prunable/verifiable chain state
optional ROX/Solana transaction = compact external anchor
```

This lets RustyOnions be fast without becoming fake, centralized, or unverifiable.

---

# 3. Design goals

## 3.1 High trust

High trust means:

```text
cryptographically verifiable
passport/reputation aware
validator accountable
wallet/ledger replayable
fraud-resistant
governance-gated during beta
```

High trust does **not** mean:

```text
trust one server forever
trust CrabLink balances
trust gateway balances
trust unaudited validator claims
trust unprovable reward claims
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
signed receipt
local wallet receipt
later epoch settlement
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
prunable checkpoint history
```

QuickChain should never require all nodes to carry all historical raw site visits forever.

## 3.4 Verifiable

Every balance and receipt must eventually be provable through:

```text
account proof
receipt inclusion proof
epoch transition proof
state root proof
validator signature proof
data availability proof
optional ROX/Solana anchor proof
```

Garbage collection is allowed only when proof roots remain.

---

# 4. Relationship to current ROC architecture

QuickChain must evolve the existing architecture, not replace it.

Current and future boundaries:

```text
svc-wallet:
  normal mutation front-door
  issue, transfer, burn, hold, capture, release
  nonce/idempotency
  wallet receipts
  account balances

ron-ledger:
  durable economic truth
  append-only/replayable log
  deterministic balance state
  future state-root engine

ron-accounting:
  usage events
  metering
  sealed snapshots
  reward-compatible exports
  not balance truth

svc-rewarder:
  deterministic reward planning
  payout manifests
  wallet issue/transfer intents
  no direct ledger mutation

ron-policy:
  side-effect-free pricing/payout/rule authority
  basis points
  integer minor units
  feature gates

svc-gateway:
  public route surface
  quotas/admission
  proxy only for product routes
  no wallet/ledger mutation

omnigate:
  product coordinator / hydrator
  quote/access coordinator
  no durable economic truth

CrabLink:
  thin browser client
  gateway-only
  explicit user confirmation
  no private-key custody in MVP
  no fake receipts or balances
```

QuickChain adds:

```text
validator committee
checkpoint blocks
receipt roots
state roots
account proofs
carrier/archive chunks
pruning rules
optional ROX/Solana roots
```

It does **not** move economic mutation into gateway, omnigate, CrabLink, accounting, or rewarder.

---

# 5. Core architecture

QuickChain has two speeds.

## 5.1 Speed 1 — Hot user path

Used for immediate UX.

```text
CrabLink
→ svc-gateway
→ omnigate quote/access route
→ svc-wallet hold/transfer/access ticket
→ ron-ledger local receipt
→ CrabLink unlocks only after backend truth
```

Examples:

```text
Visitor B opens crab://ron7.
Visitor B confirms 10 ROC site_visit.
svc-wallet returns receipt.
CrabLink unlocks site.
Creator A balance updates by backend truth.
```

This is fast and user-visible.

## 5.2 Speed 2 — Final settlement path

Used for distributed consensus and pruning.

```text
wallet receipts
+ accounting windows
+ reward manifests
+ policy hash
→ epoch transition
→ state root
→ receipt root
→ accounting root
→ reward root
→ validator signatures
→ checkpoint block
```

This is the blockchain-grade settlement layer.

---

# 6. Main data flow

```text
1. User action occurs
   site_visit / content_view / storage_put / ad_view / provider_serve

2. svc-wallet produces receipt or hold/capture/release
   receipt has txid, account, amount, op, nonce, idempotency, receipt hash

3. ron-accounting records usage event
   normalized labels, bounded fields, integer counters

4. accounting window seals
   canonical snapshot bytes
   b3:<snapshot_hash>
   accounting_snapshot_root

5. svc-rewarder computes deterministic rewards
   reward_manifest_root
   settlement intents

6. svc-wallet commits payout/issue/transfer
   wallet receipts generated

7. ron-ledger produces epoch transition
   previous_state_root → new_state_root

8. validators sign checkpoint
   quorum signatures over compact block header

9. carrier/archive nodes store chunks
   receipt batches, accounting snapshots, reward manifests, ledger chunks

10. optional ROX/Solana anchor
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

  previous_checkpoint_hash: b3:<64hex>
  previous_state_root: b3:<64hex>
  new_state_root: b3:<64hex>

  receipt_root: b3:<64hex>
  accounting_snapshot_root: b3:<64hex>
  reward_manifest_root: b3:<64hex>
  data_availability_root: b3:<64hex>

  policy_hash: b3:<64hex>
  validator_set_hash: b3:<64hex>
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
```

---

# 8. Account state model

QuickChain should use an account-based state model, not UTXO.

Reason:

```text
RustyOnions already has passports, wallet accounts, balances, nonces, holds, receipts, and creator payouts.
```

Proposed state:

```text
AccountStateV1 {
  account_id: string
  asset: "roc"
  balance_minor: string
  nonce: u64
  holds_root: b3:<64hex>
  receipt_root: b3:<64hex>
  permissions_root: b3:<64hex>
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
checkpoint X signed by validator quorum
optional anchor proves checkpoint X existed at external time Y
```

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
site_visit
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
  op: string
  from: string | null
  to: string | null
  asset: "roc"
  amount_minor: string
  nonce: u64 | null
  idempotency_key: string
  memo: string | null
  produced_at_ms: u64
  ledger_seq_start: u64
  ledger_seq_end: u64
  previous_ledger_root: b3:<64hex>
  new_ledger_root: b3:<64hex>
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
→ validator signatures
→ optional ROX/Solana anchor
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

These become accounting events:

```text
UsageEventV1 {
  schema: "quickchain.usage-event.v1"
  event_id: string
  action: string
  account_id: string
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
```

---

# 12. Consensus model

QuickChain should not begin as a fully permissionless public chain.

Recommended phases:

```text
Phase 1: single authority internal ledger
Phase 2: small replicated validator committee
Phase 3: passport-gated validator set
Phase 4: bonded/slashable validators
Phase 5: broader validator market
```

Initial consensus target:

```text
BFT-style validator committee
quorum signatures over checkpoint headers
deterministic state transition verification
fork detection
manual governance override during beta
```

Validator responsibilities:

```text
- verify previous checkpoint
- verify receipt roots
- verify accounting roots
- verify reward roots
- verify policy hash
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
```

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
future ROC bond
future slashing policy
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

---

# 14. Carrier/archive model

QuickChain should let normal users earn ROC by doing measurable useful work.

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
- Fraudulent proofs are slashable later.
```

---

# 15. Garbage collection / pruning

QuickChain must be prunable by proof.

Unsafe garbage collection:

```text
delete old history and trust current balances
```

Safe garbage collection:

```text
raw receipts/events
→ Merkle batches
→ checkpoint roots
→ validator signatures
→ challenge window expires
→ raw history becomes optional/archive/carrier responsibility
→ current state remains provable
```

Core rule:

```text
Raw history may be pruned only after it is committed into signed checkpoint roots and the challenge window has expired.
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
assigned carrier chunks
```

Archive node keeps:

```text
everything
```

---

# 16. Data availability

Checkpoint validity depends on data availability.

A checkpoint must include:

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
```

---

# 17. ROX/Solana anchor model

ROX/Solana is future/deferred.

When allowed, Solana should not store ROC history directly.

Bad design:

```text
store raw ROC blocks, receipts, and site visits inside Solana transactions
```

Good design:

```text
store compact ROC checkpoint commitments
```

ROX/Solana anchor payload should contain only compact fields such as:

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
ROX anchors proof that the facts existed.
```

Proof path:

```text
wallet receipt
→ receipt batch root
→ epoch receipt_root
→ QuickChain checkpoint_hash
→ validator quorum signatures
→ ROX/Solana anchor transaction
```

This gives:

```text
fast internal ROC
cheap high-volume site visits
prunable history
external timestamp/finality anchor
future public audit bridge
```

ROX/Solana must remain:

```text
optional
deferred
feature-gated
not required for internal ROC beta
not a hot-path dependency for page loads
not a replacement for QuickChain consensus
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
validator equivocation
carrier unavailable chunk
archive proof failure
policy hash mismatch
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
unauthorized_issue
unauthorized_burn
double_spend
validator_equivocation
carrier_failure
```

Beta handling:

```text
manual governance quarantine
checkpoint pause
validator set review
operator alert
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

[quickchain.consensus]
mode = "committee"
quorum_bps = 6700
min_validators = 4
max_validators = 21
passport_required = true
bond_required = false

[quickchain.retention]
hot_epochs = 288
warm_epochs = 8640
carrier_assignment = true
archive_required = false

[quickchain.rox_anchor]
enabled = false
cadence_epochs = 288
payload = "checkpoint_commitment_v1"
```

Rules:

```text
- QuickChain disabled by default.
- ROX anchor disabled by default.
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

## Phase 1 — Local checkpoint roots

Goal:

```text
ron-ledger can produce state_root and receipt_root locally
```

Tasks:

```text
canonical receipt hashing
account state tree
state root generation
receipt root generation
checkpoint DTO
local replay proves same roots
tests for deterministic roots
```

No distributed consensus yet.

## Phase 2 — Epoch sealing

Goal:

```text
ledger/accounting/rewarder create epoch artifacts
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
```

## Phase 3 — Replicated committee

Goal:

```text
multiple validators sign the same checkpoint
```

Tasks:

```text
validator identity
validator set hash
checkpoint proposal
signature collection
quorum verification
fork detection
operator tools
```

## Phase 4 — Carrier/archive chunks

Goal:

```text
nodes carry ledger/proof chunks and earn ROC for useful work
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

## Phase 5 — Pruning

Goal:

```text
normal nodes can prune old raw history safely
```

Tasks:

```text
retention policy
challenge window
checkpoint proof verification
state snapshot restore
receipt proof lookup
archive fallback
```

## Phase 6 — ROX/Solana anchor

Goal:

```text
optional external anchor stores compact checkpoint commitments
```

Tasks:

```text
anchor payload DTO
anchor transaction writer
anchor verifier
proof path docs
ROX governance gate
strict no-hot-path dependency
```

## Phase 7 — Open validator market

Goal:

```text
broader validator set with bonding and slashing
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

## 21.1 Local correctness

```text
ledger replay produces identical balances
ledger replay produces identical state roots
receipt replay produces identical receipt roots
idempotency prevents duplicate commits
nonce rules prevent replay
conservation holds except authorized issue/burn
```

## 21.2 Epoch correctness

```text
same input receipts → same checkpoint hash
same accounting snapshot → same accounting root
same reward input → same reward root
net deltas match account state transition
policy hash is included
```

## 21.3 Consensus correctness

```text
validator quorum signs valid checkpoint
invalid checkpoint rejected
fork/equivocation detected
validator set changes are signed/versioned
checkpoint signatures verify offline
```

## 21.4 Pruning correctness

```text
old raw events can be pruned after challenge window
account proof still verifies
receipt proof still verifies
state snapshot restore works
archive fallback works
carrier chunks recover proof data
```

## 21.5 ROX/Solana anchor correctness

```text
anchor contains compact checkpoint commitment only
anchor can be verified offline
anchor proof links checkpoint hash to external chain transaction
ROC can operate if anchor is unavailable
anchor is not required for site load
```

---

# 22. Testing plan

Future tests:

```text
ron-ledger:
  state_root_determinism.rs
  receipt_root_determinism.rs
  epoch_transition_replay.rs
  idempotency_epoch.rs

ron-accounting:
  sealed_window_root.rs
  usage_event_batch_root.rs
  duplicate_ingest_no_double_count.rs

svc-rewarder:
  reward_manifest_root.rs
  reward_epoch_replay_no_double_issue.rs
  carrier_reward_plan.rs

quickchain:
  checkpoint_dto_strict.rs
  checkpoint_hash_vectors.rs
  validator_quorum.rs
  fork_detection.rs
  pruning_proof.rs
  receipt_inclusion_proof.rs
  account_state_proof.rs
  rox_anchor_payload.rs
```

Future smoke:

```text
Visitor B pays Creator A 10 ROC.
Receipt included in receipt batch.
Receipt batch included in checkpoint.
Checkpoint signed by validator quorum.
Old receipt raw batch pruned locally.
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
quickchain_validator_signatures_total{validator,result}
quickchain_quorum_failures_total{reason}
quickchain_forks_detected_total
quickchain_challenges_total{type,result}
quickchain_pruned_epochs_total
quickchain_carrier_chunks_assigned_total
quickchain_carrier_proofs_total{result}
quickchain_archive_chunks_total
quickchain_anchor_submissions_total{chain,result}
quickchain_anchor_verify_total{chain,result}
```

Metrics rules:

```text
- Do not put account IDs in labels.
- Do not put wallet txids in labels.
- Do not put CIDs in high-cardinality labels.
- Use route/result/reason buckets.
```

---

# 24. Security rules

```text
- No unsafe Rust unless separately justified and audited.
- No floating-point money math.
- No direct ledger mutation outside approved wallet/ledger boundary.
- No signature verification shortcuts.
- No unknown fields in chain DTOs.
- No pruning without signed checkpoint roots.
- No anchor without checkpoint verification.
- No external bridge until internal ROC is proven.
- No privacy claims without a real privacy protocol.
- No anonymous alt validators by default.
- No reward minting by carriers/providers/validators directly.
```

---

# 25. Open research questions

Important unresolved decisions:

```text
1. Consensus algorithm:
   Tendermint-like BFT?
   HotStuff-like BFT?
   Raft-like beta committee first?

2. Canonical encoding:
   canonical JSON first?
   CBOR later?
   fixed binary format?

3. State tree:
   simple Merkle map?
   sparse Merkle tree?
   account trie?

4. Challenge window:
   hours?
   days?
   longer for archive chunks?

5. Validator admission:
   governance-approved first?
   passport + reputation?
   ROC bond later?

6. Carrier assignment:
   random?
   reputation-weighted?
   geo-balanced?
   storage-capacity weighted?

7. Reward formula:
   uptime?
   proofs served?
   chunks carried?
   successful challenges?
   quality score?

8. ROX/Solana anchor cadence:
   every epoch?
   daily?
   high-value checkpoints only?

9. Privacy:
   what account data is public?
   how do alt accounts avoid accidental linkage?
   what is provable without exposing browsing history?

10. Migration:
   how does current ron-ledger local state become QuickChain genesis?
```

---

# 26. QuickChain genesis strategy

When ready, QuickChain should start from a signed genesis snapshot.

Genesis input:

```text
current ron-ledger replayed balances
current account nonces
current holds
current supply
current policy hash
current validator set
genesis timestamp
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
validator_set_hash = genesis validator set
signatures = genesis committee signatures
```

Genesis must be:

```text
replayable
auditable
signed
documented
not silently mutable
```

---

# 27. Plain-English summary

QuickChain is the future ROC blockchain design.

It should not put every site visit directly on a global chain.

Instead:

```text
Users get instant signed receipts.
Creators get fast visible payouts.
Usage events are batched into accounting windows.
Wallet deltas settle into epoch state roots.
Validators sign compact checkpoints.
Carrier/archive nodes store proof chunks.
Normal nodes prune old raw history safely.
ROX/Solana may later anchor checkpoint commitments.
```

The final design target:

```text
High trust.
Blazing fast.
Efficient.
Verifiable.
Prunable.
Externally anchorable.
Built specifically for RustyOnions.
```

---

# 28. Do not implement until

Do not start QuickChain implementation until these are true:

```text
[ ] paid site visit proof works end-to-end
[ ] Visitor B balance decreases
[ ] Creator A balance increases
[ ] wallet receipt displays in CrabLink
[ ] ron-ledger replay proves balances
[ ] ron-accounting has sealed usage snapshot proof
[ ] svc-rewarder can compute deterministic payout plan
[ ] wallet/rewarder replay does not double issue
[ ] product smoke scripts prove internal ROC loop
[ ] NEXT_LEVEL paid view patterns are stable enough to know real transaction volume
```

Until then, keep QuickChain as a design blueprint only.

---

# 29. Final north-star phrase

```text
QuickChain is not “a blockchain bolted onto RustyOnions.”

QuickChain is the future settlement spine of ROC:
a receipt-first, epoch-settled, state-root verified, chunk-carried,
passport-trusted, proof-prunable ledger for the content-addressed creator web.
```