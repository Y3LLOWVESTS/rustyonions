

# OMNIGATE\_BLUEPRINT.md — 2025 Update (Canon-Aligned)

**Status:** Updated to the fixed 33-crate canon and the Six Concerns spine
**Crates impacted:** `omnigate`, `svc-gateway`, `svc-index`, `svc-storage`, `svc-mailbox`, `svc-overlay`, `svc-dht`, `ron-transport`, `ron-auth`, `svc-passport`, `ron-policy`, `svc-registry`, `ron-proto`, `oap`, `ron-metrics`, `ron-audit` (subset varies per feature).
**Pillars:** Primarily **Pillar 6 — Ingress & Edge**; touches Pillars 7, 9–12 for composition and contracts.&#x20;
**Profiles:** Works in both **Micronode** (embedded, DX-first) and **Macronode** (separate service), per Developer Suite.&#x20;

---

## 0) Purpose (What Omnigate Is)

**Omnigate** is the **north–south “view hydrator” / BFF** that composes client-facing responses by orchestrating capability-authorized calls across the substrate (index, storage, mailbox, mods) over **OAP/1**. It is **stateless** (beyond counters and caches), **policy-aware**, and strictly **capability-gated**; it **does not own** durable data or business rules. `svc-gateway` sits in front to terminate TLS, enforce quotas, and shed load; Omnigate turns **capabilities + queries** into **hydrated views**.

**Non-negotiables (canon):**

* **No DHT logic inside overlay/Omnigate** — discovery is in `svc-dht`.&#x20;
* **Transport** abstraction lives in `ron-transport` (Tor/Arti behind a feature).&#x20;
* **OAP limits:** max frame **1 MiB**; streaming chunk **64 KiB**.&#x20;
* **Global Amnesia Mode** must be honored (RAM-only artifacts when enabled).&#x20;

---

## 1) Scope / Non-Scope

**In-scope**

* **View hydration / BFF:** join data from `svc-index` (names/graph/search facets), `svc-storage` (content), `svc-mailbox` (events/fanout), optional `svc-mod` (ranking/transcode) via mailbox.&#x20;
* **Capability enforcement:** verify macaroons (`ron-auth`, `svc-passport`); apply `ron-policy` rules per request; honor registry topology.&#x20;
* **Backpressure and fairness:** respect ingress quotas from `svc-gateway`; apply local concurrency/inflight ceilings; degrade predictably.&#x20;
* **Observability & audit:** golden metrics, `/readyz` behavior, structured audit events.&#x20;

**Out-of-scope**

* **TLS termination, admission control** → `svc-gateway`.&#x20;
* **Discovery/routing table** → `svc-dht`; **sessions/gossip** → `svc-overlay`.
* **Durable truth** (ledger, storage ownership) and **policy authorship** (Omnigate only reads/enforces).

---

## 2) Architecture & Data Flow

**Ingress path (Macronode):** `LB/CDN → svc-gateway → omnigate → {svc-index, svc-storage, svc-mailbox, svc-overlay (as needed)} → client`
**Ingress path (Micronode):** `gateway(mini) → omnigate(embed) → in-proc facets (RAM by default)`
Both profiles share the same SDK and OAP semantics; **Node.js is optional** (SSR only).&#x20;

**Control plane**

* Capabilities minted/verified via `svc-passport` + `ron-auth`; registry/regions from `svc-registry`.&#x20;

**Data plane**

* **Index facets:** names → manifests; graph/search when enabled (feature-gated facets inside `svc-index`).
* **Storage:** content-addressed blobs; byte-range streaming; decompression caps enforced.
* **Mailbox:** store-and-forward for events/fanout; idempotency keys; DLQ metrics.&#x20;

**Transport**

* Use `ron-transport` with timeouts/idle caps; Tor/Arti via feature flag (no separate service).&#x20;

---

## 3) Security & Privacy (SEC)

* **Capabilities only**: every request carries a macaroon; no ambient authority. Verify in Omnigate before any downstream calls.&#x20;
* **Policy-aware hydration**: evaluate `ron-policy` rules (tenancy, geo, quotas) prior to fan-out; deny with structured errors and audit trails.&#x20;
* **Amnesia mode**: when enabled (Micronode default), Omnigate persists **no** on-disk data; caches/queues are memory-only and zeroized on shutdown.

**Acceptance (SEC)**

* [ ] All external calls require valid capabilities.
* [ ] Policy evaluation occurs before any data fan-out; denials audited.
* [ ] Amnesia conformance test proves zero on-disk artifacts.

---

## 4) Resilience & Concurrency (RES)

* **Crash-only**: supervised tasks; jittered restarts; no locks held across `.await`; bounded queues (Ryker).&#x20;
* **Backpressure**: respect gateway token buckets; local concurrency semaphores per downstream (index/storage/mailbox).
* **Hedged calls** (read paths) allowed under strict budgets to meet SLOs.

**Acceptance (RES)**

* [ ] Loom or equivalent checks on hot hydration paths.
* [ ] All channels bounded; **bus\_overflow\_dropped\_total** exported if applicable.&#x20;
* [ ] `/readyz` flips **before** saturation; degraded modes verified.

---

## 5) Performance & Scaling (PERF)

**SLOs (Macronode defaults; Micronode same AZ targets unless noted):**

* **Hydration p95 ≤ 150 ms** for common views (index + storage reads, no mods).
* **Start of byte-range < 100 ms p95** (storage integration).
* **DHT assist** stays out of BFF path; provider discovery handled upstream by services with **p99 ≤ 5 hops**.&#x20;
* **Mailbox fanout read p95 < 200 ms** for moderate fan-outs; heavy fan-outs use pull-on-read via index facet.&#x20;

**Acceptance (PERF)**

* [ ] Perf CI runs hydrate benchmarks with thresholds; flamegraphs attached on regressions.
* [ ] Storage range SLO verified under load (64 KiB streams align with OAP chunk guidance).&#x20;

---

## 6) DX & Interop (DX)

* **One SDK, two profiles**: identical client code against Micronode/Macronode.&#x20;
* **DTO hygiene**: `ron-proto` types only; `#[serde(deny_unknown_fields)]` everywhere. **No** logic in DTOs.&#x20;
* **OAP/1 stable**: HELLO negotiation; reject taxonomy consistent with gateway. (Max frame 1 MiB; stream in 64 KiB).&#x20;

**Acceptance (DX)**

* [ ] Quickstart sample (Micronode) hydrates a composed view end-to-end.&#x20;
* [ ] SDK spans (corr\_id) present across Omnigate and downstream calls.

---

## 7) Governance & Ops (GOV)

* **Metrics** (golden set per pillar): `requests_total`, `bytes_{in,out}_total`, `latency_seconds`, `inflight`, `rejected_total{reason}`, `quota_exhaustions_total`. Gateway + Omnigate counters must align.
* **Readiness**: `/readyz` fails **writes first**; hydration remains best-effort under pressure; **Retry-After** hints passed through from gateway.&#x20;
* **Registry**: honor `svc-registry` signed descriptors for topology/regions; policy changes versioned.&#x20;
* **Audit**: append-only structured events for sensitive denials and escalations.&#x20;

**Acceptance (GOV)**

* [ ] Metrics scrape cleanly; dashboards show quotas, inflight, and rejection reasons.
* [ ] Governance doc references registry + policy process.&#x20;

---

## 8) Feature Facets (No New Crates)

Higher-level product capabilities are **facets** implemented by existing services; Omnigate **composes** them:

| Facet                        | Primary owners                                    | Omnigate’s role                                  |
| ---------------------------- | ------------------------------------------------- | ------------------------------------------------ |
| **Graph** (follows/mutuals)  | `svc-index` (+ mailbox ingest)                    | Query + join, policy-filtered view               |
| **Feed** (ranking/fanout)    | `svc-mailbox`, `svc-mod` (ranking), `svc-sandbox` | Orchestrate pull vs push; attach ranking outputs |
| **Search** (inverted)        | `svc-index`                                       | Compose results + snippets                       |
| **Media** (transcode/thumbs) | `svc-storage`, `svc-mod`, `svc-edge`              | Negotiate renditions; stream safely              |

All feature facets are **optional & feature-gated** in Micronode; always sandboxed mods in Macronode.&#x20;

---

## 9) Profile Differences

**Micronode (DX-first):** Omnigate may run **embedded** with RAM caches and minimal gateway; persistence off by default (**amnesia ON**).&#x20;
**Macronode (enterprise):** Omnigate runs as a **separate service** behind `svc-gateway`; multi-tenant policies, sandboxed mods, geo residency enforced.&#x20;

---

## 10) Interfaces & Contracts

* **Inbound:** HTTP/JSON (SDK) → Gateway quotas (token buckets) → Omnigate routes (`/v1/view/*`, `/v1/feed/*`, `/v1/search/*`, `/v1/media/*`).
* **Downstream:** OAP/1 to services; DTOs strictly from `ron-proto`. **No** ambient auth, **no** imperative policy inside Omnigate.&#x20;

**Error taxonomy** mirrors gateway (e.g., 401/403 for caps, 413 for decompression bounds, 429/503 for quotas/readiness).&#x20;

---

## 11) Milestones — Bronze / Silver / Gold (GMI-2.0)

> Updates the older ring plan to the new canon, keeping its spirit (fast “very good”, then hardening).&#x20;

**M1 — Bronze: “Hello Any App”**

* **Deliver:** OAP/1 hydration routes; capability checks; gateway quotas pass-through; metrics; `/readyz` gating; Micronode embedded mode.
* **Accept:** Sample app hydrates a composed view; overload yields 429/503 with **Retry-After**; zero OOM; metrics exported.&#x20;

**M2 — Silver: “Useful Substrate”**

* **Deliver:** Storage range streaming path tuned (64 KiB chunks); mailbox fanout read paths; basic graph/search joins; SDK DX polish.
* **Accept:** Demo pulls tiles + feed via Omnigate with backpressure; latency targets intra-AZ met.&#x20;

**M3 — Gold: “Ops-Ready & Smooth Growth”**

* **Deliver:** Parser proptests in CI; leakage harness toggles; governance/registry SLA; sandboxed mods on by default in Macronode.
* **Accept:** Multi-tenant load test passes; new dev integrates in < 30 min (DX checklist).&#x20;

---

## 12) CI Gates (Definition of Done)

* **SEC:** capability & policy pre-checks; amnesia test proves zero disks.&#x20;
* **RES:** no locks across `.await`; bounded queues; readiness flips early.&#x20;
* **PERF:** hydrate SLOs enforced; storage range p95 validated; attach flamegraph on regression.&#x20;
* **ECON:** none in Omnigate directly (econ surfaces live in ledger/rewarder/ads); ensure no counters drift into durable truth.&#x20;
* **DX:** SDK quickstart runs on Micronode; DTO denial of unknown fields test.&#x20;
* **GOV:** metrics/alerts dashboards present; registry/process doc linked; `/readyz` degraded behavior unit-tested.&#x20;

---

## 13) Red-Flag Denylist (Drift Guards)

* ❌ Any mention of `svc-arti-transport` (use `ron-transport` feature `arti`).&#x20;
* ❌ DHT code inside Omnigate/overlay. Use `svc-dht`.&#x20;
* ❌ DTOs with logic; permissive deserialization (unknown fields). Keep pure types.&#x20;
* ❌ Persistent state in Micronode with amnesia enabled.&#x20;

---

## 14) References

* **Canonical 33-crate list** and crate roles.&#x20;
* **12 Pillars (Pillar 6 + cross-links), OAP limits, DTO hygiene, readiness/metrics contracts.**&#x20;
* **Developer Suite blueprint** (Micronode/Macronode profiles, facets, SDK invariants).&#x20;
* **Prior Omnigate ring plan** (updated to GMI-2.0 above).&#x20;

---

### Appendix A — Example Route Sketch (Non-normative)

* `GET /v1/view/user/:id` → caps check → index(graph) + mailbox(recent) + storage(avatar range) → compose JSON; audit on deny.
* `GET /v1/feed/home` → caps → mailbox (pull-on-read) → mod(ranking) via mailbox job → join storage thumbs → stream page.

*(Sketch only; DTOs come from `ron-proto`.)*&#x20;

---

**Definition of Ready:** This file lives at `/docs/blueprints/Omnigate_Blueprint.md` and is referenced by Pillar 6. **Definition of Done:** All CI gates in §12 wired, ring plan milestones tracked, and no denylist hits in §13.

---
