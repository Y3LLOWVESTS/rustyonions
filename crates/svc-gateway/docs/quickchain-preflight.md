# svc-gateway QuickChain Phase-0 Preflight

RO:WHAT — Crate-local QuickChain Phase-0 boundary contract for `svc-gateway`.
RO:WHY — `svc-gateway` is the public route/admission/proxy boundary and must not become economic truth, receipt truth, root truth, finality truth, bridge truth, or validator authority.
RO:INTERACTS — CrabLink/Tauri/Chrome clients, omnigate, svc-storage, svc-wallet through backend routes only, ron-ledger through svc-wallet only, ron-policy, ron-auth, svc-passport.
RO:INVARIANTS — proxy-only for product routes; no wallet/ledger mutation; no fake balances; no fake receipts; no fake finality; no root-producing code.
RO:METRICS — existing gateway HTTP/correlation/reject metrics only; no QuickChain root/finality metrics in Phase 0.
RO:CONFIG — upstream base URLs, body caps, rate/concurrency limits; no validator, bridge, anchor, staking, liquidity, settlement, or root-production knobs.
RO:SECURITY — strips caller-supplied QuickChain authority-looking headers; forwards only bounded product headers; paid access must be backend-derived.
RO:TEST — quickchain_preflight_boundary, quickchain_preflight_docs, product_routes_proxy, paid_storage_*_proxy, app_proxy.

## 0. Status

`svc-gateway` is a QuickChain Phase-0 public boundary crate.

It is:

~~~text
public route surface
admission and quota boundary
correlation/metrics boundary
proxy to omnigate/storage-backed service paths
fail-closed edge for unsupported routes
~~~

It is not:

~~~text
not a chain runtime
not a validator
not a bridge
not an anchor writer
not a checkpoint writer
not a root producer
not a wallet mutation authority
not a ledger mutation authority
not a balance source of truth
not a receipt source of truth
not finality authority
~~~

## 1. Allowed Phase-0 scope

Allowed in `svc-gateway` now:

~~~text
route exposure
header filtering
query preservation
body cap enforcement
rate/concurrency limiting
correlation IDs
HTTP metrics
proxying to omnigate
proxying raw object reads to svc-storage
passing backend responses through after filtering unsafe headers
crate-local preflight tests
crate-local QuickChain notes
~~~

Allowed economic behavior:

~~~text
quote route proxying
prepare route proxying
pay route proxying
wallet display route proxying
wallet hold route proxying to omnigate only
paid storage route proxying to omnigate only
backend-derived receipt/body display only
~~~

## 2. Forbidden Phase-0 scope

Forbidden in `svc-gateway`:

~~~text
no direct svc-wallet mutation implementation
no direct ron-ledger mutation implementation
no issue/transfer/burn/capture/release engine
no fake balances
no fake receipts
no local entitlement truth
no local paid unlock truth
no root-producing code
no checkpoint-producing code
no validator code
no bridge or external settlement code
no Solana/ROX/external L2/DA settlement path
no staking or liquidity logic
no public chain state
no pruning
no finality claims
~~~

## 3. QuickChain header boundary

Gateway may forward normal request context:

~~~text
authorization
content-type
accept
x-correlation-id
x-request-id
idempotency-key
x-ron-passport
x-ron-wallet-account
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
x-ron-tenant
x-ron-accounting-subject
x-ron-region
~~~

Those fields are not trusted by gateway as authority. Omnigate and backend services must validate them through wallet/ledger truth.

Gateway must strip caller-supplied QuickChain authority-looking headers:

~~~text
x-ron-operation-id
x-ron-account-sequence
x-ron-state-root
x-ron-receipt-root
x-ron-accounting-root
x-ron-reward-root
x-ron-checkpoint-root
x-ron-checkpoint-hash
x-ron-data-availability-root
x-ron-validator-*
x-ron-bridge-*
x-ron-anchor-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-quickchain-*
x-ron-finality
x-ron-finalized
x-ron-anchored
x-ron-entitlement
x-ron-unlock-authorized
~~~

Reason:

~~~text
operation_id is backend-assigned durable ledger-operation identity
idempotency-key is retry identity only
account_sequence is ledger-assigned
roots require canonical bytes and vectors
finality requires future settlement proof
gateway cannot decide paid access from caller-supplied claims alone
~~~

## 4. Hot path boundary

The honest hot path remains:

~~~text
CrabLink/Tauri
→ svc-gateway public route
→ omnigate quote/access route
→ svc-wallet hold/transfer/access path
→ ron-ledger accepted receipt
→ backend response
→ client display/unlock only after backend truth
~~~

`svc-gateway` may carry the request and response. It must not create the economic truth.

## 5. Required focused suites

This crate-local gate must keep these focused suites alive:

~~~text
quickchain_preflight_boundary
quickchain_preflight_docs
product_routes_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
app_proxy
~~~

Local runner:

~~~bash
scripts/dev-quickchain-preflight.sh
~~~

## 6. Parked future work

Parked outside `svc-gateway` until the blueprint gates are green:

~~~text
canonical bytes and locked vectors
state/account Merkle roots
receipt roots
accounting roots
reward roots
checkpoint signing
validator-set logic
external DA
public anchors
bridges
staking or liquidity
CrabLink chain authority
gateway/omnigate ledger mutation
~~~

Future proof surfaces must be explicit DTO/body contracts with canonical bytes, locked vectors, and independent verifier reproduction. They must not appear accidentally as trusted edge headers.

## Focused preflight suites and runner

The svc-gateway QuickChain Phase-0 preflight gate is intentionally focused on
gateway boundary behavior, proxy behavior, and documentation drift.

Focused suites:

- quickchain_preflight_boundary
- quickchain_preflight_docs
- product_routes_proxy
- paid_storage_estimate_proxy
- paid_storage_write_proxy
- app_proxy

Focused runner:

- `scripts/dev-quickchain-preflight.sh`

## Final Phase-0 gateway hardening suites

These tests complete the current gateway safety cage before the crate is parked
for QuickChain Phase-0/preflight.

- `quickchain_preflight_no_fake_receipts`
  - proves request and response headers cannot smuggle fake balances, fake unlocks,
    roots, finality, validator approval, bridge settlement, staking, liquidity,
    or external settlement through the public gateway boundary
  - allows backend receipt/quote metadata only as display/backend-validation context,
    not as gateway-owned authority

- `quickchain_preflight_cache_boundary`
  - proves cache and ETag metadata stay transport-only
  - proves client/cache/query claims do not unlock paid content
  - keeps paid access tied to backend wallet/receipt truth instead of local cache truth
