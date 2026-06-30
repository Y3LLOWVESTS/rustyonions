# Internal ROC Beta Phase 6 — Reproducible Beta Smoke Suite

RO:WHAT — Defines the reproducible local/dev smoke suite for the Internal ROC Beta value-loop proof.
RO:WHY — Phase 6 must consolidate the prior phase proofs into one repeatable green gate without adding new authority paths.
RO:INTERACTS — svc-wallet, ron-ledger, ron-accounting, svc-rewarder, ron-policy, svc-storage, svc-index, svc-gateway, omnigate, CrabLink Tauri.
RO:INVARIANTS — internal ROC first; no new ledger mutation paths; svc-wallet-only mutation; ron-ledger truth; CrabLink display/user intent only.
RO:METRICS — Future runs may collect paid-flow, receipt, replay, accounting, reward-plan, policy, and CrabLink UX proof counts.
RO:CONFIG — Uses dev/local config only; configs/roc-economics.toml remains the mutable economics source of truth.
RO:SECURITY — No ROX, Solana, bridge runtime, staking runtime, liquidity, exchange-facing behavior, fake receipts, fake balances, fake finality, silent spend, or cache-only unlock.
RO:TEST — scripts/dev-internal-roc-beta-phase6-smoke.sh plus focused crate/app checks.

---

## 0. Status

This document starts Internal ROC Beta Phase 6.

Safe label for this batch:

```text
Internal ROC Beta Phase 6 Round 1 Batch 0/1 smoke-suite skeleton is in progress.
```

Do not use the final completion label until the full script is green:

```text
Internal ROC Beta Phase 6 reproducible smoke suite complete.
```

---

## 1. Purpose

Phase 6 proves the internal ROC beta value loop can be reproduced from local/dev commands.

The proof target is:

```text
creator/visitor paid action
→ explicit quote/confirmation boundary
→ svc-wallet mutation front-door
→ ron-ledger receipt/balance truth
→ backend-derived paid access
→ backend-derived balance refresh
→ accounting snapshot/report
→ svc-rewarder non-mutating payout plan
→ ron-policy declarative gate
→ svc-wallet approved payout execution when enabled for smoke
→ ron-ledger final receipt/balance truth
→ CrabLink display-only receipt/access UX
```

---

## 2. Batch 0/1 scope

This first Phase 6 batch adds the reproducible gate surface and locks the first hard proof targets:

```text
scripts/dev-internal-roc-beta-phase6-smoke.sh
scripts/internal-roc-beta-smoke.sh
scripts/internal-roc-beta-wallet-ledger-check.sh
scripts/internal-roc-beta-replay-check.sh
scripts/internal-roc-beta-tokenomics-check.sh
scripts/internal-roc-beta-access-check.sh
scripts/internal-roc-beta-crablink-check.sh
```

This batch does not add new runtime economics logic.

It only composes already-proven focused tests into a repeatable smoke gate.

---

## 3. What the smoke suite proves

### Wallet/ledger truth

```text
paid content receipt path works through svc-wallet
paid action idempotency does not double mutate value
receipt lookup after replay is backend-derived
approved payout execution uses svc-wallet only
ron-ledger replay/conservation remains deterministic
ron-ledger accepted payout replay does not double issue
tokenomics config cannot become ledger authority
```

### Access/enforcement boundaries

```text
svc-storage stores artifacts only
svc-index stores pointers only
svc-gateway routes/enforces without ledger mutation
omnigate hydrates/access-routes without ledger mutation
paid access is backend-derived, not cache-only
```

### Accounting/rewarder/policy

```text
accounting snapshots remain derivative
rewarder plans only
rewarder does not mutate ledger
policy gates only
policy does not mutate wallet/ledger
raw engagement cannot directly mint or allocate ROC
analytics_only and metering cannot become payout material
```

### CrabLink Tauri

```text
CrabLink displays backend-derived balances/receipts
receipt cache is display-only
explicit confirmation is required before spend
failure/cancel paths do not unlock content
React/TypeScript own display and user intent only
Tauri mediates native privilege
no secrets in React state/localStorage/URLs/logs
```

---

## 4. Forbidden scope

Phase 6 must not add:

```text
ROX runtime
Solana runtime
public bridge
bridge mint/burn
bridge custody
external settlement
public validator economy
staking runtime
staking UI
staking APR/APY
staking rewards
liquidity
exchange-facing logic
gateway direct ledger mutation
omnigate direct ledger mutation
storage payment truth
index payment truth
accounting balance truth
rewarder ledger mutation
policy wallet mutation
CrabLink wallet authority
cache-only paid unlock
fake balances
fake receipts
fake finality
silent spend
raw engagement direct ROC minting
```

---

## 5. Commands

Core Rust wallet/ledger slice:

```bash
bash scripts/internal-roc-beta-wallet-ledger-check.sh
```

Replay/conservation slice:

```bash
bash scripts/internal-roc-beta-replay-check.sh
```

Tokenomics/anti-farming slice:

```bash
bash scripts/internal-roc-beta-tokenomics-check.sh
```

Storage/index/gateway/omnigate access slice:

```bash
bash scripts/internal-roc-beta-access-check.sh
```

CrabLink Tauri UX slice:

```bash
CRABLINK_ROOT=/Users/mymac/Desktop/crablink bash scripts/internal-roc-beta-crablink-check.sh
```

Full Phase 6 smoke gate:

```bash
CRABLINK_ROOT=/Users/mymac/Desktop/crablink bash scripts/dev-internal-roc-beta-phase6-smoke.sh
```

Core-only early gate:

```bash
bash scripts/dev-internal-roc-beta-phase6-smoke.sh --core-only
```

---

## 6. Exit label discipline

Allowed only after the full gate passes:

```text
Internal ROC Beta Phase 6 reproducible smoke suite complete.
```

Allowed only after Phase 0 through Phase 6 are all green:

```text
Internal ROC beta value-loop proof is COMPLETE / GREEN / PARKED.
```

Still forbidden:

```text
ROX ready
bridge ready
staking ready
external settlement ready
public chain ready
exchange ready
liquidity ready
QuickChain live
settlement live
public validator economy ready
```
