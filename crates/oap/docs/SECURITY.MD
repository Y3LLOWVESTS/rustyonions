---

title: Security Notes — oap
crate: oap
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
-------------

# Security Documentation — `oap`

This document defines the **threat model**, **security boundaries**, and **hardening requirements** specific to `oap`.
It complements the repo-wide Hardening, Interop, and Microkernel blueprints and the OAP/1 **IDB**.

> `oap` is a **pure protocol library** (framing + envelopes). It does **not** own sockets, TLS, auth, or persistence.
> Security posture therefore focuses on **input validation**, **fail-closed parsing**, and **non-interference** with host-level controls.

---

## 1) Threat Model (STRIDE)

| Category                   | Threats (examples)                                             | Relevant in `oap`? | Mitigation / Notes                                                                                                                                                                          |
| -------------------------- | -------------------------------------------------------------- | -----------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **S**poofing               | Impersonated peers, fake identities                            |     **N (in lib)** | Out of scope for `oap`. Hosts enforce peer auth (TLS/macaroon/caps). `oap` treats all inputs as untrusted.                                                                                  |
| **T**ampering              | Mutated frames, header manipulation, length forgeries          |              **Y** | Strict envelope tags; `deny_unknown_fields`-style header checks; pre-allocation **length guard** (≤ 1 MiB); typed rejects (`HeaderMalformed`, `UnknownEnvelope`, `FrameOversize`).          |
| **R**epudiation            | Missing audit, unverifiable processing                         |     **N (in lib)** | Library emits no logs; hosts must log structured events (JSON) with `corr_id` when integrating `oap`.                                                                                       |
| **I**nformation Disclosure | Leaking secrets/PII, revealing PQ flags                        |            **Low** | `oap` holds **no secrets**; advise hosts **not** to log opaque PQ flags or raw DATA.                                                                                                        |
| **D**enial of Service      | Oversize frames, pathological splits, header floods, ACK abuse |              **Y** | Hard cap **1 MiB** per frame; partial-read handling; bounded parser scratch; algebraic `ACK{up_to}` checks; no unbounded buffering; early reject on anomalies; host must bound I/O and RPS. |
| **E**levation of Privilege | Bypassing capability checks via envelopes                      |     **N (in lib)** | `oap` carries no auth; hosts must enforce capability/ACL decisions **before** consuming DATA.                                                                                               |

---

## 2) Security Boundaries

* **Inbound (to `oap`):** raw **byte slices** representing OAP/1 frames provided by hosts (after transport/TLS).
* **Outbound (from `oap`):** typed **envelopes** or errors returned to the host. No network, no filesystem, no DB from `oap`.
* **Trust Zone:** `oap` executes in the host’s process space (same tenant). Treat all input as **hostile**; never assume transport integrity.
* **Assumptions:**

  * Hosts terminate TLS (tokio-rustls), authenticate peers, and authorize operations (caps/macaroons).
  * Hosts enforce `/readyz`/quotas, rate limits, and I/O timeouts at transport/gateway layers.
  * Micronode **amnesia** may be enabled; `oap` must not introduce persistence.

---

## 3) Key & Credential Handling

* **Keys/secrets owned by hosts**, not `oap`. The library manages **no** TLS keys, macaroons, or identity material.
* **Rotation policy:** N/A in `oap`; host rotates TLS/caps ≤ 30 days and propagates effects outside the library.
* **Zeroization:** N/A (no secrets). If a host passes opaque secrets inside headers (not recommended), they must be handled **outside** `oap`.

---

## 4) Hardening Checklist (library + host integration)

> **Library-enforced (within `oap`)**

* [x] **Frame cap = 1 MiB** (reject early on declared length).
* [x] **Strict header validation** (unknown/duplicate → reject).
* [x] **No hashing** inside `oap`; DATA integrity verified at materialization boundaries by hosts.
* [x] **No blocking** calls; parsing is CPU-bounded; no sleeps/jitter.
* [x] **Cancel-safe** decode paths; partial inputs return `NeedMore`.

> **Host-enforced (required when embedding `oap`)**

* [ ] **I/O timeouts:** read/write ≤ 5s; idle ≤ 60s (tunable).
* [ ] **Concurrency cap:** bound inflight connections/workers; prefer reject over queue.
* [ ] **RPS cap / rate-limit** per peer/tenant.
* [ ] **Decompression caps** if host decompresses before feeding `oap` (ratio ≤ 10×; absolute cap enforced).
* [ ] **UDS hygiene** (if used): dir `0700`, sock `0600`, `SO_PEERCRED` allowlist.
* [ ] **Chaos**: restart under load; `/readyz` flips before collapse (host readiness).

---

## 5) Observability for Security

> `oap` does not expose metrics directly; **hosts** must surface them. Suggested names:

* **Counters:**

  * `oap_rejects_total{reason="frame_oversize|unknown_envelope|header_malformed|credit_violation|missing_obj|truncated"}`
  * `oap_frames_total{kind="HELLO|START|DATA|ACK|END|ERROR"}`
* **Histograms:**

  * `oap_read_frame_seconds`, `oap_write_frame_seconds` (host-measured around parse/encode).
* **Logs (host):** structured JSON with `service`, `reason`, `corr_id`, `peer_addr`, `tenant`. Avoid logging raw DATA or PQ capability bitmasks.
* **Health gates (host):** readiness flips on sustained reject/backpressure; fail-closed on parser construction errors.

---

## 6) Dependencies & Supply Chain

* **Security-relevant usage in `oap`:** parsing/encoding only. Typical deps: `bytes`, `serde` (for DTOs/tests), optionally a parser combinator (`winnow`/`nom`).
* **Deliberate omissions:** `oap` depends on **neither** TLS nor hashing crates; prevents scope creep.
* **Controls:**

  * Workspace-pinned versions; **`cargo-deny`** CI (licenses/bans/advisories/sources).
  * Reproducible builds where practical; SBOM generated at release (`/docs/sbom/`).
  * Fuzz targets checked into `fuzz/` with corpora (regressions preserved).

---

## 7) Formal & Destructive Validation

* **Property tests:**

  * **Length-first**: parser refuses frames with `len > 1 MiB` without allocation.
  * **Split-read equivalence**: N-chunk feed ≡ single feed (same outputs/states).
  * **ACK algebra**: monotone `ack_up_to` (no decrease/overflow).
* **Fuzzing:** header/value fuzz (dup keys, UTF-8 hazards), body length extremes (0, 1, 1 MiB, 1 MiB+1), tag mutations, `ACK` replay/overshoot.
* **Loom (dev-only):** optional models for host-style shared cursor to ensure no lock across `.await` and no double-advance under races (tests only).
* **Chaos (host):** truncated/bad frames under sustained load → deterministic rejects, not stalls; readiness flips before collapse.
* **TLA+ sketch (optional):** state machine `{Hello, Started, Streaming(seq), Ended}`; safety (no DATA before START; no DATA post-END; `ack_up_to` monotone), liveness (valid bytes progress state).

---

## 8) Security Contacts

* **Maintainer:** Stevan White
* **Security contact:** Use repository `SECURITY.md` disclosure process (confidential reporting; coordinated fix window).

---

## 9) Migration & Upgrades

* Changes that affect **validation semantics** (e.g., header strictness defaults, error codes) require:

  * **SemVer** compliance (minor for additive; **major** for breaking).
  * Updated **conformance vectors** and fuzz corpora.
  * Release notes with **security impact** (e.g., stricter rejects may surface latent client bugs).

---

## 10) Mermaid — Security Flow Diagram

```mermaid
flowchart LR
  A[Client] -->|TLS + caps (host)| H[Host Service<br/>(e.g., svc-gateway)]
  H -->|pass bytes| O[oap (framing/validation)]
  O -->|envelope or error| H
  H -->|Authorized & validated| D[Downstream service]
  H -.->|Reject unauth/oversize/DoS| E[Error + Metrics]
  style O fill:#b91c1c,stroke:#7f1d1d,color:#fff
```

**Text:** The **host** terminates TLS and enforces auth. It feeds raw bytes to **`oap`**, which validates frames strictly (1 MiB cap, header hygiene, algebraic ACK). On success, the host routes authorized envelopes downstream; on failure, the host returns an error and records security metrics.
