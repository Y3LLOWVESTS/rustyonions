

**File:** `crates/ryker/docs/SECURITY.md`

````markdown
---
title: Security Notes — ryker
crate: ryker
owner: Stevan White
last-reviewed: 2025-09-28
status: draft
---

# Security Documentation — ryker

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ryker`.  
It complements the repo-wide [Hardening Blueprint](../../HARDENING_BLUEPRINT.MD) and [Interop Blueprint](../../INTEROP_BLUEPRINT.MD).

> **Scope note:** `ryker` is a **library crate** (actor & mailbox runtime). It performs **no network I/O, no TLS, no auth**.  
> Security controls here focus on **memory hygiene**, **backpressure & deadlines**, **log hygiene**, and **supply chain**.  
> Network/authZ/authN remain **host responsibilities** (gateways/services that embed `ryker`).

---

## 1) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant in `ryker`? | Mitigation (in/around `ryker`) |
|---|---|---:|---|
| **S**poofing | Fake clients, unauth’d peers | No | Host terminates TLS and enforces macaroon caps/OIDC/etc. `ryker` sees already-authenticated calls. |
| **T**ampering | Mutated frames, corrupted DB writes | No | Host protocols perform framing, checksums/CAS. Inside `ryker`, Rust type-safety, bounds checks (e.g., `max_msg_bytes`). |
| **R**epudiation | Missing/inauditable actions | Yes | Structured JSON logs via `tracing`; include `actor`, `reason`, optional `corr_id` from host; counters for drops/restarts. |
| **I**nformation Disclosure | PII/secrets in logs, memory residue | Yes | **Never** log message contents; amnesia mode + `zeroize_on_drop=true`; short-lived buffers; bounded caps; optional zeroize hooks. |
| **D**enial of Service | Unbounded queues, slow consumers, floods | Yes | **Bounded-only** mailboxes, `try_send` → `Busy`; fairness/yield; per-message cap ≤ **1 MiB**; default deadline (10ms–60s window). |
| **E**levation of Privilege | Bypass of authZ/role boundaries | No | Host applies capability tokens (macaroons) & role separation; `ryker` exposes no privileged operations by itself. |

> Where marked **host-level**, the embedding service MUST enforce those mitigations. `ryker` provides the primitives (bounded queues, deadlines) to help services remain safe under load.

---

## 2) Security Boundaries

- **Inbound (to ryker lib):**  
  - Public API calls (e.g., `RykerConfig::from_env_validated()`, `Runtime::new`, `mailbox.send/recv`, actor construction).  
  - **Untrusted inputs** at this boundary: environment variables and (dev-only) `RYKER_CONFIG_PATH` file when allowed.

- **Outbound (from ryker lib):**  
  - **Metrics** via host’s registry (no HTTP listener in ryker).  
  - **Logs** via `tracing` (JSON-structured, host-configured sinks).  
  - **No direct network/database calls**.

- **Trust Zone:**  
  - Runs **in-process** with the host service; inherits the host’s trust zone/tenant.  
  - `ryker` state is **process memory only**; no persistence.

- **Assumptions:**  
  - Kernel invariants (Bus, Metrics, Health, Config) hold at repo level.  
  - Hosts surface `/healthz`/`/readyz` and quotas and are responsible for network edges.  
  - If amnesia mode is enabled repo-wide, host disables persistence and honors zeroization hints.

---

## 3) Key & Credential Handling

- **Keys handled by ryker:** None.  
- **Message/Data buffers:** transient, **in-memory only**.  
- **Amnesia mode:** when `amnesia.enabled=true`, require `zeroize_on_drop=true` (enforced by config validation) and avoid any hidden persistence (e.g., no tempfile dumps).  
- **Rotation policy:** N/A (no keys). Hosts rotate their own credentials.  
- **Zeroization:** transient buffers are cleared where feasible; strictly enforced when amnesia is on.  
- **Post-Quantum note:** ryker contains **no cryptography**. PQ readiness is handled by repo-wide QUANTUM.md; ryker is explicitly **PQ-N/A**.

---

## 4) Hardening Checklist (tailored to ryker)

| Requirement | Status | Proof (test/CI) |
|---|---|---|
| Bounded mailboxes only; default capacity 256 (configurable) | [ ] | Unit test: mailbox overflows return Busy |
| Per-message cap: `max_msg_bytes ∈ [1 KiB, 1 MiB]` | [ ] | Config validation tests |
| Default deadline enforced; must be within `[10 ms, 60 s]` | [ ] | Deadline bound tests |
| Backpressure over buffering: `try_send` → Busy; count `mailbox_dropped_total` | [ ] | Property test + metrics counter |
| Fairness: yield after `yield_every_n_msgs`; avoid starvation | [ ] | Loom model + unit fairness test |
| No lock across `.await` | [ ] | Clippy lint `await_holding_lock` |
| Log hygiene: structured logs; never log message bodies/secrets | [ ] | Log schema test |
| Amnesia: if enabled → `zeroize_on_drop=true` | [ ] | Zeroize property test |
| Untrusted config inputs validated; prod guard for `RYKER_CONFIG_PATH` | [ ] | Config env tests |
| Supply chain: `cargo-deny` clean; advisories (cargo-audit) in CI | [ ] | CI job check |

---

## 5) Observability for Security

- **Metrics:**  
  - `mailbox_dropped_total{actor,reason="capacity"|"deadline"}`  
  - `busy_rejections_total{actor}`  
  - `bus_lagged_total` (if broadcast used)  
  - `actor_restarts_total{actor}` (host-supervised)  
- **Logs:** JSON w/ fields: `service`, `actor`, `event`, `reason`, `corr_id` (if propagated), `counts`.  
- **Traces:** Spans for actor handle lifecycles, deadlines, and timeouts (propagate `corr_id`).  
- **Health gates:** Ryker has no endpoints; hosts must fail-closed on degraded state and reflect ryker counters in `/readyz`.

---

## 6) Dependencies & Supply Chain

- **Security-sensitive deps used by ryker:**  
  - `serde` + `humantime_serde` (config parsing; strict types)  
  - `thiserror` (typed errors)  
  - `zeroize` (amnesia support)  
- **Pinned at workspace root** (minimize duplicate versions).  
- **Controls:**  
  - `cargo-deny` (licenses/bans/advisories/sources) in CI  
  - `cargo-audit` (advisories/vulnerabilities) in CI  
  - Clippy `-D warnings`  
  - Review on **Tokio bump**

---

## 7) Formal & Destructive Validation

- **Unit/Property tests:**  
  - Config invariants (deadline/window/size bounds) fail-closed.  
  - Backpressure property: at capacity, `try_send` returns Busy; counters increment.  
  - “No lock across await” spot checks (Clippy lint).  
- **Loom (dev-only):**  
  - Producers → bounded mailbox → consumer + shutdown; assert no deadlocks/missed cancel/double-drop.  
- **Fuzz:**  
  - Fuzz `parse_size` / env parsing to reject weird encodings without panic.  
- **Chaos (host integration):**  
  - Kill/restart actors under load; verify drains occur and counters/logs reflect drops/timeouts.  
- **Amnesia validation:**  
  - Property test that buffers are zeroized on drop when amnesia=ON.

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Security contact:** security@rustyonions.dev  
- **Disclosure policy:** See repo root `SECURITY.md`.

---

## 9) Migration & Upgrades

- **Behavioral changes** to caps, deadlines, or overflow semantics → document in `CHANGELOG.md`.  
- **Deprecations** (e.g., env var renames) must include aliases + WARN for ≥1 minor release and a clear end-of-support date.  
- **Security-relevant deprecations:** must include **audit log entries** for repudiation resistance.  
- **Breaking** security boundary changes (if any) require a major version bump.

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  H[Host Service] -->|enqueue msg| R{ryker: mailbox}
  R -->|cap check (≤1MiB)| R2[handle]
  R -.->|queue full| E1[Reject Busy + metrics]
  R2 -->|deadline ok| OK[Success]
  R2 -.->|deadline hit| E2[Timeout + metrics]

  style R fill:#b91c1c,stroke:#7f1d1d,color:#fff
  style R2 fill:#b91c1c,stroke:#7f1d1d,color:#fff
  style E1 fill:#dc2626,stroke:#7f1d1d,color:#fff
  style E2 fill:#dc2626,stroke:#7f1d1d,color:#fff
````

**Text:** Host submits work to a bounded ryker mailbox. Ryker enforces size caps and a default deadline.
On overflow → Busy with metrics; on deadline exceed → Timeout with metrics; message contents are never logged.
In amnesia mode, transient buffers are zeroized on drop and no disk artifacts are produced.

---

```

