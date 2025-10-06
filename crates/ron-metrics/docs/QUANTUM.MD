

---

title: Post-Quantum (PQ) Readiness & Quantum Proofing
status: draft
msrv: 1.80.0
last-updated: 2025-10-05
audience: contributors, security auditors, ops
crate: ron-metrics
crate-type: lib
pillar: 5                  # Observability
owners: [Stevan White]
----------------------

# QUANTUM.md

## 0) Purpose

`ron-metrics` provides metrics, health, and readiness endpoints for host services and (optionally) an OTLP exporter. This document describes how `ron-metrics`:

* avoids increasing quantum exposure (by default **loopback/UDS**, GET-only);
* **observes** PQ adoption (families/labels) across the platform;
* degrades **readiness** if a deployment policy mandates PQ and the endpoint/peer cannot negotiate it;
* integrates with the workspace PQ migration (hybrid first, then selective PQ-only at edges).

Scope includes: exposure assessment, current crypto profile (via optional TLS/OTLP), PQ telemetry families, feature/config posture, migration, invariants, testing, risks, and DoD.

---

## 1) Exposure Assessment (What’s at risk?)

* **Public-key usage (breakable by Shor)**

  * **Metrics exposer (optional)**: If bound to TCP+TLS, the server handshake is currently **classical TLS 1.3** (typical KEX: X25519; signatures: ECDSA/Ed25519 in cert chain depending on platform). Default guidance is **loopback or UDS** → no PK on the wire.
  * **OTLP egress (optional)**: If enabled and using TLS to a collector, that TLS is also classical today.

* **Symmetric/Hash (Grover-affected only)**

  * Platform common: **AES-256** and/or **ChaCha20-Poly1305**; hashes often **BLAKE3-256**. These remain safe with doubled security margins; not directly used by `ron-metrics` beyond dependencies.

* **Data at rest / long-lived artifacts**

  * `ron-metrics` is **stateless**; it does not persist long-lived ciphertexts or credentials. Exposed data is ephemeral telemetry and readiness JSON.
  * **HNDL risk:** **Low**. Scrape payloads are not sensitive (no secrets/PII allowed). If TLS is used for remote scrape, recorded traffic could be decrypted in a PQ future; mitigate by **loopback/UDS** or gateway-terminated TLS with **hybrid** when available.

* **Transport/session lifetime**

  * Scrape sessions are short/keep-alive. **Short sessions → lower HNDL risk**.

* **Crate-specific blast radius**

  * Worst case (classical PKI broken): remote scrapes to a TLS-exposed `/metrics` could be read by an adversary; **no integrity impact** on business logic, but **privacy/signaling risk** if operators add custom labels with sensitive values (forbidden by policy).

> **HNDL = Harvest-Now, Decrypt-Later**. For `ron-metrics`, keep remote exposure minimal and content non-sensitive.

---

## 2) Current Crypto Profile (Today)

* **Algorithms (indirect, via platform libs)**

  * **KEX**: X25519 (typical TLS 1.3)
  * **Signatures**: ECDSA or Ed25519 (cert chain)
  * **Symmetric/AEAD**: AES-256-GCM or ChaCha20-Poly1305
  * **Hash**: BLAKE3-256 is common for app logic; TLS stacks use SHA-2 families.

* **Libraries**

  * TLS: `tokio_rustls/rustls` (server side if TLS is enabled for the exposer)
  * OTLP: depends on host exporter; typically `opentelemetry-otlp` → platform TLS

* **Key custody**

  * **TLS keys**: stored in KMS/HSM or platform secret store; **no raw keys in env/files**. Rotation ≤ 90 days.
  * `ron-metrics` does **not** manage keys; it consumes a rustls `ServerConfig` provided by the host.

* **Interfaces that carry crypto**

  * **Ingress**: HTTP GET to `/metrics|/healthz|/readyz` (usually loopback/UDS; TCP+TLS optional).
  * **Egress**: OTLP to collector (optional).

---

## 3) Target PQ Posture (Where we’re going)

* **Key exchange / encryption (PQ or Hybrid):** Adopt **Hybrid KEX (X25519 + ML-KEM)** at the **gateway/transport layers** first. `ron-metrics` benefits indirectly when the gateway fronts the exposer over TLS.
* **Signatures:** For platform certs/tokens, migrate to **ML-DSA (Dilithium)** or **SLH-DSA (SPHINCS+)** as infra supports them. `ron-metrics` does not issue signatures.
* **Transport (metrics/OTLP)**:

  * Default: **loopback/UDS** (no TLS).
  * If TCP+TLS is required and platform supports it, enable **hybrid KEX** (`pq_hybrid = true`).
* **Backwards compatibility:** Classical remains supported until **M3** (Gold) when hybrid becomes the default on public edges (gateway/collectors), not necessarily for `ron-metrics` itself (which should remain local whenever possible).

---

## 4) Feature Flags & Config (How to turn it on)

> `ron-metrics` does **not** implement PQ primitives; it **observes** PQ posture and can **enforce policy via readiness** when configured.

```toml
# Cargo features for observability only (no crypto primitives here)
[features]
otel = []          # optional OTLP egress (still classical/hybrid handled by upstream)
pq-observe = []    # emit PQ-related metrics labels/counters (on by default in workspace)
```

```ini
# Host config knobs (consumed by the host service; ron-metrics reads policy and reports)
pq_required = false          # if true AND endpoint is TCP+TLS, /readyz must fail when hybrid PQ is not negotiated
pq_mode = "off"              # "off" | "hybrid" | "pq-only" (observed; used for logs/labels)
pq_only = false              # if true at gateway/collector, classical peers are refused (observed here)
```

* **Interoperability switch:** If `pq_required=true` and the bound transport is **TCP+TLS without hybrid**, `/readyz` should reflect **degraded** until policy is satisfied.
* **Metrics toggle:** PQ families/labels are **always present** (zero values when disabled) to keep dashboards stable.

---

## 5) Migration Plan (Milestones)

* **M1 (Bronze) — Planning & Hooks**

  * Ensure PQ observability families exist:
    `pq_kex_failures_total{algo,role}`, `pq_verify_failures_total{algo,role}`,
    `zk_verify_failures_total{scheme}`, `zk_proof_latency_seconds{scheme}` (zero when unused).
  * Add logs exposing `pq={off|hybrid|pq-only}` for the exposer context (observed).
  * Baseline perf for classical `/metrics` exposures.

* **M2 (Silver) — Hybrid Enablement**

  * Gateway/collector layers enable **Hybrid KEX**; `ron-metrics` confirms posture through labels/logs.
  * Interop tests: classical↔classical, hybrid↔hybrid, hybrid↔classical (negative/positive as per policy).
  * If policy dictates, make `/readyz` **degraded** when bound with TCP+TLS but not hybrid.

* **M3 (Gold) — Default & Operationalization**

  * **Hybrid default** at public edges; `pq_required=true` for any remotely scraped `/metrics`.
  * Runbooks include enable/rollback steps.
  * Dashboards: add PQ posture panels; alerts on downgraded sessions or verification failures.

* **Post-M3 — De-risk / De-precate**

  * Gradually enable `pq_only` for edges that no longer require classical interop.
  * Periodic PQ re-evaluation and library upgrades.

---

## 6) Invariants (MUST)

* **[PQ-I1]** `ron-metrics` must **not** introduce pure ECC/RSA dependencies; PQ is handled in transports/gateways.
* **[PQ-I2]** Symmetric/Hash strength **≥ 256-bit** effective (workspace standard).
* **[PQ-I3]** PQ observability families exist **always**; zero values are acceptable.
* **[PQ-I4]** If `pq_required=true` and endpoint uses TCP+TLS without hybrid, `/readyz` is **503** with explanation and `Retry-After`.
* **[PQ-I5]** No secrets/PII in metrics; PQ labels are **low-cardinality** (`algo`, `role`, `scheme`).
* **[PQ-I6]** CI runs with pq-observe enabled; metrics contract stability maintained (SemVer rules).

---

## 7) Observability (Metrics, Logs, Readiness)

**Metric families (first-class or observed):**

* `pq_kex_failures_total{algo,role}` — count of failed handshakes by algorithm and role (client/server).
* `pq_verify_failures_total{algo,role}` — signature verification failures by algorithm/role.
* `zk_verify_failures_total{scheme}` — ZK verification failures by scheme.
* `zk_proof_latency_seconds{scheme}` — proof verification latency histogram.
* `exposition_latency_seconds{endpoint="/metrics"}` — existing histogram, helps correlate PQ changes to tail.

**Labels:** keep **`service, instance, build_version, amnesia`** plus **`algo|role|scheme`** where relevant.

**Readiness:**

* If policy mandates PQ on remotely accessible endpoints and current binding does not meet it, `/readyz` returns **503** with JSON:

  ```json
  {"degraded":true,"missing":["pq_hybrid_required"],"retry_after":5}
  ```

**Logs:**

* Structured fields: `pq_mode={off|hybrid|pq-only}`, `tls_bound={uds|loopback|tcp}`, `peer_mode=...` (if known), and counters snapshot on startup.

---

## 8) Testing & Verification

* **Unit/property**: ensure PQ families exist and are exported with zero values when disabled; label validation (low-cardinality).
* **Interop (observational)**: simulate configurations:

  * loopback/UDS (no TLS) → `/readyz` OK regardless of PQ policy (unless policy explicitly requires hybrid for remote scrape only).
  * TCP+TLS classical only + `pq_required=true` → `/readyz` 503 with explanatory `missing`.
  * TCP+TLS hybrid + `pq_required=true` → `/readyz` 200.
* **Fuzz**: readiness JSON fields including `missing=["pq_hybrid_required"]`.
* **Load**: confirm negligible impact on `/metrics` latency with PQ labels enabled.
* **Security drills**: flip `pq_required=true` during chaos; ensure correct degrade/restore behavior.

---

## 9) Risks & Mitigations

* **Perf overhead from PQ adoption (system-wide)**: Larger handshakes might affect scrape endpoints if TLS is used → **prefer loopback/UDS**, enable keep-alive and session resumption; monitor `exposition_latency_seconds`.
* **Library churn**: PQ TLS support varies → keep PQ at the **gateway/collector** boundary first; `ron-metrics` remains PQ-agnostic.
* **Downgrade attacks**: If policy is `pq_required=true`, ensure readiness fails rather than silently allowing classical; log downgrade reasons.
* **Operator-added labels**: Avoid high cardinality or sensitive values; CI checks enforce.

---

## 10) Acceptance Checklist (DoD)

* [ ] Exposure assessed; **HNDL = Low** documented.
* [ ] PQ observability families present and documented (zero allowed).
* [ ] Readiness behavior implemented for `pq_required` policy on TCP+TLS.
* [ ] Dashboards include PQ posture and failure panels.
* [ ] Tests cover policy permutations (off/hybrid/required).
* [ ] Runbook updated (enable/rollback, triage).
* [ ] Perf impact measured (no significant change in `/metrics` p95).
* [ ] SECURITY.md cross-links updated.

---

## 11) Role Presets (applied to this crate)

### kernel/lib (`ron-kernel`, `ron-bus`, `ron-metrics`, `ron-proto`)

* **Primary Targets:** **PQ-agnostic** core; expose **observability** (families/labels), enforce **policy via readiness** only when explicitly configured.
* **Defaults:** `pq_required=false`; **loopback/UDS** binding by default; PQ labels exported (zeroed) to stabilize dashboards.

---

## 12) Appendix

* **Algorithms chosen (platform-wide, not enforced here):**

  * KEX: **Hybrid(X25519 + ML-KEM)** when available at edges.
  * SIG: **ML-DSA (Dilithium)** preferred; **SLH-DSA (SPHINCS+)** acceptable where signatures are rare/archival.

* **Libraries:**

  * TLS: `rustls` (classical today; hybrid pending upstream availability).
  * OTLP: `opentelemetry-otlp` (classical TLS today; hybrid when collector/tooling supports).

* **Interop notes:**

  * Remote scrapes should traverse a **gateway** capable of hybrid TLS before reaching `ron-metrics`; local scrape remains loopback/UDS.
  * If the collector requires TLS, prefer **hybrid** when supported; otherwise accept classical until M3, with `pq_required` guiding readiness.

* **Change log stubs:**

  * 2025-Q4: Added PQ observability families to `ron-metrics`; readiness honors `pq_required`.
  * 2026-Q1: Default dashboards include PQ posture panels.

---

**Bottom line:** `ron-metrics` does not perform PQ crypto—but it **makes PQ adoption visible**, **keeps the metrics contract stable**, and **enforces policy via readiness** when configured. This ensures our observability plane is **quantum-aware without becoming a cryptographic dependency hub**.
