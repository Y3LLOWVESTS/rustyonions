**Fully updated 12 Pillars file** that folds in the Developer Suite “facet” features (Graph, Feed, Search, Media, Trust & Safety, Geo/Residency), keeps the **33-crate canon**, and cross-checks against the **Full Project Blueprint v2.0** so we don’t miss any invariants (overlay/DHT split, transport merge, amnesia, OAP constants, etc.). I preserved your structure and “review checklist” style, and added facet-aware acceptance gates where it matters.

---

# RustyOnions — The 12 Architectural Pillars (2025 Edition, Facet-Aligned)

**Status:** Finalized to the 33-crate canon; facet-aware (Developer Suite)
**Last updated:** 2025-09-22
**Purpose:** Boundary contracts for RustyOnions. Each pillar groups crates by responsibility and enforces **clear separations** (lib vs service, kernel vs runtime, durable vs transient). Every PR must prove compliance with these pillars and their facet gates.&#x20;

---

## Canonical Crate List — **exactly 33 (no extras, no renames)**

macronode, micronode, oap, omnigate, ron-accounting, ron-app-sdk, ron-audit, ron-auth, ron-bus, ron-kernel, ron-kms, ron-ledger, ron-metrics, ron-naming, ron-policy, ron-proto, ron-transport, ryker, svc-ads, svc-dht, svc-edge, svc-gateway, svc-index, svc-interop, svc-mailbox, svc-mod, svc-overlay, svc-passport, svc-registry, svc-rewarder, svc-sandbox, svc-storage, svc-wallet.&#x20;

**Canon deltas baked in:** `svc-arti-transport` merged into **`ron-transport`** (feature `arti`), `tldctl` folded into **`ron-naming`**; `svc-dht` split out; `svc-overlay` has **no DHT logic**.&#x20;

**Facet model (no new crates):** Higher-level DX features are implemented as **facets inside existing owners**:

* **Graph** (svc-index + storage + mailbox + policy + omnigate)
* **Feed** (mailbox fanout + mod ranking under sandbox + omnigate)
* **Search** (svc-index + mailbox ingest)
* **Media** (storage + mailbox jobs + mod workers under sandbox + edge + gateway quotas)
* **Trust & Safety** (gateway/overlay ceilings + sandbox tarpits + policy + audit)
* **Geo/Residency** (policy + registry + service enforcement)
  All facets are CI-labeled (e.g., `facet:graph|feed|search|media|abuse|geo`).

---

# Pillars

## Pillar 1 — Kernel & Orchestration

**Crates:** `ron-kernel`, `ron-bus`, `ryker`
**Role:** Lifecycle, supervision, crash-only restarts, bounded mailboxes. The **diamond** core: orchestration only, **no app logic**.
**Non-negotiables:** bounded broadcast bus with overflow counters; supervisor backoff+jitter; config hot-swap; amnesia flag surfaced and honored by services.&#x20;

**Review gates:**
✅ No locks across `.await` in supervisor paths; bounded broadcast with `bus_overflow_dropped_total`.
✅ Public API exports frozen: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.&#x20;

---

## Pillar 2 — Policy & Governance

**Crates:** `ron-policy`, `svc-registry`
**Role:** Declarative rules (SLAs, quotas, geo, residency). Registry holds **signed descriptors**; multi-sig updates. Policy decides; registry publishes; services enforce.&#x20;

**Review gates:**
✅ Changes are declarative, versioned, bus-announced; registry updates require multi-sig; geo maps signed.&#x20;

---

## Pillar 3 — Identity & Key Management

**Crates:** `ron-kms`, `ron-auth`, `svc-passport`
**Role:** Key custody, capability tokens (macaroons), passport issuance. PQ-hybrid posture; zeroization; sealed storage.&#x20;

**Review gates:**
✅ PQ-hybrid KEX/SIG roadmap documented; short-TTL credentials; rotation ≤ 30d; amnesia obeyed.&#x20;

---

## Pillar 4 — Audit & Compliance

**Crates:** `ron-audit`
**Role:** Append-only evidence. Structured, exportable logs; distinct from metrics and ledger.&#x20;

**Review gates:**
✅ Immutable streams; cross-links to registry/policy and ledger events.

---

## Pillar 5 — Observability

**Crates:** `ron-metrics`
**Role:** Golden metrics; `/metrics`, `/healthz`, `/readyz`, `/version`. **No policy/evidence** here.&#x20;

**Review gates:**
✅ Latency histograms, queue saturation, error taxonomy exposed; `/readyz` degrades writes before collapse.&#x20;

---

## Pillar 6 — Ingress & Edge

**Crates:** `svc-gateway`, `omnigate`, `svc-edge`
**Role:** Network entry, TLS termination, fair-queue DRR, quotas, structured errors, degraded modes; `svc-edge` for static/byte-range delivery. **Omnigate** hydrates client-facing views (BFF).&#x20;

**Review gates (facet-aware):**
✅ **Trust & Safety:** hard quotas + tarpits via sandbox where applicable; evidence on reject.&#x20;
✅ **Media:** enforce byte-range + quotas; integrity for originals and renditions.&#x20;

---

## Pillar 7 — App BFF & SDK

**Crates:** `ron-app-sdk`, `oap`, `ron-proto`
**Role:** SDK ergonomics (retries, idempotency, tracing), protocol envelopes (OAP/1), DTOs. **DTOs are pure types**.&#x20;

**Review gates:**
✅ OAP constants honored: `max_frame=1 MiB`, streaming chunk ≈64 KiB; DTOs `#[serde(deny_unknown_fields)]`; SDK idempotency + tracing spans.&#x20;

---

## Pillar 8 — Node Profiles

**Crates:** `macronode`, `micronode`
**Role:** Host compositions. **Macronode** = full service mesh; **Micronode** = single-binary DX profile with amnesia **ON by default** (RAM-only, timed purge, optional persistence).&#x20;

**Review gates:**
✅ Same APIs in both profiles; amnesia matrix tested (`MICRO_PERSIST=0/1`); no bespoke logic inside profiles.&#x20;

---

## Pillar 9 — Content Addressing & Naming

**Crates:** `ron-naming`, `svc-index`
**Role:** Map content/names → manifests/providers. `ron-naming` = schemas/types; `svc-index` = runtime lookups (backed by DHT). **Graph/Search facets live here** behind feature flags and separate executors/semaphores.

**Review gates (facet-aware):**
✅ **Graph:** p95 neighbors ≤ 50ms intra-AZ (≤150ms cross-AZ); idempotent edge writes (content-addressed).&#x20;
✅ **Search:** p95 query ≤ 150ms; ingest lag p95 < 5s; capability-aware filtering.&#x20;

---

## Pillar 10 — Overlay, Transport & Discovery

**Crates:** `svc-overlay`, `ron-transport`, `svc-dht`
**Role:** Sessions/gossip (overlay) **without** DHT logic; transport abstracts TCP/TLS/Tor (Arti as feature); discovery/routing in DHT with Kademlia/Discv5.&#x20;

**Review gates:**
✅ DHT hop bound p99 ≤ 5; single-writer per k-bucket; hedged lookups bounded; Arti via `--features arti`.&#x20;

---

## Pillar 11 — Messaging & Extensions

**Crates:** `svc-mailbox`, `svc-mod`, `svc-sandbox`
**Role:** At-least-once fanout/ingest (ACK + DLQ + idempotency); plugin runtime for ranking/transcode/ETL under sandbox isolation. **Feed facet** = mailbox + mod ranking + omnigate hydration.&#x20;

**Review gates (facet-aware):**
✅ **Feed:** ranking compute ≤ 300ms p95; fanout p95 < 2s (10–10k followers); wasm/native parity; sandbox mandatory on Macronode.&#x20;

---

## Pillar 12 — Economics & Wallets

**Crates:** `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`, `svc-ads`, `svc-interop`
**Role:** Value movement & integrity. Ledger = immutable truth; accounting = transient counters; rewards = (ZK phased); wallet = no doublespends; ads & interop expose economic surfaces (policy-gated).&#x20;

**Review gates:**
✅ Conservation property tests; accounting→ledger flush rules; interop bridges **reversible**; ads tied to quotas/policy with anti-spam tests.&#x20;

---

# Pillar-Aware “Facet Map” (for PR reviewers)

| Facet              | Primary owners                                                         | Cross-pillar ties    | Must-pass SLOs/Rules                                                              |
| ------------------ | ---------------------------------------------------------------------- | -------------------- | --------------------------------------------------------------------------------- |
| **Graph**          | `svc-index` (+ `svc-storage`, `svc-mailbox`, `ron-policy`, `omnigate`) | P6, P7, P9, P11      | p95 neighbors ≤ 50ms intra-AZ (≤150ms cross-AZ); idempotent edge writes.          |
| **Feed**           | `svc-mailbox` + `svc-mod` (under `svc-sandbox`) + `omnigate`           | P6, P7, P11          | ranking ≤ 300ms p95; fanout p95 < 2s (10–10k followers); sandboxed on Macronode.  |
| **Search**         | `svc-index` (+ `svc-mailbox` ingest)                                   | P7, P9, P11          | p95 query ≤ 150ms; ingest lag p95 < 5s; capability filter.                        |
| **Media**          | `svc-storage` + `svc-mod` workers + `svc-edge` + `svc-gateway`         | P5, P6, P11          | byte-range start < 100ms p95; integrity verified; quotas enforced.                |
| **Trust & Safety** | `svc-gateway`, `svc-overlay`, `svc-sandbox`, `ron-policy`, `ron-audit` | P2, P4, P6, P10, P11 | hard quotas; tarpits/decoys; evidence logging on reject.                          |
| **Geo/Residency**  | `ron-policy`, `svc-registry` (+ enforcement in services)               | P2, P6, P8           | 0 policy violations; ≥99.9% writes land in primary region.                        |

**CI Labels:** `facet:*` + `profile:micronode|macronode` + `amnesia:on|off` + `concern:SEC|RES|PERF|ECON|DX|GOV`. These labels route targeted fuzz/perf/chaos tests and policy checks.&#x20;

---

# Enforcement Notes (unchanged but facet-aware)

* Every crate appears **exactly once** under a pillar (33/33).&#x20;
* **Lib vs svc** separation preserved (libs are pure; services run).&#x20;
* **Refactor deltas** honored (overlay/DHT split; Arti via `ron-transport`).&#x20;
* **OAP/1 & BLAKE3** invariants are normative (frame=1 MiB; streaming chunk≈64 KiB; `"b3:<hex>"`).&#x20;
* **Amnesia** is first-class and profile-aware (Micronode ON by default).&#x20;

---

# PR Review Checklist (per Pillar)

Use this **as-code** list in reviews. (Only the deltas vs your prior checklist are shown here; unchanged items remain in force.)

### P1 Kernel & Orchestration — `ron-kernel`, `ron-bus`, `ryker`

✅ Crash-only restart with jitter; bounded broadcast + overflow counters; config rollback on validation fail; amnesia surfaced.&#x20;

### P2 Policy & Governance — `ron-policy`, `svc-registry`

✅ Signed region maps; multi-sig; geo enforced; bus events emitted.&#x20;

### P3 Identity & Keys — `ron-kms`, `ron-auth`, `svc-passport`

✅ PQ-hybrid readiness; macaroon capabilities; rotation ≤30d; zeroize; sealed storage; amnesia honored.&#x20;

### P4 Audit & Compliance — `ron-audit`

✅ Append-only; exportable; cross-link to registry/policy and ledger.&#x20;

### P5 Observability — `ron-metrics`

✅ Golden histograms/gauges; `/readyz` fail-open reads / fail-closed writes; facet SLOs visible.&#x20;

### P6 Ingress & Edge — `svc-gateway`, `omnigate`, `svc-edge`

✅ DRR + quotas; structured errors; **Facet: Media** range/quotas; **Facet: T\&S** tarpits/evidence.&#x20;

### P7 App BFF & SDK — `ron-app-sdk`, `oap`, `ron-proto`

✅ Idempotency + retries + tracing; OAP frame/chunk constants; DTO hygiene.&#x20;

### P8 Node Profiles — `macronode`, `micronode`

✅ One SDK, two profiles; amnesia matrix tests; **no custom logic** in profiles.&#x20;

### P9 Content Addressing & Naming — `ron-naming`, `svc-index`

✅ **Facet: Graph/Search** feature-gated; separate executors + semaphores; SLOs enforced.&#x20;

### P10 Overlay, Transport & Discovery — `svc-overlay`, `ron-transport`, `svc-dht`

✅ Overlay sans DHT; DHT hop ≤5 p99; Arti feature; bucket single-writer.&#x20;

### P11 Messaging & Extensions — `svc-mailbox`, `svc-mod`, `svc-sandbox`

✅ ACK+DLQ+idempotency; **Facet: Feed** ranking sandboxed; wasm/native parity.&#x20;

### P12 Economics & Wallets — `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`, `svc-ads`, `svc-interop`

✅ Ledger conservation property; accounting transient; bridges reversible; ads policy-gated.&#x20;

---

# Notes for Maintainers

* **Storage** cross-cuts P9/P12 as data plane: keep integrity checks, decompression caps (≤10× + absolute).&#x20;
* **Six Concerns** labels (SEC/RES/PERF/ECON/DX/GOV) remain the constitutional spine; this file maps the **structure**, Concerns map the **invariants** and CI teeth.&#x20;

---

**Definition of Done (for this file):**

* Mentions **only** the canonical 33 crates and exact pillar mapping.
* Reflects Developer Suite **facets** with owners, SLOs, CI labels—**no new crates** added.
* Aligns with Full Project Blueprint invariants (OAP, BLAKE3, overlay/DHT split, Arti in `ron-transport`, amnesia).

---


