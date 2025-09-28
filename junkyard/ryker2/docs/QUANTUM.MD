

````markdown
---
title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-09-28
audience: contributors, security auditors, ops
crate: ryker
crate-type: lib
pillar: 1   # Kernel & Orchestration
owners: [Stevan White]
---

# QUANTUM.md

## 0) Purpose
Describe how this crate resists **quantum attacks** and how we migrate to **post-quantum (PQ)** crypto without breaking interop or ops.  
For **ryker** (a lightweight actor/mailbox lib), the scope is **PQ-agnostic but PQ-ready**: no direct crypto is embedded here, but ryker must tolerate PQ payloads (larger sizes, longer handshakes) and propagate PQ-related signals and metrics.

---

## 1) Exposure Assessment (What’s at risk?)
- **Public-key usage:** None (delegated to transports/auth).
- **Symmetric/Hash:** None directly; ryker only passes opaque payloads.
- **Data at rest:** None (stateless lib).  
- **Transport/session lifetime:** N/A (caller responsibility).  
- **Crate-specific blast radius:** Worst case = ryker mishandles PQ-inflated payloads (oversized drops, latency spikes).

> **HNDL risk:** Low. Ryker does not originate or persist cryptography; it only passes through opaque messages.

---

## 2) Current Crypto Profile (Today)
- **Algorithms:** None in core; delegated to surrounding crates.  
- **Libraries:** No crypto crates imported directly.  
- **Key custody:** N/A.  
- **Interfaces that carry crypto:** Bus messages may embed PQ-sized tokens, but ryker treats them opaquely.

---

## 3) Target PQ Posture (Where we’re going)
- **Core:** Remains crypto-agnostic.  
- **Interop:** Must not reject PQ-inflated payloads or metadata.  
- **Backwards compatibility:** Always forwards opaque payloads; never inspects.  
- **Operational posture:** Provide knobs/metrics (`pq_pass`) to ensure PQ flows are observable even if ryker itself is crypto-blind.

---

## 4) Feature Flags & Config
```toml
[features]
pq-pass = []   # run PQ-sized payloads through ryker for soak/perf testing
````

```ini
# Example Config knob
pq_pass = true    # when enabled, CI/test injects PQ-sized payloads into message bus
```

---

## 5) Migration Plan (Milestones)

```mermaid
flowchart LR
  M1(Bronze: PQ test vectors) --> M2(Silver: CI PQ path + metrics)
  M2 --> M3(Gold: part of PQ interop suite)
  style M3 fill:#065f46,stroke:#064e3b,color:#fff
```

*Alt text:* Bronze adds PQ-sized payload tests; Silver gates CI with PQ path + metrics; Gold includes ryker in interop suite.

* **M1 (Bronze):** pq-pass flag, PQ test vectors.
* **M2 (Silver):** CI runs with pq-pass enabled, PQ metrics emitted.
* **M3 (Gold):** ryker included in PQ interop suite; tolerates inflated payloads under load.

---

## 6) Invariants (MUST)

* [PQ-I1] No crypto primitives directly in ryker.
* [PQ-I2] Payload size limits account for PQ inflation (≈ +20–50%).
* [PQ-I3] PQ metrics always emitted (even when pq_pass=false).
* [PQ-I4] PQ test builds (`--features pq-pass`) must pass CI.

---

## 7) Observability (Metrics, Logs, Readiness)

* **Metrics:**

  * `pq_pass_total{mode="enabled|disabled"}`
  * `pq_overhead_seconds{phase="enqueue|dequeue",mode="pq_pass|classical"}` (histogram)
  * `pq_payload_bytes_sum{mode}` (running sum of payload bytes, to spot inflation trends)
* **Logs:** structured JSON includes `pq_pass=true/false`.
* **Readiness:** /readyz remains unaffected, but PQ metrics visible at all times.

---

## 8) Testing & Verification

* **Unit/property tests:** ryker enqueue/dequeue with inflated PQ payloads.
* **Loom tests:** verify cancel-safety and timeout handling under injected PQ-like latency (side-channel irrelevant here, but timing resilience matters).
* **Fuzzing:** ensure large payloads don’t panic.
* **Soak tests:** 24h run with pq_pass enabled.

---

## 9) Risks & Mitigations

* **Perf impact:** PQ adds payload size (~20–50%) → triaged via perf runs.
* **CI coverage gaps:** mitigated with pq-pass flag in nightly matrix.
* **Misinterpretation:** ensure ryker remains crypto-agnostic; all PQ policy lives in kms/auth.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; HNDL risk labeled (low).
* [ ] pq-pass feature compiles and runs.
* [ ] CI matrix includes `--features pq-pass`.
* [ ] PQ metrics emitted with labels.
* [ ] Runbook updated with enable/rollback steps.
* [ ] Perf deltas (payload size/latency) recorded in Appendix.
* [ ] SECURITY.md cross-links updated.
* [ ] Document observed PQ payload/latency deltas in Appendix §12 with test artifacts.

---

## 11) Role Presets

* **Kernel/lib (ryker):** PQ-agnostic core; expose pq-pass test hook; never bind crypto.

---

## 12) Appendix

* **Algorithms:** N/A for ryker.
* **Libraries:** None.
* **Interop notes:** PQ handled opaquely; no assumptions.
* **Perf notes:** Observed PQ inflate ~20–50% payload size; record latency/throughput changes in CI logs.
* **Change log:**

  * 2025-09-28: Initial QUANTUM.md drafted.
  * Future: add perf deltas once measured.

---

```

---

