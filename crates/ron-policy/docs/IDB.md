---

title: ron-policy — Invariant-Driven Blueprint (IDB)
version: 0.1.2
status: reviewed
last-updated: 2025-10-05
audience: contributors, ops, auditors
-------------------------------------

# ron-policy — IDB

**Role (Pillar 2 • Policy & Governance).** Declarative governance for the mesh: **quotas, SLAs, geo/residency & placement, abuse ceilings (Trust & Safety), rollout toggles/feature flags, and advisory PQ/ECON hooks**. Authorship + versioning live here; **runtime enforcement is in services** (gateway, storage, index, overlay, etc.). `svc-registry` publishes signed descriptors; services subscribe and enforce.

---

## 1) Invariants (MUST)

* **[I-1] Canon alignment.** `ron-policy` is a **pure library** within the fixed 33-crate canon (Pillar 2). No crate renames/splits via this work.
* **[I-2] Declarative only.** Policies are **data** (schemas, versions, signatures). No network/DB/file I/O and no side-effects in this crate.
* **[I-3] Versioned, signed, auditable.** Every policy bundle has a monotonically increasing `version`, a content-address (`id_b3 = "b3:<hex>"`), is referenced by **multi-sig** in `svc-registry`, and emits a `PolicyChanged{version,id_b3}` bus event.
* **[I-4] Capability-first.** All allow/deny outcomes are expressed relative to a **capability** (macaroon/passport). No ambient authority.
* **[I-5] Tighten-only vs. platform bounds.** Policy may only **tighten** global hardening (e.g., OAP frame/body ≤ **1 MiB**; decompression ratio ≤ **10×**); it may **never relax** them.
* **[I-6] Residency & placement are explicit and deterministic.** Geography must be declared (allowed/required/denied regions; placement prefs). Evaluation must be deterministic for the same inputs.
* **[I-7] Amnesia-aware.** If a rule needs persistence, it **must** declare `requires_persistence = true`; Micronode may degrade/deny accordingly.
* **[I-8] Deterministic evaluation.** `(policy, input, clock)` → `Decision{allow, reason, obligations}` is **pure** and repeatable; a clock is provided by the caller.
* **[I-9] Observability hooks only.** This crate defines decision taxonomies/metric **names** but does not start servers/exporters (that’s `ron-metrics`/services).
* **[I-10] Interop-safe DTOs.** DTOs use `#[serde(deny_unknown_fields)]`; schema/semver compatibility is enforced by tests.
* **[I-11] ECON/ZK hooks are declarative.** Policies may declare **proof obligations** (e.g., rewards/anti-abuse) as **Obligations**; no keys or crypto execution here.
* **[I-12] Evaluator performance bound.** With ≤ **1k rules** for common paths, evaluation must meet **p95 < 1 ms** on a modest core; otherwise the change is rejected or requires route scoping.
* **[I-13] Churn safety / monotonic rollouts.** Any **widening** (e.g., residency expansion or raised limits) requires `metadata.break_change = true` **and** a runbook link; otherwise reject.

---

## 2) Design Principles (SHOULD)

* **[P-1] Small, composable primitives.** Quotas as rate families (**RPS, B/s, inflight, storage bytes**) with scopes `{global, per_cap, per_peer, per_route}`.
* **[P-2] Shape = who/what/where/limits/obligations.**
  `subject` (cap audience) • `object` (route/topic/cid-class) • `where` (regions/placement) • `limits` (ceilings) • `obligations` (**Trust & Safety** hints like tarpit; **ECON/ZK** proof hints).
* **[P-3] Fail-closed, degrade early.** Under breach, **shed writes first**; reads may degrade if marked safe.
* **[P-4] Layered evaluation.** **Global → Service-class → Route → Subject(cap) → Instance hints**; first hard deny wins; soft hints accumulate.
* **[P-5] No secret material.** Keys remain in `ron-kms`/`svc-passport`. Policies reference **key IDs/proof kinds**, not secrets.
* **[P-6] Ops-first rollouts.** Every change carries runbook tags, alert names, dashboards, and a rollback (canary % + time window).
* **[P-7] DX & polyglot clarity.** Authoring is human-friendly (**TOML**), canonical wire is **JSON** with versioned JSON Schema for non-Rust consumers.

---

## 3) Implementation (HOW)

### 3.1 Data model (paste-ready)

```rust
// crates/ron-policy/src/model.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyBundle {
    pub version: u64,
    pub id_b3: String,                        // "b3:<hex>"
    pub issued_at_epoch_ms: u64,
    pub residency: Residency,
    pub quotas: Vec<QuotaRule>,
    pub routes: Vec<RouteRule>,
    pub features: Features,
    pub metadata: Meta,                       // runbook, dashboards, break_change
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Residency {
    pub allowed_regions: Vec<String>,
    pub required_regions: Vec<String>,
    pub deny_regions: Vec<String>,
    pub placement: PlacementPrefs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlacementPrefs {
    pub prefer_rtt_ms: u32,                   // e.g., 50
    pub hedge_local: u8,                      // parallelism hints
    pub hedge_remote: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Scope { Global, PerCap, PerPeer, PerRoute }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum AppliesTo { Ingress, Storage, Mailbox, Overlay, Index }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Limit {
    Rps(u32),
    BytesPerSec(u64),
    Inflight(u32),
    StorageBytes(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuotaRule {
    pub scope: Scope,
    pub limit: Limit,
    pub applies_to: AppliesTo,
    pub when_anonymous: bool,
    pub burst: Option<u32>,                   // token-bucket burst
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RouteRule {
    pub route: String,                        // "GET /o/*", "POST /put"
    pub max_body_bytes: Option<u64>,          // ≤ 1 MiB (tighten-only)
    pub decompress_ratio_max: Option<f32>,    // ≤ 10.0 (tighten-only)
    pub require_cap: bool,
    pub obligations: Vec<Obligation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Features {
    pub amnesia_ok: bool,
    pub requires_persistence: bool,
    pub pq_required: bool,                    // advisory; transport/proto enforce
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub change_reason: String,
    pub runbook_url: Option<String>,
    pub dashboards: Vec<String>,
    pub break_change: bool,                   // required for widening changes
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub allow: bool,
    pub reason: &'static str,
    pub obligations: Vec<Obligation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Obligation {
    AuditTag(&'static str),
    AddHeader(&'static str, String),
    DegradeWritesFirst,
    Tarpit(u64),                              // Trust & Safety throttle (ms)
    // ECON/ZK hints (declarative only)
    RequireProof { proof_kind: &'static str, scope: Scope }, // e.g., "zk-capped-spend"
}
```

### 3.2 Deterministic evaluation (sketch)

```rust
// crates/ron-policy/src/eval.rs
use crate::model::*;

pub struct RequestCtx<'a> {
    pub route: &'a str,
    pub body_len: u64,
    pub target_region: &'a str,
    pub has_cap: bool,
    pub peer_id: &'a str,
    pub cap_audience: Option<&'a str>,
}

#[inline]
fn deny(reason: &'static str) -> Decision {
    Decision { allow: false, reason, obligations: vec![Obligation::AuditTag(reason)] }
}

pub fn evaluate(bundle: &PolicyBundle, ctx: &RequestCtx, _now_ms: u64) -> Decision {
    // 1) Residency hard checks
    if bundle.residency.deny_regions.iter().any(|r| r == ctx.target_region) {
        return deny("region.denied");
    }
    if !bundle.residency.allowed_regions.is_empty()
        && !bundle.residency.allowed_regions.iter().any(|r| r == ctx.target_region)
    {
        return deny("region.not_allowed");
    }

    // 2) Route bounds (tighten-only caps are also validated at load)
    if let Some(route) = route_match(&bundle.routes, ctx.route) {
        if route.require_cap && !ctx.has_cap { return deny("cap.required"); }
        if let Some(mb) = route.max_body_bytes { if ctx.body_len > mb { return deny("body.too_large"); } }
        if violates_decompress_guard(route, ctx) { return deny("decompress.guard"); }
    }

    // 3) Accumulate obligations (Trust & Safety, ECON/ZK, amnesia behavior)
    let mut obligations = vec![Obligation::AuditTag("policy.ok")];
    if !bundle.features.requires_persistence {
        obligations.push(Obligation::DegradeWritesFirst);
    }
    obligations.extend(route_obligations(&bundle.routes, ctx.route));

    Decision { allow: true, reason: "ok", obligations }
}

// helpers: route_match, violates_decompress_guard, route_obligations (pure, deterministic)
```

### 3.3 Wire format & signing

* **Authoring:** TOML (human-friendly).
* **Canonical wire:** JSON with versioned **JSON Schema** (polyglot interop).
* **Identity:** `id_b3 = BLAKE3(canonical_bytes)`.
* **Publication:** `svc-registry` multi-sig references the bundle; bus emits `PolicyChanged{version,id_b3}`.

> Minimal JSON Schema lives in `crates/ron-policy/schema/policybundle.schema.json` and is required for CI validation.

### 3.4 Example bundle (TOML)

```toml
version = 43
id_b3   = "b3:89ab...cdef"
issued_at_epoch_ms = 1764950400000

[residency]
allowed_regions   = ["us-east-1","eu-central-1"]
required_regions  = ["us-east-1"]
deny_regions      = ["cn-north-1"]
[residency.placement]
prefer_rtt_ms = 50
hedge_local   = 2
hedge_remote  = 1

[[quotas]]
scope = "Global"
applies_to = "Ingress"
limit = { Rps = 500 }
when_anonymous = true
burst = 100

[[routes]]
route = "GET /o/*"
require_cap = false
max_body_bytes = 1048576
decompress_ratio_max = 10.0
obligations = [
  { RequireProof = { proof_kind="zk-capped-spend", scope="PerCap" } },
  # Example Trust & Safety throttle for hot paths:
  # { Tarpit = 25 }
]

[features]
amnesia_ok = true
requires_persistence = false
pq_required = false

[metadata]
change_reason = "Tighten anon GET limits; add ZK spend hint"
runbook_url = "https://ops/runbooks/policy-43"
dashboards = ["grafana://policy/overview"]
break_change = false
```

### 3.5 Evaluation flow (Mermaid)

```mermaid
flowchart TD
  A[RequestCtx] --> B[Residency Checks]
  B -- deny --> Z[Decision{allow=false}]
  B -- ok --> C[Route Match & Caps]
  C -- deny --> Z
  C -- ok --> D[Accumulate Obligations (T&S / ECON / Amnesia)]
  D --> E[Decision{allow=true, reason="ok"}]
```

---

## 4) Acceptance Gates (PROOF)

* **[G-1] Schema & tighten-only.**
  DTOs use `deny_unknown_fields`. Validator fails CI if any `max_body_bytes > 1_048_576` or `decompress_ratio_max > 10.0`.
* **[G-2] Determinism.**
  Serde round-trip stability; equality of decisions for identical `(bundle, ctx, clock)`.
* **[G-3] Bus/registry integration.**
  Simulated multi-sig update in `svc-registry` yields a single `PolicyChanged{version+1,id_b3}` event.
* **[G-4] Amnesia matrix.**
  If `requires_persistence=true`, Micronode integration tests assert **degrade/deny**, never silent persistence.
* **[G-5] Residency conformance.**
  Resolver simulation respects `prefer_rtt_ms` & hedges; local region preferred when available.
* **[G-6] Six Concerns routing.**
  CI labels `SEC,GOV,PERF` must pass; hardening overlaps respected; no ambient authority introduced.
* **[G-7] Canon guard.**
  Grep-based CI ensures references stay within the 33-crate canon; unknown namespaces are rejected.
* **[G-8] Performance.**
  Criterion bench: 10k evaluations of a ≤1k-rule policy must meet **p95 < 1 ms** per eval on a modest core.
* **[G-9] ECON/ZK obligations.**
  If `Obligation::RequireProof` is present, services surface hooks (no-op here) and documentation links the runbook.
* **[G-10] Churn/rollback simulation.**
  Widening residency/limits vs. previous bundle **must** set `metadata.break_change=true` and provide a `runbook_url`; otherwise validator rejects. Test simulates N→N+1 widening and expects failure without `break_change`.

**Tighten-only CI hints (example):**

```
rg -n '"max_body_bytes"\s*:\s*(\d+)' crates/ron-policy | awk '{ if ($2>1048576) { print "body cap violated @ " $0; exit 2 } }'
rg -n '"decompress_ratio_max"\s*:\s*([0-9.]+)' crates/ron-policy | awk '{ if ($2>10.0) { print "decompress cap violated @ " $0; exit 2 } }'
```

**Churn lint (example):**

```
# Reject widening residency unless break_change=true appears in the new bundle
git diff --cached -U0 -- crates/ron-policy | rg '^\+.*allowed_regions' -n >/dev/null && \
git diff --cached -U0 -- crates/ron-policy | rg '^\+.*"break_change"\s*:\s*true' -n >/dev/null || \
{ echo "Residency widened without break_change=true"; exit 2; }
```

---

## 5) Anti-Scope (Forbidden)

* ❌ **No service loops, network/file/DB I/O.** This crate is pure data + evaluation.
* ❌ **No ambient trust toggles.** Decisions remain capability-relative.
* ❌ **No relaxing platform hardening.** Frame/body caps and decompression guard are upper bounds.
* ❌ **No embedding keys/secrets.** Only IDs and declarative proof hints allowed.
* ❌ **No kernel coupling or async locks.** Keep evaluation pure; no locks across `.await` (none exist in this crate).

---

## 6) References

* **Complete Crate List & 12 Pillars** — Pillar-2 mapping and boundaries.
* **Full Project Blueprint** — content addressing, OAP constants, Six Concerns, profiles, CI gates.
* **Hardening Blueprint** — IO limits, decompression guard, ingress profiles.
* **Scaling Blueprint** — residency/placement preferences, hedge patterns, runbooks.
* **Six Concerns (SEC/GOV/PERF/ECON)** — concern routing and review labels.
* **Interop & App Integration Blueprints** — DTO stability, bus events, policy usage across services.
* **JSON Schema** — `schema/policybundle.schema.json` (canonical wire contract).

---

### Definition of Done (for this IDB)

* **MUSTs first** with **tighten-only** and churn protections; each MUST maps to a **gate**.
* **Pure, deterministic** evaluator with paste-ready DTOs and example bundle.
* **PERF & ECON/ZK hooks** included without breaking purity or boundaries.
* **Visual flow** included; rollouts/rollback encoded via `metadata.break_change` + runbook.
