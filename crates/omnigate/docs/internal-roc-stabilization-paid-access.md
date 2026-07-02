# Internal ROC Stabilization — omnigate Paid Access Error Boundary

RO:WHAT — Product beta readiness boundary for omnigate paid content_view and site_visit denial/error posture.
RO:WHY — Omnigate coordinates hydration/access and wallet-backed payment flows; errors must be source-labeled while denial keeps protected payloads locked.
RO:INTERACTS — routes/v1/content_view.rs, routes/v1/site_visit.rs, routes/v1/paid.rs, errors/http_map.rs, gateway product proxy routes.
RO:INVARIANTS — quote is read-only; pay uses svc-wallet only; ron-ledger remains durable truth; omnigate does not become receipt, balance, finality, entitlement, or cache authority.
RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, protected body leakage, bridge, ROX/Solana, staking, liquidity, or external settlement runtime.
RO:TEST — cargo test -p omnigate --test internal_roc_stabilization_paid_access_error_boundary.

## Required behavior

```text
paid content_view/site_visit quote
→ read-only route validation
→ no wallet mutation
→ no protected render unlock

paid content_view/site_visit pay
→ quote/hash/recipient/payer validation
→ svc-wallet transfer
→ wallet receipt returned as backend-derived proof
→ protected render remains dependent on backend access/receipt truth
```

Omnigate may:

```text
load manifests needed to validate payout recipient
coordinate quote/pay/access views
forward safe headers to wallet/storage/index
display backend-derived receipt/access metadata
label failures with route/source context
```

Omnigate must not:

```text
mutate ron-ledger directly
invent wallet receipts
invent balances
invent finality
invent paid access
unlock from cache
treat manifest/profile/index metadata as payment proof
treat b3 as payment proof
treat gateway headers as authority
```

## Error posture

Paid access route failures must be:

```text
source_label = omnigate.paid_access_error.v1
reason = bounded route/source reason
message = static/redacted
retryable = explicit true/false
```

Denial/failure response bodies must not include protected content, private capabilities, bearer tokens, raw secrets, or bridge/external-settlement status.

## Completion label

```text
omnigate Internal ROC Stabilization paid access error boundary is GREEN when focused test and preflight pass.
```
