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
- [x] receipt-root ordering rule is specified
- [x] account-state ordering rule is specified
- [ ] no live service path consumes QuickChain DTOs as authority
