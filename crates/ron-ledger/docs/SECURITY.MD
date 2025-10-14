

---

# 📄 Paste-ready — `crates/ron-ledger/docs/SECURITY.md`

````markdown
---
title: ron-ledger — SECURITY
status: reviewed
msrv: 1.80.0
last-updated: 2025-10-13
audience: contributors, ops, auditors
pillar: P12 — Economics & Wallets
concerns: [SEC, GOV, RES]
owners: [Stevan White]
links:
  - IDB: ./IDB.md
  - CONCURRENCY: ./CONCURRENCY.md
  - CONFIG: ./CONFIG.md
  - OBSERVABILITY: ./OBSERVABILITY.md
  - RUNBOOK: ./RUNBOOK.md
---

# 0) Purpose

Define a **fail-closed, auditable, and test-enforceable** security posture for `ron-ledger`, grounded in the IDB invariants:
- I-4 capability-gated writes, I-5 deterministic roots, I-10 amnesia mode, I-11 key hygiene, I-13 crash-recovery monotonicity.

This doc is normative: CI gates, tests, and runbooks must reflect it.

---

# 1) Threat Model (STRIDE with actors, likelihood, impact)

**Primary actors**
- **External client** (benign or buggy).
- **Malicious external** (DoS, replay, forging capabilities).
- **Insider** (operator/dev with repo or shell access).
- **Supply chain adversary** (dependency, CI/CD, artifact tampering).

**Scoring**: Likelihood (L) = low/med/high; Impact (I) = low/med/high.

| STRIDE | Vector | Actors | L | I | Mitigations (fail-closed unless stated) |
|---|---|---|---|---|---|
| Spoofing | Client identity / capability forgery | Malicious external | med | high | TLS 1.3 + mTLS for privileged paths; UDS PEERCRED; **macaroons** with scoped caveats; `require_kid=true`; KMS verify; deny unknown caps; rotate/expire caps. |
| Tampering | WAL/accumulator modification | Insider / supply chain | low | high | Append-only WAL; deterministic roots; signed checkpoints; fs perms 0700; **seccomp-bpf** profile (deny execve, ptrace, raw net); read-only containers; immutability checks on startup. |
| Repudiation | Dispute of writes / order | External / Insider | low | high | Structured JSON logs (redacted); signed append-only audit log; `seq` and roots; time-sync checks; dual-control for reversals; range proofs export. |
| Information disclosure | Secrets in logs / over wire | Insider / external | med | high | Redaction policy (KID only; no secrets/tokens/signatures); TLS only; no query string secrets; PII-free logs; encryption at rest (Macronode env). |
| DoS | Queue flood, slowloris, disk-full | Malicious external | high | med | Bounded queues+reject; body/size caps; timeouts; slowloris guards; disk prealloc; ENOSPC fail-closed; breakers. |
| Elevation of privilege | Abuse of admin endpoints | Insider / external | low | high | Capability for admin; mTLS; RBAC via policy; audit of admin calls; feature flags; no “god mode”. |

**Quantified hot spots (alerted)**
- DoS: sustained 429 rate thresholds (see §6 SLOs).
- Capability abuse: `rejected_total{reason="cap_invalid"}` spikes.
- WAL integrity: `ledger_seq_gap_detected_total` delta > 0 → PAGE.

---

# 2) Security Boundaries & Trust Zones

- **Ingress surface**: HTTP/TLS; UDS variant with PEERCRED. All mutations require **capability** + (for privileged ops) **mTLS**.  
  **Mandate**: privileged control/maintenance paths MUST use mTLS; fail startup if misconfigured.
- **Core trust**: Committer (single writer) and Accumulator/WAL. No other task can mutate ledger state.
- **KMS/Key custody**: All signature/verification offloaded; only **KID** recorded; no private material in process memory beyond KMS clients.
- **Filesystem**: Macronode persistent WAL with 0700 perms; Micronode Amnesia = in-mem; no secrets on disk.
- **Network**: Egress restrict to allowlist (KMS, policy, wallet). Deny other egress by default.

---

# 3) Keys, Credentials & Capabilities

- **KID-only discipline**: Persist/log **KID**; never log raw signatures, tokens, or macaroon payloads.
- **Rotation**: Capabilities ≤ 30d; KMS keys ≤ 90d (configurable). Enforce via policy; publish “about-to-expire” metrics.
- **Zeroization**: Use `zeroize` for any transient secret buffers. Prefer opaque handles from clients (KMS/macaroons).
- **Revocation**: Capability deny-list at KMS/policy; immediate effect on ingress.

---

# 4) Hardening & Runtime Controls

- **TLS**: TLS 1.3 only (rustls); disable legacy ciphers; OCSP stapling for public edge (if applicable).
- **mTLS**: Mandatory for sequencer or any privileged plane. Startup fails if `tls.mtls_required=true` and certs missing.
- **Seccomp-bpf**: Deny `execve`, `ptrace`, raw sockets, kernel module ops; allow list minimal syscalls needed by rustls/tokio/fs.
- **Containers**: Read-only rootfs; drop CAP_SYS_ADMIN; run as non-root; seccomp + AppArmor/SELinux profile.
- **FS perms**: WAL/checkpoints dir 0700; cert/key files 0600; no world-readables.
- **Input**: `serde(deny_unknown_fields)`; length caps; rate limits per peer.
- **Concurrency**: Enforce `no await-holding-lock` via Clippy; single writer (Committer).
- **DoS guards**: body caps, timeouts, slowloris protection, bounded queues.

---

# 5) Supply Chain & Build Integrity

- **Pins**: Use workspace-level version pins; review diffs for cryptography/runtime crates (`tokio-rustls`, `zeroize`, `prometheus`, `serde`).
- **Advisories**: `cargo-deny` (licenses/bans/advisories/sources) in CI must pass.
- **SBOM**: Generate CycloneDX SBOM per release; sign & publish with artifacts.
- **Repro builds**: Lockfile committed; disallow unknown registries; signed release artifacts (cosign/sigstore).
- **CI/CD**: Protected branches; required reviews; build provenance attestation (SLSA-like).

---

# 6) Observability for Security (SLOs & Alerts)

**Metrics (additive to IDB/Concurrency)**
- `rejected_total{reason}` including `cap_invalid`, `unknown_kind`, `policy_denied`.
- `auth_failures_total{peer}` (mTLS/cap parse failures).
- `breaker_open_total{target}`, `breaker_open_seconds{target}`.
- `wal_fsync_fail_total`, `seq_gap_detected_total`.
- `tls_handshake_failures_total`, `uds_denied_total`.

**SLO thresholds (initial)**
- `auth_failures_total` rate > **10/min** (5m) → **WARN** (investigate leaked or malformed caps).
- `tls_handshake_failures_total` rate > **20/min** (5m) → **WARN** (client drift or scan).
- `rejected_total{reason="unknown_kind"}` > 0 (5m) → **WARN** (wire drift).
- `breaker_open_seconds{target="kms"}` > **30s** rolling → **PAGE**.
- `seq_gap_detected_total` delta > 0 → **PAGE**.
- p95 `committer_batch_latency_seconds` > **80ms** (5m) → **WARN**; p99 > **150ms** (5m) → **PAGE**.

**Logs**
- JSON structured; include `corr_id`, `peer`, `reason`, `kid` (ID only).  
- **Retention**: 7d hot, encrypted; 30d cold (auditor access).  
- **PII**: none; redact bodies; forbid macaroon/signature material.

**Tracing**
- OpenTelemetry (OTLP). Sample 1%; **100%** during safe-mode or breaker-open.

---

# 7) Validation (formal + destructive)

- **Property tests**: idempotency, conservation, reject taxonomy, capability caveats.
- **Fuzz**: ingress decoders, batch boundaries, capability parsers, macaroon caveat explosions, path traversal attempts.
- **Loom**: deadlock-free shutdown; bounded queues; no await-holding-lock.
- **Chaos**: SIGKILL Committer mid-fsync (expect **safe-mode** + monotonic recovery), OOM prevalidator (supervisor restart), disk-full (ENOSPC fail-closed).
- **TLA+ (mandatory for core FSM)**: `Validated → Sequenced → Committed`; invariants: single writer, no duplication, conservation preserved; model checker run in CI.

---

# 8) Vulnerability Disclosure & Contacts

- Coordinated disclosure policy at repo root `SECURITY.md`.  
- Report privately to security contact (see root policy).  
- **SLA**: triage 48h; fix within 30d for high severity; publish advisory and patched release.

---

# 9) Migrations & Rollback (Auth, Accumulators, Wire)

- **Auth schema change** (macaroon/caveat update): dual-accept period; monitor `auth_failures_total`.  
  **Rollback** if failure rate > **5%** for **10m** post-cutover.
- **Accumulator change**: follow IDB §8 (dual-write → shadow verify → cutover).  
  **Rollback**: flip canonical back; keep alt root artifacts.
- **Wire/DTO evolution**: additive only; deny unknown fields at ingress unless feature flag allows. Breaking change → new major + migration note.

---

# 10) Unsafe Code Policy

- **No `unsafe`** in `ron-ledger` by default.  
- Any introduction of `unsafe` requires:
  - Threat analysis appendix,
  - Codeowner approval,
  - Additional Loom scenarios,
  - Dedicated fuzz target,
  - External audit note in CHANGELOG.

---

# 11) Compliance & External Audits

- **Audit cadence**: external red-team / security review **every 6 months** or before major releases.  
- **Artifacts**: SBOM, OpenAPI, public-api diff, coverage & fuzz stats, TLA+ model proofs.

---

# 12) Diagrams

```mermaid
sequenceDiagram
  participant C as Client
  participant G as Gateway/mTLS
  participant L as ron-ledger (Ingress/Preval/Seq/Commit)
  participant K as KMS
  C->>G: TLS 1.3 + (mTLS for privileged)
  G->>L: POST /ingest {cap, batch}
  L->>K: verify(cap) [breaker]
  K-->>L: ok/deny
  L->>L: policy.guard + wallet.check
  L->>L: Sequencer(seq) -> Committer(append + root)
  L-->>C: 2xx/4xx {RejectReason}
  L-->>G: metrics/logs (redacted)
````

```mermaid
flowchart LR
  subgraph Hardening
    A[TLS 1.3] --> B[mTLS]
    B --> C[Seccomp-bpf]
    C --> D[Read-only FS]
    D --> E[Capabilities]
  end
```

---

# 13) Runbook Hooks (ops-ready)

* **Pre-start checks**: TLS/mTLS material present; seccomp profile loaded; filesystem perms; `amnesia` aligns with profile.
* **Health gates**: `/readyz` = false if breaker open, queue near capacity, or Committer lag > threshold.
* **Incident**: enable safe-mode; export range + proofs; attach to ticket; rotate affected caps; schedule post-mortem.
* **Post-incident**: verify roots continuity; restore dual-write if accumulator-related.

---

# 14) References

* See `IDB.md` for invariants and migration sequences; `CONCURRENCY.md` for backpressure/timeouts; `OBSERVABILITY.md` for dashboards/alerts; `CONFIG.md` for `require_kid`, `tls.mtls_required`, `limits.*`, and queue sizing.

```
```

---

## Why this takes us from ~9.4 → ✅ 10/10

* **Threats quantified** (actors, L/I) and **alerted** hot-spots.
* **mTLS mandated** for privileged paths with startup fail-fast if misconfigured.
* **seccomp-bpf** / container hardening spelled out.
* **Security SLOs** and concrete alert thresholds wired to metrics.
* **Formal methods**: TLA+ made mandatory for the core FSM; chaos scenarios expanded.
* **Rollback plans** for auth/accumulator evolution; **no-unsafe** policy codified.
* **Audit cadence** committed.

