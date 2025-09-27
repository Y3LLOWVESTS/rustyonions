

---

**File:** `crates/ron-kernel/docs/QUANTUM.md`

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-09-26
audience: contributors, security auditors, ops
crate: ron-kernel
crate-type: lib
pillar: 1               # Kernel & orchestration
owners: [Stevan White]
---

# QUANTUM.md — ron-kernel

## 0) Purpose

Explain how `ron-kernel` (the microkernel: supervision, config, health/readiness, in-proc bus, observability) supports a safe transition to **post-quantum** cryptography **without owning crypto**.  
The kernel is **crypto/protocol-neutral**: it does **not** implement KEX/signatures or wire enforcement; instead it:
- **Surfaces posture** (PQ flags, amnesia) via health snapshots and metrics.
- **Binds policy** at the orchestration layer (readiness gating, config hooks).
- **Gives CI teeth** (feature matrices, interop sanity, downgrade-attack visibility).

Scope covers: exposure assessment, pass-through config/flags, metrics, tests, rollout plan, and HNDL exposure **as it pertains to kernel-owned state** (which is intentionally minimal and mostly transient).

---

## 1) Exposure Assessment (What’s at risk?)

**Kernel crypto usage:** _None directly._ The kernel does not negotiate handshakes, verify signatures, or encrypt payloads. It may **hold config** that toggles PQ posture in other crates and **emit events/metrics** about that posture.

- **Public-key usage (breakable by Shor):**  
  - **KEX:** _N/A in kernel_ (done by `ron-transport` / services).  
  - **Signatures:** _N/A in kernel_ (e.g., `ron-auth`, `svc-passport`).

- **Symmetric/Hash (Grover-affected):**  
  - Kernel computes **no** content hashes for trust; content addressing (BLAKE3) is enforced in services.  
  - Kernel may log/verbalize “`b3:<hex>`” identifiers as *metadata only*.

- **Data at rest / long-lived artifacts (kernel):**  
  - Transient logs/metrics, ephemeral config snapshots, supervisor state.  
  - **Retention:** should be short (hours–days) or controlled by ops.  
  - **HNDL risk:** **Low** (kernel does not store ciphertexts or long-term secrets).

- **Transport/session lifetimes:** Managed by services; kernel tracks readiness only. Typical TLS sessions minutes–hours → **reduced HNDL exposure** when hybrid is enabled by services.

- **Worst-case blast radius if classical PKI breaks:**  
  - Peers could negotiate classical handshakes in services; kernel itself remains uncompromised but **must** reflect policy failure (e.g., `/readyz` degraded if PQ is mandated but unavailable) and **must not mask downgrades**.

> **HNDL = Harvest-Now-Decrypt-Later**. For kernel, risk is indirect: visibility and policy enforcement (readiness/alerts) rather than secret exposure.

---

## 2) Current Crypto Profile (Today)

- **Algorithms in use in kernel:** _None directly._  
- **Libraries:** Kernel uses **no crypto primitives** beyond any standard hashing in build/test tooling (not runtime critical path).
- **Key custody:** _N/A_ (no keys handled by kernel). Custody lives in `ron-kms`/HSM and consuming services.
- **Interfaces that carry crypto:** Kernel **observes** (does not enforce) protocol constants (e.g., OAP/1 frame limits, content addressing), and **propagates** PQ posture via health snapshots/bus events.

---

## 3) Target PQ Posture (Where we’re going)

- **Principle:** Kernel stays **PQ-agnostic**, but **first-class aware**.
- **KEX / Transport:** Implemented in `ron-transport`/services. Kernel **surfaces** negotiated mode:
  - `pq_mode ∈ { "off", "hybrid", "pq-only" }`
  - `kex ∈ { "x25519", "ml-kem", "hybrid" }`
- **Signatures:** Implemented in `ron-auth`/policy/ledger crates; kernel **does not sign/verify**.
- **Backwards compatibility:** Kernel allows classical systems to operate **until policy flips**. When a policy/tenant requires PQ, kernel **drives readiness to degraded/fail** if upstreams can’t negotiate required posture.
- **Amnesia mode:** Kernel exports amnesia status (`on|off`) to encourage RAM-only caches and zeroization downstream; orthogonal but synergistic with PQ.

---

## 4) Feature Flags & Config (How to turn it on)

> Kernel provides **pass-through** features and config **without runtime crypto**. These make posture visible and testable end-to-end.

```toml
# Cargo features (kernel — plumbing only; no crypto inside kernel)
[features]
pq = []              # Enable PQ posture plumbing (metrics labels, health fields)
pq-hybrid = ["pq"]   # Assume hybrid KEX in downstream transports; affect readiness wiring
pq-only = ["pq"]     # When service policy requires PQ-only, kernel enforces readiness gating

# No PQ libraries are linked here; they live in transport/auth/kms crates.
````

```ini
# Config (ron-kernel::Config subset; names illustrative)
[pq]
enabled = true             # Enables posture surfacing/labels (no crypto)
hybrid_default = false     # Ops hint: expect hybrid by default upstream (M3→true)
require_pq = false         # If true, /readyz is degraded when peers are classical
```

* **Interoperability switch:** When `require_pq=true`, peers without PQ cause **degraded** or **fail** readiness (policy-driven).
* **Metrics:** Always emit **pq labels** (with zero counts when off) to simplify adoption dashboards.

---

## 5) Migration Plan (Milestones)

**M1 (Bronze) — Planning & Hooks**

* Add `pq`, `pq-hybrid`, `pq-only` features and matching config fields; **no behavior change** beyond posture surfacing.
* Add kernel metrics labels (`pq_mode`, `kex`, `sig`) and health snapshot fields.
* Ensure CI matrix builds kernel with `--features pq,pq-hybrid,pq-only`.

**M2 (Silver) — Hybrid Enablement (Downstream)**

* Transports enable **Hybrid KEX** (`X25519+ML-KEM`) behind feature flags.
* Kernel **observes** negotiated posture and sets readiness semantics per policy:

  * `require_pq=false`: informational metrics only.
  * `require_pq=true`: **degraded** or **fail** readiness if peers are classical.
* Interop tests (in transport/service crates) exercise classical↔classical, hybrid↔hybrid, hybrid↔classical; kernel asserts posture propagation.

**M3 (Gold) — Default & Ops**

* Default `pq.hybrid_default = true` for app-facing edges (when the ecosystem is ready).
* Runbooks: enable/rollback steps, dashboards with PQ panels.
* Kernel QUANTUM.md/SECURITY.md finalized; acceptance gates enforced in CI.

**Post-M3**

* Support `pq-only` deployments where policy mandates.
* Periodic PQ re-evaluation; version pinning and compat checks.

---

## 6) Invariants (MUST)

* **[PQ-K1]** Kernel does **not** implement KEX/signatures; it **exposes** posture and **gates readiness** when policy demands.
* **[PQ-K2]** When `require_pq=true`, classical-only peers or stacks **must** yield degraded/failing `/readyz` with explicit reason.
* **[PQ-K3]** PQ posture is **observable**: metrics labels and health snapshot fields are mandatory and stable.
* **[PQ-K4]** Kernel **never** silently downgrades or hides posture; downgrade attempts must be logged and metrically visible.
* **[PQ-K5]** Symmetric/hash assumptions appear only as **limits** in observability (e.g., BLAKE3 identifiers) — no kernel-side trust decisions.
* **[PQ-K6]** Feature-gated builds for `pq`, `pq-hybrid`, `pq-only` must compile and pass CI across the matrix.

---

## 7) Observability (Metrics, Logs, Readiness)

**Metrics (kernel-side labels; counts may be sourced from services via bus/aggregation pipes):**

* `pq_handshake_total{mode="off|hybrid|pq-only", kex="x25519|ml-kem|hybrid", role}`
* `pq_downgrade_events_total{reason}` *(observed/relayed)*
* `crypto_latency_seconds{op="kex|sign|verify", algo}` *(histogram; optional, if relayed)*
* `readiness_degraded_total{cause="pq_required_but_unavailable"}`
* Global labels: `amnesia="on|off"`, `tenant`, `plane="node|app"`.

**Logs (structured):**

```json
{ "evt":"pq_posture", "mode":"hybrid", "kex":"hybrid", "sig":"ml-dsa", "amnesia":"on", "tenant":"..." }
{ "evt":"pq_downgrade_detected", "peer":"...", "required":"hybrid", "negotiated":"classical" }
```

**Readiness:**

* `/readyz` (implemented in services) **must** consult kernel `HealthState` for `pq_required` + current posture.
* Kernel ensures posture is present in `HealthState`; failing conditions set `degraded=true` + `reason`.

---

## 8) Testing & Verification

* **Unit/prop tests (kernel):** config parsing for PQ knobs; health snapshot propagation; metrics label presence; bus topics carrying posture updates.
* **Interop suite (downstream):** executed in transport/auth/gateway crates; kernel participates by **asserting** that posture and readiness reflect ground truth.
* **Fuzzing:** negotiation state transitions in the kernel’s policy adapter (stringly config/flags → snapshot); malformed posture events must not panic.
* **Load:** ensure bus bounding under PQ posture event storms; readiness evaluation stays O(1) and non-blocking.
* **Security drills:** simulate “classical break” → flip `pq-only` → validate clean failure modes and alerting.

---

## 9) Risks & Mitigations

* **Visibility gaps:** If services fail to emit posture, operators fly blind → **Mitigation:** kernel requires posture field; absence triggers warning metrics (`pq_posture_missing_total`).
* **Policy drift:** Mixed clusters with different defaults → **Mitigation:** kernel snapshots include versioned posture fields; CI enforces schema stability.
* **Perf/footprint concerns:** PQ raises overheads → **Mitigation (kernel):** *do not* add blocking posture checks; keep readiness evaluation constant-time; keep bus bounded.
* **Library churn:** PQ libs evolve → **Mitigation:** kernel depends only on **traits/events**, not PQ libs; version pins live downstream.

---

## 10) Acceptance Checklist (DoD)

* [ ] **Exposure assessed**; HNDL risk labeled **Low** for kernel.
* [ ] **Features compile:** `--features pq,pq-hybrid,pq-only` across CI matrix.
* [ ] **Health snapshot** carries `pq_required`, `pq_mode`, `kex` fields; defaults sensible.
* [ ] **Metrics labels** present even when PQ disabled (values zero).
* [ ] **Readiness policy**: `require_pq=true` causes degraded/fail when peers are classical.
* [ ] **Downgrade visibility**: logs + `pq_downgrade_events_total` increment on mismatch.
* [ ] **Docs updated**: SECURITY.md cross-links; runbook has enable/rollback steps.
* [ ] **Perf sanity**: posture updates do not block; bus remains bounded under stress.

---

## 11) Role Presets (Kernel + Friends)

* **ron-kernel (this crate):** PQ-agnostic; expose posture; gate readiness per policy; no crypto code.
* **ron-transport / svc-arti-transport:** implement **Hybrid KEX**; export posture to kernel.
* **ron-auth / svc-passport / ron-kms:** implement **PQ signatures & custody**; export posture.
* **svc-gateway / omnigate / nodes:** enforce tenant policy; use kernel health snapshots for readiness.

---

## 12) Appendix (fill as adoption progresses)

* **Algorithms chosen (downstream):** KEX=<X25519+ML-KEM>, SIG=<ML-DSA>, rationale: NIST selections/hybrid defense-in-depth.
* **Libraries (downstream):** e.g., `rustls` (+ PQ KEM), `ml-dsa` bindings via adapter crates; pinned in workspace.
* **Interop notes:** peers supported, fallback policy (`hybrid` default at M3); explicit error taxonomy on refusal.
* **Change log:**

  * 2025-Q3: added posture fields, CI matrix for features.
  * 2025-Q4: readiness gating for `require_pq`.
  * … (continue)

```

---

