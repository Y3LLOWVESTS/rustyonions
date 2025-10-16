
````markdown
---
title: Security Notes — ron-accounting
crate: ron-accounting
owner: Stevan White
last-reviewed: 2025-10-14
status: draft
---

# Security Documentation — ron-accounting

This document defines the **threat model**, **security boundaries**, **security invariants**, and **hardening requirements** for `ron-accounting`.  
It complements the workspace **Hardening Blueprint**, **Interop Blueprint**, **IDB**, **CONFIG**, and **CONCURRENCY** docs.

> **North Star:** `ron-accounting` is a **transient metering library**. It records counters in RAM, seals **time-sliced** usage, and exports slices to `ron-ledger` **ordered + idempotently**.  
> **Non-goals:** ledger/truth, balances/settlement, key custody, PII handling.

---

## 0) Security Invariants (MUST)

1. **No PII** in any DTO, metric label, or log line. Tenant/dimension keys are opaque (UUID/u128).  
2. **Ordered + Idempotent Exports:** no `N+1` before `N`, and duplicate `N` is a no-op.  
3. **Bounded Everything:** no unbounded queues; prefer **shed/reject** to buffering.  
4. **Fail-Closed:** invalid config, WAL unusable, or auth failure → refuse to start or reject request.  
5. **No await-held locks**; no blocking syscalls on the async reactor.  
6. **Amnesia/WAL Posture is Explicit:** Micronode → amnesia=true; Macronode → WAL bounded with fsync rules.  
7. **DTO Strictness:** `deny_unknown_fields`; max frame ≤ **1 MiB**; decompression ratio ≤ **10×** (adapter).  
8. **Secrets Never Logged:** keys/tokens/DTOS not printed; only redacted IDs/digests.  
9. **TLS 1.3 (adapter)** or UDS with `SO_PEERCRED` allowlist.  
10. **Graceful Drain or Abort:** on shutdown, stop intake, drain with deadline, then abort stragglers.

---

## 1) Data Classification & Assets

| Asset                         | Class | Notes |
|------------------------------|:-----:|------|
| Sealed usage slices (RAM/WAL)| **S2** | Business-sensitive metering, **not** personal data. Hash-linked + BLAKE3. |
| WAL files (bounded)          | **S2** | Checksummed; directory 0700; fsync on seal/dir-create. |
| Macaroon tokens              | **S3** | Secrets; memory only in amnesia; disk only via secret store; zeroized. |
| TLS private key (adapter)    | **S3** | 0600; process umask 0077; never logged. |
| Metrics                      | **S1** | Operational; no PII; label cardinality sampled. |
| Logs                         | **S1** | Structured JSON; redacted; includes digests/IDs only. |

S1=operational; S2=sensitive business; S3=secrets.

---

## 2) Threat Model (STRIDE)

| Category | Threats (examples) | Relevant? | Mitigations |
|---|---|:---:|---|
| **S**poofing | Impersonated client/export peer; fake tenant | **Y** | TLS 1.3 (adapter), mTLS or macaroon capabilities; UDS + `SO_PEERCRED` allowlist; opaque tenant IDs. |
| **T**ampering | Mutated slices in transit; WAL corruption; schema drift | **Y** | BLAKE3 digests; `prev_b3` chain; DTO strictness; WAL checksum/length; fsync; reject on mismatch. |
| **R**epudiation | “We didn’t send/receive N” | **Y** | `corr_id` propagation; per-slice `seq`, `b3`, `prev_b3`; structured audit logs + export status metrics. |
| **I**nfo Disclosure | PII leakage; token/keys in logs; debug endpoints | **Y** | No PII by contract; redaction; JSON only; metrics sampling; `/metrics` local-only; least-privilege FS perms. |
| **D**oS | Flooding, slow-loris, zip bombs, backlog growth | **Y** | Bounded queues; try_send reject; timeouts; decompression caps; fairness (RR/WFQ); op deadline; readiness flips false. |
| **E**oP | Using library features outside scope; bypassing auth | **Y** | Feature flags; adapter disabled unless built; macaroon caveats; deny unknown DTO fields; fail-closed validation. |

---

## 3) Security Boundaries & Attack Surface

**Inbound (adapter builds only)**
- `POST /export` – authenticated, sized DTOs (≤1 MiB).  
- `GET /metrics` – Prometheus, **localhost/gateway only**.  
- `GET /readyz`, `GET /livez` – health/readiness gates.  
- UDS socket (optional) – `SO_PEERCRED` → allowlist UIDs.

**Outbound**
- `ron-ledger` (HTTPS/UDS) – ordered, idempotent `put(slice)`.  
- WAL filesystem – bounded staging (Macronode).  
- Metrics pull.

**Trust Zones**
- Default: **same tenant**, non-public. Public exposure only with TLS + macaroon.

**Firewall Matrix (adapter)**
| Path | Zone | Allowed From | Notes |
|---|---|---|---|
| `/export` | App | edge/gateway | TLS + macaroon |
| `/metrics` | Ops | localhost or monitoring subnet | NEVER public |
| `/readyz` `/livez` | Ops | gateway/mesh | No secrets in body |

---

## 4) Authentication & Authorization (macaroons)

**Macaroon caveats** (examples):

| Caveat            | Example                       | Purpose |
|-------------------|-------------------------------|---------|
| `tenant`          | `tenant=a1f5…`                 | Scopes token to tenant |
| `dimension`       | `dimension=bytes`              | Scopes to dimension |
| `aud`             | `aud=ron-ledger`               | Audience pin |
| `exp`             | `exp=2025-12-31T00:00:00Z`     | Expiry |
| `nonce` (optional)| `nonce=<uuid>`                 | One-time usage patterns |
| `path` (adapter)  | `path=/export`                 | Route lock |

**AuthZ Matrix**

| Action                  | Required Caveats                     | Deny on Missing |
|-------------------------|--------------------------------------|-----------------|
| `POST /export`          | `tenant`, `dimension`, `aud`, `exp`  | 401/403 |
| Export to ron-ledger    | mTLS **or** service token            | 401/403 |
| Read metrics            | none (network restricted)            | 403 if not local/gateway |

Tokens are **short-lived** (≤30d), rotated continuously; revocation = remove secret + deny at edge.

---

## 5) Input Validation & Protocol Hardening

- DTOs `#[serde(deny_unknown_fields)]`; versioned schema; canonical encoding.  
- Frame size **≤1 MiB**; chunking ~64 KiB; check declared size before allocate.  
- **Decompression cap ≤10×** (adapter) with absolute expanded size guard; reject pre-allocation bombs.  
- Numeric bounds: saturating adds on counters; no panics on overflow.  
- Content-type pinned; refuse exotic encodings.

---

## 6) Filesystem & Runtime Hardening

- WAL dir **0700**, owner=effective UID; must support `fsync`; fail with `ERR_WAL_DIR_UNUSABLE`.  
- TLS key **0600**; process `umask 0077`.  
- Containers: run as **non-root**, drop Linux capabilities, seccomp default, read-only FS except WAL dir, memory/cpu limits.  
- No world-writable mounts; no sharing WAL dir across tenants.  
- **Spawn-blocking only for fsync/rename**, bounded by deadline; never block reactor threads.

---

## 7) Denial-of-Service & Backpressure

- **Bounded mpsc** queues everywhere (`pending_slices`, per-stream lanes).  
- **try_send + reject** with `Busy/OrderOverflow`; counters incremented.  
- **Fairness**: round-robin default; WFQ optional with small integer weights (1..=8).  
- Exporter **global op deadline 10s**; retries respect remaining budget.  
- Readiness flips **false** on: exporter backlog>threshold, WAL unusable, or high error-rate.

---

## 8) Observability for Security

**Metrics**
- `rejected_total{reason="unauth"|"capacity"|"order_overflow"|"decompress_cap"|"ratio_cap"|"wal_full"}`  
- `auth_failures_total{service="ron-accounting"}`  
- `tls_handshake_failures_total` (adapter)  
- `accounting_wal_corrupt_total`  
- `accounting_exports_total{status="ok"|"dup"|"retry_network"|"retry_remote_5xx"|"fail"}`  
- `accounting_degraded` (0/1)

**Logs (JSON only)**
- Must include: `ts`, `level`, `service`, `tenant`, `dimension`, `seq`, `reason`, `corr_id`, `peer_addr`.  
- **Never** include secrets or full DTOs; IDs/digests only.

**Tracing**
- Propagate `corr_id`/traceparent where available; mask labels.

---

## 9) Secrets Management

- Load tokens/keys from secret manager or mounted files with **strict perms**; do **not** embed in images.  
- In **amnesia** mode: no secret persistence; all secrets ephemeral.  
- Wrap secrets with `zeroize::Zeroizing`; drop as early as possible.  
- CI scans: `git-secrets`/`trufflehog` job; PR blocked on findings.

---

## 10) Cryptography & Integrity

- **TLS:** 1.3+ only; strong cipher suites (workspace policy); certificate rotation ≤30d; OCSP/stapling per edge policy.  
- **BLAKE3** for slice content digest; **hash-link** `prev_b3` chain per stream for forensic continuity.  
- **Canonical serialization** to ensure cross-arch digest stability (little/big endian parity tested).

---

## 11) Multi-Tenancy & Isolation

- Single active **writer per (tenant,dimension)** on a node; per-stream ordering preserved by design.  
- For HA: ensure only one node becomes **active** writer per stream (via ledger-side lease or ingress coordination); others buffer/forward.

---

## 12) Supply Chain & Build/Release Security

**Policies**
- **MSRV** pinned (1.80.0).  
- **Unsafe Rust:** forbidden unless justified; require code-review tag `@security/unsafe-ack`.  
- **FFI:** none; adding FFI requires separate review.

**Tooling**
- `cargo-deny` (advisories, bans, licenses) in CI.  
- Reproducible builds; **SBOM** (CycloneDX/SPDX) published per release → `docs/sbom/`.  
- (If shipping container) Base image minimal (distroless/alpine per policy), user non-root, CAP drop, seccomp.

**Provenance**
- Attach build provenance (SLSA-style) where infra permits; sign artifacts (cosign/oidc).

---

## 13) Acceptance Gates (PROOF) — Security

_Map to IDB Gates and this doc’s invariants._

1. **AuthN/Z Gate**: invalid/expired/macroon-less requests → **401/403**; `auth_failures_total` increments; logs contain `tenant`, `dimension`, `reason`.  
2. **DTO Strictness Gate**: unknown field → **400**; oversize → **413**; decompression ratio exceed → **429/400** with `rejected_total{reason="decompress_cap"|"ratio_cap"}`.  
3. **Ordering Gate**: property test shows `N+1` cannot be exported before `N` unless `N` is marked **shed**; duplicates count as `status="dup"`.  
4. **WAL Integrity Gate**: corrupt record is skipped, `accounting_wal_corrupt_total` increments; healthy records replay; fsync deadlines enforced.  
5. **Backpressure Gate**: with tiny caps, sustained input yields **rejects** not memory growth; `readyz=false` beyond threshold.  
6. **Secrets Gate**: static & runtime scanners detect no secrets in logs/images; TLS key perms enforced (0600) at startup.  
7. **Chaos Gate**: 10–30% network failure → bounded backlog; consistent drain post-recovery; readiness flips false/true.  
8. **No-PII Gate**: automated grep/static rule ensures no PII-like field names in DTOs/labels; CI blocks if found.  
9. **No Await-Lock Gate**: clippy `await_holding_lock` denied; loom tests pass on concurrency models.

---

## 14) Incident Response & Runbooks

**Signals of trouble**
- Spike in `rejected_total{reason="unauth"}` → credential leak or clock skew.  
- Rising `wal_corrupt_total` → disk/media failure or code regression.  
- Persistent `accounting_degraded=1` → exporter down or downstream throttling.

**Immediate actions**
1. Flip traffic to **local-only** (if adapter public) or disable adapter feature.  
2. Rotate macaroons/TLS keys; invalidate leaked tokens.  
3. For WAL issues: remount to healthy disk; keep **caps**; replay via `wal_replay_task`.  
4. Capture diagnostic bundle: last 1000 lines of logs (redacted), metrics snapshot, config snapshot (no secrets).

**Post-mortem checks**
- Verify **no PII emission** in logs/metrics.  
- Confirm ordering/idempotence preserved in ledger.  
- File CVE/advisory if third-party dependency at fault; bump & release with SBOM.

---

## 15) Configuration Hardening Checklist

- [ ] `amnesia=true` (Micronode) **or** `wal.enabled=true` + caps (Macronode).  
- [ ] WAL dir `0700`, owner=process UID, fsync on close & dir-create.  
- [ ] Adapter exposed only behind TLS + macaroon; `/metrics` not public.  
- [ ] DTO max 1 MiB; decompression cap ≤10×; deny unknown fields.  
- [ ] Channel caps set: `pending_slices_cap`, `ordered_buffer_cap`.  
- [ ] Fairness `round_robin` (default) or `wfq` weights small (1..=8).  
- [ ] Logging JSON; **no** DTO bodies or secrets.  
- [ ] Clippy denies + loom tests enabled in CI.  
- [ ] `RUST_LOG` excludes sensitive modules; sampling enabled for high-cardinality metrics.

---

## 16) Migration & Upgrades

- Auth/key changes (macaroon format, TLS policy) → **major version bump** with explicit migration steps.  
- Deprecations: keep env var aliases ≥1 minor; WARN when used.  
- Changing `window_len_s` requires **restart**; reject hot-flip attempts.

---

## 17) Mermaid — Security Flows

### 17.1 Request/Auth/Export
```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + Macaroon| B(ron-accounting adapter)
  B -->|Validate DTO (size, fields, ratio)| B
  B -->|put(slice) ordered + idempotent| C[ron-ledger]
  B -.->|Reject unauth/oversize/ratio| D[Error + Security Metrics]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
````

### 17.2 WAL Integrity Path

```mermaid
sequenceDiagram
  autonumber
  participant R as ron-accounting
  participant FS as WAL
  R->>FS: write sealed(slice, b3, prev_b3, checksum)
  FS-->>R: fsync (≤5s, blocking thread)
  R->>R: on restart wal_replay()
  R-->>R: skip corrupt; count wal_corrupt_total; continue
```

---

## 18) Appendices

**A. Example macaroon verifier (sketch)**

```rust
fn verify_macaroon(m: &Macaroon, now: SystemTime, req: &Request) -> Result<(), Error> {
    m.check("aud", "ron-ledger")?;
    m.check("tenant", req.tenant.as_str())?;
    m.check("dimension", req.dimension.as_str())?;
    m.check_time("exp", now)?;
    m.check("path", "/export")?;
    Ok(())
}
```

**B. Redaction policy**

* Redact tokens/keys fully.
* Redact tenant/dimension to short IDs only when printed at INFO; full IDs allowed at DEBUG with `redact=true` guard.

**C. Forbidden**

* No custom crypto; no plaintext transport on public networks; no PII fields; no unbounded queues; no debug endpoints in prod.

```
```
