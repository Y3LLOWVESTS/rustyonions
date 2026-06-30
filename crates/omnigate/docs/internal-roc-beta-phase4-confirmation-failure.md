# Internal ROC Beta Phase 4 Round 2 — omnigate Confirmation / Failure Boundary

RO:WHAT — Documents the omnigate Phase 4 Round 2 paid-action confirmation and failure UX boundary.
RO:WHY — Omnigate coordinates paid quote/access/hydration but must not become wallet, ledger, receipt, balance, finality, entitlement, bridge, staking, liquidity, or external settlement authority.
RO:INTERACTS — routes/v1/content_view.rs, routes/v1/site_visit.rs, routes/v1/paid.rs, svc-wallet transfer path, svc-gateway product proxy, CrabLink Tauri confirmation UX.
RO:INVARIANTS — quote is read-only; pay goes through svc-wallet only; errors are source-labeled/redacted; denials never leak protected body; render/unlock depends on backend receipt/access truth.
RO:SECURITY — no direct ledger mutation, no fake receipt, no fake balance, no fake finality, no silent spend, no cache-only unlock, no ROX/Solana/bridge/staking/liquidity/external settlement.
RO:TEST — cargo test -p omnigate --test internal_roc_beta_phase4_confirmation_failure_boundary.

## Phase 4 Round 2 scope

Omnigate participates only as the product hydration/access/quote coordinator.

Allowed:

```text
prepare/quote responses include enough display-safe detail
recipient/split labels are safe and bounded
errors are redacted/source-labeled
denial never leaks protected body
pay routes call svc-wallet only
accepted wallet receipts can support paid access rendering
```

Forbidden:

```text
omnigate wallet authority
omnigate ledger authority
omnigate balance truth
omnigate receipt truth
omnigate finality truth
omnigate cache entitlement authority
cache-only paid unlock
policy-only paid unlock
manifest-only paid unlock
b3-only paid unlock
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

Paid quote responses must include bounded display-safe material for CrabLink:

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

These fields are not receipts.

These fields do not mutate balances.

These fields do not unlock paid content.

## Denial behavior

A denied paid route may return a redacted/source-labeled problem body.

It must not return protected content body.

It must not return protected site body.

It must not return protected article/post/comment body.

It must not return paid asset bytes.

## Completion label

```text
Internal ROC Beta Phase 4 Round 2 omnigate confirmation/failure boundary is GREEN / PARKED.
```
