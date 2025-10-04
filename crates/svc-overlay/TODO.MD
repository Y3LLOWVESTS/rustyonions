

---

# `crates/svc-overlay/` — Final Scaffold (with fixes)

```
crates/svc-overlay/
├─ Cargo.toml
├─ README.md
├─ CHANGELOG.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ render-mermaid.yml
│     ├─ concurrency-guardrails.yml
│     └─ contract-apis.yml
├─ docs/
│  ├─ ALL_DOCS.md
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  ├─ state.mmd
│  ├─ IDB.md
│  ├─ specs/
│  │  └─ OAP-1.md
│  └─ api-history/
│     └─ svc-overlay/
│        ├─ v0.1.0-metrics.txt
│        ├─ v0.1.0-cli.txt
│        ├─ v0.1.0-http.json
│        └─ v0.1.0-libapi.txt
├─ benches/
│  ├─ oap_codec.rs
│  └─ handshake.rs
├─ fuzz/
│  ├─ fuzz_targets/
│  │  ├─ oap_frame_parse.rs
│  │  └─ gossip_lane.rs
│  └─ corpus/
│     ├─ valid_frame
│     ├─ oversize_frame
│     └─ split_frame
├─ examples/
│  ├─ libapi_embed.rs
│  └─ pq_embed.rs
├─ tests/
│  ├─ http_contract.rs
│  ├─ metrics_schema.rs
│  ├─ readiness_under_pressure.rs
│  ├─ interop_vectors.rs
│  ├─ pq_negotiation.rs
│  ├─ integration/
│  │  ├─ overlay_admin_roundtrip.rs
│  │  ├─ oap_session_handshake.rs
│  │  └─ overlay_oap_streaming.rs
│  └─ loom/
│     └─ loom_overlay.rs
├─ .devcontainer/
│  └─ devcontainer.json
└─ src/
   ├─ main.rs
   ├─ lib.rs
   ├─ api/
   │  └─ mod.rs
   ├─ cli.rs
   ├─ config.rs
   ├─ bootstrap.rs
   ├─ observe.rs
   ├─ admin/
   │  ├─ mod.rs
   │  ├─ health.rs
   │  ├─ ready.rs
   │  ├─ metrics.rs
   │  └─ version.rs
   ├─ supervisor.rs
   ├─ listener.rs
   ├─ shutdown.rs
   ├─ readiness/
   │  ├─ mod.rs
   │  └─ sampler.rs
   ├─ protocol/
   │  ├─ mod.rs
   │  ├─ oap.rs
   │  ├─ flags.rs
   │  ├─ handshake.rs
   │  ├─ cbor.rs
   │  └─ error.rs
   ├─ transport/
   │  ├─ mod.rs
   │  ├─ tls.rs
   │  ├─ quic.rs
   │  └─ tor.rs
   ├─ conn/
   │  ├─ mod.rs
   │  ├─ supervisor.rs
   │  ├─ reader.rs
   │  ├─ writer.rs
   │  └─ error.rs
   ├─ gossip/
   │  ├─ mod.rs
   │  ├─ engine.rs
   │  └─ types.rs
   ├─ auth/
   │  ├─ mod.rs
   │  └─ macaroon.rs
   ├─ pq/
   │  ├─ mod.rs
   │  └─ negotiate.rs
   ├─ limits.rs
   ├─ errors.rs
   └─ types.rs
```

---

## What each file/directory is for

### Top-level

* **Cargo.toml** — Crate metadata and features (`arti`, `quic`, `tls`, `pq`, `libapi`, `cli`). **Note at top:** deps are inherited from `[workspace.dependencies]` to avoid version drift; crate adds only truly local features.
* **README.md** — The crate’s “living contract” (overview, SLOs, invariants, diagrams, proof gates). Mirrors your final README.
* **CHANGELOG.md** — SemVer history; record any changes to CLI flags, HTTP/metrics shape, or `libapi` surface.
* **LICENSE-*** — Dual licensing (MIT/Apache-2.0).
* **.gitignore** — Standard Rust/benches/target/coverage ignores.

### GitHub Workflows (split for clarity)

* **ci.yml** — Full matrix: build, fmt, clippy (`-D warnings`), unit + doc tests, **coverage floor ≥ 85%**, **cargo-audit**, and `cargo deny check` (re-uses root `deny.toml`).
* **render-mermaid.yml** — Renders `.mmd → .svg` on PR (diagrams-as-code).
* **concurrency-guardrails.yml** — Loom interleavings + sanitizers; enforces “no `await_holding_lock`/`unwrap`” lint wall.
* **contract-apis.yml** — **Public API** diff (with `cargo public-api`), metrics/HTTP schema golden diff vs `docs/api-history/`.

### docs/

* **ALL_DOCS.md** — Your combined per-crate docs (Security, Interop, Quantum, Perf, Runbook, etc.)—single source of truth.
* **arch.mmd / sequence.mmd / state.mmd** — Mermaid sources (architecture, common path, lifecycle). PRs that change topology must update these.
* **IDB.md** — Crate-local Invariant-Driven Blueprint (I-/P-/G-/F- blocks) mirroring the README’s Section 2—kept as a separate, greppable reference.
* **specs/OAP-1.md** — Local OAP/1 reference + vectors used by tests; avoids protocol drift.
* **api-history/svc-overlay/** — Golden snapshots for **metrics names/labels**, **HTTP responses**, **CLI flags**, **lib public API**. CI diffs PR output against these to catch accidental breaking changes.

### benches/

* **oap_codec.rs** — Microbench for frame encode/decode at 1 KiB / 64 KiB / 1 MiB (ensures we don’t regress hot loops).
* **handshake.rs** — Control-plane handshake latency; measures PQ-hybrid delta vs classic to keep SLO budgets honest.

### fuzz/

* **fuzz_targets/oap_frame_parse.rs** — Fuzzer target for OAP envelope parsing (sizes, splits, caps).
* **fuzz_targets/gossip_lane.rs** — Exercises lane scheduler/backpressure with adversarial inputs.
* **corpus/** — **Seeds** to accelerate coverage:

  * `valid_frame` — minimal valid OAP frame.
  * `oversize_frame` — >1 MiB to assert hard cap behavior.
  * `split_frame` — fragmented frame to stress reassembly.

### examples/

* **libapi_embed.rs** — Minimal embedding of the service (feature `libapi`) to prove a tiny, stable public surface.
* **pq_embed.rs** — Shows enabling **hybrid PQ** posture through `ron-transport` flags/feature; documents expected negotiation results.

### tests/

* **http_contract.rs** — `/healthz`/`/readyz` truthiness and `/metrics` exposability (shape, types).
* **metrics_schema.rs** — Emits sample scrape; asserts names/labels match `api-history`.
* **readiness_under_pressure.rs** — Saturation drives **early degrade** (writes fail first), then recovery observed.
* **interop_vectors.rs** — Golden vectors for HELLO/ACK, oversize/ratio rejection; OAP invariants enforced.
* **pq_negotiation.rs** — Matrix for off↔off, hybrid↔hybrid, hybrid↔off with explicit refusal where policy requires PQ-only.
* **integration/** — Black-box E2E checks:

  * `overlay_admin_roundtrip.rs` — Spins admin plane and validates endpoints end-to-end.
  * `oap_session_handshake.rs` — Drives session establishment over the real stack.
  * `overlay_oap_streaming.rs` — Happy-path streaming with bounded backpressure and latency assertions.
* **loom/loom_overlay.rs** — Concurrency interleavings for single-writer invariant, bounded queues, and orderly shutdown (no locks across `.await`).

### .devcontainer/

* **devcontainer.json** — Repro dev environment (Rust 1.80, rust-analyzer, cargo tools) to get contributors productive instantly.

### src/ (small, single-purpose modules)

* **main.rs** — Binary entry: parse CLI → config → tracing/metrics → spawn supervisor → await shutdown. *Tiny wire-up only.*
* **lib.rs** — (feature `libapi`) re-exports the small **embed API** (spawn/handle/config). Keeps public surface microscopic.

#### api/

* **mod.rs** — `OverlayConfig` (validated snapshot), `OverlayHandle` (shutdown/join), `spawn(cfg)`; stable public surface for in-process use.

#### bring-up + observability

* **cli.rs** — CLI flags → config overrides. Names must match `docs/api-history/...-cli.txt`.
* **config.rs** — Typed config, defaults, env mapping (`__`), validation of caps/timeouts/PQ posture.
* **bootstrap.rs** — Tracing & Prometheus exporter setup; central histogram buckets so call-sites stay thin.
* **observe.rs** — **Metric registry helpers + names/labels** defined once (no ad-hoc metrics elsewhere).

#### admin plane (strictly separate)

* **admin/mod.rs** — Router builder; composes handlers.
* **admin/health.rs** — Liveness (fast, dep-light).
* **admin/ready.rs** — Readiness aggregator (listeners bound, PQ posture, queue pressure, FD headroom).
* **admin/metrics.rs** — `/metrics` exposition endpoint.
* **admin/version.rs** — Build/version surface.

#### supervision + sockets

* **supervisor.rs** — Orchestrates sub-services (listener, gossip pool, admin, config watcher); crash-only restart with jitter.
* **listener.rs** — Accept loop; hands streams to per-connection supervisor; **no protocol logic here**.
* **shutdown.rs** — Cancellation tokens & helpers; orderly shutdown patterns.

#### readiness aggregation

* **readiness/mod.rs** — Readiness state machine; exposes gauges/counters.
* **readiness/sampler.rs** — Queue-depth sampling window & thresholds (drives degrade/recover).

#### protocol (OAP/1 etc.)

* **protocol/mod.rs** — Wire building blocks; isolates framing from transports.
* **protocol/oap.rs** — Frame codec with **1 MiB hard cap** and safe size/ratio checks; no I/O here.
* **protocol/flags.rs** — REQ/RESP/EVENT/COMP/ACKREQ bits + helpers.
* **protocol/handshake.rs** — HELLO/ACK negotiation; version/features table; error mapping.
* **protocol/cbor.rs** — Deterministic DAG-CBOR codecs; keeps determinism rules centralized.
* **protocol/error.rs** — Protocol-local error taxonomy (FrameTooLarge, BadVersion, RatioCap, …).

#### transport adapters (feature-gated)

* **transport/mod.rs** — Abstraction so callers don’t care if it’s TCP/TLS/QUIC/Tor.
* **transport/tls.rs** — TLS 1.3 via rustls; ALPN; explicit timeouts; no session tickets unless enabled.
* **transport/quic.rs** — QUIC (optional); single bidi stream; anti-amplification; 0-RTT disabled by default.
* **transport/tor.rs** — Tor/Arti path (optional); bootstrap readiness; IP distrust notes.

#### per-connection lifecycle

* **conn/mod.rs** — Connection types and builders; wires reader/writer tasks.
* **conn/supervisor.rs** — Owns connection state; backpressure; coordinates shutdown.
* **conn/reader.rs** — **Read-only**: bytes → OAP frames → bounded ingress queue. Never writes.
* **conn/writer.rs** — **Single owner** of writes: bounded egress → timeouts → metrics.
* **conn/error.rs** — Connection error kinds (`Busy`, `Timeout`, `Canceled`, …) for clean handling.

#### gossip plane

* **gossip/mod.rs** — Facade used by the rest of the service (`broadcast`, subscriptions).
* **gossip/engine.rs** — Worker pool; fair-shed first; all queues bounded; fanout controls (α/β).
* **gossip/types.rs** — Tiny DTOs for gossip envelopes/topics; **no app semantics**.

#### security & PQ

* **auth/mod.rs** — Capability-token hooks; keeps crypto away from hot loops.
* **auth/macaroon.rs** — Macaroon verification adapter; safe logging (fingerprints only).
* **pq/mod.rs** — PQ posture types/metrics; “off/hybrid/pq-only” status visible to readiness.
* **pq/negotiate.rs** — Handshake helpers to detect downgrades; metrics for posture selection.

#### cross-cutting

* **limits.rs** — Centralized caps (1 MiB frame, safe decompression ratio guard), reject reasons.
* **errors.rs** — Crate-level error unification + mapping to protocol/HTTP.
* **types.rs** — Shared newtypes/aliases (peer IDs, counters) to avoid stringly-typed code.

---

## Final notes

* **No duplication of security policy files** here: CI uses the **root** `deny.toml`/license/sources policy; `cargo audit` runs in this crate’s `ci.yml`.
* **Coverage gate** is enforced here (≥85%) to catch regressions where they happen.
* **Everything is small and split**; if any file grows beyond ~300–400 LOC later, split it again (e.g., `gossip/scheduler.rs`, `protocol/range.rs`).

