---

title: Security Notes — svc-registry
crate: svc-registry
owner: Stevan White
last-reviewed: 2025-10-08
status: draft
-------------

# Security Documentation — svc-registry

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-registry`.
It complements the repo-wide Hardening, Interop, and IDB blueprints.

> One-line role recap: `svc-registry` is the **authoritative source** of versioned, **signed** service/node/region descriptors. It verifies signatures, applies quorum (multi-sig) rules, appends to an **append-only** audit chain, and announces updates over the kernel bus. It does **not** mint identity or evaluate policy.

---

## 0) Data & Trust Overview (Context)

* **Data handled**

  * *Public-ish:* versioned descriptors, registry metadata, health/metrics.
  * *Sensitive (control-plane):* signer lists, approval objects, capability tokens (macaroons) presented on write paths.
  * *Operational secrets:* TLS private key (service endpoint), optional macaroon files for local admin tooling.
* **Trust posture**

  * Reads may be public (rate-limited); **writes require explicit capability + multi-sig** approvals.
  * No ambient authority; no emergency bypass toggles.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (examples)                                                          | Relevant? | Primary Mitigations                                                                                                                                                                                                        |
| -------------------------- | --------------------------------------------------------------------------- | :-------: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Impersonated client; forged approver; local UDS peer spoof                  |   **Y**   | TLS 1.3 (tokio-rustls) for TCP; UDS with `SO_PEERCRED` allow-list; macaroon capabilities with scoped caveats; signer identities pinned in signer-set artifact                                                              |
| **T**ampering              | Altered payloads; unauthorized commit; storage corruption                   |   **Y**   | Content addressing (BLAKE3) on payloads; threshold signatures verified pre-commit; single-writer CAS on `HEAD{version,hash}`; fsync on commit/checkpoint; append-only log; checkpoints are signed                          |
| **R**epudiation            | “We never approved this” / missing audit trail                              |   **Y**   | Structured JSON logs with `corr_id`, signer IDs and input hashes; append-only hash chain with `prev_hash`; bus audit events; time-sync tolerance & clock-skew checks                                                       |
| **I**nformation Disclosure | Token/key leakage; path traversal; metrics data leak                        |   **Y**   | No secrets in logs; zeroize sensitive buffers; `serde(deny_unknown_fields)`; `/metrics` on localhost by default; body/field allow-lists; amnesia mode disables local secret persistence                                    |
| **D**enial of Service      | Flooding, slow-loris, decompression bombs, queue overrun, subscriber stalls |   **Y**   | Timeouts (read/write/idle); RPS & inflight caps; body size cap & decompression ratio cap; bounded channels with `Busy` rejections; broadcast with lag/drop-to-latest + reconciliation; readiness degrades **writes first** |
| **E**levation of Privilege | Bypass multi-sig; config-based backdoor; confused deputy                    |   **Y**   | Quorum enforced in code (property tests); config cannot influence signer/quorum; macaroon scopes `{registry:propose,approve,commit}`; endpoints least-privilege; no “maintenance mode”                                     |

---

## 2) Security Boundaries

* **Inbound**

  * HTTP(S) API (Axum/Tower):

    * `POST /registry/proposals` (capability required)
    * `POST /registry/approvals/{proposal_id}` (capability required)
    * `POST /registry/commit/{proposal_id}` (capability + quorum preconditions)
    * `GET /registry/head` (read; rate-limited)
    * `GET /registry/{version}` (read; rate-limited)
    * `GET /registry/stream` (SSE/WS; bounded, lossy)
  * Optional **UDS** socket with `SO_PEERCRED` allow-list for on-host calls.
* **Outbound**

  * Kernel **bus** publish: `RegistryUpdated{version,b3}`, audit events.
  * Metrics endpoint `/metrics` for Prometheus (pull).
  * Storage backend (sled/sqlite) on local disk (Macronode) or ephemeral (Micronode).
* **Trust zones**

  * Internet / untrusted (for reads if exposed).
  * Cluster / service mesh (recommended surface).
  * Local host (UDS, highest trust for ops).
* **Assumptions**

  * Time is reasonably synchronized (tolerances applied to approval timestamps).
  * Downstream bus/metrics collectors are authenticated at their perimeter.
  * Host FS permissions are enforced by deployment (data dir `0700`).

---

## 3) Key & Credential Handling

* **Key/secret types**

  * TLS private key for the service endpoint (if TLS terminates here).
  * Capability tokens (macaroons) used by **clients** to invoke write endpoints (not minted here).
  * **No private signer keys** are stored by this service; it verifies signatures from external signers.
* **Storage**

  * TLS keys on disk with `0600` and service-user ownership.
  * Macaroon files (optional for local admin client) on disk with `0600`; never logged; read into memory then zeroized.
  * **Amnesia mode:** no local persistence of secrets or mutable state beyond RAM; audits emitted to bus only.
* **Rotation & revocation**

  * TLS: rotate ≤ 90 days (deployment policy).
  * Macaroons: rotate ≤ 30 days; short TTLs recommended; capability attenuation supported.
  * Signer set rotation/revocation occurs **via signed registry commits**; revoked keys are rejected immediately at approval time.
* **In-memory hygiene**

  * Sensitive buffers use `zeroize::Zeroizing` where applicable.
  * Avoid long-lived global caches of tokens; prefer per-request verification.

---

## 4) Hardening Checklist (enforced/required)

* [x] HTTP **read=5s**, **write=5s**, **idle=60s** timeouts (configurable).
* [x] Concurrency cap: **inflight ≤ 512** (per listener), **RPS ≤ 500**.
* [x] Request body cap: **1 MiB**; content-encoding ratio cap **≤ 10×** and absolute post-inflate cap.
* [x] Bounded queues everywhere; **`try_send` → Busy** on overload; no await on full queues in request path.
* [x] UDS: directory `0700`, socket `0600`, enforce `SO_PEERCRED` + allow-list.
* [x] TLS: **tokio-rustls** only; TLS 1.3 preferred ciphers; OCSP stapling optional.
* [x] `/metrics` bound to localhost by default; scrape via sidecar/gateway.
* [x] **No emergency bypass** of quorum; no config knob alters signer/quorum rules.
* [x] Chaos test: restart under load; `/readyz` flips write-unready first; drains within deadline.

---

## 5) Observability for Security

* **Metrics (security-relevant)**

  * `rejected_total{reason="unauth"|"invalid_sig"|"revoked_key"|"decompress_cap"|"ratio_cap"|"body_cap"}`
  * `auth_failures_total{surface="propose|approve|commit"}`
  * `tls_handshake_failures_total`
  * `busy_rejections_total{endpoint}`
  * `bus_lagged_total{consumer}`
  * `commit_cas_fail_total`
* **Logs**

  * **Structured JSON**; include: `service`, `endpoint`, `corr_id`, `peer_addr`, `result`, `err.code`, `b3`.
  * Never log tokens, private keys, or full payloads; log **hashes** and signer IDs.
  * On supersede: include `supersede_reason`, targeted `HEAD` hash.
* **Health gates**

  * `/readyz` = **false** if storage fsync fails, commit CAS loops persist, or bus back-pressure exceeds threshold; reads may remain available per IDB.

---

## 6) Dependencies & Supply Chain

* **Critical crates**

  * `tokio`, `tower` (async runtime & middleware)
  * `axum` (HTTP server)
  * `tokio-rustls` + `rustls` (TLS 1.3)
  * `serde` + `serde_json`/CBOR (strict DTOs with `deny_unknown_fields`)
  * `blake3` (content addressing)
  * `ed25519-dalek` / `ring` (sig verification; exact choice per `ron-proto`)
  * `zeroize` (in-memory secret hygiene)
* **Controls**

  * Versions pinned at workspace root; **`cargo-deny`** for vuln/licence advisories.
  * **Supply chain policy**: disallow yanked crates; audit new transitive deps in PR.
  * **SBOM** generated at release and stored under `/docs/sbom/`.

---

## 7) Formal & Destructive Validation

* **Property tests**

  * **Quorum safety**: no commit without M-of-N; revoked keys invalidate approvals.
  * **Audit-chain integrity**: `prev_hash` chain cannot be altered undetected.
  * **CAS safety**: single-writer; concurrent commit attempts produce at most one new HEAD.
* **Fuzz**

  * HTTP/JSON (and CBOR) decoders: malformed, truncated, duplicate approvals, ratio-bombs.
* **Loom (concurrency)**

  * Producer→Committer→Publisher model with shutdown; assert no deadlock, no double-commit.
* **Chaos**

  * Stall bus subscribers; storage latency spikes; verify `/readyz` degradations and **no data loss** (readers reconcile via HEAD).
* **TLA+ (optional)**

  * Spec for HEAD versioning + supersede; safety: no forks; liveness: eventual commit with fair scheduling.

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Security escalation:** Use repo root `SECURITY.md` contact and disclosure policy.
* **On-call (ops):** See operational runbook for rotations and pager duty mapping.

---

## 9) Migration & Upgrades

* **Auth surface changes** (capability formats, scopes) ⇒ **major version** with documented migration.
* **Signer-set schema changes** ⇒ major version; provide dual-read period and deprecation headers in API.
* **Deprecations** must include:

  * Sunset ≥ 180 days **or** ≥ 2 minor releases (whichever longer).
  * Announcements via `/version`, `/schema`, and release notes.

---

## 10) Secure Defaults & Profiles

* **Macronode (persistent):** fsync enabled; checkpoints every 10m; retention policy active; `/metrics` on localhost; TLS on.
* **Micronode (amnesia):** RAM-only; no local audit persistence; emits audits on bus; data dir must be tmpfs; retention/compaction disabled or tmpfs-safe.

---

## 11) Abuse Cases & Responses

* **Approval replay:** Detect same `(proposal_id, signer_id)` pair; reject with `err.code=DuplicateApproval`.
* **Flooded `proposals` queue:** Return `429 Busy`; increment `busy_rejections_total`; advise client backoff.
* **Slow event consumer:** `bus_lagged_total++`; consumer must reconcile via `GET /registry/head`.
* **Config tampering attempt:** Config reload cannot alter quorum/signers; reject and log `err.code=ForbiddenConfig`.

---

## 12) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon (write paths)| B(svc-registry)
  A -->|HTTP (read)| B
  B -->|Verify sigs + quorum + CAS HEAD| B
  B -->|Publish RegistryUpdated| C[Kernel Bus]
  B -.->|Reject: unauth/invalid_sig/body_cap/ratio_cap/busy| D[Error + Metrics + JSON log]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

---

## 13) Reviewer Checklist (PR Gate)

* [ ] No secrets in logs; new logs include `corr_id`, `b3`, `err.code`.
* [ ] New endpoints specify **caps required**, rate limits, and body caps.
* [ ] DTOs use `deny_unknown_fields`; no accidental field exposure.
* [ ] Any change to commit path preserves **single-writer** and **CAS**.
* [ ] Bus emissions remain **non-blocking** to commit.
* [ ] Config changes do **not** enable quorum/identity bypass.
* [ ] Tests updated: property/loom/fuzz where applicable.

---

**Definition of Done (Security):**
All STRIDE categories addressed with concrete mitigations; hardening caps in place; logs/metrics wired for security visibility; destructive tests codified; no governance bypass via configuration; secrets managed with least privilege and zeroization.
