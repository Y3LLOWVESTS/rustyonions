````markdown
---
title: Security Notes — svc-dht
crate: svc-dht
owner: Stevan White
last-reviewed: 2025-10-10
status: draft
---

# Security Documentation — svc-dht

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `svc-dht`.  
It complements the repo-wide Hardening, Interop, Concurrency, Config, and IDB blueprints.

---

## 1) Threat Model (STRIDE)

> Context: `svc-dht` is a discovery service that speaks a Kademlia/Discv5-style DHT over `ron-transport` (TLS/QUIC/TCP, optional Tor). It accepts peer RPCs (`FIND_NODE`, `FIND_VALUE`, `PROVIDE`, `STORE`) and exposes observability (`/metrics`, `/healthz`, `/readyz`, `/version`). Provider records are **signed** and **expiring**. Amnesia mode is supported.

| Category | Threats | Relevant in `svc-dht`? | Mitigation |
|---|---|---|---|
| **S**poofing | Fake peers/IDs, DNS poisoning of bootstrap seeds | **Yes** | TLS via `ron-transport` where configured; strict peer-ID format; bootstrap diversity and certificate pinning (where applicable); signed provider records (Ed25519 + optional ML-DSA-87) |
| **T**ampering | On-wire mutation, forged provider records, route poisoning | **Yes** | TLS integrity; record signature verification; content addressing (BLAKE3 `b3:`); single-writer buckets; deterministic error taxonomy |
| **R**epudiation | “We never sent that” claims; missing audit trail | **Yes** | Structured JSON logs; stable `lookup_id`/correlation IDs; signed control frames; clocked timestamps |
| **I**nformation Disclosure | Provider hints, peer addresses, keys in logs; HNDL (Harvest-Now-Decrypt-Later) | **Yes** | No secrets in logs; redaction; amnesia mode (RAM-only); TLS 1.3 PFS today; roadmap: **hybrid TLS KEX** (X25519 + ML-KEM/Kyber) when available; minimize data retention |
| **D**enial of Service | Flood (RPS/inflight), slow-loris, decompression bombs, neighbor table stuffing, eclipse attempts | **Yes** | RPS + inflight quotas; strict timeouts; frame/body caps; decompression ratio caps; bounded channels; ASN/geo diversity floor + entropy; bounded hedging with cancel; early `Busy` shedding |
| **E**levation of Privilege | Unauthorized ops, abuse of metrics/health/control sockets | **Yes** | Capability tokens (macaroons) if enabled; no admin mutators over public DHT; metrics/health on localhost/UDS; UDS PEERCRED allowlist; validate config & fail-closed |

**Post-Quantum / HNDL note:** While TLS 1.3 provides PFS (mitigating HNDL against classical adversaries), long-term confidentiality is treated as a PQ risk. Roadmap: enable **hybrid TLS key exchange** (X25519 + **ML-KEM/Kyber**) in `ron-transport` once available; continue **dual-signing** envelopes (Ed25519 + **ML-DSA-87**).

**Assumptions (non-goals):**
- DHT peer set is partially adversarial, but cannot break standard primitives (TLS/signatures).
- `svc-dht` stores no end-user PII; provider records are public by design.
- In amnesia mode, persistence is **off**; loss after restart is acceptable.

---

## 2) Security Boundaries

- **Inbound (public or semi-public):**
  - DHT RPCs over `ron-transport` (TLS/TCP or QUIC; optional Tor/relay).
  - HTTP: `/metrics` (Prometheus), `/healthz`, `/readyz`, `/version` *(default bind localhost)*.
  - Optional UDS for local control plane.

- **Outbound:**
  - `ron-transport` dials to peers and bootstrap seeds.
  - Logs/metrics sinks (pull scrape preferred; no push by default).

- **Trust Zone:**
  - Runs as **non-root** in a standard container/VM; network-exposed to public DHT.
  - `/metrics` and control sockets should be **localhost/UDS** only in production.

- **Assumptions:**
  - Kernel (bus/metrics/health/config) invariants hold.
  - Transport/KMS enforce their own quotas & readiness.
  - FS namespace is **service-scoped**; no shared writable volumes unless required.

---

## 3) Key & Credential Handling

- **Key/secret types:**
  - **TLS server cert/key** (PEM) if serving TLS endpoints.
  - **Node identity** (transport-specific; e.g., QUIC keys).
  - **Macaroon root key** (if capability tokens enabled for admin endpoints).
  - **Signature verification keys** for provider/control envelopes (Ed25519, ML-DSA-87).

- **Storage:**
  - On disk: PEM files (0600), owned by service user. Never world-readable.
  - In memory: wrap sensitive bytes in `zeroize::Zeroizing`; drop ASAP.
  - **Amnesia mode:** disallow on-disk storage; refuse to start if disk-backed stores are configured.

- **Rotation:**
  - TLS/macaroons: rotate **≤ 30 days**, reload via SIGHUP; fail-closed on invalid.
  - Bootstrap pins/certs: review/rotate quarterly.
  - PQ migration: enforce `pq.require_pq_on`; after that, classical-only records warn (optionally reject per policy).

- **Zeroization & libraries (verification):**
  - Use `zeroize::Zeroizing`; constant-time compares via `subtle::ConstantTimeEq`.
  - Prefer `ed25519-dalek` (or `ring` where suitable) for Ed25519; **ML-DSA-87** via `pqclean` bindings behind a local trait.  
  - ❌ **No custom crypto**.

---

## 4) Hardening Checklist (from Blueprint & IDB)

- [ ] **Timeouts**: HTTP read/write 5s; transport RPC 1500ms; hedge after 250ms; idle 60s.  
- [ ] **Concurrency caps**: inflight ≤ 512; RPS ≤ 500 (profile-dependent).  
- [ ] **Body/frame caps**: HTTP body ≤ 1 MiB; transport frame ≤ 1 MiB.  
- [ ] **Decompression**: ratio ≤ 10× with absolute cap; reject on exceed.  
- [ ] **Buckets**: single-writer actor; no `.await` under lock; commit via snapshot swap.  
- [ ] **Diversity**: per-bucket ≤ 40% per ASN **and** entropy ≥ 1.5 bits; prefer diverse replacements.  
- [ ] **Amnesia**: when enabled, refuse disk persistence; logs ephemeral.  
- [ ] **UDS**: dir `0700`, socket `0600`, enforce `SO_PEERCRED` allowlist.  
- [ ] **Observability**: `/metrics` on localhost or behind firewall; scrub sensitive fields (none expected).  
- [ ] **Chaos**: restart under load → `/readyz` fail-closed until bootstrap/B% buckets filled.  
- [ ] **No custom crypto**: use vetted libs only.  
- [ ] **Run as non-root**; drop Linux caps; seccomp profile applied.  
- [ ] **PQ migration controls**: dual-sign active; `pq_dual_sign_ratio ≥ 0.95`; warn classical-only after `require_pq_on`.  
- [ ] **Container MAC**: AppArmor/SELinux profile enforced; read-only root FS.

---

## 5) Observability for Security

- **Metrics (minimum):**
  - `rejected_total{reason="unauth"|"quota"|"ratio_cap"|"frame_cap"|"bad_sig"|"stale"|"not_ready"}`
  - `tls_handshake_failures_total`
  - `pq_dual_sign_ratio`, `pq_classical_only_total`, `pq_verify_fail_total{alg}`
  - `dht_bucket_asn_entropy_bits{bucket}` (gauge), `eclipse_suspect_total`
  - `busy_rejections_total{endpoint}`, `io_timeouts_total{op}`

- **Logs (JSON):**
  - Fields: `service`, `lookup_id`, `op`, `peer_id`, `peer_addr`, `asn`, `reason`, `error`, `hop_count`, `duration_ms`.
  - No secrets; redact tokens; log certificate/key **fingerprints** only.

- **Health gates:**
  - `/readyz` stays **red** until `bootstrap.required` reachable **and** `ready_bucket_fill_pct` achieved; on quota storms, degrade writes first.
  - `/healthz` is liveness only; never expose sensitive state.

- **Alert expressions (Prometheus):**
  - **Hop SLO p99 > 5 (5m):**  
    `(1 - histogram_quantile(0.99, sum by (le) (rate(dht_lookup_hops_histogram_bucket[5m])))) < 0.01`
  - **PQ compliance (30m warn):**  
    `avg_over_time(pq_dual_sign_ratio[30m]) < 0.95`
  - **Diversity erosion (30m warn):**  
    `avg_over_time(dht_bucket_asn_entropy_bits[15m]) < 1.5`
  - **Quota storm (10m warn):**  
    `sum(rate(rejected_total{reason="quota"}[5m])) > 5`

---

## 6) Dependencies & Supply Chain

- **Security-sensitive crates (pin at workspace root):**
  - `tokio-rustls` (TLS 1.3), `quinn` (QUIC) via transport adapters.
  - `ed25519-dalek` / `ring` (Ed25519 verify), `pqclean` (ML-DSA-87 verify) behind trait.
  - `blake3` (content addressing), `zeroize`, `subtle` (constant-time compares).
  - `serde`, `rmp-serde` (if MsgPack), `humantime-serde`.

- **Controls:**
  - `cargo-deny` in CI (advisories, bans, duplicates, sources).
  - **SBOM** generated on release and published under `docs/sbom/`.
  - **Vendoring policy:** If a security-sensitive crate becomes unmaintained, pin & vendor under `third_party/` with license headers; add periodic diff checks.
  - **Repro policy:** `Cargo.lock` committed; CI runs `cargo-deny check`; optional `-Z minimal-versions` job to minimize attack surface.

---

## 7) Formal & Destructive Validation

- **Property tests:** reject unsigned/oversized/stale provider records; enforce hop budget invariants.
- **Fuzzing (libFuzzer/AFL):** frame & record parsers; target **> 90%** edge coverage; **0 crashes in 10 minutes** at -O1 on CI baseline; corpus includes invalid sigs/TTL/lengths.
- **Loom (dev-only):** model producer → mpsc(work) → lookup → mpsc(route) → bucket-writer + shutdown; assert no deadlocks, no missed shutdown, no reordering that violates invariants.
- **Chaos:** kill/respawn Lookup workers & BucketWriter under **3× RPS for 60s**; `/readyz` goes red within **≤ 2s** and returns green within **≤ 300s** post-recovery.
- **Interops:** simulate **70% single-ASN** seeding; ensure avg unique-ASN ≥ 3 and **entropy ≥ 1.5 bits** under load.
- **(Optional) TLA+:** model lookup contraction + bucket updates: safety (no invalid insert), liveness (eventual completion with bounded faults).

---

## 8) Security Contacts

- **Maintainer:** Stevan White  
- **Backup/Ops:** RustyOnions Platform Team (on-call)  
- **Disclosure policy:** See repo root `SECURITY.md` (responsible disclosure; PGP key/fingerprint there).

---

## 9) Migration & Upgrades

- **Auth/Key changes:** any breaking change to signatures, PQ policy, or macaroon schema ⇒ **major** version bump + migration guide.
- **Deprecations:** keep env aliases ≥ 1 minor; emit warnings on use; document EoL dates.
- **PQ rollout:** dual-sign during migration; after `require_pq_on`, warn (optionally reject via policy flag) classical-only records.

---

## 10) Mermaid — Security Flow Diagram (REQUIRED)

```mermaid
flowchart LR
  A[Peer / Client] -->|TLS/QUIC (PFS; plan hybrid PQ)| B(svc-dht)
  A -.->|Unsigned / Oversized / Quota exceeded| D[Reject + Metrics + Typed Error]
  B -->|Verify sig: Ed25519| B
  B -->|Verify sig: ML-DSA-87 (if present)| B
  B -->|Validated request| C[BucketWriter / Lookup Engine]
  B -->|/metrics (localhost)| M[Prometheus]
  style B fill:#b91c1c,stroke:#7f1d1d,color:#fff
  style D fill:#1f2937,stroke:#111827,color:#fff
````

**Text:** Peers connect over secure transport. `svc-dht` validates framing, quotas, and signatures (Ed25519 + optionally ML-DSA-87) before handing requests to routing/lookup actors. Bad inputs are rejected with metrics and typed errors. Metrics are scraped locally.

---

## 11) Appendix — Concrete Guardrails (Copy-Paste)

* **TLS listener (tokio-rustls)**

  * Min protocol: TLS1.3; disable legacy ciphers; enable OCSP stapling if available.
  * Enforce SNI when virtual-hosting.

* **Constant-time comparisons**

  * Use `subtle::ConstantTimeEq` for digest/key equality checks on hot paths.

* **Reject early**

  * Apply quotas **before** expensive validation; return `Busy`/`NotReady` deterministically.

* **Log discipline**

  * Never log provider payloads; log count/size only.
  * Log peer IDs and ASN; avoid raw addresses where policy requires privacy.

* **Container runtime**

  * Drop Linux capabilities; read-only root FS; tmpfs for `/tmp`; non-root UID/GID; seccomp.

* **Container MAC & k8s examples**

  ```yaml
  # AppArmor (example): profiles/svc-dht.apparmor
  profile svc-dht flags=(attach_disconnected,mediate_deleted) {
    # deny dangerous syscalls; allow net, read-only fs, tmpfs
  }

  # Kubernetes securityContext (example)
  securityContext:
    allowPrivilegeEscalation: false
    readOnlyRootFilesystem: true
    runAsNonRoot: true
    runAsUser: 10001
    capabilities:
      drop: [ "ALL" ]
    seccompProfile:
      type: RuntimeDefault
  ```

---

## 12) Security Trade-offs

* **Amnesia vs. Forensics:** Amnesia (RAM-only) reduces data exposure but limits post-incident auditability. Mitigation: amplify **structured logging** with `lookup_id`/peer context (no secrets); optionally export to remote SIEM if policy allows.
* **Hedging vs. Attack Surface:** Hedged probes cut tail latency but increase network cost; bounded by `beta` and aggressively canceled to prevent amplification.
* **Diversity vs. Reachability:** ASN/entropy floors can slow convergence in sparse regions; temporary relaxations require explicit change control and alerting.

---

```
```
