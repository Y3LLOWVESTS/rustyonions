
---

# 📦 `crates/micronode/` — scaffold (PQ-ready + chaos/fuzz + specs)

```
crates/micronode/
├─ Cargo.toml
├─ README.md
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ .gitignore
├─ .clippy.toml
├─ .rustfmt.toml
├─ deny.toml                          # (NEW) crate-local override; prefer repo-root cargo-deny but keep local bans if needed
├─ configs/
│  ├─ micronode.toml
│  ├─ micronode.dev.toml
│  ├─ micronode.amnesia.off.toml      # (NEW) explicit persistence ON profile for amnesia matrix tests
│  ├─ micronode.pq.toml
│  └─ micronode.pq.required.toml
├─ docs/
│  ├─ ALL_DOCS.md
│  ├─ API.md
│  ├─ CONFIG.md
│  ├─ CONCURRENCY.md
│  ├─ GOVERNANCE.md
│  ├─ INTEROP.md
│  ├─ PERFORMANCE.md
│  ├─ QUANTUM.md
│  ├─ RUNBOOK.md
│  ├─ SECURITY.md
│  ├─ IDB.md
│  ├─ api-history/
│  │  ├─ micronode.http.json
│  │  ├─ metrics.scrape.txt
│  │  └─ pq-matrix.json
│  └─ diagrams/
│     ├─ micronode_arch.mmd
│     └─ micronode_arch.svg
├─ specs/                              # (NEW) local protocol/test-vector references for dev & CI
│  ├─ oap-1.md                         # OAP/1 synopsis (frames, 1 MiB/64 KiB/≤10×, error taxonomy)
│  ├─ oap_vectors.json                 # canonical good/bad frames (used by fuzz/property tests)
│  └─ pq_handshake_cases.json          # hybrid/required posture cases, downgrade/refusal vectors
├─ scripts/
│  ├─ run_dev.sh
│  ├─ fs_spy_amnesia.sh
│  ├─ gen_diagrams.sh
│  ├─ smoke_oap_limits.sh
│  ├─ chaos_degrade_shed.sh
│  └─ pq_matrix_ci.sh
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ app.rs
│  ├─ state.rs
│  ├─ limits.rs
│  ├─ errors.rs
│  ├─ types.rs
│  ├─ cli/
│  │  ├─ mod.rs
│  │  ├─ args.rs
│  │  └─ run.rs
│  ├─ config/
│  │  ├─ mod.rs
│  │  ├─ schema.rs
│  │  ├─ load.rs
│  │  ├─ env_overlay.rs
│  │  ├─ cli_overlay.rs
│  │  ├─ validate.rs
│  │  └─ hot_reload.rs
│  ├─ observability/
│  │  ├─ mod.rs
│  │  ├─ metrics.rs
│  │  ├─ health.rs
│  │  ├─ ready.rs
│  │  ├─ version.rs
│  │  └─ logging.rs
│  ├─ security/
│  │  ├─ mod.rs
│  │  ├─ auth_macaroon.rs
│  │  ├─ tls_rustls.rs
│  │  ├─ amnesia.rs
│  │  ├─ pq_toggle.rs
│  │  ├─ pq_config.rs
│  │  └─ pq_observe.rs
│  ├─ adapters/
│  │  ├─ mod.rs
│  │  ├─ index_client.rs
│  │  ├─ mailbox_client.rs
│  │  ├─ storage_client.rs
│  │  ├─ overlay_client.rs
│  │  └─ policy_client.rs
│  ├─ facets/
│  │  ├─ mod.rs
│  │  ├─ graph.rs
│  │  ├─ search.rs
│  │  ├─ feed.rs
│  │  └─ media.rs
│  ├─ concurrency/
│  │  ├─ mod.rs
│  │  ├─ backpressure.rs
│  │  ├─ shutdown.rs
│  │  └─ registry.rs
│  └─ http/
│     ├─ mod.rs
│     ├─ admin.rs
│     └─ routes.rs
├─ tests/
│  ├─ admin_parity.rs
│  ├─ amnesia_proof.rs
│  ├─ oap_limits.rs
│  ├─ backpressure.rs
│  ├─ facets_proxy.rs
│  ├─ pq_modes.rs
│  └─ pq_fallback.rs
├─ tests_property/
│  ├─ oap_fuzz.rs
│  └─ pq_handshake_props.rs
├─ tests_chaos/
│  └─ degrade_shed.rs
├─ fuzz/
│  ├─ config_from_env_fuzz.rs
│  └─ pq_kex_fuzz.rs                 # (NEW) fuzz hybrid handshakes (adapter-driven)
├─ tests_loom/
│  └─ shutdown_interleavings.rs
├─ benches/
│  ├─ oap_frame_perf.rs
│  ├─ readiness_walk.rs
│  └─ pq_overhead.rs
└─ examples/
   └─ quickstart.rs
```

---

## 🧭 Roles (tight, file-by-file)

* **deny.toml (NEW)** — Local cargo-deny policy (bans/advisories/sources). Prefer a single repo-root `deny.toml`; keep this only if micronode needs stricter per-crate rules.
* **configs/micronode.amnesia.off.toml (NEW)** — Persistence-enabled profile to exercise amnesia matrix (OFF/ON) in CI.
* **specs/ (NEW)** — Local, versioned protocol vectors for tests:

  * `oap-1.md` — OAP/1 behavior & limits summary (1 MiB frame, 64 KiB chunk, ≤10× decompress).
  * `oap_vectors.json` — Valid/invalid frames for `oap_fuzz.rs` and `oap_limits.rs`.
  * `pq_handshake_cases.json` — OFF/HYBRID/REQUIRED handshake paths (success, fallback, refusal).
* **fuzz/pq\_kex\_fuzz.rs (NEW)** — Fuzz hybrid KEX paths (e.g., classical↔PQ adapter boundaries) to catch downgrade/timeout edge cases.

Awesome—here’s your **Micronode Scaffold Field-Guide**: a tight, “what/why/how/when” reference you can glance at while building. No code—just roles, boundaries, and acceptance checks so the team ships small, clean modules.

---

# 🎯 Build Principles (applies to everything)

* **Tiny files** (<200–300 LOC target), single responsibility, unit tests co-located or in `tests/`.
* **No scope creep**: Micronode **proxies** to owner services; it does **not** re-own Graph/Search/Feed/Media.
* **Guardrails everywhere**: amnesia-by-default, OAP caps (1 MiB frame, 64 KiB chunk, ≤10× decompress), admin plane parity (`/metrics`, `/healthz`, `/readyz`, `/version`).
* **Concurrency**: bounded queues/semaphores; never hold a lock across `.await`; graceful drain FSM.
* **Quantum (PQ)**: posture = `off|hybrid|required`; adapters live outside; this crate only **toggles & observes**.

---

## Root files

* **Cargo.toml**

  * **Purpose**: Declare bin (`micronode`) and slim features (`graph`, `search`, `feed`, `media`, `pq-hybrid`).
  * **I/O**: Exposes only `lib::run()` + config types.
  * **Gotchas**: Keep feature surfaces additive; pin workspace versions.
  * **Done when**: `cargo check -p micronode` passes for all feature combos.

* **README.md**

  * **Purpose**: Quickstart + invariants + admin endpoints.
  * **Done when**: `examples/quickstart.rs` runs as written.

* **LICENSE-APACHE / LICENSE-MIT**: dual licensing, no changes.

* **.gitignore**: excludes targets, local env, generated diagrams.

* **.clippy.toml / .rustfmt.toml**

  * **Purpose**: Enforce style and safety (e.g., forbid blocking in async).
  * **Done when**: `cargo clippy -- -D warnings` is clean.

* **deny.toml**

  * **Purpose**: Local supply-chain bans/advisories/sources (fallback to repo-root policy).
  * **Done when**: `cargo deny check` is green.

---

## configs/

* **micronode.toml**: Canon defaults for prod (amnesia off unless explicitly enabled; OAP caps; admin binds).
* **micronode.dev.toml**: Dev profile (amnesia on, loopback admin, verbose logs).
* **micronode.amnesia.off.toml**: Persistence-enabled profile to exercise amnesia matrix.
* **micronode.pq.toml**: PQ **hybrid** sample (classical↔PQ allowed; metrics on).
* **micronode.pq.required.toml**: PQ **required** (fail-closed if adapter unavailable).
* **Done when**: `src/config/*` loads+validates; `tests/pq_modes.rs` & `tests/amnesia_proof.rs` pass per profile.

---

## docs/

* **ALL\_DOCS.md; API.md; CONFIG.md; CONCURRENCY.md; GOVERNANCE.md; INTEROP.md; PERFORMANCE.md; QUANTUM.md; RUNBOOK.md; SECURITY.md; IDB.md**

  * **Purpose**: Policy “constitution” per concern.
  * **Done when**: Each has a corresponding test or CI gate proving the claim (e.g., QUANTUM ↔ `pq_*` tests/benches; CONCURRENCY ↔ loom test; SECURITY ↔ fs-spy & admin loopback).

* **api-history/**

  * `micronode.http.json`: Golden admin surface; used by `tests/admin_parity.rs`.
  * `metrics.scrape.txt`: Golden Prom scrape; names must stay stable.
  * `pq-matrix.json`: Expected behaviors for `pq=off|hybrid|required`.

* **diagrams/**

  * `micronode_arch.mmd/svg`: Mermaid source/render; “docs as code”.

---

## specs/  *(dev/CI test vectors)*

* **oap-1.md**: OAP/1 behaviors (frames, caps, error taxonomy).
* **oap\_vectors.json**: Valid/invalid frames (used by fuzz/property tests).
* **pq\_handshake\_cases.json**: Expected paths for hybrid/required, including refusal/downgrade events.
* **Done when**: Tests consume these vectors (no hard-coded literals in tests).

---

## scripts/

* **run\_dev.sh**: Launch using `micronode.dev.toml`; prints admin URLs.
  **Done when**: Starts, `/readyz` flips true, logs show amnesia=ON.
* **fs\_spy\_amnesia.sh**: Watches filesystem for writes in amnesia mode.
  **Done when**: CI fails if any write escapes RAM-only paths.
* **gen\_diagrams.sh**: Renders Mermaid → SVG (idempotent).
* **smoke\_oap\_limits.sh**: Sends >1 MiB & >10× decompress payloads; expects 413/refusal.
* **chaos\_degrade\_shed.sh**: Fault injection + load to validate 429/Retry-After & readiness transitions.
* **pq\_matrix\_ci.sh**: Executes OFF/HYBRID/REQUIRED test matrix.
  **Note**: All scripts are **safe to run locally** and in CI; no secrets assumed.

---

## src/ (top-level)

* **main.rs**: Parse CLI → `lib::run()`. No heavy logic.
  **Done when**: Covered by `tests/*` via `cargo run` smoke.
* **lib.rs**: Public surface (Config + `run()`); re-exports minimal types.
  **Done when**: No internal wiring leaks into public API.
* **app.rs**: Build `AppState`, wire middleware/routes, apply OAP caps globally.
  **Inputs**: Config, feature flags. **Outputs**: Server handles.
  **Done when**: `/metrics|/healthz|/readyz|/version` live; routes reflect feature gating.
* **state.rs**: `AppState` (config snapshot, metrics registry, semaphores, clients).
  **Rule**: No business logic; no long-held locks.
* **limits.rs**: OAP constants & helpers (1 MiB frame, 64 KiB chunk, ≤10× decompress).
  **Used by**: ingress middleware, tests.
* **errors.rs**: Error taxonomy (413/429/503, integrity/limit/busy).
  **Rule**: Stable mapping to HTTP codes.
* **types.rs**: DTOs for admin & simple app surfaces; derive `Serialize/Deserialize`.

---

### src/cli/

* **args.rs**: CLI flags (`--config`, addrs, feature toggles, pq mode); precedence = CLI > ENV > file > defaults.
* **run.rs**: Command dispatch; sets up tracing; calls `app::bootstrap()` and serves.
* **mod.rs**: Re-exports; keep tiny.

**Acceptance**: Changing any flag is reflected in `/version` build/runtime block.

---

### src/config/

* **schema.rs**: Strong types for `limits`, `amnesia`, `tls`, `pq`, per-facet semaphores.
  **Rule**: Defaults here; doc-comment each field.
* **load.rs**: Read file(s), TOML parsing, include-paths safe-list.
* **env\_overlay.rs / cli\_overlay.rs**: Merge overlays with clear precedence.
* **validate.rs**: Enforce OAP caps; reject unknown/forbidden keys (anti-scope).
  **Rule**: Validation is pure & fast; no IO.
* **hot\_reload.rs**: Snapshot-and-swap config (atomic `Arc<Config>`); emits `ConfigUpdated` on bus.
  **Concurrency**: Never hold lock across `.await`.

**Acceptance**: `SIGHUP` (or explicit trigger) flips `/version.config_sha` and `/readyz` remains true.

---

### src/observability/

* **metrics.rs**: Prometheus registry; **golden metrics** (names used in `metrics.scrape.txt`).
* **health.rs**: `/healthz` (internal self-health: process live, basic deps reachable).
* **ready.rs**: `/readyz` (readiness gates, sheds under pressure).
* **version.rs**: `/version` (build info, features, pq posture, amnesia).
* **logging.rs**: JSON logging; structured fields (`pq_mode`, `amnesia`, shed state, limits).

**Acceptance**: `tests/admin_parity.rs` diffs `/version` shape & metric names against snapshots.

---

### src/security/

* **auth\_macaroon.rs**: Capability tokens; least-privilege, short TTL; no ambient admin.
* **tls\_rustls.rs**: TLS plumbed via `tokio_rustls::rustls::ServerConfig`.
* **amnesia.rs**: RAM-only guard, temp-fs abstraction, zeroize on shutdown.
* **pq\_toggle.rs**: Runtime posture enum `{Off, Hybrid, Required}`; single switch used by TLS/auth.
* **pq\_config.rs**: Parse `pq.*` keys, validate, normalize to typed posture; expose to `AppState`.
* **pq\_observe.rs**: Metrics & logs: `pq_mode`, `pq_kex`, `pq_sign`, `pq_fallback_total`, `pq_refused_total`.

**Acceptance**:

* `pq=required` without adapters → start-up **fails closed**; logged & metered.
* `pq=hybrid` with adapter failure → classic fallback allowed; counters increment.

---

### src/adapters/

* **index\_client.rs / mailbox\_client.rs / storage\_client.rs / overlay\_client.rs**

  * **Purpose**: Thin, typed proxies to owner services; bounded concurrency; timeouts; retries per policy.
* **policy\_client.rs**

  * **Purpose**: Read-only calls to policy (residency/geo constraints) to gate facet routes.

**Acceptance**: `tests/facets_proxy.rs` confirms calls leave the process (no local re-ownership) and are bounded.

---

### src/facets/

* **graph.rs / search.rs / feed.rs / media.rs**

  * **Purpose**: Feature-gated HTTP handlers that **proxy** to adapters with per-facet semaphores.
  * **Rule**: No local queue growth beyond configured bounds; return Busy/429 with `Retry-After`.

**Acceptance**: Under synthetic load, `tests/backpressure.rs` shows 429 within budget; no OOM or tail-latency explosion.

---

### src/concurrency/

* **backpressure.rs**: Global & per-facet semaphores; 429/`Retry-After` policy; shed logging.
* **shutdown.rs**: Drain FSM (Running→Draining→Stopped), deadlines, cancel tokens.
* **registry.rs**: (Optional) declarative topology for docs/tests (task names, channels).

**Acceptance**: Loom test covers drain interleavings; chaos test shows readiness transitions.

---

### src/http/

* **admin.rs**: `/metrics`, `/healthz`, `/readyz`, `/version` (admin plane parity).
* **routes.rs**: App routes; all ingress wrapped with OAP caps, streaming defaults.

**Acceptance**: `tests/admin_parity.rs` + `metrics.scrape.txt` comparison pass.

---

## tests/ (integration)

* **admin\_parity.rs**: Admin surface & metrics name stability vs snapshots.
* **amnesia\_proof.rs**: Spawn with amnesia; assert **no disk writes**; check zeroize on shutdown.
* **oap\_limits.rs**: >1 MiB & >10× decompress → 413/refusal; chunking at 64 KiB.
* **backpressure.rs**: Load → 429 within 50 ms; no unbounded buffers.
* **facets\_proxy.rs**: Proves out-of-process calls; bounded semaphores.
* **pq\_modes.rs**: OFF/HYBRID/REQUIRED behavior matrix; required must fail closed without adapter.
* **pq\_fallback.rs**: HYBRID: force adapter error → classic fallback + metrics/logs.

**Acceptance**: All pass in CI across matrix `{pq=off,hybrid,required}`.

---

## tests\_property/

* **oap\_fuzz.rs**: Property checks for frames/limits/error taxonomy using `specs/oap_vectors.json`.
* **pq\_handshake\_props.rs**: Idempotence/timeouts/error mapping using `specs/pq_handshake_cases.json`.

---

## tests\_chaos/

* **degrade\_shed.rs**: Fault injection + soak; verifies shed behavior and `/readyz` flips accordingly.

---

## fuzz/

* **config\_from\_env\_fuzz.rs**: Stress env/overlay parser; ensure no panics, consistent precedence.
* **pq\_kex\_fuzz.rs**: Hybrid KEX boundary fuzz; ensures refusal/downgrade mapping is stable.

---

## tests\_loom/

* **shutdown\_interleavings.rs**: Loom test of drain FSM; validates no deadlocks, correct final states.

---

## benches/

* **oap\_frame\_perf.rs**: Encode/decode p95/p99 budget tracking.
* **readiness\_walk.rs**: Shed→ready latencies under step load.
* **pq\_overhead.rs**: PQ OFF vs HYBRID vs REQUIRED overhead; CI gate for acceptable delta.

---

## examples/

* **quickstart.rs**: Minimal run (amnesia ON, loopback admin); mirrors README Quickstart.

---

# ✅ Suggested “Definition of Done” per concern (quick checklist)

* **SEC**: amnesia proof + admin loopback; PQ required mode fail-closed path covered; macaroon TTL enforced.
* **RES**: backpressure 429 within budget; graceful drain loom test green.
* **PERF**: OAP encode/decode p95 tracked; PQ overhead within threshold (e.g., ≤10%).
* **GOV**: admin parity snapshots updated; `deny`/clippy clean.
* **DX**: quickstart runs; diagrams render; examples compile.
* **ECON**: *(N/A for micronode—no ledger/issuance here; proxies only).*

