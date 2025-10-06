

---

# 🏛 GOVERNANCE.md

---

title: Governance & Economic Integrity
status: draft
msrv: 1.80.0
last-updated: 2025-10-05
audience: contributors, ops, auditors, stakeholders
crate-type: policy
------------------

# GOVERNANCE.md

## 0. Purpose

This document defines the **rules of engagement** for `ron-metrics`’s **metrics-contract and endpoint policy**.

It ensures:

* Transparent and auditable decision-making for any change to **metric families, labels, units, or endpoints**.
* Enforcement of **contract invariants** (stable names/labels, low cardinality, no secrets/PII).
* Clear authority boundaries and appeal paths.
* SLA-backed commitments for `/metrics`, `/healthz`, `/readyz`.

It ties into:

* **Observability Canon** (golden metrics, label base set).
* **Hardening Blueprint** (GET-only, loopback/UDS by default, TLS invariant).
* **Perfection Gates A–O** (esp. Gate **I**: bounded contract invariants; Gate **M**: appeal paths; Gate **F/L**: perf/scaling protected).

> `ron-metrics` is **not** an economic or ledger crate. All “bounded issuance / no doubles” language maps here to **“no breaking drift / no duplicate or renamed metrics without process.”**

---

## 1. Invariants (MUST)

**Non-negotiable rules for this crate’s public contract:**

* **[I-M1] Contract Stability**: Existing **metric family names** and **label keys** are stable. Renames/removals are **breaking** and require a major version bump + RFC.
* **[I-M2] Base Labels**: All first-party families MUST carry `service`, `instance`, `build_version`, `amnesia`.
* **[I-M3] Units & Suffixes**: Allowed suffixes are only `*_seconds`, `*_bytes`, `*_total`; unit semantics never change post-release.
* **[I-M4] Cardinality & Privacy**: No high-cardinality or unbounded labels; **no secrets/PII** in names, labels, HELP strings, or values.
* **[I-M5] PQ/ZK Presence**: PQ/ZK families exist even when features are disabled (values may be zero) to stabilize dashboards.
* **[I-M6] Endpoint Policy**: Exposer is **GET-only**. Non-GET → `405`. Bind **loopback/UDS** by default; if remote scrape, TLS type is **`tokio_rustls::rustls::ServerConfig`** only.
* **[I-M7] Amnesia Truthfulness**: `amnesia="on|off"` reflects the actual runtime mode; it is not optional.
* **[I-M8] OTLP Egress Safety**: Feature `otel` may export, but does **not** alter the Rust public API; mapping is 1:1 (resource/attributes).
* **[I-M9] Readiness Semantics**: `/readyz` **degrades before collapse** (fail-open reads / fail-closed writes) and returns JSON + `Retry-After` on `503`.
* **[I-M10] Auditability**: Every change to the metrics contract or endpoint behavior is **logged, diffed, and review-gated** (API snapshot + registry spec + CHANGELOG).

---

## 2. Roles & Authority

### Roles

* **Observability Policy Owner (OPO)** — owns the metrics canon and this document.
* **Registry Steward (RS)** — maintains the **metrics registry spec** and API snapshots.
* **Security & Privacy Reviewer (SPR)** — enforces [I-M4], TLS invariant [I-M6], and emergency redactions.
* **Ops On-Call (Ops)** — owns runbook execution, scrape posture, and SLA monitoring.
* **Release Manager (RM)** — enforces SemVer gates and CHANGELOG/API snapshot alignment.
* **Auditor** — independent read-only review of diffs, CI logs, and artifacts.

### Authority Boundaries

* **Who can merge contract changes?** RS, with **2 approvals** minimum: OPO **and** one of SPR/RM.
* **Who can flip OTLP egress?** Host service owners; **does not** waive invariants or change the Rust API.
* **Who can push emergency redaction?** SPR (see §4 Emergency) may **hotfix-remove or mask** a leaking family/label; postmortem + PR required within 24h.
* **Ops** cannot change label keys or families at runtime; only **values** of base labels reflect deployment reality.

---

## 3. Rules & SLAs

* **Endpoint SLAs** (steady state, loopback unless stated; also see PERFORMANCE.md):

  * `/healthz`: p95 ≤ **2 ms**, availability **≥ 99.99%**.
  * `/readyz` (ready): p95 ≤ **3 ms**, availability **≥ 99.99%**; not-ready returns `503` with `Retry-After`.
  * `/metrics` (first-party set): p95 ≤ **5 ms** local; large registries documented separately (≤ **60 ms** p95 target).
  * 5xx (excluding intended `503` for not-ready) **< 0.1%**.

* **Change-Review SLA** (contract changes):

  * **Triage within 1 business day**, decision within **3 business days** after required approvals.
  * Each contract change must update **API.md**, **INTEROP.md** (if schema-visible), **TESTS.md** (new checks), and **CHANGELOG**.

* **Contract Freeze**: 48h before a tagged release, no contract changes except emergency redactions (SPR-approved).

---

## 4. Governance Process

### Normal Changes (Additive, Non-breaking)

1. **Proposal PR** includes:

   * `docs/metrics-registry.yaml` (or TOML) diff with new family/labels.
   * Rationale + cardinality analysis; privacy checklist; perf impact estimate.
   * Tests: taxonomy & base-label checks, `/metrics` sample update.
2. **CI Gates**: public-api diff (informational), taxonomy script (suffix/base labels), promtool check, perf smoke.
3. **Approvals**: OPO + (SPR **or** RM).
4. **Merge** → snapshot stored under `/docs/api-history/ron-metrics/<version>.txt`.

### Breaking Changes (Renames/Removals/Units)

* **RFC required**; bump **major** version; migration guide + dashboard rewrites attached.
* Additional approvals: **OPO + SPR + RM** (3 total).
* A grace deprecation period is preferred (dual-write/dual-expose where feasible).

### Emergency Powers (Security/Privacy Breach)

* **Trigger**: data leak (secrets/PII) or operational hazard (runaway cardinality).
* **Action**: SPR may land a **hotfix redaction** (remove family/label or blank values) bypassing freeze **with OPO notification**.
* **Disclosure**: audit entry and postmortem within **24h**; follow-up PR restores canonical process.

### Parameter Changes

* Any change to **SLOs**, **readiness policy**, or **transport invariants** follows the Normal or Breaking path accordingly and updates RUNBOOK/PERFORMANCE/INTEROP.

---

## 5. Audit & Observability

* **Append-only audit**: PR titles prefixed `[metrics-contract]`, linked to CI artifact containing:

  * `/metrics` sample before/after (small + large registry modes),
  * taxonomy check output, promtool validation, perf smoke JSON.
* **Governance metrics** (exported by host or tests):

  * `metrics_contract_changes_total{action="add|remove|rename|label_add|label_drop"}`
  * `metrics_exposition_failures_total{cause}`
  * `metrics_page_size_bytes` (gauge, sampled)
  * `exposition_latency_seconds{endpoint}` (histogram)
* **Verifiability**: CI stores API snapshot diffs and the registry YAML as artifacts for auditors.

---

## 6. Config & Custody

* **Config surface (host-owned)** must declare:

  * Bind target (`RON_METRICS_ADDR` **or** `RON_METRICS_UDS`).
  * TLS paths if used (`RON_METRICS_TLS_CERT`, `RON_METRICS_TLS_KEY`) – **must** be rustls-compatible.
  * `RON_AMNESIA` (on/off) – reflected in labels.
  * `RON_OTEL_ENDPOINT` (if `otel`) — egress only.

* **Custody**:

  * TLS/private keys in **ron-kms/HSM** or platform secret store.
  * **No raw keys** in repo/env files; rotation **≤ 90 days** or after suspected compromise.
  * OTLP creds (if any) governed by platform secrets; rotating them must not change the Rust API.

---

## 7. Appeal Path

When a change is rejected or redacted:

1. **Raise** an appeal issue tagged `appeal:metrics-contract` referencing the PR/RFC.
2. **Review Board**: OPO + SPR + RM convene within **2 business days**; evidence-based decision.
3. **Outcome** is recorded in the audit log; if policy changes result, update GOVERNANCE.md and templates.
4. **No silent rollbacks**: production keeps either the old or the revised contract; transitional dual-exposure is preferred, never a hidden revert.

---

## 8. Acceptance Checklist (DoD)

* [ ] Invariants [I-M1]…[I-M10] are implemented and enforced by **tests + CI**.
* [ ] Roles/authority boundaries documented; reviewers assigned in CODEOWNERS.
* [ ] Proposal lifecycle + emergency process documented and **tested** (tabletop or simulated).
* [ ] Metrics & audit artifacts exported/stored per release.
* [ ] SLA thresholds monitored (RUNBOOK/PERFORMANCE) with alerts.
* [ ] Appeal path exercised at least once in a drill (Gate **M**).

---

## 9. Appendix

* **Blueprints:**

  * Hardening (endpoint policy, TLS, loopback/UDS default).
  * Scaling/Performance (SLOs & budgets).
  * Perfection Gates **I** (bounded contract), **F/L** (perf/scaling), **M** (appeals).

* **References:**

  * API.md (public surface + SemVer for metrics contract).
  * INTEROP.md (schemas for `/readyz`, Prom v0).
  * PERFORMANCE.md, RUNBOOK.md, TESTS.md (gates, SLOs, runbooks).
  * Governance registry file: `docs/metrics-registry.yaml` (authoritative list of families/labels/HELP/units).

* **History:**

  * Record notable contract transitions (e.g., addition of `zk_proof_latency_seconds`, label additions like `{role}`, emergency redactions and their postmortems).

---

✅ With this governance, `ron-metrics` treats **metrics as API**: stable, audited, privacy-safe, and SLAs enforced. Changes are deliberate, observable, and reversible only through a clear, accountable process.
