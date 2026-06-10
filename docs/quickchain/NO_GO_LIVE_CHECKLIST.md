# QuickChain No-Go-Live Checklist

RO:WHAT — Explicit checklist of conditions that block QuickChain from live settlement use.
RO:WHY — Prevent accidental launch of chain behavior before proofs, governance, replay, and internal ROC are ready.
RO:INTERACTS — QUICKCHAIN.MD, ron-ledger, svc-wallet, ron-accounting, svc-rewarder, svc-registry, ron-kms, CrabLink.
RO:INVARIANTS — no live chain; no fake balances; no fake receipts; no public bridge; no external settlement.
RO:METRICS — future readiness and no-go counters only.
RO:CONFIG — QuickChain must remain disabled until every required gate is green.
RO:SECURITY — any unchecked item blocks live QuickChain.
RO:TEST — review checklist plus future automated readiness gates.

## Status

QuickChain is blocked from live settlement.

This is intentional.

## Hard blockers

QuickChain must not go live while any item below is unchecked.

### Internal ROC economy

- [ ] Visitor B paid action decreases Visitor B balance through svc-wallet
- [ ] Creator A balance increases through backend wallet/ledger truth
- [ ] CrabLink displays backend-derived receipt
- [ ] No fake receipt path exists in CrabLink
- [ ] Paid unlock requires backend truth
- [ ] ron-ledger replay proves balances
- [ ] svc-wallet idempotency prevents double commit
- [ ] hold/capture/release is replay-safe

### Accounting and rewards

- [ ] ron-accounting sealed windows are deterministic
- [ ] sealed accounting snapshots have stable roots
- [ ] svc-rewarder produces deterministic manifests
- [ ] rewarder does not mutate balances directly
- [ ] wallet commits reward intents and returns receipts
- [ ] reward replay cannot double issue

### Canonicalization and roots

- [ ] canonical byte encoding is frozen
- [ ] checkpoint byte vectors are golden
- [ ] account-state byte vectors are golden
- [ ] receipt byte vectors are golden
- [ ] state root vectors are golden
- [ ] receipt root vectors are golden
- [ ] domain separation strings are frozen
- [ ] no map-order ambiguity remains

### Validators and governance

- [ ] validator set comes from svc-registry
- [ ] validator set changes are quorum-gated
- [ ] validator signature preimage is domain-separated
- [ ] validator double-sign guard exists
- [ ] equivocation evidence exists
- [ ] emergency freeze path exists
- [ ] audit records exist for governance actions

### Data availability and pruning

- [ ] receipt batch chunks are stored and retrievable
- [ ] accounting snapshot chunks are stored and retrievable
- [ ] reward manifest chunks are stored and retrievable
- [ ] data availability challenges exist
- [ ] archive fallback exists
- [ ] pruning remains disabled until proof restore tests pass
- [ ] challenge window is enforced before pruning

### External anchors

- [ ] no ROX/Solana anchor in active MVP
- [ ] no bridge mutates ROC balances
- [ ] anchor verification is read-only
- [ ] external anchor is not required for hot-path UX
- [ ] public bridge has independent audit before launch

### CrabLink

- [ ] CrabLink remains display/user intent only
- [ ] no validator private keys in CrabLink
- [ ] no chain authority in React
- [ ] no paid unlock from cached proof alone
- [ ] proof viewers clearly label unverified/dev data

## Rule

If any checkbox is unchecked, QuickChain is not live.
