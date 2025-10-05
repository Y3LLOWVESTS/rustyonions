

---

````markdown
---
title: svc-storage — Invariant-Driven Blueprint (IDB)
version: 0.1.1
status: reviewed
last-updated: 2025-10-04
audience: contributors, ops, auditors
concerns: [SEC, RES, PERF, GOV]
---

# svc-storage — Invariant-Driven Blueprint (IDB)

## 1) Invariants (MUST)

- **[I-1] Canonical content addressing.** Every stored object/manifest is addressed by `b3:<hex>` (BLAKE3-256 of the *unencrypted* bytes / manifest root). Full digest is verified before serving. Weak prefixes MAY route but MUST NOT be used as proof.
- **[I-2] OAP/HTTP bounds.** Protocol frames ≤ **1 MiB**; storage I/O streams in ~**64 KiB** chunks (perf/backpressure knob, not a protocol limit).
- **[I-3] Safe decompression only.** Enforce **ratio ≤ 10×** and an **absolute output cap**; reject on exceed with structured errors.
- **[I-4] Byte-range correctness.** Support strong `ETag: "b3:<hex>"`, correct 200/206/416 semantics, and `Vary: Accept-Encoding`.
- **[I-5] Backpressure + aliasing safety.** All queues/channels are **bounded**; no lock is held across `.await`; single-writer discipline per connection.
- **[I-6] Amnesia mode honored.** With amnesia **ON** (Micronode default): RAM-only caches, no persistent spill; readiness degrades rather than spilling.
- **[I-7] UDS hardening (internal planes).** Socket dir `0700`, socket `0600`, enforce **SO_PEERCRED** allow-list.
- **[I-8] Durability is policy-driven.** Replication factor (RF) and optional EC (Reed–Solomon) are enforced with bounded, paced repair; RF gauges reflect pre/post state.
- **[I-9] Service SLOs.** Intra-region read p95 **start < 80 ms** (media range p95 < 100 ms); 5xx < 0.1%, 429/503 < 1%.
- **[I-10] Capability-only access.** No ambient trust; writes and privileged ops require capabilities (macaroons).
- **[I-11] Crypto agility (PQ-ready).** Manifests and integrity headers MUST NOT hard-code signature/KEM choices; storage verifies content by BLAKE3 and treats signatures as *metadata* verified upstream (e.g., by ron-auth/kms). This preserves PQ swap-in without breaking interop.

## 2) Design Principles (SHOULD)

- **[P-1] Owned bytes on hot paths.** Prefer `bytes::Bytes` end-to-end; avoid borrowing short-lived buffers into responses.
- **[P-2] Stream, don’t buffer.** Use ~64 KiB streaming I/O to bound memory and latency.
- **[P-3] Plane separation.** Naming/discovery live in `svc-index`/`svc-dht`; storage serves bytes and durability.
- **[P-4] Policy-first placement.** Enforce residency/geo policy; prefer local RTT < 50 ms; use conservative hedged reads.
- **[P-5] Observability as a feature.** `/metrics`, `/healthz`, `/readyz`, `/version` are mandatory; degraded modes are visible.
- **[P-6] PQ/ZK friendliness.** Keep manifests algorithm-neutral (alg tags, versioned schema) so PQ or ZK attestations can attach later without format breaks.

## 3) Implementation (HOW)

### [C-1] HTTP/Axum protection (drop-in)
- Timeouts: request/read/write **5s**; concurrency cap **512**; body cap **1 MiB**; RPS cap **500/instance**; safe decompression (≤10× + absolute cap).
- Map errors: 400/404/413/416/429/503 with machine-parsable bodies; include `retry-after` for 429/503.

### [C-2] CAS GET flow (sketch)
1. Parse address → `b3:<hex>`.
2. Lookup chunk map (local or peer).
3. Stream chunks (~64 KiB), verify BLAKE3 on the fly.
4. If `Range` present, serve 206 with correct `Content-Range`.
5. Set `ETag: "b3:<hex>"`, `Cache-Control` per policy.

### [C-3] Safe decompression guard
- Wrap decoders with a counting reader enforcing: (a) byte cap, (b) ratio cap ≤ 10×.
- On exceed: abort, return 413, increment `decompress_reject_total{reason="cap"}` and `integrity_fail_total{reason="decompress_cap"}`.

### [C-4] Replication/EC repair worker
- Enforce RF/EC with **bounded concurrency per peer/volume** and **global pacing ≤ 50 MiB/s per cluster**.
- Expose `rf_target`, `rf_observed`, `repair_bytes_total`, `repair_seconds_total`.

### [C-5] Concurrency discipline
- Never `.await` while holding a lock; copy minimal state and drop guard.
- Single writer per connection; readers on dedicated tasks.
- Prefer `tokio::sync::mpsc` with explicit `capacity`.

### [C-6] Amnesia switch
- When ON: disable disk tiers unless explicitly permitted; use tmpfs for ephemeral spill; label metrics with `amnesia="on"`; fail-closed readiness on would-spill events.

### [C-7] UDS hardening
- Create socket dir with `0700`, socket `0600`.
- Verify **SO_PEERCRED**; increment `peer_auth_fail_total` on reject.

### [C-8] Repair lifecycle (ops mental model)
```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Scan: periodic inventory
    Scan --> Plan: diff RF/EC targets vs observed
    Plan --> ThrottleCheck: budget & pacing window
    ThrottleCheck --> Repairing: if budget available
    ThrottleCheck --> Idle: if none
    Repairing --> Verify: checksum BLAKE3
    Verify --> Commit: mark durable
    Commit --> Idle
    Repairing --> Degraded: exceed error budget
    Degraded --> Idle: operator action / cooldown
````

## 4) Acceptance Gates (PROOF)

### Tests

* **[G-1] Integrity property test.** Round-trip store→get verifies BLAKE3; corrupted chunk → 502 + `integrity_fail_total{reason="checksum"}`.
* **[G-2] Bounds enforcement.** HTTP >1 MiB body → 413; OAP fuzzer rejects >1 MiB frames; streaming uses ~64 KiB.
* **[G-3] Decompression bombs.** Red-team corpus exceeds ratio/size caps → 413; counters increment.
* **[G-4] Concurrency/aliasing.** Clippy denies (`await_holding_lock`, `unwrap_used`, etc.); loom/TSan for critical paths pass in CI.
* **[G-5] Amnesia matrix.** amnesia=ON leaves **no on-disk artifacts** after full test suite; amnesia=OFF persists per config; API behavior identical.
* **[G-6] UDS perms + peer-cred.** CI fails if dir ≠ `0700` or sock ≠ `0600`; negative peer-cred test rejected.
* **[G-7] RF/EC repair.** Sim harness drops replicas; `ronctl repair` restores RF within pacing budgets; gauges reflect pre/post.

### Metrics (golden, must exist)

* `storage_get_latency_seconds{route,range}` (histogram), `storage_put_latency_seconds`.
* `chunks_read_total`, `chunks_written_total`, `bytes_in_total`, `bytes_out_total`.
* `integrity_fail_total{reason}`, `decompress_reject_total`, `quota_exhaustions_total`.
* `rf_target`, `rf_observed`, `repair_bytes_total`, `repair_seconds_total`.
* Health endpoints: `/metrics`, `/healthz`, `/readyz`, `/version` up and accurate.

### SLO checks

* Intra-region read **p95 start < 80 ms**; media range **p95 < 100 ms**.
* Error budgets: 5xx < 0.1%, 429/503 < 1% (rolling 30-day).

### CI-Greps (anti-scope with teeth)

* Ban SHA-2 addressing for objects:

  * `grep -RInE '\bsha(1|224|256|384|512)\b' crates/svc-storage/src | exit 1` (allowlist only in docs/tests if needed)
* Ban ambient auth:

  * `grep -RIn 'X-Internal-Bypass' crates/svc-storage/src && exit 1`
* Ban unbounded channels:

  * `grep -RIn 'unbounded_channel' crates/svc-storage/src && exit 1`

## 5) Anti-Scope (Forbidden)

* ❌ Implementing naming/discovery (belongs to `svc-index` + `svc-dht`).
* ❌ Using SHA-2 addresses or treating 64 KiB as a protocol frame size (it is a storage I/O knob).
* ❌ Ambient/implicit authorization; privileged ops MUST be capability-bound.
* ❌ Unbounded queues; long-held locks across `.await`; multi-writer per connection.
* ❌ Persistent state when **amnesia=ON**.

## 6) References

* Complete crate canon & roles (svc-storage = CAS, chunking, replication, retrieval).
* Full Project Blueprint — OAP constants, BLAKE3 addressing, amnesia, pillars.
* Concurrency & Aliasing Blueprint — no await-holding-lock; owned bytes; single-writer discipline.
* Hardening Blueprint — input caps, decompression safety, UDS peer-cred.
* Scaling Blueprint — SLOs, RF/EC, repair pacing & gauges, placement policy.
* App Integration Blueprint — storage in App/SDK path and plane separation.
* **RUNBOOK.md (svc-storage)** — operational drills, pager policy, repair procedures (link when merged).

---

**Definition of Done:**
All invariants mapped to explicit tests/metrics; SLOs pinned; CI scripts enforce anti-scope greps; references point only to canon docs; this file is registered in the crate’s doc index and checked in CI.

```
