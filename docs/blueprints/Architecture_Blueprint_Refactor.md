Absolutely—here’s the drop-in **MasterBlueprint.md** with only the minimal, “God-tier” upgrades I’m confident will strengthen correctness, reviewability, and long-term safety without changing your scope, crates, or pillar boundaries. I kept your structure and wording, and only added: (1) invariant tiering, (2) a dependency-graph gate in CI, (3) loom/miri hooks for concurrency/UB checks, and (4) a tiny “Additional Interfaces (stubs)” appendix to close the last interface gaps. I also fixed a tiny formatting hiccup in Rule #5.

---

# RustyOnions – Exhaustive Architecture Blueprint (Refactor Edition)

> **Purpose**: This blueprint is the definitive, implementation-ready reference for the major refactor. It encodes boundaries, invariants, interfaces, acceptance gates, and enforcement mechanisms across the **12 pillars** and **33 product crates**.
>
> **Use**: Treat each pillar section as review gates for PRs. Every crate change must include the pillar’s checklists under *Acceptance Gates*.

---

## 0. Canonical Topology

* **Total product crates (fixed)**: **33**
* **Do not add/remove** product crates during the refactor; non-product tooling (e.g., `xtask`, `fuzz`, `model`, `docs`, `examples`) may be added under `/tools` (publish = false).
* **Cross-cutting doctrine**: *narrow interfaces, acyclic deps, capability-passing (no ambient authority), independent tests, no global mutable state, and strict DTO contracts.*

**Canonical 33 (collision-safe names)**

1. ron-kernel
2. ron-bus
3. ryker
4. ron-policy
5. ron-kms
6. ron-auth
7. svc-passport
8. ron-audit
9. **ron-metrics** *(was `metrics`)*
10. svc-gateway
11. omnigate
12. ron-app-sdk
13. oap
14. micronode
15. macronode
16. svc-storage
17. svc-index
18. **ron-naming** *(was `naming`)*
19. tldctl
20. svc-overlay
21. **ron-transport** *(was `transport`)*
22. svc-arti-transport
23. svc-mailbox
24. svc-sandbox
25. ron-ledger  *(absorbs reporting/“billing”)*
26. svc-wallet
27. **ron-accounting** *(was `accounting`)*
28. svc-rewarder
29. svc-registry
30. svc-mod
31. svc-ads
32. **svc-interop** *(was `interop`)*
33. ron-proto

---

## 1. Pillars Overview

We organize the system into **12 pillars**. Each pillar lists: **Scope**, **Boundaries**, **Anti-scope**, **Key interfaces**, **Invariants**, **Design principles**, **Implementation patterns**, **Acceptance gates**.

### Pillar 1 — Kernel & Orchestration

**Crates**: `ron-kernel`, `ron-bus`, `ryker`

**Scope**

* Process lifecycle, supervision trees, restart/backoff policy
* Actor runtime (`ryker`) wiring under the kernel
* Event bus fan-out (`ron-bus`) as *intra-system* signalling

**Boundaries**

* No business logic, no persistence, no network protocol parsing
* Kernel supervises; others implement domain behavior

**Anti-scope**

* No direct ledger/ron-accounting calls
* No HTTP/gRPC handlers (route via `svc-gateway`/`omnigate`)

**Key Interfaces**

* `KernelService` trait: `start()`, `shutdown(grace: Duration)`
* `Bus<Event>` publish/subscribe with bounded queues; backpressure required

**Invariants**

* *I1*: Supervisor never blocks on unbounded `.await` holding a lock
* *I2*: Panics in children are contained; restart policy applied with jittered backoff
* *I3*: Bus fan-out cannot drop critical signals silently (bounded queue + DLQ or NACK pathway)

**Design Principles**

* Small kernel, pluggable services; actor isolation; bounded mailboxes

**Implementation Patterns**

* `tokio::task::Builder` with names; `tower::Service` adapters for backoff
* `select!` with cancellation tokens for graceful shutdown

**Acceptance Gates**

* No `.await` while holding `Mutex`/`RwLock` (lint or `xtask` AST)
* Property tests: fan-out delivery guarantees with bounded queue simulation
* Chaos test: child panics restart within SLO (<1s median)

---

### Pillar 2 — Policy & Capability Control

**Crates**: `ron-policy`

**Scope**

* Pricing/quotas/splits, authorization policies
* Pure decision layer (no side effects)

**Boundaries**

* No DB writes, no network calls; input-only (context), output-only (decision)

**Anti-scope**

* No token mint/burn; no ledger mutation

**Key Interfaces**

* `Policy::evaluate(ctx: PolicyCtx) -> Decision` (serializable)

**Invariants**

* *I4*: Policy is deterministic for a given context
* *I5*: Policy decisions are explainable (audit fields present)

**Design Principles**

* Declarative rules; versioned policy bundles

**Implementation Patterns**

* `serde(tag="type")` enums for rules; deny unknown fields

**Acceptance Gates**

* `#[non_exhaustive]` on public enums; `deny_unknown_fields` on DTOs
* Golden-tests for decision changes; diff review required

---

### Pillar 3 — Identity & Key Management

**Crates**: `ron-kms`, `ron-auth`, `svc-passport`

**Scope**

* Key custody and rotation, capability envelopes (macaroons/attestations)
* Passport issuance and revocation (module inside `svc-passport`)

**Boundaries**

* No business authorization; only cryptographic identity and proof

**Anti-scope**

* No application session state

**Key Interfaces**

* `Kms::sign(msg) -> Signature`; `Kms::rotate(key_id)`
* `Auth::verify(envelope) -> Claims`

**Invariants**

* *I6*: Zeroizable key material, no long-lived plaintext in memory
* *I7*: All external presentation tokens are time-bound and audience-scoped

**Design Principles**

* Explicit capabilities over roles; PQ-ready curves behind feature flags

**Implementation Patterns**

* `zeroize` on secrets; `ring`/`rustls` for primitives (or FFI modules)

**Acceptance Gates**

* Secrets implement `Zeroize`; memory tests verify zeroization
* Fuzz tests for envelope parsers; `cargo deny` clean

---

### Pillar 4 — Audit & Compliance

**Crates**: `ron-audit`

**Scope**

* Append-only logs, evidence journaling, integrity proofs

**Boundaries**

* Not ron-metrics; not ledger entries (economic)

**Key Interfaces**

* `Audit::record(event: AuditEvent) -> Receipt`

**Invariants**

* *I8*: Audit storage is append-only; tamper-evident

**Acceptance Gates**

* Red-team test: mutation attempts produce detectable divergence

---

### Pillar 5 — Observability & Ops

**Crates**: `ron-metrics`

**Scope**

* Health/readiness, metrics, tracing glue, redaction rules

**Boundaries**

* No policy, no audit, no money logic

**Invariants**

* *I9*: PII never surfaces in logs or labels; redaction rules enforced

**Acceptance Gates**

* Static grep for PII-ish fields; integration test asserts `/metrics` label cardinality budgets

---

### Pillar 6 — Ingress (Gateway)

**Crates**: `svc-gateway`

**Scope**

* HTTP ingress, termination, quotas, edge validation

**Boundaries**

* No domain logic; routes to BFF/SDK layer

**Invariants**

* *I10*: All external DTOs use `serde` with `deny_unknown_fields`, `rename_all="snake_case"`

**Acceptance Gates**

* Contract tests for DTO compatibility; fuzz inputs for path/query parsers

---

### Pillar 7 — App BFF & SDK

**Crates**: `omnigate`, `ron-app-sdk`, `oap`, `micronode`, `macronode`

**Scope**

* DTO shaping for apps, composition of use-cases, developer ergonomics

**Boundaries**

* Not ingress, not kernel, not ledger

**Invariants**

* *I11*: BFF composes domain services, never embeds storage/network primitives directly

**Acceptance Gates**

* Snapshot tests for public DTOs; doc tests runnable in <30 minutes to first success

---

### Pillar 8 — Content Addressing & Naming

**Crates**: `svc-storage`, `svc-index`, `ron-naming`, `tldctl`

**Scope**

* Storage (immutable files/blocks), index (lookup), naming (semantic mapping), TLD admin

**Boundaries**

* Storage ≠ Index ≠ Naming; distinct state machines and SLOs

**Invariants**

* *I12*: Content hashes are canonical; any write is either idempotent or versioned

**Acceptance Gates**

* Load tests for PUT/GET; property tests for hash canonicalization

---

### Pillar 9 — Overlay & Transport

**Crates**: `svc-overlay`, `ron-transport`, `svc-arti-transport`, `svc-mailbox`

**Scope**

* Moving bits: overlay manifests, transports (QUIC/TLS/TOR), encrypted store-and-forward

**Boundaries**

* No persistence decisions (hand off to storage); no policy decisions

**Invariants**

* *I13*: No plaintext payloads in memory beyond decrypt window
* *I14*: Envelope replay protection per channel

**Acceptance Gates**

* TSan clean under stress; replay tests across simulated partitions

---

### Pillar 10 — Discovery / Safety

**Crates**: `svc-sandbox`

**Scope**

* Containment for risky workloads, sandboxes, feature flag canaries

**Invariants**

* *I15*: Sandbox processes cannot access kernel capability without explicit grant

**Acceptance Gates**

* Escape tests; seccomp/AppArmor policies validated in CI (where applicable)

---

### Pillar 11 — Economics & Wallets

**Crates**: `ron-ledger`, `svc-wallet`, `ron-accounting`, `svc-rewarder`

**Scope**

* Value movement (ledger), user balances (wallet), usage accounting, reward systems

**Boundaries**

* Payment provider adapters are feature modules; economic invariants centralized in ledger

**Invariants**

* *I16*: **Conservation**: Σ credits == Σ debits in any transaction family
* *I17*: **Idempotency**: retried request keys (ULID) cannot double-spend

**Acceptance Gates**

* Property tests for conservation over random splits; integration rollback tests; coverage >85% on ledger paths

---

### Pillar 12 — Governance & Interop

**Crates**: `svc-registry`, `svc-mod`, `svc-ads`, `svc-interop`, `ron-proto`

**Scope**

* Registry lifecycle rules, moderation + appeals, ads escrow; cross-system bridges; shared DTO/error contracts

**Invariants**

* *I18*: Registry operations are auditable and reversible within policy; interop DTOs versioned with non-breaking forward-compat rules

**Acceptance Gates**

* Contract compatibility tests across minor versions; governance state machine exhaustive-match tests

---

## 2. Cross-Pillar Rules (Hard Boundaries)

1. **Acyclic deps**: No cycles permitted between product crates. Any discovered cycle must be broken via interface crate or trait injection.
2. **No ambient authority**: All capabilities (keys, network, storage) are passed explicitly.
3. **DTO discipline**: `serde(tag="type", rename_all="snake_case", deny_unknown_fields)` on all public payloads.
4. **Enum-first modeling** over strings/bools; newtypes for IDs.
5. **No `.await` while holding a lock**; no `tokio::spawn` outside supervised contexts.
6. **Public API hygiene**: `#[non_exhaustive]` for exported structs/enums; no `pub` fields in structs (constructors/Builders only).
7. **Secrets** zeroized; no long-lived plaintext; configs use sealed wrappers.
8. **Time & randomness** behind traits for testability.

> **Invariant tiers (new, review-only):** mark each I# as **Critical** (build-blocking) or **Advisory** (warn). PRs touching any Critical invariant **must** include at least one test tagged with that invariant ID.

---

## 3. Enforcement (Lint Wall + XTask + CI)

**Rustc/Clippy lints** (workspace-level):

* `unreachable_pub`, `private_interfaces`, `private_bounds`, `missing_docs` (public)
* Clippy: `exhaustive_enums`, `exhaustive_structs`, `enum_glob_use`, `cast_lossless`, `missing_docs_in_private_items` (if adopted)
* Deny: `pub_struct_field` (custom via xtask AST), `locks_across_await` (xtask), `stringly_states` (xtask regex → AST), `serde_contracts` (xtask)

**xtask** (custom checks):

* AST rules: forbid `pub` fields; enforce `#[non_exhaustive]`; detect `.await` while holding `Mutex/RwLock`; ensure finite enums for state machines; ensure `Zeroize` on secrets; **forbid free `tokio::spawn`** outside supervised contexts
* Contract scan: `serde` attributes present on all public DTOs
* Invariant tags: grep for `I-<num>` in tests; require at least one test per **Critical** invariant in affected crates

**CI**

* `cargo clippy --workspace --all-features -- -D warnings`
* `cargo test --workspace --all-features` (plus `--no-run` build check)
* **Dependency Graph Gate (new)**: generate and upload a DAG artifact per PR (`cargo-depgraph`/`cargo-modules`); fail on cycles
* Sanitizers (optional job, nightly): ASan + TSan test passes for overlay/**ron-transport**/ledger
* **UB & Concurrency check (new optional jobs)**: `miri test` (UB) and `loom` model tests (see §6) for concurrency hotspots
* Supply chain: `cargo deny check` clean
* Docs sync: README checked against crate docs (optional `cargo-sync-readme`)

---

## 4. Interfaces (Selected)

**Bus**

```rust
pub trait Event: Send + Sync + 'static {}
pub trait Bus<E: Event> {
    fn publish(&self, evt: E) -> Result<(), BusError>;
    fn subscribe(&self) -> Subscription<E>;
}
```

**Kernel Supervision**

```rust
pub trait KernelService {
    async fn start(&self, cancel: CancellationToken) -> anyhow::Result<()>;
    async fn shutdown(&self, grace: Duration) -> anyhow::Result<()>;
}
```

**Policy**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Decision { Allow { quota: u64 }, Deny { reason: String } }
```

**Ledger**

```rust
pub struct DebitRequest {
    pub id: Ulid,
    pub amount: u64,
    pub splits: Vec<(Recipient, BasisPoints)>,
}
```

---

## 5. Performance & SLOs (per Pillar)

* **Kernel**: child restart median <1s; 99p < 5s
* **Gateway**: p99 handler < 100ms at P50 RPS; no allocation spikes > 2× baseline
* \*\*Overlay/\*\***ron-transport**: sustained throughput >= target; packet loss tolerance tested; replay window bounded
* **Storage/Index**: PUT p99 < 200ms for 1MB; index lookup p99 < 50ms under load
* **Ledger**: transaction commit p99 < 150ms; idempotent retries under 2× attempt duration

---

## 6. Testing Strategy

* **Property tests** for conservation (ledger), canonical hashing (storage), routing invariants (overlay)
* **Contract tests** for all public DTOs (serde configs; deny unknown fields)
* **Fuzz tests** for parsers (gateway/**svc-interop**)
* **Concurrency tests** for kernel/bus (bounded queue behavior under load)
* **Snapshot tests** for SDK outputs & BFF DTOs
* **NEW (recommended):** **Loom** model tests for critical lock-free/async invariants (e.g., bus fan-out, replay windows); **Miri** on core crates to catch UB in unsafe/FFI edges

---

## 7. Refactor Execution Plan

1. **Freeze surfaces**: generate `crates_ranked.csv` from dump; lock top-5 crates as first wave (kernel, bus, **ron-naming**, ledger, gateway)
2. **Introduce shims**: re-export old API paths from facade modules; deprecate old names
3. **Enforce lint wall** across workspace (clippy + xtask) with allow-lists per crate to burn down warnings
4. **Feature cleanup**: use `feature_hotspots.csv`; collapse or flip default features
5. **Break cycles** (if any) with interface traits or protocol crates (`ron-proto`) — verified by the **Dependency Graph Gate**
6. **Consolidate DTOs** into `ron-proto`; enable `deny_unknown_fields` and version headers
7. **Sanity perf passes** per pillar; capture SLOs in dashboards (ron-metrics)

---

## 8. Migration & Back-compat

* **Semver policy**: `#[non_exhaustive]` on public enums/structs; additive-only on minor
* **Deprecations**: annotate and re-export for ≥1 minor release; CI ensures no hard breaks for dependents
* **Data migrations**: versioned schemas; dual-read/dual-write windows as needed

---

## 9. Ops Playbooks

* Runbook templates for each pillar: startup checks, health signals, rollback steps
* Alert routing by SLO burn; dashboards per pillar (gateway, overlay, ledger)

---

## 10. Ownership & Review

* Assign **Pillar Owners** and **Crate Maintainers**; PRs require at least one owner approval for the pillar
* Review must include *Acceptance Gates* checklist; CI must pass xtask + lint wall

---

## 11. Appendix A — Acceptance Gates (Master Checklist)

For each PR touching a crate, copy the gate list relevant to its pillar and check:

* [ ] No locks held across `.await` (AST lint clean)
* [ ] No `tokio::spawn` outside supervised contexts
* [ ] DTOs: `serde(tag, rename_all, deny_unknown_fields)` present; contract tests updated
* [ ] `#[non_exhaustive]` on public enums/structs; no `pub` fields
* [ ] Zeroize on secrets; no plaintext in logs
* [ ] Invariant tests present for affected `I-*` tags (**Critical invariants must have tests**)
* [ ] Performance budgets not regressed (bench/CI trend)
* [ ] Docs and examples updated; READMEs synced (optional)

---

## 12. Appendix B — Tooling Profiles

* **Fast profile**: clippy + tests + xtask; no sanitizers
* **Full profile** (nightly): +ASan +TSan +cargo-deny +doc sync +**depgraph artifact** +**miri** +**loom** (where present)

---

## 13. Appendix C — Module Placement (to avoid crate explosion)

* `svc-passport::revocation`
* `ron-kms::secrets`
* `svc-overlay::{discovery,dht,routing}`
* `ron-metrics::{logging,ops}`
* `ledger::adapters` or `**ron-accounting**::payment`

> Promotion rule: Only elevate a module to a new crate if it (1) needs independent deploy cadence, (2) has a crisp interface, and (3) yields build/CI parallelism wins.

---

## 14. Appendix D — Additional Interfaces (stubs) *(new, tiny, to close gaps)*

**Audit Receipt (Pillar 4)**

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub id: Ulid,
    pub event_hash: blake3::Hash,
    pub chain_hash: blake3::Hash, // Merkle/chain tip for tamper evidence
    pub ts: chrono::DateTime<chrono::Utc>,
}
```

**Overlay Replay Guard (Pillar 9)**

```rust
pub trait ReplayWindow {
    fn seen(&self, key: &[u8]) -> bool;       // returns true if duplicate
    fn record(&self, key: &[u8], ttl: Duration);
}
```

**Wallet Surface (Pillar 11)**

```rust
#[async_trait::async_trait]
pub trait Wallet {
    async fn balance(&self, acct: AccountId) -> Result<Amount>;
    async fn credit(&self, acct: AccountId, tx: CreditTx) -> Result<ReceiptId>;
    async fn debit(&self, acct: AccountId, tx: DebitRequest) -> Result<ReceiptId>; // idempotent by ULID
}
```

---

**This document is the binding reference for the refactor.** Commit it at `blueprints/Architecture_Blueprint_Refactor.md` and enforce it via `xtask` and CI gates.
