# RustyOnions Microkernel Optimization Blueprint (Aligned, Drift-Proof)

> Perfect—sleep now, ship tomorrow. This is the **single source of truth** for the microkernel plan. It locks the invariants, preserves the tiny-kernel boundary, and sequences a one-day execution plan with guardrails, checklists, and acceptance criteria.

## 0) Invariants (Must Appear in Every Related Doc)

- **Addressing (normative):** `b3:<hex>` using **BLAKE3-256** over the plaintext object (or manifest root). Truncated prefixes are allowed for routing; services **MUST** verify the full digest before returning bytes. OAP/1 frames **MUST NOT** be hashed; include `obj:"b3:<hex>"` in DATA headers for object IDs.
- **OAP/1 default:** `max_frame = 1 MiB` (per **GMI-1.6**). Note: **64 KiB is a storage streaming chunk size** (implementation detail) and must **not** be conflated with the OAP/1 `max_frame`.
- **Kernel public API (frozen):**
  `Bus`, `KernelEvent::{ Health {service, ok}, ConfigUpdated {version}, ServiceCrashed {service, reason}, Shutdown }`,
  `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
- **Kernel contracts (don’t break):**
  - **Bus is monomorphic** (no generics).
  - **TLS types** use `tokio_rustls::rustls::ServerConfig` (not `rustls::ServerConfig`).
  - Metrics present: `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds` (plus service-layer `rejected_total{reason=...}`).
- **Normative spec pointer:** OAP/1 is defined by **GMI-1.6**. Any `/docs/specs/OAP-1.md` in this repo is a **mirror stub that links to GMI-1.6** to prevent drift.
- **Perfection Gates ↔ Milestones:** Final Blueprint maps A–O gates to **M1/M2/M3** in the Omnigate Build Plan.
- **PQ crypto readiness:** OAP/2 may add **hybrid X25519+Kyber** key exchange (feature-gated in `oap`). This blueprint does **not** change OAP/1.

---

## 1) Executive Summary (What We’re Doing)

- **Define and lock OAP/1** as the tiny, framed service protocol for the services plane (spec = GMI-1.6; `max_frame = 1 MiB`; storage chunk 64 KiB separate).
- **Upgrade the kernel bus** to a robust broadcast channel with topic-style filtering helpers.
- **Add comprehensive tests** (bus basic/topic/load; OAP HELLO/START/DATA/END/ACK + error; chaos tests).
- **Create thin, non-kernel crates**:
  - `oap` (frame codec + DATA packing helpers),
  - `gateway` (entrypoint/tenancy/quota scaffolding),
  - `sdk` (client ergonomics),
  - `ledger` (token economics hook; optional accounting adapter).
- **Provide runnable demos** (in-memory and TCP) proving end-to-end OAP/1 without touching the kernel’s public surface.
- **Future-proof** with chaos testing, TLA+ models, and PQ crypto planning.

**Outcome**: Minimal kernel, clear protocol, strong orchestration, ready for private nodes and token credits.

---

## 2) Guiding Principles

- **Kernel stays tiny:** Transport + supervision + metrics + bus only.
- **No app semantics in kernel:** OAP/1, Gateway, SDK, Ledger live in separate crates.
- **No new mandatory deps in kernel:** Topic filtering via helper function, not external stream/filter crates.
- **Everything is testable:** Fast, hermetic tests with `tokio::io::duplex` and small-capacity bus for lag behavior.
- **Security-first:** APP_E2E and DoS protections baked in.

---

## 3) Current Gaps → Target Outcomes

| Area             | Current              | Target                                                         |
| ---------------- | -------------------- | -------------------------------------------------------------- |
| Service protocol | Undefined / implicit | **OAP/1** framed spec + minimal `oap` codec (GMI-1.6)          |
| Event bus        | Simple               | **Robust broadcast** + `recv_matching` helper (topic-style)    |
| Tests            | Partial              | Bus basic/topic/load + OAP happy path + error + chaos cases    |
| Gateway          | None                 | Minimal handler for HELLO/START → emits events, stubs quotas   |
| SDK              | None                 | HELLO/START + `put_doc` (real bytes) + `subscribe`             |
| Economics        | None                 | **Ledger** with `UsageRead` trait, optional accounting adapter |
| Demos            | None                 | In-memory + TCP OAP/1 round-trip demos                         |

---

## 4) Architecture Sketch (Text)

- **ron-kernel (microkernel)**: `transport.rs`, `supervisor/*`, `metrics.rs`, **`bus.rs`** (broadcast + helpers)
- **oap (new crate)**: `OapFrame` + envelopes + **DATA packing** (header+body, `obj:"b3:<hex>"` in header)
- **gateway (new crate)**: Reads OAP frames, emits `KernelEvent`s, future spot for quotas
- **sdk (new crate)**: HELLO/START/`put_doc`/`subscribe`, transport-agnostic
- **ledger (new crate)**: Credit computation; optional feature to adapt kernel `accounting` later
- **demos/tests**: Prove all of the above in-memory and via TCP without changing kernel’s public API

---

## 5) Implementation Plan (Time-Boxed)

### Phase A (1–2 hours): Workspace + Crate Scaffolds

1. Add workspace members: `crates/oap`, `crates/gateway`, `crates/sdk`, `crates/ledger`.
2. Create crates with minimal `Cargo.toml` and `lib.rs`.
3. **No kernel changes yet.**

**Acceptance:** `cargo check` succeeds for all crates.

### Phase B (1–2 hours): OAP/1 Codec + Tests

1. Implement `OapFrame { ver, typ, payload }` and enums (HELLO, START, DATA, ACK, END, ERROR).
2. **DATA packing** helpers:
   - Payload layout: `[u16 header_len][header JSON bytes][raw body bytes]`
   - Header includes `obj:"b3:<hex>"` for object ID (BLAKE3-256).
   - `encode_data_payload`, `decode_data_payload`, `data_frame(...)`
3. Tests with `tokio::io::duplex`:
   - `hello_roundtrip`
   - `start_data_end_with_body` (real bytes, `obj:"b3:<hex>"`)
   - `ack_roundtrip`
   - `invalid_type_errors`
   - `quota_error_frame`

**Acceptance:** `cargo test -p oap` green.

### Phase C (45–60 min): Bus Upgrade in Kernel + Tests

1. Replace `crates/ron-kernel/src/bus.rs` with broadcast-backed bus:
   - `Bus::new`, `subscribe`, `publish`, `publish_lossy` (monomorphic, no generics)
   - **Helper module `sub`**: `recv_with_timeout`, `try_recv_now`, `recv_matching(predicate, timeout)`
2. Tests (`ron-kernel/tests`):
   - `bus.rs` basic pub/sub
   - `bus_topic.rs` topic-style filtering using `recv_matching`
   - `bus_load.rs` small capacity → lag handling

**Acceptance:** `cargo test -p ron-kernel --tests` green.

### Phase D (60–90 min): Gateway + SDK + Demos

1. **gateway**: `Gateway::handle_conn(Read, Write)`
   - On `HELLO` → echo
   - On `START` → publish a kernel event and send `ACK` with credit
   - Keep quotas/tenancy as TODOs (documented)
2. **sdk**:
   - `hello(app_proto_id)`
   - `start(req_id, method)`
   - `put_doc(collection,id,bytes)` → `START` + `DATA (header+body, obj:"b3:<hex>")` + client `END`
   - `subscribe(topic)` → `START` only (server semantics TBD)
   - `into_io()` for reading frames in demos
3. **demos**:
   - `gateway/examples/demo.rs`: in-memory `duplex` proof (HELLO→START→ACK)
   - `gateway/examples/tcp_demo.rs`: TCP proof

**Acceptance:**
- `cargo run -p gateway --example demo` runs to completion, prints a health event.
- `cargo run -p gateway --example tcp_demo` runs to completion, receives `ACK`.

### Phase E (30–45 min): Ledger + Optional Accounting Adapter

1. **ledger**:
   - `UsageRead` trait (decoupled from kernel)
   - `Ledger::settle_with(usage, tenant_id)` using weights `alpha_out_per_byte`, `beta_in_per_byte`
2. **Feature-gated adapter** (`ledger` feature `accounting-adapter`):
   - Implement `UsageRead` for `ron-kernel::accounting::Counters`
   - `Ledger::integrate_accounting(&Counters, tenant_id)`

**Acceptance:**
- `cargo build -p ledger` (without adapter) green
- `cargo build -p ledger --features accounting-adapter` green (if `Counters` exists)

---

## 6) Public API Freeze (Kernel)

The kernel **must** re-export exactly:

```rust
pub use {
  Bus,
  KernelEvent,
  Metrics,
  HealthState,
  Config,
  wait_for_ctrl_c,
};
```

`KernelEvent` variant set:

```rust
enum KernelEvent {
  Health        { service: String, ok: bool },
  ConfigUpdated { version: u64 },
  ServiceCrashed{ service: String, reason: String },
  Shutdown,
}
```

Breaking this API requires a **major version bump** and migration notes.

---

## 7) Docs & Files to Touch/Create

**Kernel**
- Replace: `crates/ron-kernel/src/bus.rs`
- Add tests: `crates/ron-kernel/tests/bus.rs`, `bus_topic.rs`, `bus_load.rs`

**OAP Crate**
- Add: `crates/oap/Cargo.toml`, `crates/oap/src/lib.rs`
- Add tests: `crates/oap/tests/roundtrip.rs`, `quota_error.rs`

**Gateway Crate**
- Add: `crates/gateway/Cargo.toml`, `crates/gateway/src/lib.rs`
- Add demos: `crates/gateway/examples/demo.rs`, `tcp_demo.rs`

**SDK Crate**
- Add: `crates/sdk/Cargo.toml`, `crates/sdk/src/lib.rs`

**Ledger Crate**
- Add: `crates/ledger/Cargo.toml`, `crates/ledger/src/lib.rs`
- Add (optional): `crates/ledger/src/accounting.rs` (feature-gated)

**Docs**
- Add: `docs/specs/OAP-1.md` **stub** linking to **GMI-1.6**
- Add: `docs/security.md` (PQ crypto transition plan)

**Testing**
- Add: `testing/chaos/Cargo.toml`, `testing/chaos/src/lib.rs` (Jepsen-style harness)
- Add: `specs/bus.tla`, `specs/oap-impl.tla` (TLA+ models)

---

## 8) Commands (Run in Order)

```
cargo check
cargo test -p oap
cargo test -p ron-kernel --tests
cargo run -p gateway --example demo
cargo run -p gateway --example tcp_demo
cargo build -p ledger
cargo build -p ledger --features accounting-adapter
cargo run -p chaos --test bus_drop
cargo run -p chaos --test oap_frame_drop
tlc specs/bus.tla
tlc specs/oap-impl.tla
```

---

## 9) Acceptance Criteria (Definition of Done)

- **Protocol**
  - `OAP-1.md` stub present; real spec = GMI-1.6.
  - `oap` tests pass, including DATA with `obj:"b3:<hex>"` and error case.
- **Bus**
  - Kernel builds with new `bus.rs` (monomorphic).
  - Three bus tests pass (basic/topic/load).
  - No new kernel-wide dependencies introduced.
- **Gateway/SDK**
  - In-memory and TCP demos succeed (ACK observed).
  - SDK supports `hello`, `start`, `put_doc` (bytes, `obj:"b3:<hex>"`), `subscribe`, `into_io`.
- **Ledger**
  - Compiles standalone; optional accounting adapter does not break builds.
- **Security**
  - APP_E2E enforced; frames with `payload.len() > max_frame` rejected.
- **Observability**
  - Metrics emitted: `bus_lagged_total`, `service_restarts_total`, `request_latency_seconds`, `rejected_total{reason=...}`.
- **No kernel API breakage**
  - Existing kernel consumers compile untouched.

---

## 10) Test Matrix (Quick)

- **OAP frames:** HELLO, START, DATA(header+body, `obj:"b3:<hex>"`), END, ACK, ERROR(invalid type), ERROR(quota)
- **Bus:** pub/sub happy path; topic filtering only matches intended events; load/lag doesn’t panic
- **Demos:** in-memory duplex + TCP show HELLO→START→ACK
- **SDK:** `put_doc` with `Vec<u8>` payload and `obj:"b3:<hex>"`; `subscribe` sends a START; `into_io` used in demos
- **Chaos:** Bus message loss and OAP frame drops handled gracefully

---

## 11) Performance & Defaults

- **OAP/1 default `max_frame = 1 MiB`** (negotiable via HELLO per GMI-1.6).
- **Storage streaming chunk = 64 KiB** (implementation detail).
- **max_inflight**: 32 (HELLO-negotiated; production may use 64).
- **ACK credit**: Start at 64 KiB; instrument later.
- **Bus channel capacity**: Default per-subscriber buffer ≥ 8; tune to 64 for active nodes.

---

## 12) Security & Resilience Notes

- **APP_E2E flag:** Treat DATA as opaque; services never decrypt in Gateway.
- **Guardrails:**
  - Reject frames where `payload.len() > negotiated max_frame`.
  - When compression is added, bound decompressed size (≤ 8× `max_frame`) or 413.
  - Always validate `VER == 0x1` and a known message type.
- **DoS:**
  - Per-connection `max_inflight` and timely `ACK` gating.
  - Gateway remains the choke-point for quotas; kernel stays unburdened.
- **PQ Crypto:** OAP/2 may add hybrid X25519+Kyber (feature-gated in `oap`, planned in `docs/security.md`).

---

## 13) Telemetry & Observability

- Emit `KernelEvent::Health { service, ok }` on successful HELLO.
- Emit `KernelEvent::ServiceCrashed { service, reason }` with structured logs.
- Metrics to populate:
  - `bus_lagged_total` (kernel)
  - `service_restarts_total` (kernel)
  - `request_latency_seconds` (kernel)
  - `rejected_total{reason=...}` (service-layer, tied to `ServiceCrashed`)
  - `ron_request_latency_seconds` (request timing)
- TODO hooks in Gateway for:
  - Bytes in/out per tenant
  - Current inflight per connection
  - Errors by code (unauthorized/quota/bad_frame)

---

## 14) Risks & Mitigations

- **Risk:** API drift
  **Mitigation:** Replace only `bus.rs` internals; keep signatures intact; compile kernel tests first.
- **Risk:** Over-coupling economics early
  **Mitigation:** Feature-gate accounting adapter; keep `UsageRead` generic.
- **Risk:** DATA framing mismatch between SDK and services
  **Mitigation:** Single canonical DATA helper in `oap` with `obj:"b3:<hex>"`; both sides use it.

---

## 15) Branching & Commits (Suggested)

- Branch: `feat/oap-bus-gateway-sdk`
- Commits (small, reviewable):
  1. Workspace + crate stubs
  2. OAP codec + tests (`obj:"b3:<hex>"`)
  3. Kernel bus + tests (monomorphic)
  4. Gateway + SDK + demos
  5. Ledger + optional adapter
  6. Docs/specs OAP-1 stub + security.md
  7. Chaos testing harness
  8. TLA+ models
  9. Tidy + CI

---

## 16) Sanity Checks (Run After Merge)

**Docs grep (ensure no SHA-256 remnants):**
```
rg -n "sha-?256|sha256:" docs/ *.md crates/ -S
```

**Kernel API freeze (must include `reason`):**
```
rg -n "ServiceCrashed\s*\{\s*service\s*,\s*reason\s*\}" crates/ron-kernel docs -S
```

**OAP/1 default vs streaming chunk (no misuse of 64 KiB as max_frame):**
```
rg -n "max_frame\s*=\s*64\s*Ki?B" -S
```

**Admin endpoints quick smoke:**
```
curl -sS http://127.0.0.1:9096/healthz
curl -sS http://127.0.0.1:9096/readyz
curl -sS http://127.0.0.1:9096/metrics | sed -n '1,80p'
```

---

## 17) Post-Merge Next Steps (Not Tomorrow)

- Add tenancy/quota checks to Gateway (connect real `accounting`).
- Implement subscription semantics in a Mailbox service using bus topic filtering.
- Add Tor example with `tor_socks5` and `tor_ctrl` configs (per Scaling v1.3.1).
- Start “private-lite” config template and SDK guide.
- Implement ZK hook in `ledger` for private usage reporting.
- Run performance simulation for bus/OAP throughput in `testing/performance`.

---

**End of blueprint.**