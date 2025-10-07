---

title: OAP/1 — Overlay Access Protocol (IDB)
version: 1.0.0
status: reviewed
last-updated: 2025-10-06
audience: contributors, ops, auditors
crate: oap
crate-type: protocol/lib
pillars: P7 (App BFF & SDK), P10 (Overlay/Transport/Discovery)
concerns: SEC, RES, PERF, DX, ECON, GOV
msrv: 1.80.0
------------

# OAP/1 — Invariant-Driven Blueprinting (IDB)

## 1) Invariants (MUST)

* **[I-1] Protocol of record:** OAP/1 is the canonical framed protocol with envelopes
  `HELLO, START, DATA, END, ACK, ERROR`. A **hard cap `max_frame = 1 MiB`** applies to any single frame.
* **[I-2] Stream chunking:** Streaming I/O uses **64 KiB chunks**; chunk size is an I/O knob and **must not be conflated** with the 1 MiB frame cap.
* **[I-3] Content addressing:** When `DATA` bytes represent a content-addressed object, the header **MUST** include `obj:"b3:<hex>"` (BLAKE3-256). Objects are verified where they are materialized (e.g., storage/overlay), **not** inside OAP framing.
* **[I-4] Neutral layering:** OAP owns only framing and envelope grammar. **No sockets, TLS, auth, or DTO schemas** live here (transport in `ron-transport`, DTOs in `ron-proto`, auth/ledger elsewhere).
* **[I-5] Error taxonomy is normative:** Implementations must emit structured, enumerable errors (e.g., `bad_hello`, `frame_oversize`, `unknown_envelope`, `missing_obj`, `obj_mismatch`, `credit_violation`, `timeout`, `truncated`).
* **[I-6] Amnesia compatibility:** With Global Amnesia Mode ON (micronode default), OAP behavior and limits are identical; OAP itself **must not** introduce disk spills or persistence.
* **[I-7] PQ-neutral forward path:** OAP/1 must not block PQ adoption. `HELLO` can advertise PQ capability flags (e.g., `pq_hybrid=true`) without altering OAP/1 envelope semantics.
* **[I-8] Observability:** Embedding services **must** expose golden metrics and readiness. All rejects increment `oap_rejects_total{reason}`, and `/readyz` flips under sustained backpressure thresholds.

---

## 2) Design Principles (SHOULD)

* **[P-1] Be boring at the edge:** Keep envelopes tiny and explicit; push policy/semantics to services (gateway, index, storage).
* **[P-2] Fail early, tell the truth:** Validate headers and lengths before allocation; reject oversize/malformed frames with precise error codes and counters.
* **[P-3] Clean layering:** DTOs/types live in `ron-proto`; timeouts, TLS, socket options live in `ron-transport`; OAP remains grammar + flow control.
* **[P-4] One contract across profiles:** Micronode and Macronode speak the same OAP/1; topology and ops differ, not the protocol.
* **[P-5] Backpressure friendliness:** Use windowed `ACK` with bounded in-flight frames; collapse gracefully rather than queuing unbounded work.
* **[P-6] Economic neutrality:** Envelopes **SHOULD NOT** embed value/economic semantics. Proofs, settlement, and incentives live in `ron-ledger`, `svc-rewarder`, or higher-level DTOs.

---

## 3) Implementation (HOW)

> The `oap` crate is a **pure protocol library**. This section describes intended mechanics for **consumers** (e.g., `svc-gateway`, `svc-overlay`) without coupling to transport or auth.

* **[C-1] Envelope grammar (reference shape + visual):**

  ```rust
  // docs-only illustrative sketch; see specs/oap-1.md for the normative wire format
  enum Envelope {
      Hello { version: u32, max_frame: u64, pq_hybrid: bool }, // widths illustrative
      Start { stream: String },                         // topic/stream identifier
      Data  { obj: Option<String>, seq: u64, len: u32 }, // followed by len bytes
      Ack   { up_to: u64 },                             // credit window
      End   { seq: u64, status: u16 },                  // normal close
      Error { code: u16, reason: String },              // structured failure
  }
  ```

  ```mermaid
  sequenceDiagram
    participant C as Client
    participant S as Service (gateway/overlay)
    C->>S: HELLO{version,max_frame,pq?}
    S-->>C: HELLO{version,max_frame,pq?}
    C->>S: START{stream}
    loop windowed transfer
      C->>S: DATA{obj?,seq,len}+<len bytes>
      S-->>C: ACK{up_to}
    end
    C->>S: END{seq,status}
    S-->>C: ACK{up_to} (final)
    note over C,S: Any violation → ERROR{code,reason}; metrics increment & readiness may flip
  ```

* **[C-2] Limits wiring:** Enforce **1 MiB** at the frame boundary (validate header lengths before allocation). Perform I/O in **64 KiB** chunks for steady memory/latency.

* **[C-3] Address binding:** Require `obj` header when bytes represent an object; compute/verify BLAKE3 only at the component that materializes bytes (e.g., storage GET/PUT).

* **[C-4] Transport budgets:** Socket lifetimes and read/write/idle timeouts are **owned by `ron-transport`**. OAP maintains sequence numbers and credit windows but not timers.

* **[C-5] Observability shape (golden set):**
  `oap_frames_total{kind}`, `oap_rejects_total{reason}`, `oap_frame_bytes`,
  `oap_inflight_frames`, `oap_ack_window`,
  `oap_read_frame_seconds` (Histogram), `oap_write_frame_seconds` (Histogram).
  All rejects increment `oap_rejects_total{reason}`; sustained reject/backpressure flips `/readyz`.
  *Optional, non-normative ECON/GOV hook:* implementations **may** add an opt-in `economic="true|false"` label when frames are processed within a ledger/rewarder context; this label must not affect protocol semantics.

* **[C-6] Error mapping:** Errors map 1:1 to an enumerable set for interop and metrics. Unknown envelopes and length mismatches are **protocol errors** (not transport errors).

* **[C-7] PQ hooks:** `HELLO` may carry opaque capability flags (e.g., `pq_hybrid=true`) which are forwarded to transport/crypto layers. OAP/1 message flow is unchanged by PQ presence.
  *Note:* `pq_hybrid` is an **example** flag; treat all HELLO PQ flags as opaque feature bits forwarded downstream.

---

## 4) Acceptance Gates (PROOF)

* **[G-1] Conformance vectors (must pass):**
  Happy-path `HELLO→START→(DATA×N)→END` with/without `obj`; oversize frame → `frame_oversize`; missing `obj` when required → `missing_obj`; `unknown_envelope` and `truncated` cases; windowed `ACK` correctness.
* **[G-2] Fuzz + property tests:** Frame parser and header codec fuzzing (corpus includes truncated frames, huge declared lengths, duplicate/unknown headers).
* **[G-3] Perf & backpressure:** Under ramped load, p95 read/write frame latency remains stable within agreed SLOs; `/readyz` flips **before** collapse when in-flight exceeds bounds.
* **[G-4] CI lint walls:**

  * Assert presence of `max_frame = 1 MiB` and `64 KiB` in spec and code comments.
  * Forbid lock-across-await in any OAP server embedding code.
  * Deny arbitrary sleeps in protocol paths.
* **[G-5] Profile parity matrix:** Same vectors pass on Micronode and Macronode; amnesia matrix confirms **no OAP-attributable disk writes** on Micronode.
* **[G-6] PQ smoke (expanded):** `HELLO` negotiation with `pq_hybrid=true` succeeds; DATA/ACK semantics remain byte-identical; include a canonical vector with PQ flag set.
* **[G-7] Polyglot round-trip:** Rust ↔ TypeScript ↔ Python codecs produce **byte-identical** frames for canonical vectors (CI matrix job).
* **[G-8] Red-team hardening vectors:**
  Compression-bomb ratio ≤ **10×**, malformed length fields, duplicate `seq`, replayed `END`, and `ACK` credit abuse—all must reject deterministically and increment `oap_rejects_total{reason}`.

---

## 5) Anti-Scope (Forbidden)

* **[F-1] No kernel creep:** No supervision, bus, health, or config logic in OAP.
* **[F-2] No DHT/overlay semantics:** Discovery, sessions, and gossip live in overlay/DHT layers, not in envelopes.
* **[F-3] No hashing of frames:** Content verification is for storage/overlay; OAP frames themselves are not hashed.
* **[F-4] No ambient authority:** Auth/capabilities are enforced by gateway/registry/passport—not by OAP.
* **[F-5] No divergence of constants:** The 1 MiB frame cap is protocol-fixed; streaming chunk is 64 KiB by default for I/O tuning but must not redefine the cap.
* **[F-6] No app facets baked in:** No Feed/Graph/Search/Media semantics in envelopes; those belong to higher-level services and DTOs.

---

## 6) References

**Core**

* **FULL_PROJECT_BLUEPRINT.MD** — Data-plane and protocol seams; readiness/health conventions.
* **MICROKERNEL_BLUEPRINT.MD** — Layering rules and kernel boundaries.
* **INTEROP_BLUEPRINT.MD** — Normative constants, vectors, and DTO hygiene (`deny_unknown_fields`).
* **SCALING_BLUEPRINT.MD** — Backpressure, boundedness, and collapse-graceful patterns.

**Supporting**

* **HARDENING_BLUEPRINT.MD** — Fuzz/property/soak patterns; red-team vectors and compression-ratio limits.
* **12_PILLARS.MD** — Pillars P7/P10 responsibilities and facet boundaries.
* **SIX_CONCERNS.MD** — SEC/RES/PERF/DX (plus ECON/GOV pointers to ledger/rewarder).
* **APP_INTEGRATION_BLUEPRINT.MD** — Gateway embedding, metrics surface, and readiness semantics.

---

### Reviewer Checklist (paste into PR)

* [ ] Envelopes & constants match canon (HELLO/START/DATA/END/ACK/ERROR; **1 MiB** frame cap; **64 KiB** streaming chunks).
* [ ] `DATA` uses `obj:"b3:<hex>"` when bytes represent an object; verification occurs at materialization boundaries.
* [ ] DTOs isolated in `ron-proto`; sockets/TLS/timeouts in `ron-transport`; OAP code remains transport/auth neutral.
* [ ] Golden metrics exposed; all rejects increment `oap_rejects_total{reason}`; `/readyz` flips under sustained backpressure.
* [ ] Fuzz/property/perf suites present; red-team vectors implemented; profile parity & amnesia matrix green.
* [ ] PQ smoke vector (`pq_hybrid=true`) passes; polyglot round-trip (Rust/TS/Python) yields byte-identical frames.
