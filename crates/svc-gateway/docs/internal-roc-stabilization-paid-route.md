# Internal ROC Stabilization — svc-gateway Paid Route Error Boundary

RO:WHAT — Product beta readiness boundary for svc-gateway paid route denial/error posture.
RO:WHY — Gateway is the public boundary for paid content/site routes, so failures must be source-labeled, redacted, retry-safe, and non-authoritative.
RO:INTERACTS — routes/product.rs, routes/paid_storage.rs, errors.rs, headers/proxy.rs, content_view/site_visit proxy tests, omnigate paid routes.
RO:INVARIANTS — svc-gateway proxies intent only; protected payload truth, receipt truth, balance truth, and paid access truth remain backend-owned.
RO:SECURITY — no fake receipt, fake balance, fake finality, silent spend, cache-only unlock, gateway wallet/ledger mutation, bridge, ROX/Solana, staking, liquidity, or external settlement runtime.
RO:TEST — cargo test -p svc-gateway --test internal_roc_stabilization_paid_route_error_boundary.

## Required behavior

```text
CrabLink / client intent
→ svc-gateway public paid route
→ proxy to omnigate with safe context headers and body
→ omnigate/backend decides quote/access/payment result
→ svc-wallet remains mutation front-door
→ ron-ledger remains durable economic truth
```

Gateway may:

```text
proxy content_view quote/pay
proxy site_visit quote/pay
preserve idempotency-key as retry metadata
preserve safe x-ron context headers
copy safe backend response headers
return upstream status/body
return source-labeled 502 transport failures
```

Gateway must not:

```text
quote or price paid access itself
fetch protected paid payloads on denial
invent receipts
invent balances
invent finality
invent paid entitlement
unlock from cache
treat client headers as payment proof
mutate wallet or ledger directly
```

## Error posture

Gateway transport failures must be:

```text
code = upstream_unavailable
retryable = true
source_label = svc-gateway.public_edge_error.v1
reason = bad_method | omnigate_connect | omnigate_read | other bounded reason
```

The error body must not leak secrets, bearer tokens, capabilities, private keys, protected content, or raw upstream internals.

## Completion label

```text
svc-gateway Internal ROC Stabilization paid route error boundary is GREEN when focused test and preflight pass.
```
