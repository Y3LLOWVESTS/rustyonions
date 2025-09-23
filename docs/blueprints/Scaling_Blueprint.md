
---

# Scaling\_Blueprint.md — 2025 Update (Canon-Aligned v1.4)

**Status:** Replaces v1.3.1; no kernel API changes.
**Crates impacted (subset of 33):** `macronode`, `micronode`, `svc-gateway`, `omnigate`, `svc-index`, `svc-storage`, `svc-mailbox`, `svc-overlay`, `svc-dht`, `svc-edge`, `svc-mod`, `svc-sandbox`, `ron-transport`, `ron-metrics`, `ron-audit`, `ron-policy`, `svc-registry`, `svc-rewarder`, `ron-ledger`, `ron-accounting`.&#x20;
**Pillars touched:** P6 Ingress & Edge, P7 App BFF & SDK, P8 Node Profiles, P9 Naming/Index, P10 Overlay/Transport/DHT, P11 Messaging/Ext, P12 Economics.&#x20;
**Profiles:** Works for **Micronode** (single-binary, amnesia-first) and **Macronode** (multi-service). Same SDK and OAP semantics on both.&#x20;

---

## 0) Single-Source Invariants (copy/pin across ops docs)

* **Canonical crates = 33**; no additions/removals/renames. Enforce lib (`ron-*`) vs service (`svc-*`) separation.&#x20;
* **OAP/1 constants:** `max_frame = 1 MiB` (protocol), streaming **chunk = 64 KiB** (storage I/O knob), explicit error taxonomy; Interop blueprint holds the normative spec.&#x20;
* **Overlay split:** `svc-overlay` (sessions/gossip) **without DHT**; **DHT is `svc-dht`** (Kademlia/Discv5). Transport abstraction is `ron-transport` with optional `arti` feature for Tor.&#x20;
* **Content addressing:** **BLAKE3-256** (`b3:<hex>`) canonical for objects/manifests; verify full digest before returning bytes.&#x20;
* **Profiles:** Micronode defaults to **Global Amnesia Mode** (RAM-only, zeroization); Macronode persistent by default. Same SDK across both.

---

## 1) Scope / Non-Scope

**In-scope:** How to **scale** ingress, overlay, discovery, index, storage, mailbox, and mods across single-node → small cluster → global deployment; SLOs, runbooks, RF repair, EC, and migration notes. **No kernel API changes**.&#x20;

**Out-of-scope:** App business logic, policy authorship (lives in `ron-policy`), and SDK details (Interop blueprint).&#x20;

---

## 2) Profiles & Planes

* **Micronode** — single binary with embedded facets: mini-gateway, index cache, RAM storage tier, single-shard mailbox; **amnesia ON** by default. For prototypes/edge/private deployments.&#x20;
* **Macronode** — LB→`svc-gateway`→`omnigate`→{index, storage, mailbox, overlay}+policy/registry+sandboxed mods. For multi-tenant/geo deployments.&#x20;

**North–South path (Macronode):** `CDN → svc-gateway (quotas/DRR) → omnigate (hydrate) → svc-index / svc-storage / svc-mailbox / svc-overlay / svc-dht`. **Transport** via `ron-transport` (TLS; Tor via feature flag).

---

## 3) Topologies (NOW → NEXT → GLOBAL)

### 3.1 Single Node (Micronode)

* One process; RAM caches; byte-range enabled; mailbox single shard; `/readyz` degrades writes first. Target **p95 GET start < 100 ms** intra-AZ.&#x20;

### 3.2 Small Cluster (2–10 nodes)

* L4 LB health-eject on `/readyz` ≤ 15s; `svc-index` fronts DHT for resolve; `svc-storage` adds one disk tier + read cache; mailbox 2–4 shards; optional `svc-edge`.&#x20;

### 3.3 Regional Mesh

* `svc-dht` for provider discovery (α=3, k=20; Ed25519-signed provider records; TTL 24h; republish 12h). Hedged dials (200–300 ms).&#x20;

### 3.4 Multi-Region / Global

* **Placement policy:** prefer RTT < 50 ms local; hedge 2 local + 1 remote; nightly rebalance; inter-region selects < 20%/24h. SLO: intra-region **p95 < 80 ms**, inter-region **p95 < 200 ms**.&#x20;

---

## 4) SLOs & Golden Metrics

**Global:** `requests_total`, `bytes_{in,out}_total`, `latency_seconds`, `inflight`, `rejected_total{reason}`, `quota_exhaustions_total`. Structured JSON logs include `service`, `reason`, `content_id`, `latency_ms`.&#x20;

**Read path SLOs (public GETs):** p95 start < 80 ms intra-region; < 200 ms inter-region; 5xx < 0.1%; 429/503 < 1%. Storage streams in **64 KiB** chunks; OAP frame **1 MiB** cap.

**Facet SLOs (reference):** Graph neighbors p95 ≤ 50 ms; Search p95 ≤ 150 ms; Media range start < 100 ms; Feed rank compute ≤ 300 ms; Abuse: hard quotas + tarpits; Geo: 0 policy violations.&#x20;

---

## 5) Plane-by-Plane Scaling

### 5.1 Ingress & Edge

* `svc-gateway` enforces DRR, quotas, and structured rejects; `/readyz` fails **writes first** under pressure. `svc-edge` serves static/ranged content where appropriate.&#x20;

### 5.2 Overlay, Transport & Discovery

* **Sessions/gossip** in `svc-overlay`; **discovery** in `svc-dht` (no DHT in overlay). Use `ron-transport` with strict read/write/idle timeouts; Tor via `arti` feature when needed.&#x20;

### 5.3 Naming & Index

* `svc-index` resolves name→manifest→providers (backed by DHT) and offers graph/search **facets** behind semaphores; heavy work via mailbox.&#x20;

### 5.4 Storage

* Content-addressed (`b3:<hex>`), byte-range; hot tier cache; EC (Reed–Solomon) for durability; safe decompression (≤10× + absolute).&#x20;

### 5.5 Mailbox & Mods

* `svc-mailbox` is at-least-once with ACK+DLQ; idempotency keys enforced. Mods run in `svc-mod` under `svc-sandbox` with CPU/mem ceilings.&#x20;

### 5.6 Governance & Policy

* `svc-registry` supplies signed topology/regions; `ron-policy` enforces quotas/residency; both referenced by services (no ambient authority).&#x20;

---

## 6) Runbooks (Operator-Ready)

### 6.1 DHT Failover & RF Repair (updated)

**Signals:** `ron_dht_missing_records > 0` (10m warn/30m crit), `rf_observed < rf_target` (5m), `ron_dht_lookup_ms` p95 > 500 ms.
**Triage:** check `svc-dht` `/healthz`, bootstrap reachability, sample providers for hot key.
**Repair:**

```
ronctl repair --hash b3:<hex> --rf 3 --prefer-region us-east-1
```

Verify RF gauges; optionally warm caches for top N objects. **Exit:** A1/A2/A3 cleared ≥ 30m, p95 restored.&#x20;

### 6.2 Cross-Region Placement Preference

**Policy:** prefer RTT < 50 ms; hedge 2 local + 1 remote; drop to winner after 1–2 chunks.
**Daily:**

```
ronctl rebalance --region us-east-1 --rf 3 --top 100000
```

**Exit:** intra-region p95 < 80 ms; inter-region selects < 20%/24h.&#x20;

### 6.3 Amnesia Mode Hygiene (Micronode)

Run with `MICRO_PERSIST=0` by default; verify **zero on-disk artifacts**; timed key purge; ephemeral logs. Toggle via env/config matrix in CI.&#x20;

### 6.4 Rolling Upgrades

Drain via `/readyz` fail-writes; ensure mailbox DLQ drain, index facet semaphores quiesce; verify `rejected_total{reason=degraded}` patterns then flip back.&#x20;

---

## 7) Migration & Alignment Notes

* **Arti merge:** remove any `svc-arti-transport` mentions; use `ron-transport` + `--features arti`. Denylist in `xtask`.&#x20;
* **tldctl → ron-naming:** schemas/types only; no runtime.&#x20;
* **BLAKE3 consolidation & OAP max\_frame checks:** grep for legacy SHA or 64 KiB frame myths; storage chunk remains 64 KiB, distinct from OAP frame.&#x20;

---

## 8) Erasure Coding (Reed–Solomon)

Keep v1.3 parameters; **repair pacing ≤ 50 MiB/s per cluster** to avoid cache thrash; prioritize hottest content first. RF gauges must reflect pre/post repair deltas.&#x20;

---

## 9) Testing & CI Gates — Six Concerns (Definition of Done)

**SEC** — capabilities only; policy/registry signatures verified; amnesia matrix proves zero disk when off.&#x20;
**RES** — no locks across `.await`; bounded channels (Ryker); `/readyz` flips early; chaos on overlay/DHT.&#x20;
**PERF** — SLOs enforced (range start, intra/inter-region p95); hedged dials budgets; backpressure visible (quota/reject metrics).&#x20;
**ECON** — no econ semantics leak into OAP; rewards pipeline (accounting→ledger→rewarder) produces consistent audit hashes.&#x20;
**DX** — same SDK on both profiles; Interop vectors pass over TLS+Tor; DTOs `deny_unknown_fields`.&#x20;
**GOV** — runbooks present; alerts wired; registry/process versioned; dashboards show RF, DHT lookup p95, and reject reasons.&#x20;

---

## 10) Deny-Drift Guardrail

* ❌ `svc-arti-transport` (use `ron-transport` + `arti`).&#x20;
* ❌ DHT logic inside overlay or kernel (must be `svc-dht`).&#x20;
* ❌ DTOs with logic or permissive deserialization. Keep DTOs pure.&#x20;
* ❌ Persistent state in Micronode when **amnesia ON**.&#x20;

---

## 11) File Drop (where this lives)

```
docs/blueprints/Scaling_Blueprint.md       (this file)
docs/runbooks/DHT_Failover.md              (extract §6.1)
docs/runbooks/Cross_Region_Placement.md    (extract §6.2)
deploy/configs/config.public.toml          (gateway/index/storage/mailbox/overlay)
deploy/configs/config.dht.toml
deploy/scripts/test_cluster.sh
deploy/scripts/test_dht.sh
deploy/scripts/test_offline_sync.sh
```

(Keep `mock-mailbox` tiny dev crate for offline sync smoke.)&#x20;

---

## 12) TL;DR

* **Ship now:** Micronode one-binary; small-cluster LB + health eject; RF repair & placement runbooks; EC pacing; SLOs + dashboards.
* **Scale globally:** DHT discovery + policy-guided placement, hedged dials, multi-region rebalancing.
* **Canon preserved:** 33 crates, overlay/DHT split, `ron-transport` merge, OAP constants, profiles parity.

---

**Definition of Ready:** This blueprint cites **12\_Pillars.md** + **Developer Suite** and references Interop spec for OAP. **Definition of Done:** §9 gates pass in CI; denylist (§10) clean; runbooks extracted; SLOs met across both profiles.

