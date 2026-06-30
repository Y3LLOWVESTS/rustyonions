# Internal ROC Beta Phase 4 Round 2 — svc-gateway Confirmation / Failure Boundary

RO:WHAT — Documents the svc-gateway Phase 4 Round 2 paid-action confirmation and failure UX boundary.
RO:WHY — The public gateway must expose safe quote/pay route contracts without becoming wallet, ledger, receipt, balance, finality, entitlement, bridge, staking, liquidity, or external settlement authority.
RO:INTERACTS — src/routes/product.rs, src/routes/paid_storage.rs, src/headers/proxy.rs, omnigate paid routes, CrabLink Tauri confirmation UX.
RO:INVARIANTS — gateway is proxy/admission boundary only; prepare/quote responses are display-safe; errors are source-labeled/redacted; denials do not leak protected bodies; paid unlock requires backend truth.
RO:SECURITY — no direct ledger mutation, no direct wallet mutation, no fake receipt, no fake balance, no fake finality, no silent spend, no cache/header-only unlock, no ROX/Solana/bridge/staking/liquidity/external settlement.
RO:TEST — cargo test -p svc-gateway --test internal_roc_beta_phase4_confirmation_failure_boundary.

## Phase 4 Round 2 scope

svc-gateway participates only as the public route/proxy/admission boundary.

Allowed:

```text
prepare/quote responses include enough display-safe detail
recipient/split labels are safe and bounded
errors are redacted/source-labeled
denial never leaks protected body
gateway forwards backend-derived wallet/ledger receipt material
gateway preserves idempotency/correlation context
```

Forbidden:

```text
gateway wallet authority
gateway ledger authority
gateway balance truth
gateway receipt truth
gateway finality truth
gateway paid entitlement authority
cache-only paid unlock
header-only paid unlock
fake receipt
fake balance
fake finality
silent spend
bridge runtime
staking runtime
liquidity runtime
ROX/Solana runtime
external settlement
```

## Required quote display fields

Gateway route contracts must preserve enough backend material for CrabLink to show:

```text
amount_minor
display_amount
action
asset
payer_account
recipient_account
quote_id
quote_hash
client_idempotency_key
expires_in_seconds
source_label
```

These fields are display material only.

They are not ledger truth.

They are not wallet mutation authority.

They are not paid access truth until the backend wallet/access path accepts payment.

## Denial behavior

A denied paid route may return a redacted/source-labeled problem body.

It must not return protected content body.

It must not return protected site body.

It must not return protected article/post/comment body.

It must not return paid asset bytes.

## Completion label

```text
Internal ROC Beta Phase 4 Round 2 svc-gateway confirmation/failure boundary is GREEN / PARKED.
```
