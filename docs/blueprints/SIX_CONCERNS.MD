

# SIX\_CONCERNS.md — Canon Spine (2025 Edition)

**Status:** Finalized — aligned to 33-crate canon, Developer Suite, and Full Project Blueprint.
**Last updated:** 2025-09-22
**Purpose:** These six cross-cutting **Concerns** form the **constitutional spine** of RustyOnions. Each is a *non-negotiable gate* that crates must satisfy. They map onto the **12 Pillars** structurally, but here we enforce **system-wide invariants**, **CI teeth**, and **drift-proof cross-checks**.

# Appendix A — Crate → Six Concerns Mapping (33/33 exact)

| Crate          | Concerns                  |
| -------------- | ------------------------- |
| macronode      | RES, PERF, GOV            |
| micronode      | SEC, RES, PERF, DX        |
| oap            | SEC, RES, PERF, DX        |
| omnigate       | SEC, PERF, GOV, DX        |
| ron-accounting | ECON, PERF, RES           |
| ron-app-sdk    | DX, SEC, RES              |
| ron-audit      | SEC, GOV                  |
| ron-auth       | SEC, GOV                  |
| ron-bus        | RES, PERF                 |
| ron-kernel     | SEC, RES, PERF, GOV       |
| ron-kms        | SEC, RES                  |
| ron-ledger     | ECON, SEC, GOV            |
| ron-metrics    | PERF, RES, GOV            |
| ron-naming     | SEC, GOV                  |
| ron-policy     | SEC, GOV                  |
| ron-proto      | DX, RES                   |
| ron-transport  | SEC, RES, PERF            |
| ryker          | RES, PERF                 |
| svc-ads        | ECON, GOV, DX             |
| svc-dht        | SEC, RES, PERF, GOV       |
| svc-edge       | PERF, DX, GOV             |
| svc-gateway    | SEC, PERF, RES, GOV       |
| svc-index      | SEC, PERF, RES, GOV       |
| svc-interop    | DX, PERF, SEC             |
| svc-mailbox    | SEC, RES, PERF, ECON      |
| svc-mod        | DX, SEC, GOV              |
| svc-overlay    | SEC, RES, PERF            |
| svc-passport   | SEC, GOV                  |
| svc-registry   | GOV, SEC                  |
| svc-rewarder   | ECON, PERF, GOV           |
| svc-sandbox    | SEC, RES, GOV             |
| svc-storage    | SEC, PERF, RES, GOV       |
| svc-wallet     | ECON, SEC, RES, DX        |


---

## Concern 1 — Concurrency & Aliasing

**Why:** Async systems fail from race conditions, deadlocks, aliasing bugs.
**Scope:** Ownership, locks, async tasks, cancellation, supervised restart.
**Crates most impacted:** `ron-kernel`, `ron-bus`, `ryker`, `ron-transport`, `svc-overlay`, `svc-dht`, `svc-index`, `svc-storage`.

**Invariants (must):**

* Never hold locks across `.await`.
* One writer per socket/connection.
* One broadcast receiver per task; overflow visible in metrics.
* Use `bytes::Bytes` for owned, zero-copy payloads.
* Config updates atomic via `Arc<Config>` (ArcSwap/RwLock).
* Cancel-safe tasks; async-Drop patterns enforced.
* Crash-only restart with jittered backoff (no blocking drops).
* Amnesia mode respected (no hidden state in tasks).

**Tooling & CI gates:**

* Clippy denies: `await_holding_lock`, `unwrap_used`, `expect_used`.
* Loom for kernel readiness interleavings.
* Miri on logic-heavy crates.
* ThreadSanitizer mandatory on critical crates.
* Tokio flavor matrix (multi vs single thread).
* TLA+ specs for readiness DAGs.

---

## Concern 2 — Hardening

**Why:** Threats = malformed inputs, overload, DoS, decompression bombs, protocol abuse.
**Scope:** Input validation, quotas, rate limits, compression guards, chaos-safe restart.
**Crates most impacted:** `svc-gateway`, `svc-edge`, `svc-storage`, `svc-index`, `svc-sandbox`, `micronode`.

**Invariants (must):**

* Structured error envelopes (400/404/413/429/503).
* Quotas applied before heavy work (fail fast).
* Decompression ratio ≤ 10:1; hard caps on absolute size.
* Connection limits + inflight bounds.
* Red-team test suite runs in CI (malformed frames, quota storms, compression bombs).
* Amnesia honored (RAM-only caches; no leak across restarts).

**Tooling & CI gates:**

* Chaos restart harness (simulate OOM, FD exhaustion).
* Fuzz/property tests for frame parsers.
* Long-running soak tests (24h) with zero FD leaks.
* Explicit `allow-sleep` tags for intentional waits.

---

## Concern 3 — Interop

**Why:** RustyOnions is a protocol substrate (OAP/1, capability tokens). Interop is survival.
**Scope:** Protocol norms, capability enforcement, SDK surface, polyglot adoption.
**Crates most impacted:** `oap`, `ron-app-sdk`, `ron-proto`, `omnigate`, `svc-interop`, `svc-mailbox`.

**Invariants (must):**

* **OAP/1:** `max_frame=1 MiB`; streaming chunks ≈64 KiB.
* BLAKE3 addressing only (`"b3:<hex>"`).
* SDKs: retries, deadlines, idempotency, tracing spans.
* Capability tokens = macaroons v1; revocation supported.
* Zero kernel creep: protocols defined in `oap`/`ron-proto`, not `ron-kernel`.
* Polyglot SDK plan (Rust, TS, Python, Swift).

**Tooling & CI gates:**

* Spec in `specs/oap-1.md` normative with vectors.
* Fuzz harness for frame parsers.
* Interop tests across SDKs (roundtrip).
* Omnigate hydration tests for Vest-style demo.

---

## Concern 4 — Microkernel

**Why:** Kernel is the diamond core; everything else plugs in.
**Scope:** Supervision, Bus, Metrics, Config, amnesia, restart semantics.
**Crates most impacted:** `ron-kernel`, `ron-bus`, `ryker`, `ron-metrics`.

**Invariants (must):**

* Frozen public API exports: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* Supervisor restarts children crash-only with backoff/jitter.
* Bus = broadcast; overflow counters visible.
* Config hot-swap atomic; rollback on invalid config.
* HealthState: readiness = true only after all deps ready.
* Amnesia flag surfaces globally; micronode runs with amnesia=ON by default.
* PQ crypto hooks: KMS supports hybrid exchanges (Kyber).

**Tooling & CI gates:**

* Loom tests for Bus/Health.
* CI denies `unsafe_code`.
* Metrics sanity tests (no duplicate names).
* TLA+ specs for health DAG.

---

## Concern 5 — Scaling

**Why:** Must scale from single laptop (micronode) to global mesh (macronode).
**Scope:** Performance, load, caching, replication, perf harnesses.
**Crates most impacted:** `svc-dht`, `svc-overlay`, `svc-storage`, `svc-index`, `macronode`, `micronode`.

**Invariants (must):**

* DHT lookups p99 ≤ 5 hops; bounded α/β concurrency.
* Storage replication bounded per peer; fanout quotas.
* Index/search latency p95 ≤ 150ms; ingest lag < 5s.
* Graph neighbor fetch p95 ≤ 50ms intra-AZ.
* Perf harness required: Criterion benches + flamegraph + load rig.
* Node profiles must scale down (micronode amnesia) and up (macronode full service).

**Tooling & CI gates:**

* Criterion benches in `testing/performance`.
* Load rigs simulating 10k+ conns.
* Perf dashboards tracked in metrics.
* Scaling Soak tests in CI (overnight).

---

## Concern 6 — App Integration

**Why:** DX is adoption. Apps (Vest, others) must integrate in minutes.
**Scope:** SDK ergonomics, docs, DX tools, BFF surfaces, governance UX.
**Crates most impacted:** `ron-app-sdk`, `omnigate`, `svc-gateway`, `svc-mailbox`, `svc-wallet`, `svc-ads`.

**Invariants (must):**

* SDK guides, quickstarts, examples present.
* Omnigate hydrates views; no logic in kernel.
* Polyglot SDK parity (Rust, TS, Python, Swift).
* Vest demo works end-to-end with OAP + Mailbox + Storage.
* Governance docs: registry SLA, revocation paths.
* DX facets (Graph, Feed, Search, Media) must hit SLOs.

**Tooling & CI gates:**

* Docs tested (examples build/run).
* `ronctl` CLI tested for cap mint/rotate.
* Developer suite obligations (perf harness, docs, red-team).

---

## Concern ↔ Crate Matrix

| Concern                | Primary Crates Impacted                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------- |
| Concurrency & Aliasing | `ron-kernel`, `ron-bus`, `ryker`, `ron-transport`, `svc-overlay`, `svc-dht`, `svc-index`, `svc-storage` |
| Hardening              | `svc-gateway`, `svc-edge`, `svc-storage`, `svc-index`, `svc-sandbox`, `micronode`                       |
| Interop                | `oap`, `ron-app-sdk`, `ron-proto`, `omnigate`, `svc-interop`, `svc-mailbox`                             |
| Microkernel            | `ron-kernel`, `ron-bus`, `ryker`, `ron-metrics`                                                         |
| Scaling                | `svc-dht`, `svc-overlay`, `svc-storage`, `svc-index`, `macronode`, `micronode`                          |
| App Integration        | `ron-app-sdk`, `omnigate`, `svc-gateway`, `svc-mailbox`, `svc-wallet`, `svc-ads`                        |

---

## Cross-cutting Developer Suite Obligations

* Every Concern requires **CI hooks**: fuzz, perf, chaos, loom, sanitizer.
* Every Concern must reference **amnesia mode** (Micronode default).
* PQ crypto and ZK hooks must not leak into kernel; live only at service layer.
* Governance docs live alongside Concern blueprints, drift-proofed.

---

✅ **Definition of Done (for this file):**

* Mentions **only canonical 33 crates**.
* Aligns with **latest blueprints** (Concurrency v1.3, Hardening v1.1, Interop GMI-1.6, etc.).
* Encodes **new invariants** (amnesia, PQ/ZK, DX suite).
* Provides **concern ↔ crate matrix** for operational clarity.

---
