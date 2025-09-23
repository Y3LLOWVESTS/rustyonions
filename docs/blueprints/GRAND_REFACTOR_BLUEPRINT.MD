---

Title: RustyOnions — Grand Refactor Blueprint (Canon v2025‑09)
Status: DRAFT → LOCK ON MERGE
Editors: Stevan White (PO), GPT‑5 Thinking (co‑author)
Last‑Updated: 2025‑09‑22 (Europe/London)
MSRV: 1.80.0 (workspace)
Scope: Entire monorepo (kernel, services, libs, transports, econ/policy)
Out‑of‑Scope: Product copy, marketing sites, non‑canon crates
-------------------------------------------------------------

# 0) Executive Summary

This blueprint codifies the *single source of truth* for the RustyOnions refactor. It freezes public APIs, clarifies crate roles and boundaries, merges legacy drift, and defines an acceptance‑gated migration plan. It composes our prior blueprints—Microkernel, Hardening, Scaling, Concurrency & Aliasing, App Integration, Interop, Omnigate, Developer Suite—into one refactor constitution.

**Objectives**

* Minimal, auditable *microkernel*; services outside.
* Strict crate taxonomy (lib vs. service vs. econ/policy vs. transport).
* Six‑Concerns spine (SEC, RES, PERF, GOV, ECON, DX) enforced in code + CI.
* Canonical public interfaces (Bus, KernelEvent, Metrics, Config, Transport).
* Operational excellence: metrics, runbooks, amnesia mode, TLS with rustls.
* Overlay/DHT clarified; Omnigate/Gateway authority bounded; Interop stable.

**Outcomes**

* A complete rename/merge plan, broken into three waves with back‑out.
* Per‑crate *IDB* (Invariant‑Driven Blueprint) stubs to fill during refactor.
* Perfection Gates (proof) to ship each wave with measurable quality.

---

# 1) Canon & Scope

## 1.1 System Canon

RustyOnions is a microkernel‑first overlay network with a capability‑secured service ecosystem. Authority is explicit, custody is minimal, and economic integrity is verifiable. The kernel is tiny; everything else composes via buses, transports, and protocols.

## 1.2 Taxonomy & Naming Conventions

* **Libraries (lib)** use prefix `ron-`.
* **Runtime Services (service)** use prefix `svc-`.
* **Node Profiles / Binaries**: `micronode` (single‑tenant dev), `macronode` (full).
* **Blueprint‑only terms** (e.g., *Omnigate*) map to concrete crates (e.g., `svc-gateway`).
* **No duplicate roles**; CLIs live inside the crate they control (e.g., `tldctl` → subcmd of naming).

---

# 2) Canonical Crate Atlas (33)

> **Note:** Items with (✓) are unchanged; (Δ) are renamed/merged; (★) require new stubs; (†) verify at merge.

**Kernel & Core**

1. `ron-kernel` (✓) — Microkernel, orchestration, supervision, bus re‑exports.
2. `ron-bus` (✓) — Tokio broadcast bus; no generics in public type.
3. `ryker` (✓) — Lightweight actor/mailbox util (internal only where needed).

**Transports & Overlay**
4\. `ron-transport` (Δ) — Unifies TCP/TLS (rustls) acceptors; *deprecate* `svc-arti-transport`—Tor integration becomes a feature of overlay or a subcrate.
5\. `svc-overlay` (✓) — Overlay service exposing OAP/1 endpoints; DHT hooks.
6\. `svc-mailbox` (✓) — Async message delivery; bounded custody; expiring queues.

**Gateway & Interop**
7\. `svc-gateway` (✓) — The concrete ‘Omnigate’; ingress/egress policy, rate‑limits.
8\. `ron-proto` (✓) — Shared DTOs/serde; semantic versioned.
9\. `svc-interop` (✓) — Bridges (HTTP/WS/GRPC/Tor) with strict adapters.

**Identity, Policy, Naming**
10\. `ron-auth` (✓) — Capability materialization, tokens.
11\. `ron-policy` (✓) — Compile‑time/Runtime policies.
12\. `ron-naming` (Δ) — Library for logical names & discovery; **absorbs** `tldctl` as `ron-naming` CLI subcommand `naming ctl ...`.
13\. `svc-passport` (✓) — Authn/Authz runtime façade.

**Economics & Ledgering**
14\. `ron-ledger` (✓) — Double‑entry primitives; verifiable moves.
15\. `ron-accounting` (✓) — Higher‑level accounting flows.
16\. `svc-wallet` (✓) — End‑user wallets (custody‑minimal).
17\. `svc-rewarder` (✓) — Rewards issuance

**Registry, Index, Storage**
18\. `svc-registry` (✓) — Service/manifest registry.
19\. `svc-index` (✓) — Content/pack index; sled/rocks backends.
20\. `svc-storage` (✓) — CAS, BLAKE3 addressing, integrity checks.

**Apps & SDK**
21\. `ron-app-sdk` (✓) — Client SDK; Rust + FFI.
22\. `micronode` (✓) — Single‑tenant developer node profile (Firebase‑like).
23\. `macronode` (✓) — Full node profile (multi‑tenant, ops‑ready).

**Observability & Ops**
24\. `ron-metrics` (✓) — Prometheus metrics, healthz/readyz.
25\. `ron-audit` (✓) — Audit log, tamper evident.
26\. `svc-sandbox` (✓) — Execution sandbox for untrusted modules.

**Modules, Ads, Mods**
27\. `svc-mod` (✓) — Module registry/loader.
28\. `svc-ads` (✓) — Ads domain rules, serving façade.

**Indexing & Edge**
29\. `svc-edge` (★†) — (Re‑instated) Edge‐optimized gateway façade (CDN‑ish policies); thin, delegates to `svc-gateway`.

**Interop Families (split out when needed)**
30\. `svc-interop-http` (★†) — Optional if `svc-interop` grows too fat.
31\. `svc-interop-grpc` (★†) — Optional split for stubbing and perf.

**Utilities**
32\. `oap` (✓) — Overlay Access Protocol constants/specs.
33\. `omnigate` (Δ) — **Blueprint alias only**; all code is in `svc-gateway`.

*Removed/absorbed:* `svc-arti-transport` (into `ron-transport` features), `tldctl` (into `ron-naming` CLI). If a Tor‑only service is still desired, introduce `svc-tor-bridge` later as a thin façade over `ron-transport` with arti features.

---

# 3) Six Concerns → Crate Mapping (spine)

| Crate                      | Concerns            |
| -------------------------- | ------------------- |
| ron-kernel                 | SEC, RES, PERF, GOV |
| ron-bus                    | RES, PERF           |
| ryker                      | RES, PERF           |
| ron-transport              | RES, SEC, PERF      |
| svc-overlay                | SEC, PERF, GOV      |
| svc-mailbox                | SEC, RES            |
| svc-gateway                | SEC, PERF, GOV, DX  |
| ron-proto                  | DX, RES             |
| svc-interop                | DX, RES, SEC        |
| ron-auth                   | SEC, GOV            |
| ron-policy                 | SEC, GOV            |
| ron-naming                 | SEC, GOV            |
| svc-passport               | SEC, GOV, DX        |
| ron-ledger                 | ECON, SEC, GOV      |
| ron-accounting             | ECON, PERF, RES     |
| svc-wallet                 | ECON, SEC, DX       |
| svc-rewarder               | ECON, GOV           |
| svc-registry               | GOV, RES            |
| svc-index                  | RES, PERF           |
| svc-storage                | RES, SEC            |
| ron-app-sdk                | DX, SEC, RES        |
| micronode                  | DX, RES, SEC        |
| macronode                  | RES, PERF, GOV      |
| ron-metrics                | PERF, RES, GOV      |
| ron-audit                  | SEC, GOV            |
| svc-sandbox                | SEC, GOV            |
| svc-mod                    | GOV, DX             |
| svc-ads                    | GOV, ECON           |
| svc-edge                   | PERF, GOV           |
| svc-interop-http           | DX, RES             |
| svc-interop-grpc           | DX, RES             |
| oap                        | DX, GOV             |
| (blueprint alias) omnigate | —                   |

> This table is the *auditable spine*. CI will verify each crate declares its concerns in `Cargo.toml` metadata and that required docs/tests exist.

---

# 4) Public API Freezes (must‑haves)

## 4.1 `ron-kernel` re‑exports

* `Bus`, `KernelEvent::{Health{service,ok}, ConfigUpdated{version}, ServiceCrashed{service}, Shutdown}`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
* **Bus**: Tokio broadcast‑based; cloneable; monomorphic public type.

## 4.2 Transport API

* `TransportConfig { addr: SocketAddr, name: &'static str, max_conns: usize, read_timeout: Duration, write_timeout: Duration, idle_timeout: Duration }`.
* `spawn_transport(cfg, metrics, health, bus, tls_override) -> Result<(JoinHandle<()>, SocketAddr)>` (async).
* TLS: use `tokio_rustls::rustls::ServerConfig` only.

## 4.3 Metrics

* `Metrics::new()`; `metrics.clone().serve(addr).await -> (JoinHandle<()>, SocketAddr)` exposing `/metrics`, `/healthz`, `/readyz`.
* Prometheus: `IntCounterVec::new(Opts, labels)`, `Histogram::with_opts(HistogramOpts)`, registered to default registry.
* Health API: `health() -> &HealthState` with `set`, `snapshot`, `all_ready`.

## 4.4 Overlay/HTTP stack

* `axum` 0.7; handlers end with `.into_response()`; DTOs derive `Serialize/Deserialize`. Crate deps: `sha2=0.10`, `base64=0.22`, `hex=0.4`.

---

# 5) Global Refactor Program (3 Waves)

## Wave 0 — Prep (Week 0)

* Pin workspace deps (tokio 1.47.1, tokio‑rustls 0.26.x, tower‑http 0.6.6, prometheus 0.14, reqwest 0.12, axum 0.7.9).
* Turn on `cargo-deny` (v0.18) with allowlist incl. Unicode‑3.0, Unicode‑DFS‑2016, CC0‑1.0, CDLA‑Permissive‑2.0, OpenSSL; deny unknown git/registries.
* Create Per‑crate IDBs and Perfection Gates (doc stubs) + RUNBOOK.md for services.

**Gates:** workspace builds; `cargo-deny` green; fmt+clippy; one demo bin per pillar runs.

## Wave 1 — Rename/Merge (Weeks 1–2)

* Absorb `tldctl` into `ron-naming` (CLI subcommands: `ctl`, `resolve`, `register`).
* Remove `svc-arti-transport`; add `arti` feature to `ron-transport` + `svc-overlay`.
* Re‑introduce `svc-edge` as thin gateway façade; route everything to `svc-gateway`.
* Normalize crate metadata (`package.name`, categories, `repository`, `documentation`).

**Gates:** all renamed crates publishable; old imports fail (CI check); demos green.

## Wave 2 — Boundary Hardening (Weeks 3–4)

* Kernel: shrink surface; no service‑specific logic inside.
* Gateway: capability checks at ingress; rate‑limit per identity/route.
* Overlay: split DHT concerns out of Gateway; Tor feature via transport; mailbox TTLs.
* Interop: adapters only; no business logic; DTO versions locked.

**Gates:** contract tests between services; fuzz critical parsers; property tests for ledger moves; chaos test for supervision.

## Wave 3 — Performance & Ops (Weeks 5–6)

* Add `bus_lagged_total`, `bus_queue_depth` gauge; latencies histograms per route.
* Heat path profiling (criterion, flamegraph); backpressure policies documented.
* Runbooks finished; dashboards shipped (Grafana JSON + alerts).

**Gates:** SLOs met (p99 latency, error budgets), soak test 24h pass, canaries.

---

# 6) Cross‑Cutting Invariants

1. **No locks across `.await`** in supervisory paths (loom where feasible).
2. **Amnesia Mode:** optional RAM‑only caches, zeroization, timed purge; *default off*.
3. **Key Custody:** minimal; define KMS boundaries; rotate keys; rustls native roots only.
4. **Economic Integrity:** double‑entry no‑double‑spend; conservation proofs.
5. **Backpressure First:** never accept more than can be processed; shed gracefully.
6. **Observability Is Non‑optional:** metrics, structured logs, traces; runbooks.

---

# 7) Module Boundaries & Ownership

* **Kernel owns** supervision, config watch, process registry, `Bus`.
* **Services own** HTTP surfaces, storage, domain logic.
* **SDK owns** client ergonomics; no backdoors; all traffic via public services.
* **Gateway owns** ingress policy; **Overlay owns** routing and DHT; **Interop** owns adapters only.
* **Ledger/Accounting** own value semantics; services call them via explicit APIs.

---

# 8) Protocols, Transports, TLS

* Prefer rustls; `tokio_rustls::rustls::ServerConfig` in public types.
* OAP/1 constants defined in `oap`; enforce via shared `ron-proto` DTOs.
* Transport timeouts: read/write/idle must be configured; sane defaults set in profiles.
* Optional Tor support = `arti` feature on transport/overlay; do not fork the stack.

---

# 9) Data & Storage: Sled, CAS, Index

* Shared pack index (`svc-index`) → same DB as gateway if co‑deployed; ensure one canonical `RON_INDEX_DB` path per node profile.
* `svc-storage`: CAS with BLAKE3, integrity verified on read; corruption quarantine.
* Naming/Registry maintain *logical → physical* resolution; cache with TTL + invalidation bus.

---

# 10) Security & Hardening

* **Capabilities everywhere**: passport issues scoped tokens; gateway validates.
* **PII minimization**: redact at edges; structured logs with fields, no secrets.
* **Sandbox**: deny‑by‑default; resource quotas; observable terminations.
* **Audit**: append‑only with hash chains; cross‑check in CI with unit proofs.
* **Amnesia Mode**: VM/sandbox knobs, RAM caches, timed purge; documented risks.

---

# 11) Scaling & Performance

* **Load Profiles** per service; explicit p50/p95/p99 targets.
* **Rate Limits** at gateway (token/class based); queue depth‑aware shedding.
* **Concurrency**: bounded mailboxes; cooperative shutdown; async drop policy.
* **Bench Discipline**: Criterion benches for hot paths; regressions gate PRs.

---

# 12) Governance & Lifecycle

* **Policy** crate holds domain/ads/mod rules; services read only declared subsets.
* **Versioning**: SemVer on `ron-proto`; migrations documented; deprecations signaled.
* **Docs Canon**: Every crate carries: README, IDB, RUNBOOK (services), SECURITY, PERFORMANCE, CONCURRENCY, QUANTUM, OBSERVABILITY.

---

# 13) Developer Suite & DX

* **Micronode** is the dev experience: one binary, local TLS, dashboards, hot reload.
* **Scaffolds**: codegen templates for services with locked deps & CI skeleton.
* **No Node.js required** to build core; optional for web UI only.

---

# 14) Interop & App Integration

* **Interop** is adapters plus DTO normalization; no business logic.
* **App Integration** uses `ron-app-sdk`; client permissions are capabilities; never hidden endpoints.
* **Versioned Contracts**: clients peg to proto versions; feature gates map to service capabilities.

---

# 15) Omnigate (Concrete = `svc-gateway`)

* Strict ingress policy; JWT/Passport validation; per‑route SLOs.
* Caches (short TTL); consistent error taxonomy; retry hints for clients.
* Canary routes & dark traffic wiring for safe deploys.

---

# 16) Migration Plan — Detailed Checklist

**Repo hygiene**

* [ ] Root workspace `[workspace.dependencies]` pins.
* [ ] Single version of tower‑http, rand, thiserror (prefer v2), etc.
* [ ] `deny.toml` updated; sources/ban/advise checks green.

**Crate moves**

* [ ] `tldctl` → `ron-naming` subcommand.
* [ ] `svc-arti-transport` removed; `arti` feature implemented.
* [ ] `svc-edge` scaffolded; delegates to `svc-gateway`.

**APIs**

* [ ] `ron-kernel` re‑exports exact; `Bus` type monomorphic.
* [ ] `Transport` API signatures match; TLS type uses tokio\_rustls.
* [ ] `ron-metrics` serves `/metrics|/healthz|/readyz`.

**Testing**

* [ ] Contract tests between `svc-gateway` ↔ `svc-overlay` ↔ `svc-interop`.
* [ ] Ledger property tests (conservation/no double spend).
* [ ] Fuzz DTO parsers (arbitrary JSON/CBOR if present).
* [ ] Supervision chaos: child panics restart with jittered backoff.

**Ops**

* [ ] Runbooks per service complete.
* [ ] Grafana dashboards shipped; alerts wired.
* [ ] Amnesia mode documented; flags present; zeroization verified.

---

# 17) Acceptance Gates (Perfection)

A change cannot merge unless **all** apply:

* ✅ Builds on clean CI with fmt+clippy (no warnings in touched crates).
* ✅ `cargo-deny` green across licenses/bans/advisories/sources.
* ✅ Public API diffs reviewed (rustdoc JSON compare) for kernel, transport, proto.
* ✅ SLOs met in CI load test job; performance budgets respected.
* ✅ Docs present: README, IDB, RUNBOOK (services), SECURITY, PERFORMANCE, CONCURRENCY, QUANTUM, OBSERVABILITY.
* ✅ Back‑out plan documented for each wave.

---

# 18) Risk Register (Top 8)

1. **API Drift** between services — *Mitigation*: contract tests + rustdoc diff.
2. **TLS Type Mismatch** — *Mitigation*: compile‑time tests enforce tokio\_rustls types.
3. **Index Split‑Brain** — *Mitigation*: single `RON_INDEX_DB`; startup asserts.
4. **Backpressure Bugs** — *Mitigation*: queues sized; shedding + SLO tests.
5. **Security Scope Creep** — *Mitigation*: policy read‑only adapters; audits.
6. **Tor Integration Fragility** — *Mitigation*: feature‑flag only; single transport.
7. **Observability Gaps** — *Mitigation*: gates demand dashboards + alerts.
8. **Economic Invariants Break** — *Mitigation*: property tests; audit trails.

---

# 19) What To Do Next (48‑hour plan)

* Day 1 AM: Land workspace dependency pins + `deny.toml` baseline; create IDBs.
* Day 1 PM: Rename/absorb `tldctl`, deprecate `svc-arti-transport`, scaffold `svc-edge`.
* Day 2 AM: Kernel/Transport API conformance tests; axum 0.7 audit across services.
* Day 2 PM: Contract/fuzz/prop tests skeletons; Runbook template drop‑in.

---

# 20) Appendices

## A) IDB (Invariant‑Driven Blueprint) Stub (copy into each crate)

```
---
title: <crate> — IDB
version: 0.1.0
status: draft
last-updated: 2025‑09‑22
audience: contributors, ops
concerns: [SEC|RES|PERF|GOV|ECON|DX]
---

## 1) Invariants (MUST)
- …

## 2) Design Principles (SHOULD)
- …

## 3) Implementation Patterns (HOW)
- …

## 4) Acceptance Gates (PROOF)
- …
```

## B) RUNBOOK Skeleton (services)

* Purpose, SLOs, Endpoints, Dashboards
* Deploy, Rollback, Canary
* Incidents (playbooks), Limits, Quotas, Amnesia flags

## C) Concurrency Checklist

* No blocking in Drop; cooperative shutdown; bounded channels; loom plan

## D) Observability Checklist

* Metrics: counters, histograms, gauges; structured logs; trace spans

## E) Quantum Readiness (PQ)

* Inventory crypto; plan PQC pilots; TLS ciphersuites policy; migration map

---

**This document is the working constitution for the refactor.** Update the Atlas and the Six‑Concerns mapping if and only if the crate roles change; otherwise, treat drift as a bug and gate it in CI.
