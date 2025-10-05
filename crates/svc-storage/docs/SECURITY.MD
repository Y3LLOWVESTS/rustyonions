---

title: Security Notes — svc-storage
crate: svc-storage
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
-------------

# Security Documentation — svc-storage

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-storage`.
It complements the repo-wide Hardening Blueprint and Interop/IDB documents for RustyOnions.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats                                                                   | Relevant in `svc-storage`? | Mitigation                                                                                                                                                                                                                             |
| -------------------------- | ------------------------------------------------------------------------- | -------------------------: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Impersonated clients/services, unauthenticated writes                     |                      **Y** | TLS 1.3 via **tokio-rustls** (internal or edge), capability tokens (macaroons) on privileged endpoints, UDS with **SO_PEERCRED** allow-lists for intra-node calls                                                                      |
| **T**ampering              | Payload mutation, on-disk corruption, man-in-the-middle on internal links |                      **Y** | **BLAKE3 content addressing** (ETag=`"b3:<hex>"`), streaming checksums, atomic writes, fsync policy on manifests, optional replication/EC repair                                                                                       |
| **R**epudiation            | Unattributable actions, insufficient audit trail                          |                      **Y** | Structured **JSON logs** with `corr_id`, `actor`, `cap_id` (if present), and decision codes; immutable audit events via broadcast bus (drop-oldest allowed but counted)                                                                |
| **I**nformation Disclosure | Leaking object bytes/metadata, secret material in logs, path traversal    |                      **Y** | Capability-gated writes; default public-read policy configurable; **amnesia mode** (no persistent secrets); never log secrets; redact addresses to short prefix; strict path canonicalization (no `..`, **O_NOFOLLOW**/**create_new**) |
| **D**enial of Service      | Flood, slow-loris, oversized bodies, decompression bombs, repair storms   |                      **Y** | **5s** per-op timeouts, **RPS cap**/**inflight cap**, **1 MiB** body limit, **≤10×** decompression ratio cap + absolute cap, bounded channels, global repair **pacing** (e.g., 50 MiB/s)                                               |
| **E**levation of Privilege | Bypass of authN/Z, capability misuse, confused deputy via gateway         |                      **Y** | Require **macaroons** for PUT/privileged ops; scope/caveats checked; no ambient trust headers; defense-in-depth when behind gateway (re-verify capability locally)                                                                     |

**Out-of-scope (explicit):** key management HSM/KMS (owned by ron-kms/ron-auth), economic settlement (svc-rewarder), naming/discovery (svc-index/svc-dht).

---

## 2) Security Boundaries

* **Inbound (exposed by svc-storage):**

  * `GET /o/{b3_addr}` — content-addressed object reads (supports Range).
  * `PUT /o` (or `POST /put`) — content ingest (requires capability).
  * `GET /metrics`, `GET /healthz`, `GET /readyz`, `GET /version` — observability.
  * Optional **UDS** endpoint for intra-node use (gateway/overlay only).
* **Outbound (dependencies):**

  * **svc-index / svc-dht** for placement/provider hints.
  * Peer **svc-storage** instances for replication and repair.
  * Prometheus scraper (pull) and central logging (push/shipper) if configured.
* **Trust Zone:**

  * Designed to be **internet-facing reads** with policy; **writes** and admin/repair are **capability-gated**.
  * When deployed behind **svc-gateway**, treat gateway → storage as **semi-trusted** only; still enforce local caps and auth.
* **Assumptions:**

  * Kernel invariants hold (Bus, Metrics, Health, Config).
  * Upstream (gateway/clients) respect backoff and `Retry-After`.
  * Disk paths are isolated per tenant/instance; process runs as **non-root** with restricted FS privileges.

---

## 3) Key & Credential Handling

* **Key/Secret Types**

  * TLS server certificate/key (if terminating TLS locally).
  * **Macaroon** verification key/secret (HMAC root or delegated verifier).
  * Optional UDS peer allow-list (UIDs).
* **Storage**

  * Secrets live on disk **only** when amnesia=false; permissions `0600`.
  * In **amnesia mode**: all secrets are memory-only; no persistent spill; readiness fails-closed if spill would occur.
* **Rotation**

  * TLS: rotate per ops policy (≤ 90 days typical); reload via SIGHUP/config watcher, cold-rebind if necessary.
  * Macaroon roots/keys: rotate ≤ **30 days**; **revocation** via caveat versioning/deny-list and short TTLs; reject stale capabilities.
* **Zeroization**

  * Wrap secrets in `zeroize::Zeroizing` and clear buffers after parse; avoid debug formatting.
* **Handling Rules**

  * Never log tokens, keys, or full object addresses; log only first **8 hex** chars of `b3:` for correlation.
  * Validate PEM/key file permissions on boot; refuse world-readable keys.

---

## 4) Hardening Checklist (from Blueprint)

* [ ] **Ingress timeouts:** read=5s, write=5s, idle=60s (configurable).
* [ ] **Concurrency cap:** `server.max_inflight` default 512; enforced at accept and per-route layers.
* [ ] **Rate cap:** `server.max_rps` default 500; respond `429` with `Retry-After`.
* [ ] **Request cap:** `limits.max_body_bytes` = **1 MiB**; reject `413` on exceed.
* [ ] **Decompression safety:** ratio cap **≤10×** plus absolute output cap; abort with structured error + counters.
* [ ] **UDS hygiene:** parent dir `0700`, socket `0600`, peer auth via **SO_PEERCRED** allow-list.
* [ ] **Path safety:** open files with `create_new`/`O_EXCL` for manifests; disable following symlinks (`O_NOFOLLOW`); deny `..` traversal.
* [ ] **Atomic writes:** write temp file → fsync → rename; fsync directory for manifests.
* [ ] **Chaos test:** restart under sustained load; `/readyz` flips to **unready** at drain start; no object loss.
* [ ] **Process posture:** run as least-privileged user; drop ambient capabilities; consider seccomp/AppArmor in hardened profiles.

---

## 5) Observability for Security

* **Metrics (minimum):**

  * `rejected_total{reason="unauth"|"busy"|"payload_too_large"|"decompress_cap"|"ratio_cap"}`
  * `auth_failures_total{service="svc-storage", kind="macaroon"}`
  * `tls_handshake_failures_total` (if terminating TLS locally)
  * `integrity_fail_total{reason="checksum"|"decompress_cap"}`
  * `rf_observed`, `rf_target`, `repair_bytes_total` (durability posture)
* **Logs (JSON structured):** fields `ts`, `level`, `service`, `route`, `status`, `reason`, `corr_id`, `peer`, `actor`, `cap_id_prefix`, `b3_prefix` (max 8).

  * No object bytes or secrets in logs.
  * One-line outcome per request (accept/reject/latency).
* **Health/Readiness gates:**

  * `/healthz` (process alive) stays green even when degraded.
  * `/readyz` fails-closed during drain, missing dependencies, or would-spill in amnesia mode.

---

## 6) Dependencies & Supply Chain

* **Sensitive crates:** `tokio-rustls` (TLS 1.3), `axum` (HTTP), `bytes`, `blake3`, `prometheus`, `zeroize`, decompression libs (e.g., `flate2`, `zstd`) guarded by caps, optional `reed-solomon-erasure` for EC.
* **Version pinning:** via workspace root `[workspace.dependencies]` (avoid duplicate major versions).
* **Policy:** `cargo-deny` in CI for licenses/advisories/duplicates/sources; disallow unknown registries.
* **SBOM:** generate on release (CycloneDX/SPDX) and store in `docs/sbom/`.
* **Build Reproducibility:** prefer `--locked`; verify third-party checksums.

---

## 7) Formal & Destructive Validation

* **Property tests:**

  * Digest invariants (served bytes re-hash to requested `b3:`).
  * Idempotent PUT (replays don’t duplicate durable state).
  * Path canonicalization rejects traversal/symlinks.
* **Fuzzing:**

  * HTTP header/body parsers; decompression streams (truncated/oversized/zip-bomb corpus).
* **Loom (dev):**

  * Listener → bounded work queue → worker under shutdown; assert no deadlocks, no lost shutdown paths, no double-drop.
* **Chaos:**

  * Kill/respawn workers under load; verify `/readyz` flipping and RF gauges not regressing below policy.
* **TLA+ sketch (optional but encouraged):**

  * Repair planner state machine: safety (never exceed pacing budget) and liveness (eventually reaches RF target).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Security contact:** [security@rustyonions.local](mailto:security@rustyonions.local) (placeholder)
* **Responsible disclosure:** See repository `SECURITY.md` (90-day coordinated disclosure, PGP key available).

---

## 9) Migration & Upgrades

* **Auth/Key changes:** breaking changes to capability validation or TLS handling require a **major version bump** and RUNBOOK steps.
* **Decompression policy changes:** raise caps only with explicit risk sign-off; lowering caps is backward-compatible but may reject previous workloads.
* **Deprecations:** document aliases for env/flags for ≥1 minor; emit warnings; provide conversion notes.

---

## 10) HTTP Security Headers (Service Defaults)

* `ETag: "b3:<hex>"` (strong validator), `Cache-Control` per policy.
* `X-Content-Type-Options: nosniff` to prevent MIME sniffing issues.
* `Vary: Accept-Encoding` when ranges/compression apply.
* Do **not** enable dynamic response compression for opaque binary blobs unless explicitly configured (reduces side-channel surface like BREACH).

---

## 11) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Client] -->|TLS 1.3 + macaroon (PUT)| B(svc-storage)
  A -->|TLS 1.3 (GET)| B
  B -->|BLAKE3 verify + policy| C[Disk/Peers]
  B -.->|Reject: unauth/oversized/ratio| D[Error + Metrics + JSON log]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text description:** Clients call svc-storage over TLS (or UDS internally). svc-storage verifies capabilities for privileged ops, enforces size/ratio/time limits, and validates content by BLAKE3 before serving. Violations return errors and increment security metrics with structured logs.

---

## 12) Operational Playbook (Security-relevant Excerpts)

* **Compromise drill:** rotate macaroon root, invalidate old caveat versions, redeploy; roll TLS certs; force config reload.
* **Data at rest:** enable replication/EC; schedule periodic **scrub** (re-hash sample set) and repair if drift detected.
* **Access review:** quarterly review of UDS `allow_uids`, public bind usage, and capability scopes.
* **Incident logging:** preserve raw JSON logs; export to WORM storage; attach `corr_id` to customer communications.

---

**Security Definition of Done:**
All inbound privileged paths are capability-gated; hard limits are enforced with metrics; secrets never persist in amnesia mode; TLS/UDS hygiene checks block misconfiguration; structured logs + metrics provide an audit trail; and fuzz/loom/chaos tests cover critical paths.
