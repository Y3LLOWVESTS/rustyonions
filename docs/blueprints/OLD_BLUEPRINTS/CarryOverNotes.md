Perfect—here’s a **carry-over note pack** you can paste into another instance to keep everyone laser-aligned on the next two steps:

---

# RustyOnions – Notes for the Next Two Steps

**Scope:** (1) Update the 8 existing blueprints to reflect the new crate list and roles (exactly 33), then (2) condense those updated blueprints into the perfected **Six Concerns** spine.
**Non-negotiable:** The crate list is fixed at **33**. No additions, no removals, no renames.

---

## 0) Canon & Guardrails

### Canonical Crate List (33)

`macronode, micronode, oap, omnigate, ron-accounting, ron-app-sdk, ron-audit, ron-auth, ron-bus, ron-kernel, ron-kms, ron-ledger, ron-metrics, ron-naming, ron-policy, ron-proto, ron-transport, ryker, svc-ads, svc-dht, svc-edge, svc-gateway, svc-index, svc-interop, svc-mailbox, svc-mod, svc-overlay, svc-passport, svc-registry, svc-rewarder, svc-sandbox, svc-storage, svc-wallet`

### Taxonomy Rules

* **`ron-` = library/core primitive** (no long-running service loops, no network IO side-effects).
* **`svc-` = runtime service** (long-lived actors, endpoints, queues, runbooks).
* **Profiles:** `macronode` (full mesh), `micronode` (light/edge).

### Canonical Deltas to Reflect Everywhere

* `svc-arti-transport` → **merged into `ron-transport`** (behind `arti` feature).
* `tldctl` → **folded into `ron-naming`** (schemas/types only).
* **New:** `svc-dht` (first-class Kademlia/Discv5 service).
* **Split of concerns:** `svc-overlay` = sessions/gossip/routing (no DHT logic).
* OAP constants: **frame 1 MiB**, **chunk 64 KiB**, explicit error taxonomy.
* **Global Amnesia Mode:** single flag surfaced by `ron-kernel`, honored by services.

### Structural Docs (already aligned; use as anchors)

* **12 Pillars:** structural “anatomy” (where each crate belongs).
* **Six Concerns (perfected):** cross-cutting “constitution” with **must-pass invariants** and CI hooks.

---

## 1) Phase One — Update the 8 Blueprints (make them current)

**Goal:** Each blueprint references only the 33 crates, the overlay/DHT split, and the merged transport. Each ends with acceptance gates that map to CI.

### Standard Header to Add to Every Blueprint

* **Scope / Non-scope**
* **Crates impacted (subset of the 33)**
* **Six Concerns touched (SEC/RES/PERF/ECON/DX/GOV)**
* **Acceptance checklist (must-pass gates)**
* **Runbooks/metrics** (if the doc touches ops)

### Per-Blueprint To-Do (what to change & the gates)

**A. App\_Integration\_Blueprint.md**

* Replace any service-transport confusion with **`ron-transport` (+`arti` feature)**.
* SDK quickstart must call: `svc-mailbox` (SEND/RECV/ACK + idempotency), `svc-storage` (GET/PUT by content id), `svc-index` (resolve), `svc-dht` (providers/find), via **OAP**.
* Capability flow: `svc-passport` issuance → `ron-auth` (macaroons) usage.
* **Gates:** quickstart works against **micronode**; SDK has retries/idempotency/tracing; capabilities only (no ambient auth); interop bridges are reversible.

**B. Concurrency\_And\_Aliasing\_Blueprint.md**

* Name the stateful services explicitly: `svc-overlay`, `svc-dht`, `svc-mailbox`, `svc-storage`.
* Enforce **no locks across `.await`**; **single-writer** per DHT k-bucket; bounded mailboxes everywhere (Ryker).
* **Gates:** Loom on hot paths; backpressure metrics present (`bus_overflow_dropped_total`); crash-only supervisors with jitter.

**C. Full\_Project\_Blueprint.md**

* Update all diagrams and flows to 33 crates; show **overlay minus DHT**; show **Arti under `ron-transport`**.
* Economics pipeline: `ron-accounting` (transient) → `ron-ledger` (truth) → `svc-rewarder` (distribution, ZK phased).
* **Gates:** every diagram crate-accurate; phase gates reference Six Concerns checklists; no legacy names.

**D. Hardening\_Blueprint.md**

* **Global Amnesia Mode:** describe flag plus enforcement per service (RAM-only caches, zeroization, ephemeral logs).
* Bound all inputs: decompression caps (storage), size limits (gateway/edge/mailbox), rate/timeouts everywhere.
* Abuse plane: `svc-sandbox` (tarpit, decoy), `svc-ads` (quotas/policy ties).
* **Gates:** STRIDE tables per service; amnesia flag tested; abuse surfaces capped and observable.

**E. Interop\_Blueprint.md**

* OAP constants (1 MiB / 64 KiB), HELLO negotiation, error taxonomy.
* `ron-proto`: DTOs are **pure types** with `#[serde(deny_unknown_fields)]`.
* Federation: allow-lists, `svc-registry` signed descriptors, capability translation (no ambient trust).
* **Gates:** compile-checked OAP examples; schema compatibility tests; federation flow documented & capability-based.

**F. Microkernel\_Blueprint.md**

* Kernel surface: supervision, bus, transport hooks, config hot-reload; **no app/policy/econ logic**.
* Transport: **`ron-transport`** is the integration point; Arti feature-gated, not a service.
* Observability: standard histograms/gauges (`ron-metrics`), `/readyz` degraded contract.
* **Gates:** kernel surface minimal & audited; transport features compile both ways; readiness signals uniform.

**G. Omnigate\_Blueprint.md**

* Distinguish roles: `svc-gateway` (enforcement) vs `omnigate` (entry bundle).
* Fair-queue DRR; quotas; degraded modes when SLOs breached.
* End-to-end OAP paths: gateway→overlay→mailbox/index/storage/dht.
* **Gates:** SLOs published; `/readyz` reflects shedding; Bronze/Silver/Gold tied to Six Concerns checks.

**H. Scaling\_Blueprint.md**

* **DHT sims:** α-queries, hedged lookups, **p99 hops ≤ 5**, target bucket occupancy, churn recovery.
* Storage replication policy; cross-region budgets; CDN/edge patterns.
* Perf CI thresholds; flamegraphs artifacts on regression.
* **Gates:** sim harness with pass/fail; perf thresholds wired to CI; cross-region SLOs documented.

### Global Red-Flag Sweep (while editing)

* ❌ Any mention of `svc-arti-transport` or `tldctl`.
* ❌ Any DHT logic inside `svc-overlay`.
* ❌ Ambient authority (non-capability) paths.
* ❌ Unbounded channels or locks across `.await`.
* ✅ OAP constants correct; DTO hygiene exact.

**Definition of Done (Phase One):**

* All 8 blueprints updated with standard header + acceptance checklist; only the 33 crates appear; overlay/DHT/transport merges reflected; links to 12 Pillars + Six Concerns included.

---

## 2) Phase Two — Condense into the Six Concerns (make it enforceable)

**Goal:** Produce a single **`SIX_CONCERNS.md`** that *absorbs* the updated blueprints and provides real, CI-enforced invariants.

### The Six Concerns (final)

1. **Security & Privacy (SEC)** — PQ crypto, capabilities, amnesia, DP, STRIDE.
2. **Resilience & Concurrency (RES)** — crash-only, supervision, single-writer, bounded queues, chaos.
3. **Performance & Scaling (PERF)** — latency/throughput SLOs, DHT hop bounds, replication, profiling.
4. **Economic Integrity (ECON)** — ledger conservation, transient accounting, rewards (ZK phased), wallet no-double-spend, ads spend caps.
5. **DX & Interop (DX)** — SDK ergonomics, OAP/DTO hygiene, reversible bridges, federation allow-lists.
6. **Governance & Ops (GOV)** — policy versioning, registry multi-sig, runbooks, metrics/alerts, quarterly reviews.

### What to Include in `SIX_CONCERNS.md`

* **One section per concern** with:

  * Scope (what it owns)
  * **Must-pass invariants** (the checklists)
  * CI/tooling hooks (fuzz, Loom, perf CI, xtask checks, etc.)
  * “Absorbs sections from” (the updated blueprints)
* **Appendix A:** **Crate → Concern mapping** (all 33 crates mapped; no duplicates, no omissions).
* **Appendix B:** CI routing (labels trigger the right concern jobs).
* **Appendix C:** Authoring guide (which content belongs in which concern section per crate).

**Definition of Done (Phase Two):**

* `SIX_CONCERNS.md` published with per-concern invariants; mapping table covers 33/33 crates; CI label routing documented; quarterly review (Concern 6) specified.

---

## 3) CI & Review Glue (make all of this bite)

* **PR Labels:** `concern:SEC|RES|PERF|ECON|DX|GOV` + `pillar:<#>` to auto-route checks.
* **`xtask check-concerns`:**

  * Verify each touched crate’s docs have the relevant concern sections.
  * Scan for “no locks across `.await`”, unbounded channels, missing `/readyz`, missing STRIDE table, missing SLOs.
* **Perf & Chaos Jobs:** DHT sim, overlay chaos, mailbox idempotency, storage decompression caps.
* **Schema Guard:** `ron-proto` compatibility check (deny unknown, tagged enums).
* **Crypto Posture:** build matrix with `arti` feature and PQ on/off; `cargo-deny` on TLS/crypto deps.
* **Docs Coverage Gate:** README + Pillar + Concern stubs required for any new module paths.

---

## 4) Risk Ledger & Mitigations

* **Risk:** Legacy mentions sneak back (e.g., `svc-arti-transport`).

  * **Mitigation:** grep + xtask denylist; review check.
* **Risk:** Concern overlap causes review confusion (RES vs PERF).

  * **Mitigation:** Clear line: *RES = survives faults; PERF = hits SLOs under load.*
* **Risk:** Missing runbooks degrade ops.

  * **Mitigation:** Concern 6 gate requires runbooks + `/readyz` degraded behavior tests.

---

## 5) Deliverables Checklist (for the other instance)

* ✅ 8 updated blueprints (each with header + acceptance checklist; 33-crate accurate).
* ✅ `12_Pillars.md` (final) referenced by all blueprints.
* ✅ `SIX_CONCERNS.md` with per-concern invariants + CI hooks.
* ✅ Crate→Concern mapping appendix (33/33).
* ✅ `xtask check-concerns` design notes + PR label scheme.
* ✅ A single “What Changed” delta page summarizing:

  * Arti merge into `ron-transport`
  * `svc-dht` creation; overlay slimmer
  * `tldctl` → `ron-naming`
  * Global amnesia flag surfaced by kernel

---

## 6) One-Page Authoring Template (paste into each blueprint as you edit)

**Title (Blueprint Name) – 2025 Update**

* **Scope / Non-scope:**
* **Crates impacted:** (subset of 33)
* **Six Concerns touched:** SEC / RES / PERF / ECON / DX / GOV
* **Key invariants:** (bullets; reference Six Concerns)
* **Runbooks & metrics:** (endpoints, degraded modes, alerts)
* **Acceptance checklist:**

  * [ ] Gate 1 …
  * [ ] Gate 2 …
  * [ ] Gate 3 …
* **References:** 12 Pillars, SIX\_CONCERNS, related blueprints.

---

These notes are designed to be **plug-and-play** for another instance: they capture the canon, the exact changes to make in each of the 8 blueprints, and the **how** of condensing into the Six Concerns with teeth (checklists + CI). If you follow this, you’ll end up with **drift-proof docs** that match your 33 crates perfectly and a review process that keeps them that way.
