

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-09-27
audience: contributors, security auditors, ops
crate: ron-bus
crate-type: lib
pillar: 1              # Kernel & Orchestration
owners: [Stevan White]
---

# QUANTUM.md — ron-bus

## 0) Purpose
`ron-bus` is a **pure in-process broadcast bus**. It does **not** perform key exchange, signatures, TLS, or persistence.  
This document explains how `ron-bus` remains **PQ-neutral** while enabling **system-wide PQ migration** (hybrid KEX, PQ signatures) in the crates that *do* own crypto or transport.

---

## 1) Exposure Assessment (What’s at risk?)
- **Public-key usage (breakable by Shor):**  
  - **None inside ron-bus.** No TLS/KEX/signature operations occur here.
- **Symmetric/Hash (Grover-affected only):**  
  - **None inside ron-bus.**
- **Data at rest / long-lived artifacts:**  
  - **None.** ron-bus holds ephemeral in-memory events; no on-disk artifacts.
- **Transport/Session lifetime:**  
  - N/A. ron-bus does not create network sessions. Event lifetime is memory-bound and short-lived.
- **Crate-specific blast radius:**  
  - If classical PKI were broken, **ron-bus itself is unaffected**. Impact arises only when **hosts mirror events** over non-PQ transports or sign them with classical keys.
- **HNDL risk:**  
  - **Low for ron-bus itself.**  
  - **Inherited** by hosts that externalize events. HNDL risk then depends on external transport/auth/storage stacks (see their QUANTUM.md).

> **Policy:** ron-bus payloads must **not** contain secrets/PII or long-term confidential data. This preserves HNDL low even if hosts mirror events externally.

---

## 2) Current Crypto Profile (Today)
- **Algorithms in use:** *None (no crypto in this crate).*  
- **Libraries:** *None crypto-related.*  
- **Key custody:** *N/A.*  
- **Interfaces that carry crypto:** *N/A.* (Any crypto-bearing interfaces live in transport/auth/KMS crates.)

---

## 3) Target PQ Posture (Where we’re going)
- **ron-bus posture:** remain **PQ-agnostic** and **crypto-free**; expose **observability hooks** so hosts can report PQ posture when publishing events.
- **System posture (owned by other crates):**  
  - **KEX/Transport:** Hybrid (X25519 + ML-KEM) in `ron-transport` / gateways.  
  - **Signatures/Tokens:** ML-DSA or SLH-DSA options in `ron-auth` / `svc-passport`.  
  - **Back-compat:** Classical supported during M1–M2; default to hybrid in M3.
- **ron-bus compatibility:** Zero API changes required for host PQ migration.

---

## 4) Feature Flags & Config (How to turn it on)
`ron-bus` does not implement crypto toggles. It provides **optional labels** that hosts can pass through when emitting metrics/logs.

```toml
# Cargo features for ron-bus (intentionally minimal)
[features]
pq-labels = []   # Enable optional metrics/log fields to carry host PQ posture
````

```ini
# Example host Config (not enforced by ron-bus; shown for context)
pq_hybrid = true            # host transport negotiates hybrid KEX
pq_sign_algo = "ml-dsa"     # host token signatures (if applicable)
pq_only = false             # host refuses classical peers when true
```

* **Interoperability:** ron-bus never negotiates crypto. Hosts decide negotiate/refuse behavior.
* **Metrics toggle:** when `pq-labels` is enabled, hosts can attach fields like `pq={off|hybrid|pq-only}`, `kex=...`, `sig=...` to event-related metrics/logs.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze) — Hooks**

  * Confirm ron-bus is crypto-free; add `pq-labels` feature.
  * Document HNDL hygiene (no secrets in payloads).
  * CI: build/test with workspace PQ features on (ensures no coupling).

* **M2 (Silver) — Hybrid Enablement (System)**

  * Transports/auth enable **Hybrid KEX** / PQ signatures.
  * ron-bus unchanged; hosts attach PQ labels to logs/metrics for correlation.

* **M3 (Gold) — Default Hybrid**

  * System defaults: `pq_hybrid=true` at edges; selective `pq_only=true` per policy.
  * ron-bus unchanged; runbooks/observability show PQ posture alongside bus metrics.

* **Post-M3 — De-risking**

  * Sunset pure-classical on external edges as partners support PQ.
  * ron-bus remains neutral; no crypto dependencies added.

---

## 6) Invariants (MUST)

* **[PQ-I1]** ron-bus introduces **no crypto dependencies** and **no classical-only assumptions**.
* **[PQ-I2]** Bus payloads **must not carry secrets** or long-lived confidential data (HNDL hygiene).
* **[PQ-I3]** Enabling workspace PQ features must **not change** ron-bus behavior or API.
* **[PQ-I4]** If `pq-labels` is enabled, ron-bus **must not** enforce policy—labels are **observability-only**.
* **[PQ-I5]** ron-bus **must not** block system PQ rollout: no coupling to transport/auth crypto choices.

---

## 7) Observability (Metrics, Logs, Readiness)

* **Metrics (host-exposed, bus-updated):**

  * `bus_overflow_dropped_total{pq}` — include `pq={off|hybrid|pq-only}` label **if** `pq-labels` is enabled and host provides it.
  * `bus_queue_depth{pq}` — same labeling rules.
* **Logs (host):** Structured JSON may include PQ posture fields when events are published/consumed.
* **Readiness (host):** `/readyz` may fail if **policy requires PQ** and peer stacks cannot negotiate; **ron-bus does not gate readiness**.

---

## 8) Testing & Verification

* **Unit/integration:** Verify that enabling `--features pq-labels` compiles and **does not** alter bus semantics.
* **Workspace interop matrix (in other crates):** classical↔classical, hybrid↔hybrid, hybrid↔classical.
* **Security drill:** Force `pq_only=true` in hosts; confirm ron-bus continues normal operation and events/logs carry posture labels.
* **Perf:** Confirm no measurable overhead from labels (should be zero/near-zero).

---

## 9) Risks & Mitigations

* **Risk:** Accidental introduction of crypto coupling into ron-bus.

  * **Mitigation:** CI denylist for PQ libs in ron-bus; code review checklist.
* **Risk:** HNDL via sensitive payloads mirrored externally.

  * **Mitigation:** Policy: **no secrets/PII in events**; reviewers reject violations.
* **Risk:** PQ label misuse as policy control.

  * **Mitigation:** Labels are observability-only; enforce via lint/docs/tests.

---

## 10) Acceptance Checklist (DoD)

* [ ] HNDL risk documented as **Low** for ron-bus; hygiene policy in place.
* [ ] `pq-labels` feature compiles; zero behavior/API changes.
* [ ] CI builds ron-bus with workspace PQ features enabled (no coupling).
* [ ] Observability fields documented for hosts (metrics/logs).
* [ ] SECURITY.md references this PQ hygiene (no secrets in payloads).
* [ ] RUNBOOK/OBSERVABILITY cross-links updated.

---

## 11) Role Presets (ron-bus context)

* **Kernel/lib preset:** PQ-agnostic core; no crypto; optional labels only.
* **Transport/auth/kms (elsewhere):** implement hybrid KEX / PQ signatures and custody; expose posture to metrics/logs that ron-bus-adjacent dashboards can consume.

---

## 12) Appendix

* **Algorithms chosen (system-wide, not in ron-bus):**

  * KEX: Hybrid **X25519 + ML-KEM (Kyber)** (targets M3 default).
  * Signatures: **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** for tokens/receipts.
* **Libraries:** Tracked in transport/auth/KMS crates; pinned at workspace root.
* **Interop notes:** ron-bus API unaffected by PQ negotiation; hosts attach posture to observability only.
* **Change log:**

  * 2025-09-27 — Initial PQ-neutral spec, added `pq-labels` hook.

```

---
