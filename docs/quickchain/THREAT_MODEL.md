# QuickChain Threat Model

RO:WHAT — Threat model for future QuickChain settlement, roots, proofs, pruning, validators, carriers, archives, and anchors.
RO:WHY — Blockchain-grade settlement fails when threat models are added after implementation.
RO:INTERACTS — ron-proto, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, ron-policy, svc-registry, ron-kms, ron-audit, svc-storage.
RO:INVARIANTS — wallet remains mutation front-door; ledger remains truth; accounting is not balance truth; rewarder does not mutate balances.
RO:METRICS — future challenge/fork/quorum/data-availability metrics.
RO:CONFIG — future challenge windows, validator sets, chain params, carrier retention.
RO:SECURITY — no public bridge, no privacy claims, no validator market, no pruning until proofs work.
RO:TEST — future property/fuzz/loom/chaos tests per threat class.

## 1. Security posture

QuickChain is future-gated.

The current safe scope is:

- docs
- DTOs
- canonical byte experiments
- validation tests
- future root design

The current unsafe scope is:

- live consensus
- live validator signing
- public bridge
- external settlement
- staking
- liquidity
- pruning
- fake checkpoint proofs
- fake receipt proofs
- fake balances

## 2. Core threats

### 2.1 Canonicalization split-brain

Threat:

Same logical checkpoint produces different bytes on different nodes.

Defense:

- strict DTOs
- no unknown fields
- no floats
- no unordered maps in Phase 0 hashed DTOs
- exact byte tests
- domain-separated roots later

### 2.2 Double spend

Threat:

Same funds are committed twice.

Defense:

- svc-wallet remains mutation front-door
- ron-ledger remains durable truth
- idempotency keys
- account nonces
- replay tests
- no QuickChain direct mutation path

### 2.3 Replay attack

Threat:

Old signed payloads or receipts are accepted again.

Defense:

- chain_id in future signed payloads
- height/epoch in future signatures
- nonce/idempotency
- expiry for capabilities
- domain-separated signature preimages

### 2.4 Fake receipts

Threat:

UI, gateway, omnigate, accounting, rewarder, or QuickChain code invents receipt truth.

Defense:

- only wallet/ledger committed receipts count
- receipt display caches are display-only
- DTOs are not authority
- CrabLink cannot unlock paid content from fake/local proof data

### 2.5 Validator equivocation

Threat:

Validator signs two different checkpoints at the same height.

Defense later:

- equivocation evidence DTO
- validator local double-sign guard
- registry signer identity
- audit record
- challenge/quarantine path

### 2.6 Data availability failure

Threat:

Checkpoint is signed but receipt/accounting/reward chunks are missing.

Defense later:

- data_availability_root
- carrier/archive assignments
- challenge windows
- archive fallback
- reward withholding for missing data

### 2.7 Unsafe pruning

Threat:

Raw history is deleted before proofs are sufficient.

Defense later:

- pruning disabled by default
- signed checkpoints
- proof verification
- challenge window expiry
- archive recovery tests

### 2.8 Bridge exploit

Threat:

External anchor or bridge becomes source of truth.

Defense:

- no public bridge in current MVP
- no ROX/Solana in current MVP
- anchors later commit compact checkpoint hashes only
- internal ROC facts remain internal truth
- anchor verifier cannot mutate balances

### 2.9 Privacy leakage

Threat:

Receipts, account proofs, or carrier chunks reveal browsing history or link alt identities.

Defense:

- no privacy claims yet
- alt passport linkage remains private by default
- minimize public receipt details later
- scope account proof views
- no account IDs in metrics labels

## 3. Phase 0 acceptance

Phase 0 is acceptable only if:

- [ ] all QuickChain work is disconnected from runtime economics
- [ ] no service treats QuickChain DTOs as authority
- [ ] no fake proof is presented as real
- [ ] canonical byte tests pass
- [ ] docs clearly mark unsafe future work
