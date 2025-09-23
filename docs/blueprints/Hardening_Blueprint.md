# RustyOnions Hardening Blueprint — v2.0 (2025 Refactor)

**Status:** Canon-aligned; replaces v1.1
**Scope:** Applies to **all 33 crates** and every runtime profile (macronode, micronode). No exceptions.&#x20;
**Anchors:** 12 Pillars (final), Six Concerns spine (SEC/RES/PERF/ECON/DX/GOV), Carry-Over Notes.

## 0) Context & Deltas (what changed since v1.1)

* **Crates are fixed at 33**; all blueprint language must use these names verbatim.&#x20;
* **Overlay split:** `svc-dht` now owns Kademlia/Discv5; `svc-overlay` focuses on sessions/gossip only. **No DHT logic in overlay.**&#x20;
* **Transport merge:** `svc-arti-transport` was folded into **`ron-transport`** under the `arti` feature (lib, not service).&#x20;
* **Naming cleanup:** `tldctl` folded into **`ron-naming`** (schemas/types; runtime stays in `svc-index`).&#x20;
* **Six Concerns** are the governance spine for cross-cutting invariants and CI gates: **SEC, RES, PERF, ECON, DX, GOV.**&#x20;

---

## 1) North Star

> **Hardened by default**: the system **fails well**, **refuses bad input**, **keeps secrets**, and **tells the truth** under stress—without bespoke knobs per crate. (v1.1 spirit retained.)&#x20;

---

## 2) DoH — Definition of Hardened (v2.0)

A crate (lib or service) is **Hardened** iff all applicable items below hold. Defaults are **strict** and **opt-down only** via explicit config.

### 2.1 Global protocol & IO limits (OAP/1, HTTP, storage)

* **OAP**: reject frames **> 1 MiB**, process data in **64 KiB** chunks; streaming preferred over buffering.
* **HTTP services** (e.g., `svc-gateway`, `omnigate`, `svc-edge`, `svc-storage`):

  * **Timeout:** 5s default;
  * **Concurrency cap:** 512 inflight;
  * **RPS cap:** 500 req/s per instance (tune by svc);
  * **Request body cap:** 1 MiB;
  * **Decompression guard:** ratio ≤ 10× and absolute output cap. (Carry-forward from v1.1.)&#x20;
* **UDS servers** (e.g., `svc-index`, `svc-storage`, `svc-overlay`): socket dir `0700`, socket `0600`, check **SO\_PEERCRED** allow-list.&#x20;

### 2.2 Security & Privacy (SEC)

* **Capabilities only**: No ambient authority. `ron-auth`/`svc-passport` issue/verify tokens with short TTL; external auth is translated, not trusted.&#x20;
* **Key custody**: `ron-kms` zeroizes secrets on drop; sealed storage; PQ-hybrid readiness (e.g., X25519+Kyber/Dilithium).&#x20;
* **Amnesia Mode** (kernel-surfaced flag): RAM-only caches, ephemeral logs, timed purge of keys (Tor HS, session creds), and **no on-disk spill** in micronode.

### 2.3 Resilience & Concurrency (RES)

* **Crash-only supervision** in kernel; jittered backoff; never hold a lock across `.await`. `ron-bus` queues bounded; lag/drop metrics required.&#x20;
* **Actors/mailboxes** (`ryker`): bounded queues with backpressure; starvation tests.&#x20;
* **DHT** (`svc-dht`): single-writer per k-bucket; churn recovery; **p99 ≤ 5 hops** target.&#x20;

### 2.4 Performance & Scaling (PERF)

* **Golden metrics**: latency histograms, error counters, saturation gauges; `/metrics`, `/healthz`, `/readyz`, `/version` everywhere.
* **Perf CI**: thresholds for request latency, DHT hops, storage throughput; flamegraph artifact on regression.&#x20;

### 2.5 Economic Integrity (ECON)

* **Ledger truth** in `ron-ledger`; `ron-accounting` is transient counters; `svc-rewarder` distribution; **no doublespends** in `svc-wallet`.&#x20;
* **Ads/interop** are capability-gated economic surfaces; quotas + policy tie-in (`ron-policy`).&#x20;

### 2.6 DX & Interop (DX)

* **DTO hygiene:** `ron-proto` uses `#[serde(deny_unknown_fields)]`; schema compatibility tests; reversible interop bridges.&#x20;

### 2.7 Governance & Ops (GOV)

* **Policy & Registry**: versioned rules in `ron-policy`; `svc-registry` multi-sig updates; pillar/concern PR gates enforced in CI.

---

## 3) Pillar-Specific Hardening Highlights (what’s unique per group)

> Use this along with the Pillar review checklist during PRs.&#x20;

* **P1 Kernel & Orchestration (`ron-kernel`, `ron-bus`, `ryker`)**
  Supervision invariants; bounded bus; `bus_overflow_dropped_total`; no app logic in kernel.&#x20;
* **P3 Identity & Keys (`ron-kms`, `ron-auth`, `svc-passport`)**
  Zeroization; short-TTL creds; capability tokens (macaroons-style).&#x20;
* **P6 Ingress & Edge (`svc-gateway`, `omnigate`, `svc-edge`)**
  DRR fair-queue, early quotas, degraded modes and observable shedding on `/readyz`.&#x20;
* **P10 Overlay/Transport/Discovery (`svc-overlay`, `ron-transport`, `svc-dht`)**
  Arti behind `ron-transport` feature; overlay without DHT; discovery limits and hop SLOs.
* **P9 Naming & Index (`ron-naming`, `svc-index`)**
  Types vs runtime separation; UDS with peer-cred; audited writes.&#x20;
* **P11 Messaging & Extensions (`svc-mailbox`, `svc-mod`, `svc-sandbox`)**
  At-least-once with idempotency keys; DLQ metrics; strict sandbox defaults; tarpit decoys.&#x20;
* **P12 Economics & Wallets (`ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`, `svc-ads`, `svc-interop`)**
  Conservation proofs, spend caps, reversible bridges, capability gating.&#x20;

---

## 4) Service-Class Hardening Profiles (drop-in checklists)

> Apply the relevant profile(s) to each service; libs use the DTO/crypto/consistency tests portions.

### 4.1 Ingress Profile (gateway/omnigate/edge)

* **Limits:** 5s/512/500rps/1 MiB + 10× decompress; streaming aggregation only.&#x20;
* **Auth:** capability attach early; anonymous paths rate-limited and sandboxed.&#x20;
* **Observability:** DRR queue depth gauges; `/readyz` flips to “shed” under breach.&#x20;

### 4.2 Overlay/DHT/Transport

* **Transport:** idle/read/write timeouts; mTLS/Tor via `ron-transport` feature flags.&#x20;
* **Overlay:** no table logic; enforce backpressure; per-peer byte/rps ceilings.&#x20;
* **DHT:** single-writer buckets; hop caps; per-op rate limits and eclipse/Sybil controls.&#x20;

### 4.3 Storage/Index/Naming

* **BLAKE3 verify** on read; `MAX_OBJECT_BYTES` and streamed IO; UDS + peer-cred; corrupted-chunk → 502 + `integrity_fail_total`. (From v1.1 deep-dive.)&#x20;

### 4.4 Mailbox/Mods/Sandbox

* **Mailbox:** visibility timeouts, ACK/DLQ, idempotency keys; poison-message quarantine.&#x20;
* **Mods:** deny ambient FS/network; capability-scoped; wasm/native parity.&#x20;
* **Sandbox:** cheap decoys; no reflection; clear SOC metrics.&#x20;

### 4.5 Identity/KMS/Passport

* **Keys:** sealed storage, rotation, log **key IDs only**; ephemeral keys honored by amnesia mode.&#x20;

### 4.6 Ledger/Wallet/Accounting/Rewarder/Ads/Interop

* **Ledger invariants:** conservation, non-negativity; property tests; append-only.&#x20;
* **Wallet:** no doublespends; sequence numbers/idempotency; short receipts.&#x20;
* **Ads/Interop:** quotas + policy enforcement; capability translations only.&#x20;

---

## 5) Amnesia Mode (spec)

**Intent:** Minimize persistence/retention to reduce risk & liability—especially for **micronode** (default on).
**Controls:** RAM caches only; periodic zeroization; suppress disk logs; time-boxed key material; aggressive `/readyz` degradation instead of spill; config-surface in kernel, honored by all services.

---

## 6) Test & Verification (make it bite)

* **Property tests** (e.g., `ron-ledger` conservation; mailbox idempotency).&#x20;
* **Fuzzing** for `oap`, `ron-bus`, `ron-proto` (frame parse, envelope decode, DTO round-trip).&#x20;
* **Chaos**: kill/restart under load; `/readyz` must flip before listen socket closes; DRR continues shedding without collapse.&#x20;
* **Perf sims**: DHT hop distribution (target **p99 ≤ 5**), overlay backpressure, storage throughput.&#x20;

---

## 7) CI & Tooling Gates

* **Label routing:** `concern:SEC|RES|PERF|ECON|DX|GOV` + `pillar:#` to trigger targeted jobs (lint walls, fuzz, sims, perf).&#x20;
* **Self-test container** (per service):

  1. `/readyz` waits for healthy;
  2. send 2 MiB → expect **413**;
  3. scrape `/metrics` → `request_latency_seconds` present. (Carried forward.)&#x20;
* **UDS perms check:** fail CI if dir≠`0700` or socket≠`0600`; verify **SO\_PEERCRED** acceptance.&#x20;
* **Schema guard:** `ron-proto` compat + `deny_unknown_fields` test.&#x20;
* **Feature matrix:** build with/without `arti` for `ron-transport`; amnesia mode on/off in micronode.

---

## 8) Minimal Implementation Sketches (drop-in)

* **HTTP hardening layer** (Axum/tower): timeout, concurrency/rps caps, body cap, tracing—(from v1.1; keep as shared helper).&#x20;
* **Safe decompression**: enforce ratio + absolute output; stream or refuse.&#x20;
* **UDS peer-cred**: `SO_PEERCRED` allow-list on accept. (v1.1 snippet stands.)&#x20;

---

## 9) Migration Plan (7-day, no-drift)

1. **Ingress first**: adopt hardening layer + quotas in `svc-gateway`/`omnigate`/`svc-edge`; wire degraded `/readyz`.&#x20;
2. **Overlay/DHT**: enforce per-peer limits; split DHT ops to `svc-dht`; add hop SLO metrics.&#x20;
3. **Storage/Index**: streaming IO + integrity checks + UDS peer-cred; add `integrity_fail_total`.&#x20;
4. **Mailbox/Mods/Sandbox**: idempotency + DLQ; sandbox deny-by-default.&#x20;
5. **Identity/KMS**: short-TTL capability issuance; zeroization tests; amnesia hooks.&#x20;
6. **Ledger/Wallet/Accounting**: property tests for conservation, doublespend guards; perf thresholds.&#x20;
7. **CI wiring**: enable concern labels, UDS perms checks, schema guard, feature matrix builds.&#x20;

---

## 10) Required Metrics (taxonomy)

* **HTTP:** `http_requests_total{route,method,status}`, `request_latency_seconds{route,method}`, `inflight_requests{route}`.&#x20;
* **Bus/Actors:** `bus_overflow_dropped_total`, `bus_queue_depth`.&#x20;
* **DHT:** `dht_lookup_hops_histogram`, `dht_success_total`, `dht_bucket_occupancy`.&#x20;
* **Storage:** `storage_get/put_latency_seconds`, `chunks_read/written_total`, `integrity_fail_total`.&#x20;

---

## 11) Acceptance Checklist (paste this into each crate’s README)

**Hardening v2.0 – Must-Pass:**

* [ ] Limits (timeout/concurrency/RPS/body/decompress) enforced or N/A.&#x20;
* [ ] `/metrics`, `/healthz`, `/readyz`, `/version` implemented (or N/A for libs).&#x20;
* [ ] Capability auth only; no ambient authority; short-TTL creds.&#x20;
* [ ] UDS perms `0700/0600` + **SO\_PEERCRED** allow-list (if applicable).&#x20;
* [ ] DTOs have `deny_unknown_fields`; schema compat test (if applicable).&#x20;
* [ ] Bus/actors bounded; lag/drop metrics present.&#x20;
* [ ] OAP limits (1 MiB frame, 64 KiB chunk), streaming IO where possible.
* [ ] Property/fuzz/chaos tests in place for critical paths.&#x20;
* [ ] Amnesia mode honored (if applicable to runtime).&#x20;

---

### Notes

* This blueprint **supersedes** the previous v1.1 by baking in the refactor (33 crates exact, overlay/DHT split, transport merge) and elevating Six Concerns to first-class CI gates. Keep Pillar and Concern mappings in PR templates to prevent drift.

---
