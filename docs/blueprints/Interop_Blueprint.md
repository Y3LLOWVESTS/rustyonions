

# Interop\_Blueprint.md — 2025 Update (Canon-Aligned)

**Status:** Updated for the 33-crate canon; overlay/DHT split; `ron-transport` (with `arti` feature); tldctl→`ron-naming`; Micronode/Macronode profiles.
**Primary crates:** `oap`, `ron-proto`, `ron-app-sdk`, `ron-transport`, `svc-gateway`, `svc-interop`, `svc-index`, `svc-storage`, `svc-mailbox`, `svc-overlay`, `svc-dht`, `ron-auth`, `svc-passport`, `ron-policy`, `svc-registry`, `ron-metrics`, `ron-audit`.&#x20;
**Profiles:** **Micronode** (embedded, amnesia-first) and **Macronode** (multi-service, enterprise). Same SDK/API on both.&#x20;

---

## 0) Executive TL;DR

* **Interop = one neutral contract**: **OAP/1 envelope** + **pure DTOs in `ron-proto`** + **tiny `ron-app-sdk` (Rust/TS/Python/Swift)**. Everything else composes around it (gateway quotas, mailbox, storage, index).&#x20;
* **Deltas locked in:** `svc-arti-transport` → **merged into `ron-transport`** (`arti` feature); `tldctl` → **folded into `ron-naming`**; **DHT separated** into `svc-dht`; **OAP constants**: `max_frame = 1 MiB`, streaming **chunks = 64 KiB**.
* **Profiles:** Micronode can run interop stack embedded (RAM-only default); Macronode fans out to services behind gateway with quotas/fair-queue. Same client code on both.&#x20;

---

## 1) Goals & Non-Negotiables

* **Kernel stays tiny** (no app logic; no policy/econ). Public API fixed by canon.&#x20;
* **Capabilities-only access** (macaroons via `ron-auth`/`svc-passport`); **no ambient authority**.&#x20;
* **DTO hygiene**: `ron-proto` = types only; `#[serde(deny_unknown_fields)]`; no IO or logic.&#x20;
* **Transport abstraction** via `ron-transport` (TLS; Tor via `arti` feature); timeouts/idle caps mandatory.&#x20;
* **OAP invariants**: `1 MiB` max frame; `64 KiB` stream chunk; bounded decompression; explicit error taxonomy.&#x20;
* **Global Amnesia Mode** honored (RAM-only when on).&#x20;

---

## 2) Scope / Non-Scope

**In scope**

* **Interoperability contract & matrices** (SDK parity across Rust/TS/Py/Swift).&#x20;
* **OAP/1 spec (normative)** + HELLO negotiation + error taxonomy + test vectors.&#x20;
* **Federation/bridging** via `svc-interop` (capability translation, reversible).&#x20;

**Out of scope**

* Kernel features, economic settlement logic, or app-specific semantics (live in respective services).&#x20;

---

## 3) Layered Model (reference)

```
Apps ── ron-app-sdk ── OAP/1 ── ron-transport(TLS/Tor) ── {svc-gateway → [svc-index, svc-storage, svc-mailbox, svc-overlay, svc-dht]} 
            ↑ DTOs in ron-proto; Caps via ron-auth/svc-passport; Policies via ron-policy; Topology via svc-registry
```

Micronode may embed facets; Macronode runs them as services. Same SDK calls on both.&#x20;

---

## 4) OAP/1 — Normative Contract (updated quick reference)

* **Header fields**: `len,u32 | ver=1 | flags,u16 | code,u16 | app_proto_id,u16 | tenant_id,u128 | cap_len,u16 | corr_id,u64 | [cap] | [payload]`.
* **Flags**: `REQ, RESP, EVENT, START, END, ACK_REQ, COMP, APP_E2E, RESERVED*`.
* **HELLO** (`app_proto_id=0`): returns server caps (`max_frame=1048576`, `max_inflight=64`, flags list, versions, transports).
* **Parser rules**: bound checks **before alloc**; `cap` allowed only with `START`; `APP_E2E` opaque; zstd bounded inflate (≤8× frame cap) or `413`.
* **Error taxonomy**: `0/2xx/206`, client `4xx` (with `Retry-After` on 429), server `5xx`.
* **Vectors**: include HELLO, START with/without cap, mailbox subscribe with `ACK_REQ`, bounded compression, invalid sequences, Tor parity.&#x20;

> Full detail (ABNF, state machine, vectors A–T) retained from prior edition and remains normative.&#x20;

---

## 5) SDK Parity (Rust / TypeScript / Python / Swift)

* **Common contract**: retries with jittered backoff (3 attempts), deadlines, idempotency keys, typed errors; propagate `traceparent`; respect `Retry-After` on 429.
* **Rust/TS/Py/Swift skeletons** preserved; update transport selection via `ron-transport` features.&#x20;

**Acceptance (SDK)**

* Interop matrix runs **byte-for-byte** vectors over TCP+TLS and Tor; streams validated by chunk hashes.&#x20;

---

## 6) Services & Interop Surfaces

* **Gateway**: DRR + token buckets; structured 429/503 with hints.&#x20;
* **Mailbox**: SEND/RECV/ACK/SUBSCRIBE/DELETE; at-least-once; DLQ.
* **Storage**: GET/HAS; byte-range streaming; content-addressed (`b3:<hex>`).
* **Index**: name→manifest/provider; backed by `svc-dht`.&#x20;
* **Overlay**: sessions/gossip; **no DHT logic** (that lives in `svc-dht`).&#x20;
* **Interop bridges (`svc-interop`)**: capability-translated, **reversible**, governed via policy/registry.&#x20;

---

## 7) Security & Privacy

* **Zero-trust**: every op requires a valid macaroon (tenant/method/TTL/limits). Short TTL; revocation converges ≤1s; rotation hygiene in place.&#x20;
* **Transport**: TLS1.3; Tor supported via `ron-transport` feature; no ambient trust.&#x20;
* **Privacy**: `APP_E2E` never decrypted by services; leakage harness and DP metrics live outside the hot path.&#x20;

---

## 8) Federation & Governance

* **Registry** (`svc-registry`): signed descriptors; topology/regions; mirror to DHT for discovery.
* **Policy** (`ron-policy`): declarative allow/deny, quotas, geo; **enforced by services**, authored centrally.
* **Bridges** (`svc-interop`): capability translation with explicit allow-lists; **no ambient inheritance**.&#x20;

---

## 9) Observability & Economics

* **Golden metrics**: `requests_total, bytes_{in,out}_total, rejected_total{reason}, latency_seconds, inflight, quota_exhaustions_total, bus_overflow_dropped_total`.&#x20;
* **Readiness**: `/readyz` degrades **writes first**; structured rejects; Mac/Micro parity.&#x20;
* **Economics** live in Pillar 12 (ledger/rewarder/wallet/ads) and are **out of interop hot path**; do not leak econ semantics into OAP.&#x20;

---

## 10) Profiles (Micronode vs Macronode)

* **Micronode**: single-binary; RAM-only by default (amnesia ON); features gated; same OAP/SDK.
* **Macronode**: multi-service; quotas/fair-queue; sandboxed mods; geo/policy enforcement.&#x20;

---

## 11) Conformance & Interop Matrix

* **Vectors A–T** must pass on all SDKs; negative cases (zstd bomb, duplicate START, cap on non-START) must **reject safely** without leaks/crashes.&#x20;
* **Matrix**: (Rust×TS×Py×Swift) × (TLS×Tor) × (Micronode×Macronode). Report includes timing bins and parity hashes.&#x20;

---

## 12) CI Gates — Six Concerns (Definition of Done)

**SEC:** Cap enforcement; DTO `deny_unknown_fields`; transport posture; revocation latency test.&#x20;
**RES:** No locks across `.await`; bounded queues; `/readyz` flips early; fuzz/property tests on parser.&#x20;
**PERF:** Frame/chunk limits enforced; p95 targets met (range start <100ms intra-AZ); backpressure budgets.&#x20;
**ECON:** None on the wire; verify no econ leakage in OAP/SDK.&#x20;
**DX:** SDK parity; interop matrix; quickstart runs on Micronode with same code.&#x20;
**GOV:** Registry/policy references included; structured 429/503 with `Retry-After`; audits emitted on sensitive denies.&#x20;

---

## 13) Drift Denylist (Hard Guards)

* ❌ **`svc-arti-transport`** anywhere (use `ron-transport` + `arti` feature).&#x20;
* ❌ DHT logic inside overlay or kernel (must live in `svc-dht`).&#x20;
* ❌ DTOs carrying logic or permissive deserialization.&#x20;
* ❌ Reintroducing `tldctl` (now part of `ron-naming`).&#x20;

---

## 14) Config & Formal (pointers)

Use the dev/prod config templates and TLA+ sketch retained from prior edition; still normative for OAP/1 state transitions.&#x20;

---

## 15) References

* **12 Pillars & canonical 33 crates** (mapping + invariants).
* **Developer Suite (Micronode/Macronode)**.&#x20;
* **Interop (prior edition):** full vectors, ABNF, conformance suite (carried forward).
* **Carry-over notes:** required deltas & acceptance gates to purge drift.&#x20;

---

**File path:** `/docs/blueprints/Interop_Blueprint.md`
**Definition of Ready:** This doc references the 33-crate list and the Six Concerns.
**Definition of Done:** All §12 gates are wired in CI; denylist (§13) enforced; interop matrix green.

---
