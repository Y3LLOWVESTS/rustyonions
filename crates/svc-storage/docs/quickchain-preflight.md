# svc-storage QuickChain Phase-0 Preflight

RO:WHAT — Phase-0 boundary notes for keeping `svc-storage` as byte/object infrastructure while QuickChain remains future settlement infrastructure.
RO:WHY — Paid storage and content addressing sit near the ROC value loop, so storage needs explicit guardrails before gateway/omnigate enforcement work continues.
RO:INTERACTS — `svc-storage`, `svc-gateway`, `omnigate`, `svc-wallet`, `ron-ledger`, `ron-accounting`, `svc-rewarder`, `ron-proto::quickchain`, `ron-policy`, `svc-index`, CrabLink/Tauri.
RO:INVARIANTS — b3 hashes identify bytes only; storage is not wallet/ledger truth; no fake balances/receipts/finality; no roots/checkpoints/validators/bridges/anchors/external settlement.
RO:METRICS — Existing storage metrics may report bytes, request status, paid-write status, and accounting export status only; no balance/finality/root metrics.
RO:CONFIG — Paid-write verifier, wallet receipt lookup, settlement, economics, body cap, and accounting export knobs remain explicit and fail-closed where authority is required.
RO:SECURITY — Cache/offline bytes cannot unlock paid content alone; accepted paid access must come from backend wallet/ledger-derived truth through the gateway/omnigate/wallet path.
RO:TEST — `quickchain_preflight_boundary`, `quickchain_preflight_b3_integrity`, `quickchain_preflight_no_direct_mutation`, `quickchain_preflight_paid_cache`, and `quickchain_preflight_docs`.

## 0. Status

This document is a QuickChain Phase-0/preflight guardrail for `svc-storage`.

It is not a chain implementation.
It does not authorize Phase 1 roots.
It does not authorize checkpoints.
It does not authorize validators.
It does not authorize pruning.
It does not authorize bridges or external settlement.

svc-storage remains content-addressed byte/object infrastructure.

## 1. Storage authority model

`svc-storage` may say:

```text
I have bytes.
I stored bytes.
These bytes hash to b3:<64 lowercase hex>.
This object has length N.
This byte range is available.
Integrity check passed or failed.
This paid-write request carried a backend-verifiable wallet hold proof.
This optional accounting export was attempted for metering.
```

`svc-storage` must not say:

```text
ROC balance changed.
A receipt is final.
A checkpoint included this receipt.
A validator approved this object.
An external anchor settled this object.
A bridge finalized payment.
A cached object alone grants paid access.
```

## 2. Economic authority boundaries

The durable economic authority model remains:

```text
svc-wallet = economic mutation front-door
ron-ledger = durable replayable truth
ron-accounting = usage snapshots and metering, not balance truth
svc-rewarder = payout planning, not mutation
svc-storage = bytes, ranges, b3 integrity, and paid-write admission checks
```

`svc-storage` may call wallet-facing HTTP endpoints only through explicit paid-write settlement seams. It must not expose wallet mutation routes as storage routes. It must not depend on `ron-ledger` or `svc-wallet` as normal production dependencies.

The current allowed paid-storage sequence is:

```text
quote/estimate
→ explicit user confirmation upstream
→ svc-wallet hold
→ /paid/o carries wallet-derived proof
→ svc-storage verifies proof/context and stores bytes
→ optional wallet capture/release through svc-wallet front-door
→ optional usage export to ron-accounting for metering
```

Accounting export failure must not become ledger rollback or fake settlement authority.

## 3. b3/content-addressing boundaries

Canonical object IDs are:

```text
b3:<64 lowercase hex>
```

For storage, a b3 CID is content truth only. It is not:

```text
an account root
a receipt root
a checkpoint root
a proof of payment
settlement finality
validator approval
external anchor proof
```

All ingest paths must derive object CIDs by hashing bytes. Caller-provided b3 strings are lookup keys only and must not cause storage to store unverified bytes under a claimed CID.

## 4. QuickChain forbidden scope inside svc-storage

Do not add any of the following to `svc-storage`:

```text
roots
state roots
receipt roots
accounting roots
reward roots
checkpoint roots
checkpoint writers
checkpoint signing
validator signatures
validator sets
consensus state
fork choice
pruning
public anchors
bridges
external settlement
Solana
ROX
staking
liquidity
exchange-facing logic
wallet issue/transfer/burn routes
ledger mutation routes
fake receipt fields
fake balance fields
fake finality fields
cache-only paid unlock
raw engagement payout authority
```

## 5. Paid cache/offline rule

A local cache may improve reads only after b3 verification.

A cache must not decide paid access by itself. Paid unlock must be derived from backend wallet/ledger truth, exposed through gateway/omnigate policy and wallet receipt flows. A cached object can prove byte integrity, not economic entitlement.

## 6. Bounded media rule

`svc-storage` may serve bounded ranges and object bytes. Large media should prefer range/segment behavior. Storage must not create unsafe command-style full-file surfaces or claim DRM/anti-rip protection.

## 7. Phase-0 acceptance checks for this crate

The first local preflight gate should prove:

```text
- docs exist and state the storage authority model
- /o ingest derives b3 from bytes
- /paid/o rejects missing/malformed proof before storing
- source routes do not expose QuickChain/root/checkpoint/validator/bridge/anchor endpoints
- normal dependencies do not include direct wallet/ledger mutation crates
- docs reject fake balances, fake receipts, fake finality, and cache-only paid unlock
```

## 8. Future work, still not authorized here

Later crates may consume storage chunks as data availability inputs, receipt batch chunks, accounting snapshot chunks, or archive payloads. That does not make `svc-storage` a checkpoint writer or a ledger authority.

Root-producing code belongs later, after canonical bytes and golden vectors are locked and independently reproducible. When that gate opens, roots still belong in the ledger/QuickChain execution path, not in storage object routes.

## 9. Exact Phase-0 preflight doctrine phrases

These exact phrases are intentionally kept for the local QuickChain preflight docs gate:

```text
no fake balances
no fake receipts
no fake finality
no roots
no validators
no bridges
no external settlement
```

Storage may report bytes, b3 integrity, object length, ranges, paid-write admission status, and metering/export status. Storage must not invent balances, receipts, finality, roots, validator claims, bridge claims, or external settlement claims.

## 10. Bounded range/media preflight

The storage read path may serve bytes and bounded ranges for valid b3 objects.

The storage read path must not treat malformed CIDs as ordinary authority-bearing objects. Malformed b3 input rejects before lookup. Valid but unknown b3 input remains a normal object miss.

The storage read path must not claim paid access authority. It may return object bytes, object length, strong ETag, and Content-Range metadata. It must not claim unlock status, wallet receipt truth, balance truth, checkpoint truth, root truth, validator truth, bridge truth, or external settlement truth.

Range/media behavior is intentionally honest: range responses are transport convenience only. They are not DRM, not authorization, not ownership proof, and not payment finality.

## 11. Observability and metrics preflight

Storage observability is allowed to report bounded service health, byte counters, paid-write admission status, settlement status, and accounting-export status.

Storage observability must not expose private identifiers or chain authority. Metrics must not label by CID, wallet transaction ID, wallet receipt hash, payer account, escrow account, balance, root, checkpoint, validator, bridge, finality, or external settlement status.

Metrics are not accounting truth, wallet truth, ledger truth, receipt truth, root truth, or finality truth. They are operational signals only.

## 12. Quote-only economics preflight

Storage may expose a quote-only paid-write estimate endpoint for user experience and wallet preflight planning.

The paid-storage estimate path must remain side-effect-free. It must not create wallet holds, capture funds, release funds, mutate ledger state, write storage bytes, export accounting events, create roots, claim checkpoints, claim validator approval, claim bridge settlement, or claim payment finality.

All quote values use integer minor units only. No floats are allowed for ROC amounts, byte quantities, prices, holds, captures, releases, or minimums.

The quote endpoint may report route, action, asset, byte count, amount_minor, minimum_hold_minor, pricing_mode, and operator-visible economics policy path. It must not report balances, receipt truth, wallet mutation truth, ledger truth, root truth, or finality truth.

## 13. Settlement-boundary preflight

Storage may participate in the internal ROC paid-write loop only through explicit wallet-front-door calls.

The only allowed mutation-facing settlement path for paid storage is the explicit svc-wallet capture/release adapter. Storage must not directly mutate ron-ledger, create balances, mint ROC, burn ROC, issue ROC, transfer ROC, create state roots, create receipt roots, claim checkpoints, claim validator approval, claim bridge settlement, claim anchors, or claim payment finality.

Settlement planning uses integer minor units only. Capture must be greater than zero and must not exceed the proven wallet hold amount. Any remainder is released through svc-wallet, not through storage-owned balance logic.

Settlement idempotency keys are deterministic operation safeguards. They are not chain authority, not receipt roots, not finality, not bridge proofs, and not external settlement.
