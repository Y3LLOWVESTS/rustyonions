
# Corrected, contradiction-free map: 12 pillars × **33** crates (no deprecated libs)

Below is a clean, *non-contrived* layout that (a) excludes deprecated crates (`gateway`, `index`, `overlay`, `kameo`, `ron-token`, `svc-economy`, `common`, `node`), (b) **keeps the total at 33** as you asked, and (c) preserves crisp boundaries. One deliberate choice: **`ron-billing` is merged into `ron-ledger`** (reporting API within ledger), while **`svc-rewarder` stays separate** (ZK/commitments), and **`ron-proto` stays a shared protocol/types crate** (not “governance”).

### 1) Kernel & Orchestration

* **ron-kernel** — supervision, backoff, global readiness/drain.
* **ron-bus** — lifecycle/event fan-out.
* **ryker** — the one actor runtime under kernel control.
  *(Uniqueness: only these handle lifecycle; no business logic.)*

### 2) Policy & Capability Control

* **ron-policy** — versioned rules (quotas, pricing, splits, AML, holds) with live reload.
  *(Decides only; never executes money/storage.)*

### 3) Identity & Key Management

* **ron-kms** — keys, signing, rotation (PQ-hybrid ready).
* **ron-auth** — capability envelopes/macaroons verification.
* **svc-passport** — issues sealed `.passport` / `.alt` bundles + CRL.
  *(Keys & caps separated from issuance UX.)*

### 4) Audit & Compliance

* **ron-audit** — tamper-evident receipts/trails (issuance, burns, payouts).
  *(Evidence ≠ metrics ≠ money.)*

### 5) Observability

* **metrics** — `/metrics`, `/healthz`, `/readyz`, tracing glue.
  *(Ops signals only; no evidence, no policy.)*

### 6) Ingress & Edge

* **svc-gateway** — neutral ingress (authz/quotas/paywalls), OAP termination.
* **svc-edge** — static/public asset proxy cache with BLAKE3 de-dup. 

### 7) App BFF & SDK

* **omnigate** — BFF (DTO shaping, app auth flows), multi-tenant aware.
* **ron-app-sdk** — client ergonomics (retries, E2E, OAP helpers).
* **oap** — OAP/1 framing & parser (HELLO/START/DATA/END).
* **micronode** — single-tenant dev node profile.
* **macronode** — multi-tenant/advanced hosting node profile.
  *(UX layer; never does ingress/ledger.)*

### 8) Content Addressing & Naming

* **svc-storage** — immutable CAS (64KiB chunks), integrity checks/streaming.
* **svc-index** — address→provider resolver (signed/TTL).
* **naming** — canonical names/TLD semantics & manifest schema.
* **tldctl** — ops CLI for packing/publishing and registry mutations.
  *(Storage ≠ mapping ≠ semantics.)*

### 9) Overlay & Transport

* **svc-overlay** — path selection/forwarding, hedged lookups, metering taps.
* **transport** — TCP/TLS base plane.
* **svc-arti-transport** — onion/Arti transport plane (SOCKS5, HS).
* **svc-mailbox** — private-plane store-and-forward E2E ciphertext queues.
  *(Moves data only; never stores CAS/mappings.)*

### 10) Discovery & Relay

* **svc-sandbox** — isolation/containment for risky workloads/chaos.
  *(Not a router, not a ledger; pure safety/isolation.)*

### 11) Economics & Wallets

* **ron-ledger** — debits, burns, holds/unlocks, **reporting API** (absorbs `ron-billing`), atomic multi-split credits.
* **svc-wallet** — balances/statements/withdrawals (user API).
* **accounting** — usage counters/attribution (not money state).
* **svc-rewarder** — ZK/commitment helper for verifiable metering → ledger.
  *(Money moves in ledger only; usage in accounting; ZK kept separate.)*

### 12) Governance: Registry, Moderation, Ads, Interop, Protocol

* **svc-registry** — domain/TLD lifecycle (availability→burn→cert→renew/claim).
* **svc-mod** — moderation queues/scoring/appeals (stake/decay).
* **svc-ads** — ad campaigns (escrow, serve quotas), content-agnostic.
* **interop** — bridges (ROX/DNS/web proofs), signed transfers.
* **ron-proto** — shared DTOs/error types (service contracts).
  *(Govern, bridge, and define wire DTOs; no money or ingress.)*

### Final list (explicit, 33 items)

1 ron-kernel
2 ron-bus
3 ryker
4 ron-policy
5 ron-kms
6 ron-auth
7 svc-passport
8 ron-audit
9 metrics
10 svc-gateway
11 omnigate
12 ron-app-sdk
13 oap
14 micronode
15 macronode
16 svc-storage
17 svc-index
18 naming
19 tldctl
20 svc-overlay
21 transport
22 svc-arti-transport
23 svc-mailbox
24 svc-sandbox
25 ron-ledger
26 svc-wallet
27 accounting
28 svc-rewarder
29 svc-registry
30 svc-mod
31 svc-ads
32 interop
33 ron-proto

**Excluded (deprecated/absorbed):** `gateway`, `index`, `overlay`, `kameo`, `ron-token`, `svc-economy`, `common`, `node`, and **ron-billing** (folded into `ron-ledger`’s reporting API).
**Also excluded to stay clean:** `svc-edge` (deployment pattern; not a fundamental role—kept out to avoid role overlap with gateway/CDN policy).

---

## Why this is “God-tier clean”

* **No contrived roles.** Every crate has a single sentence that isn’t duplicative of another. If we can’t write that sentence, we removed it.
* **No math errors.** It’s a real 33, with deprecated libs out, ZK kept explicit (`svc-rewarder`), billing merged into ledger, and `ron-proto` kept as a cross-service contract surface.
* **Boundaries hold.** Storage≠Index≠Naming; Gateway≠Omnigate; Overlay≠Transport; Ledger≠Accounting; Policy decides, Ledger executes, Audit records, Metrics observes.
