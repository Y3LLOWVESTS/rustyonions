
Below is a tightened, drop-in **PERFORMANCE.md** that incorporates those upgrades.

---

title: Performance & Scaling — ron-policy
status: draft
msrv: 1.80.0
crate_type: lib
last-updated: 2025-10-06
audience: contributors, ops, perf testers

---

# ⚡ PERFORMANCE.md — ron-policy

## 0. Purpose

`ron-policy` is a **pure, deterministic library** for validating and evaluating declarative policy bundles. It performs **no I/O**, keeps **no global state**, and must remain **side-effect free**. This doc defines its performance profile, harnesses, knobs, and CI gates.

It ties into:

* **Scaling Blueprint v1.3.1** (read-path budgets, runbooks)
* **Omnigate** (policy must evaluate **before fan-out**)
* **Perfection Gates**: F (perf regressions barred), L (scaling chaos-validated by consumers)

### Anti-Scope (to prevent drift)

* MUST NOT perform network or disk I/O.
* MUST NOT depend on wall-clock/timezones except via explicit `(clock)` input.
* MUST NOT introduce nondeterminism (randomness, global caches, thread-locals).
* MUST keep evaluation results a pure function of `(bundle, ctx, clock)`.

---

## 1. SLOs / Targets (Library)

### 1.1 Latency / Throughput

* **Eval p95 < 1 ms** on hot paths with ≤ **1,000 rules** in the matched route-class.
* **Throughput/core** (informative): ≥ **250k evals/min** sustained on modern x86/ARM with LTO+release.
* Complexity: **O(m)** in candidate rules for the route-class; precompute indices to keep `m` small.

### 1.2 Determinism & Stability

* Identical `(bundle, ctx, clock)` → identical `Decision{allow|deny, reason, obligations}`.
* DTOs use `#[serde(deny_unknown_fields)]`; canonical field ordering for stable hashes if consumers need them.

### 1.3 Platform Budget Fit (informative)

* Policy eval must be a **micro-budget** step within service SLOs (e.g., GET p95 80/200ms intra/inter). Sub-ms keeps ample headroom for I/O and hydration in Omnigate.

---

## 2. Benchmarks & Harness

### 2.1 Micro-bench (Criterion)

Scenarios (commit under `benches/`):

* `hot_allow_500`: 500 rules, allow path
* `hot_deny_500`: 500 rules, deny with reason
* `hot_mixed_1k`: 1,000 rules, 50/50 allow/deny
* `pathological_caps`: near caps to verify hard limits remain sub-ms

Run:

```
cargo bench -p ron-policy
```

Artifacts: `testing/performance/baselines/ron-policy/msrv-1.80.0/v{SEMVER}/*.json`

### 2.2 Integration / Load (Consumer Services)

Services (e.g., gateway/index) run wrk/bombardier profiles to confirm policy eval is not the bottleneck. Dashboards track `policy_eval_latency_seconds` and `policy_decisions_total`.

### 2.3 Profiling

* **cargo flamegraph** for hotspots (matchers, reason formatting).
* **perf** / **coz** for causal profiling.
* **heaptrack** to bound allocations/op; target **zero-alloc allow path**.

### 2.4 Fuzz & Property Tests (Perf-relevant)

* **cargo-fuzz** target: generate bundles/contexts that maximize candidate rule sets; assert:

  * evaluation terminates within cap,
  * allocations/op stay within envelope,
  * no panics on valid inputs.
* **proptest**: route-class tries/prefix sets; property “narrowing a class cannot increase candidate set”.

### 2.5 Chaos/Perf Blend (in Consumers)

* Latency injection, slow-loris at gateway, compression-bomb inputs to confirm **tighten-only caps** are enforced ahead of eval.

---

## 3. Scaling Knobs

### Library Knobs

* **Precompute indices** on bundle validate/load (prefix/trie or DFA for paths/methods/audiences).
* **Reason strings** as enum→`&'static str` to avoid heap churn on hot allow path.
* Prefer **borrowed slices / `bytes::Bytes`**; avoid clones in hot loop.
* Optional feature flag `policy_smallvec` for small fixed-cap stacks (e.g., obligations buffer).

### Consumer Knobs (Guidance)

* **Parallelism cap**: start with `min(2 * cores, 64)` concurrent evals per replica; tune under load.
* **Snapshoting**: parse/validate once; **atomic swap** `Arc<BundleSnapshot>`; keep last-good on failure.
* **No locks across `.await`** on code surrounding eval; prefer per-request locals.
* **Horizontal scale**: add replicas; policy eval is in-process and embarrassingly parallel.

---

## 4. Bottlenecks & Known Limits

* **Rule explosion / over-broad classes**: mitigate via precompute + lint rules; cap total rules and per-class candidates.
* **Serde decode on reload**: one-time cost; keep hot path free of parsing.
* **Regex hazards**: prefer trie/prefix; avoid user-controlled unbounded regex in hot path.
* **Caps (tighten-only)**: enforce **body ≤ 1 MiB** and **decompress ratio ≤ 10×** prior to policy evaluation.
* **Snapshot churn**: excessive reloads can evict CPU cache benefits—debounce reload frequency.

**Milestone tie-in**: These are Bronze-level baselines; Gold requires verified zero-alloc allow path and documented upper bounds at caps.

---

## 5. Regression Gates (CI)

CI fails if any of the below regress vs baselines for MSRV and current semver:

* **Eval p95** ↑ **> 10%** on `hot_mixed_1k`.
* **Throughput (ops/s)** ↓ **> 10%** on `hot_mixed_1k`.
* **Allocations/op or CPU/op** ↑ **> 15%** (heaptrack/perf samples).
* **Code size** (lib) ↑ **> 15%** without justification (binary bloat guard).

Baselines live under:

```
testing/performance/baselines/ron-policy/msrv-1.80.0/v{SEMVER}/
```

Waivers allowed only if traced to upstream dependency and risk-assessed in PR.

---

## 6. Perf Runbook (Triage)

1. **Flamegraph**: check matcher dispatch, candidate set sizing, reason formatting.
2. **Metrics**: inspect `policy_eval_latency_seconds{decision}`; spikes in `deny` often indicate rule-set drift.
3. **Candidate set**: recompute stats; ensure indices active; run lints for over-broad route classes.
4. **Caps**: confirm 1 MiB / 10× enforced ahead of eval.
5. **Consumer stalls**: verify no locks across `.await`; check `tokio-console` in services.
6. **Snapshot hygiene**: if reload failed, revert to last-good; inspect validation errors.
7. **Heap**: run heaptrack; trim allocations on deny path if excessive.
8. **Fuzz repro**: re-run fuzz corpus on current build to catch perf cliffs.

---

## 7. Acceptance Checklist (DoD)

* [ ] Criterion benches pass; **p95 < 1 ms** (≤1k rules); artifacts committed.
* [ ] Flamegraph captured & linked once per minor release.
* [ ] Fuzz/property tests included; CI runs minimal corpus nightly.
* [ ] Loom check on **snapshot swap** path (consumer crate) documented; no perf-affecting races.
* [ ] Baselines versioned by **MSRV** and **semver**.
* [ ] Observability: consumers emit `policy_eval_latency_seconds` + `policy_decisions_total`.
* [ ] Runbook validated during a soak (≥ 2h) in at least one consumer.

---

## 8. PQ & Amnesia Notes

* **PQ posture**: `ron-policy` is crypto-agnostic; if callers add hybrid/PQ token verification before policy, keep policy eval sub-ms by preserving precompute/zero-alloc hot path.
* **Amnesia**: lib holds no state; consumers (e.g., micronode) must handle snapshot zeroization/ephemerality per their hardening profile.

---

## 9. Appendix

### Reference Workloads (Consumers)

* gwsmoke/bombardier: GET/HEAD/RANGE + policy-heavy routes; 24h soak on echo/mailbox with policy toggles.

### PromQL (Consumer Dashboards)

```promql
histogram_quantile(0.95, sum(rate(policy_eval_latency_seconds_bucket[5m])) by (le))
```

### Mermaid — Eval Path (consumer context)

```mermaid
sequenceDiagram
  autonumber
  participant S as Service (gateway/index)
  participant P as ron-policy (lib)
  participant R as RouteClass Index
  S->>S: Receive request (ctx built)
  S->>P: evaluate(bundle, ctx, clock)
  P->>R: select candidate rules (prefix/trie)
  R-->>P: candidate set
  P->>P: match & obligations (zero-alloc allow path)
  P-->>S: Decision{allow|deny, reason, obligations}
  S->>S: If allow → proceed to fan-out; if deny → respond 403
```

### Handy Commands

Micro-bench:

```
cargo bench -p ron-policy
```

Perf (Linux):

```
perf record --call-graph=dwarf -- cargo bench -p ron-policy
perf report
```

Heap allocations:

```
heaptrack cargo bench -p ron-policy
heaptrack_print heaptrack.*.gz
```

---


