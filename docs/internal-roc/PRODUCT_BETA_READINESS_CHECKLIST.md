# Internal ROC Product Beta Readiness — Final Aggregate Checklist

RO:WHAT — Checklist for the final Internal ROC product-beta readiness park/smoke/docs sweep.
RO:WHY — Makes the final acceptance criteria explicit and reproducible without reopening parked phases.
RO:INTERACTS — final aggregate park script, backend stabilization pair scripts, CrabLink Tauri park gate, Internal ROC docs.
RO:INVARIANTS — No new ledger mutation paths; no cache-only paid unlock; no fake economic truth; no bridge/runtime drift.
RO:METRICS — Future checklist runs may record pass/fail output and terminal proof.
RO:CONFIG — No new config is required.
RO:SECURITY — All economic authority remains with svc-wallet → ron-ledger.
RO:TEST — bash scripts/check-internal-roc-product-beta-readiness.sh and bash scripts/dev-internal-roc-product-beta-readiness-park.sh.

## 1. Backend truth path

Required:

```text
[ ] ron-proto stays DTO/wire only
[ ] ron-ledger stays durable replay/conservation truth
[ ] svc-wallet remains the only mutation front-door
[ ] ron-accounting remains snapshot/report non-authority
[ ] svc-rewarder remains capped planning only
[ ] ron-policy remains declarative gating only
[ ] svc-gateway remains public enforcement/proxy boundary
[ ] omnigate remains hydration/access composition only
[ ] svc-storage remains b3/artifact/admission non-authority
[ ] svc-index remains pointer/lookup/navigation non-authority
```

## 2. CrabLink product readiness

Required:

```text
[ ] Tauri app gate runs from CrabLink repo
[ ] React render/build remains green
[ ] gateway-first path remains green
[ ] typed/redacted Tauri bridge remains green
[ ] explicit paid gates remain green
[ ] backend-derived receipts remain green
[ ] backend-derived balance refresh remains green
[ ] stale/failure labels remain green
[ ] idempotent retry UX remains green
[ ] no silent spend remains green
[ ] no cache-only unlock remains green
[ ] no fake balance/receipt/finality remains green
[ ] display-only receipt cache remains green
[ ] render-lock on paid denial remains green
```

## 3. Forbidden shortcuts

Required blocked states:

```text
[ ] CrabLink direct ledger mutation blocked
[ ] CrabLink fake receipt blocked
[ ] CrabLink fake balance blocked
[ ] CrabLink fake finality blocked
[ ] CrabLink cache-only unlock blocked
[ ] gateway fake paid unlock blocked
[ ] omnigate fake entitlement blocked
[ ] svc-index pointer-only unlock blocked
[ ] svc-storage b3-only unlock blocked
[ ] ron-accounting balance truth blocked
[ ] svc-rewarder payout execution without svc-wallet blocked
[ ] ron-policy receipt truth blocked
[ ] raw engagement direct minting blocked
```

## 4. Bridge boundary

Required:

```text
[ ] Bridge remains docs / threat-model / decision-gate only
[ ] ROX remains non-runtime
[ ] Solana remains non-runtime
[ ] staking remains non-runtime
[ ] liquidity remains non-runtime
[ ] external settlement remains non-runtime
[ ] no public bridge path
[ ] no mint-burn path
[ ] no staking yield
[ ] no public validator economy
[ ] no exchange-facing logic
```

## 5. Final command

```bash
bash scripts/dev-internal-roc-product-beta-readiness-park.sh
```

Optional full pair repark:

```bash
RUN_INTERNAL_ROC_PAIR_PARKS=1 bash scripts/dev-internal-roc-product-beta-readiness-park.sh
```
