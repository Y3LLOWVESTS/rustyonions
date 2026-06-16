# Omnigate QuickChain Phase-0 Preflight

RO:WHAT — QuickChain Phase-0 boundary contract for the `omnigate` crate.
RO:WHY — Omnigate hydrates product views and coordinates paid access, so it must not become ROC economic truth or future chain authority.
RO:INTERACTS — svc-gateway, svc-wallet, ron-ledger, svc-storage, svc-index, ron-policy, CrabLink/Tauri, ron-proto QuickChain DTOs.
RO:INVARIANTS — hydration boundary; product coordination boundary; product hydration and coordination boundary; no direct ledger mutation; no fake receipts; no fake balances; no cache-only paid unlock; no roots/checkpoints/validators/bridges.
RO:METRICS — Existing HTTP/downstream metrics may describe route behavior but must not become economic truth, balance truth, receipt truth, root truth, or finality truth.
RO:CONFIG — Uses existing OMNIGATE_* downstream configuration; QuickChain remains disabled and future-only.
RO:SECURITY — Wallet-facing paid flows must go through backend wallet/ledger truth; cache/client/header/query claims are not entitlement authority.
RO:TEST — crates/omnigate/scripts/dev-quickchain-preflight.sh plus quickchain_preflight_* integration tests.

## 0. Status

This document is a Phase-0 preflight guard.

It does not enable QuickChain.

It is not chain logic and must not become a public Layer 1.

It does not authorize:

```text
roots
checkpoints
validators
settlement
bridges
anchors
staking
liquidity
external settlement
ROX
Solana
live chain state
direct ledger mutation from omnigate
```

`omnigate` remains a product hydration and coordination boundary.

## 1. Omnigate role

`omnigate` may:

```text
hydrate product views
resolve crab/b3/site/product DTOs
coordinate storage/index/wallet-facing service calls
prepare quotes
coordinate explicit paid access requests
relay backend-derived receipts for display/validation context
fail closed when backend truth is unavailable
```

`omnigate` must not become:

```text
not ledger truth
not wallet truth
not receipt truth
not balance truth
not QuickChain runtime
root authority
checkpoint authority
finality authority
validator authority
bridge authority
staking authority
liquidity authority
external settlement authority
cache entitlement authority
```

## 2. Correct paid flow

Correct flow:

```text
client
→ svc-gateway
→ omnigate
→ svc-wallet
→ ron-ledger
→ backend receipt
→ omnigate/gateway relay display context
→ CrabLink render/unlock based on backend truth
```

Allowed:

```text
omnigate may call the configured wallet service/front-door for explicit paid flows.
omnigate may relay backend-derived receipt metadata.
omnigate may show route/display DTOs that clearly identify backend truth.
```

Forbidden:

```text
client/cache/header/query
→ omnigate decides paid=true
→ unlock without backend wallet/receipt truth
```

Forbidden:

```text
omnigate
→ direct ron-ledger mutation
```

Forbidden:

```text
omnigate
→ fake receipt_id / fake receipt_hash / fake finalized=true
```

## 3. Operation identity rules

These rules must remain explicit:

```text
operation_id is backend-assigned durable ledger operation identity
idempotency_key is retry identity only
Idempotency-Key is retry identity only
account_sequence is ledger-assigned
hold_id identifies one hold lifecycle
backend receipt is display/validation context
receipt display cache is display-only
cache is convenience only
b3 hashes are content truth, not economic truth
```

## 4. Cache and hydration boundary

Cache cannot unlock paid content.

The exact Phase-0 invariant is: cache cannot unlock paid content.

Cache and hydration metadata cannot unlock paid content.

Transport/cache fields are not economic authority:

```text
Cache-Control
ETag
If-None-Match
If-Modified-Since
If-Unmodified-Since
Last-Modified
```

Content-addressing is integrity, not payment proof:

```text
b3 proves bytes
b3 does not prove payment
manifest hydration does not prove payment
local receipt cache does not prove payment without backend validation
```

Paid render/access must be based on backend wallet/ledger truth or a backend-validated access proof.

## 5. Forbidden QuickChain creep inside omnigate

Do not add these to `omnigate`:

```text
QuickChain roots
state root calculation
receipt root calculation
checkpoint creation
checkpoint hash authority
validator set authority
validator signatures
bridge settlement
anchor settlement
external settlement
staking
liquidity
ROX
Solana
public chain authority
root-producing code
DB-order roots
wall-clock roots
raw engagement protocol payouts
no roots/checkpoints/validators/bridges
```

## 6. Route expectations

Current expected boundary:

```text
/v1/wallet/:account/balance
  display/read route only; backend-derived when available; fallback must be explicitly non-authoritative

/v1/wallet/hold
  explicit wallet-front-door request only; no direct ledger mutation

/v1/content/view/quote
  read-only quote/hydration

/v1/content/view/pay
  wallet-front-door paid flow only; no fake receipt construction

/v1/sites/:name/visit/quote
  read-only quote/hydration

/v1/sites/:name/visit/pay
  wallet-front-door paid flow only; no fake receipt construction

/v1/streams/*
  viewer media must fail closed without backend receipt/access validation

/v1/chat/*
  paid send must append only after backend wallet success
```

## 7. Focused gate

Run:

```bash
crates/omnigate/scripts/dev-quickchain-preflight.sh
```

The gate must prove:

```text
docs exist and state boundary clearly
focused preflight script exists
no direct ron-ledger import/mutation path
no fake receipt/balance/finality construction
no root/checkpoint/validator/bridge authority routes
paid access depends on backend wallet/receipt truth
cache/hydration cannot unlock paid content alone
wallet calls, if present, are explicit service/front-door boundaries
existing paid route regression tests still pass
clippy -D warnings passes for omnigate
```

## Transport/header authority boundary

Transport headers are not economic authority.

Client-supplied receipt, paid, unlocked, entitlement, finality, balance, or cache headers must never prove paid access inside omnigate. Headers are request context or routing metadata only.

Allowed caller identity/context headers such as `x-ron-wallet-account` and `x-ron-passport` may identify payer intent or passport context, but they cannot prove payment, cannot unlock paid content, cannot fabricate a receipt, and cannot replace backend wallet/ledger truth.

Forbidden authority examples include `x-ron-receipt`, `x-ron-receipt-id`, `x-ron-receipt-hash`, `x-ron-paid`, `x-ron-unlocked`, `x-ron-entitlement`, `x-ron-finalized`, `x-ron-balance`, and `x-ron-spend-authority`.

For paid render, paid chat, site visit, content view, and stream-lite viewer access, omnigate must rely on backend wallet receipt truth or wallet receipt lookup. Any cached/local/display-only receipt context remains non-authoritative until validated by backend wallet/ledger truth.
