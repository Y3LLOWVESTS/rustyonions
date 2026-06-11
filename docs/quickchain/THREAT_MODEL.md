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
