

````markdown
---
title: Security Notes — ron-audit
crate: ron-audit
owner: Stevan White
last-reviewed: 2025-10-07
status: draft
---

# Security Documentation — ron-audit

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `ron-audit`.  
It complements the repo-wide Hardening Blueprint and Interop docs and is **normative** for reviews.

`ron-audit` is a **library** that standardizes tamper-evident audit evidence (BLAKE3-addressed, hash-chained records) and provides sink traits (RAM/WAL/export). It **does not** host endpoints or hold private keys; hosts (e.g., `svc-gateway`, `svc-storage`) wire it into their runtime.

---

## 1) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant? | Primary Mitigations (this crate or host contract) |
|---|---|:--:|---|
| **S**poofing | Forged actor/subject identities; fake writer | **Y** | Canonical DTO with `deny_unknown_fields`; explicit `writer_id` + per-stream monotone `seq`; host-level capability/macaroons validation before emit |
| **T**ampering | On-disk WAL edits, record mutation, chain truncation | **Y** | BLAKE3 `self_hash`; `(prev → self_hash)` hash-chain; Merkle **checkpoints** + signed manifests (host signs); verification on read |
| **R**epudiation | Missing logs; inability to prove authorship/order | **Y** | Append-only sinks; idempotent dedupe key; per-(writer,stream) ordering; export manifests; auditability via re-derivable roots |
| **I**nformation Disclosure | PII leakage in `attrs`; IP exposure | **Y** | **Amnesia** defaults; bounded `attrs` (≤1 KiB); privacy modes (`drop` / prefix / salted hash); forbid secrets/tokens; JSON structured logs without secret material |
| **D**enial of Service | Queue floods; disk-stall; compression/size bombs | **Y** | **Bounded** mpsc; shed (`Busy`) not block; record size cap (≤4 KiB canonical); timeouts on WAL/export; breaker + `spawn_blocking` for slow I/O |
| **E**levation of Privilege | Unauthorized emits/exports | **Y** | Host must authenticate/authorize before calling `emit`; export signer key is **host-provided** (no signing here); config validation fail-closed |

**Notes:** TLS, rate-limits, and macaroon verification live in **hosts**. `ron-audit` assumes authenticated/authorized callers.

---

## 2) Security Boundaries

- **Inbound (library API):**
  - `emit(sink, AuditRecord)` — host-only API; no network surface.
  - Sink construction (`RamSink`, `WalSink`, `ExportSink`) — host creates with validated `AuditConfig`.
- **Outbound:**
  - Optional write to **WAL** directory (Macronode profile).
  - Optional write of **export manifests/checkpoints** (host-signed).
- **Trust Zone:**
  - Runs **in-process** with the host service; inherits host’s trust boundary.
  - Micronode defaults to **amnesia (RAM-only)**; Macronode may use durable media (strict perms).
- **Assumptions:**
  - Host enforces capability checks and quotas **before** calling `emit`.
  - Host exposes observability (`/metrics`) and implements shutdown/drain.
  - Filesystem paths are **service-local** and isolated; no shared world-writable mounts.

---

## 3) Key & Credential Handling

`ron-audit` does **not** hold private keys. Checkpoint signatures are **host-provided**.

- **Key types (host):** export checkpoint signer key (e.g., Ed25519 or future ML-DSA), macaroon root keys (for host auth), TLS keys (host).
- **In this crate:** only **tags/IDs** (`signer_key_id`, `alg`) ever appear; no secret material.
- **Storage & rotation (host policy):**
  - Signer keys in HSM or sealed store; rotate ≤ 90 days; publish key IDs.
  - If `privacy.ip_mode=hash`, the **hash salt** file must be `0600`, rotated per policy (e.g., 30–90 days) with clear auditor guidance on reversibility.
- **Zeroization:** If helper structs ever wrap sensitive bytes (e.g., salts read by host and passed down), use `zeroize::Zeroizing` and drop eagerly. This crate **discourages** passing raw secrets through its API.

---

## 4) Hardening Checklist (crate + host)

- [ ] **Amnesia posture:** `amnesia=true` ⇒ disallow WAL; only RAM/export (in-memory staging).  
- [ ] **Queue bound:** `audit_work` mpsc is bounded (cfg `queue_depth`), uses `try_send` + `Busy` rejection.  
- [ ] **Record bounds:** `max_attrs_bytes ≤ 1 KiB`, `max_record_bytes ≤ 4 KiB` (canonical, excl. `self_hash`).  
- [ ] **No secrets/PII:** forbid tokens; IP handling per policy (`drop` default).  
- [ ] **Timeouts:** enqueue ≤ 25 ms (host), WAL append ≤ 50 ms soft deadline; exports run with budgets.  
- [ ] **I/O isolation:** WAL/Export dirs owned by service user, `0700` dir, files `0600`.  
- [ ] **No blocking on runtime:** use `spawn_blocking` for fsync/large IO; keep batches small.  
- [ ] **Shedding over blocking:** never block hot paths; increment `audit_drop_total{reason}`.  
- [ ] **Verification on read:** re-hash every record before serving auditors; fail-closed.  
- [ ] **Checkpoints:** periodic Merkle roots + **host-signed** checkpoints; export manifests include ranges/roots/digests.  
- [ ] **Config validate fail-closed:** reject `wal.fsync=never` in prod; require dirs to exist; enforce amnesia/WAL incompatibility.

---

## 5) Observability for Security

- **Metrics (host emits, contract defined by this crate):**
  - `audit_appended_total{kind,service}`
  - `audit_drop_total{reason,service}` (e.g., `audit_backpressure`, `io_backpressure`, `bounds_exceeded`)
  - `audit_queue_depth{service,stream}`
  - `io_timeouts_total{op="wal|export"}`
  - `audit_export_checkpoints_total{service}`
  - `audit_pii_redacted_total{service}`, `audit_pii_blocked_total{service}`
- **Logs:** JSON; include `service`, `stream`, `writer_id`, `seq`, `prev`, `self_hash`, `reason`, `corr_id`. **Never** log tokens or full IPs.
- **Health gates:** host `/readyz` should fail-closed on sustained WAL failures, manifest write failures, or queue saturation (SLO breach).

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates:**
  - `blake3` — record hashing (tamper evidence).
  - `serde`, `serde_json` — strict DTOs with `deny_unknown_fields`.
  - `parking_lot` or `std::sync` — short critical sections (no `.await` under lock).
  - `zeroize` — for any transient sensitive buffers (if host passes salts).
- **Not in this crate:** TLS (handled by hosts with `tokio-rustls`), macaroons, HSM/KMS.
- **Controls:**
  - Workspace-root `cargo-deny` (licenses/advisories/bans) **required** in CI.
  - Repro builds in CI; SBOM exported at release.

---

## 7) Formal & Destructive Validation

- **Property tests (crate):**
  - Chain integrity: `(prev → self_hash)` continuity under any interleaving consistent with `(writer_id,stream,seq)`.
  - Idempotency: same canonicalization → same dedupe key → no duplicate append.
  - Bounds: reject oversize `attrs`/record; never panic.
- **Fuzzing (crate):**
  - Canonicalization inputs: NFC/NFD, large unicode, edge integers; JSON key reordering; ensure stable `self_hash` or clean reject.
- **Loom (crate):**
  - Model producer → bounded queue → single consumer with shutdown; assert no deadlock/missed shutdown (dev-only feature).
- **Chaos (host):**
  - Disk-full, FD-exhaustion, slow-IO injection; breaker opens; queues shed; chain remains consistent across restart.
- **Forensic E2E (host tooling):**
  - Recompute Merkle roots from WAL + manifests; detect a single tampered record via root mismatch; verify checkpoint signatures (host).

---

## 8) Data Handling & Privacy

- **Do not store secrets** or bearer tokens in audit records or logs.  
- **IP handling:** default **drop**; if required, store `/24` (IPv4) or `/64` (IPv6) prefix, or salted hash (salt file `0600`, rotated).  
- **PII budget:** prefer IDs/fingerprints; record redaction events via metrics.  
- **Amnesia default:** Micronode runs RAM-only; operators must **opt-in** to durability.

---

## 9) Migration & Upgrades

- **Canonicalization changes** or record structure changes require a **major** schema version and updated test vectors.  
- **WAL/Manifest format** changes: version fields + migration tool (host repo).  
- **Deprecations**: provide compat period (≥1 minor); warn on old env keys.

---

## 10) Security Contacts

- **Maintainer:** Stevan White  
- **Security disclosure:** See repo root `SECURITY.md` (coordinated disclosure; private contact email).

---

## 11) Mermaid — Security Flow Diagram (Library in Host Context)

```mermaid
flowchart LR
  A[Host Service] -- validated, authorized call --> B{{ron-audit: emit()}}
  B --> C[Sink Worker(s)]
  C -->|append| D[(WAL/Export Manifests)]
  C -->|metrics| E[(/metrics via host)]
  B -. bounds/PII checks .-> F[Reject & Shed (Busy)]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

**Text:** The host authenticates/authorizes. `emit()` enforces bounds/canonicalization. Workers append to WAL/exports or shed under backpressure; host emits security-relevant metrics.

---

```
```
