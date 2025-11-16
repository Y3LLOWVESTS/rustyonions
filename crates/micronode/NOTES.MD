### BEGIN NOTE - NOVEMBER 5 2025 - 11:30 CST

# Micronode — Carry-over Notes from svc-gateway (Beta)

## 0) Philosophy to keep

* **Ingress invariants near the edge:** cap, time bound, and concurrency-limit *before* any heavy logic. Deterministic reject paths with metrics.
* **Minimal per-route layer stacks:** add only what the route needs; keep fast paths hot.
* **Label-prewarmed metrics:** touch counters/histograms/gauges at boot so dashboards don’t start with gaps.
* **No ambient authority:** every “side effect” must be explicit (feature-flag, env, or config).
* **Pedantic hygiene:** `cargo fmt`, `clippy -D warnings`, deterministic scripts, stamp `/version`.

---

## 1) Reusable components/patterns to lift as-is

### Middleware/Guards (clone or template)

* **Correlation ID** (`x-request-id`): attach early; propagate downstream.
* **HTTP metrics** (per route):

  * `gateway_http_requests_total{method,route,status}`
  * `gateway_request_latency_seconds{method,route}` (hist)
* **Readiness guards** (for `/readyz` equivalent and/or critical routes):

  * **Timeout** middleware (env-knobbed).
  * **Concurrency cap** middleware (inflight budget).
* **Body cap** for POST/PUT (env-knobbed): 413 early; increment a `rejections_total{reason="body_cap"}`.
* **Decode guard** (optional but recommended): reject stacked encodings (415), reject compressed bodies unless you *explicitly* support it; track `decode_cap`, `stacked_encoding`.
* **Lock-free rate-limit** (token bucket) for hot-reject dev routes and optionally for public endpoints (per-node global; later per-route/per-tenant).

> For Micronode, rename metrics to `micronode_http_*` and `micronode_rejections_total` to avoid cross-service collisions.

### Observability (copy patterns)

* Prewarm label series at boot.
* Readiness sampler (gauges):

  * `*_ready_inflight_current`
  * `*_ready_error_rate_pct` (windowed % of 4xx/5xx you choose)
  * `*_ready_queue_saturated` (stub until you have queues)
* Consistent problem envelopes for 4xx/5xx (JSON with `{code,message,retryable}`).

### Build/version stamping

* `build.rs` stamping `MICRONODE_BUILD_TS` (UNIX seconds).
* `/version` returns `{ name, version, built_at_unix }` (no SHA unless you decide otherwise).

### Scripts (port 1:1, rename)

* `scripts/run_micronode.sh` (fmt, clippy, build, start, wait `/healthz`, show `/readyz`, metrics head, PID file).
* `scripts/stop_micronode.sh`
* `scripts/bench_micronode.sh` (wrk/hey harness; env for THREADS/CONNS/DUR)
* `scripts/sanity_rate_limit.sh` (if you expose `/dev/rl`)
* `scripts/test_body_cap.sh`, `scripts/decode_guard_smoke.sh` (keep as smokes)

> Keep your “no leading `#` in bash commands intended to run” convention.

---

## 2) Environment knobs to carry forward (rename `SVC_GATEWAY_*` → `MICRONODE_*`)

**Bind & logging**

* `BIND_ADDR` (default `127.0.0.1:53xx`)
* `RUST_LOG=info,micronode=debug`

**Dev exposure & metrics**

* `MICRONODE_DEV_ROUTES=1`
* `MICRONODE_DEV_METRICS=1`
* `MICRONODE_DEV_READY=1` (forces ready for benches)

**Readiness guards**

* `MICRONODE_READY_TIMEOUT_MS` (default 1000)
* `MICRONODE_READY_MAX_INFLIGHT` (default 1024)
* `MICRONODE_READY_SLEEP_MS` (dev hook; default 0)

**Caps/decoding**

* `MICRONODE_MAX_BODY_BYTES` (default 1 MiB)
* `MICRONODE_DECODE_ABS_CAP_BYTES` (decoded cap)
* (Optional) `MICRONODE_ALLOW_COMPRESSED=0` (keep off unless needed)

**Rate limit (global, lock-free)**

* `MICRONODE_RL_RPS`
* `MICRONODE_RL_BURST`
* `MICRONODE_RL_TARPIT_MS`

---

## 3) Metrics names to fork

* `micronode_http_requests_total{method,route,status}`
* `micronode_request_latency_seconds{method,route}`
* `micronode_rejections_total{reason="rate_limit|body_cap|decode_cap|stacked_encoding|policy|quota|auth"}`
* `micronode_ready_inflight_current`
* `micronode_ready_error_rate_pct`
* `micronode_ready_queue_saturated`

> Add domain-specific counters early (e.g., `micronode_kv_ops_total{op}`, `micronode_task_spawns_total`).

---

## 4) Router assembly model (apply same discipline)

* **Admin plane:** `/healthz`, `/readyz`, `/metrics`, `/version`.
* **Dev plane (behind env):** `/dev/echo`, `/dev/rl`, plus any admin-only helpers.
* **Feature routes:** add per-feature stacks (e.g., auth → policy → body cap → handler).
* Keep *outermost* layers: correlation → http_metrics; put heavy guards *inner* on specific routes.

---

## 5) Axum/Tower compatibility contracts to keep

* **Axum 0.7**, **Tower 0.5**, **tower-http 0.6** (avoid diamond-dep hell).
* No `towers` v0.4 remnants; avoid mixed feature sets.
* If you add TLS: `tokio-rustls 0.26.x` (align with workspace guidance).

---

## 6) CI & quality gates (lift the fortress)

* `fmt`, `clippy -D warnings`, `test`, `cargo-deny`, `cargo-audit` (and optionally `geiger`).
* Optional perf guard job:

  * Run `bench_micronode.sh` on label/nightly; assert ≤10% regression from a stored baseline for `/healthz` and a representative feature route.

---

## 7) Bench methodology to reuse

* Use `wrk` or `hey`; test:

  1. **Fast path** (e.g., `/healthz`) — baseline overhead.
  2. **Guarded path** (e.g., `/readyz`) — timeout + concurrency.
  3. **Hot-reject path** (rate limit/body cap) — ensure reject speed is high and observable.
* Report both **client RPS** and **server-side service time** from histogram (`sum/count`).
* Bench on power; pin `THREADS/CONNS/DUR` for comparable runs.

---

## 8) Security & hardening check-list (carry exactly)

* No `unsafe`.
* Reject unsupported encodings; never transparently decompress request bodies by default.
* Enforce caps *before* deserialization/parse.
* Deterministic error envelopes; no secret leakage in messages.
* Systemd/Docker hardening (post-beta):

  * Read-only root FS, `NoNewPrivileges`, seccomp profile, minimal image.
* Correlation IDs in logs and responses (fast incident triage).

---

## 9) Micronode MVP outline (leveraging carry-overs)

**Admin plane (day 1)**

* `/healthz`, `/readyz`, `/metrics`, `/version` — same as gateway.

**Core feature plane (phase 1)**

* Minimal **KV/Document** APIs backed by your default store (sled or pluggable):

  * `PUT /kv/{bucket}/{key}` (body cap, optional auth/policy)
  * `GET /kv/{bucket}/{key}`
  * `DELETE /kv/{bucket}/{key}`
* Dev helpers:

  * `/dev/echo` and `/dev/rl` (same as gateway) for smokes/benches.

**Guards on feature routes**

* Body cap (configurable per route or global).
* Concurrency caps on write paths.
* Rate limiting (optional: per-route or global).

**Observability**

* Per-route metrics (copy gateway pattern).
* Domain metrics (kv ops, bytes in/out).
* Readiness gauges (same sampler, later wire real signals: store open, WAL caught up, etc.).

---

## 10) File tree sketch (start point)

```
crates/micronode/
  src/
    lib.rs
    main.rs
    state.rs
    config/
      mod.rs
      env.rs
    routes/
      mod.rs
      health.rs
      ready.rs
      metrics.rs
      version.rs
      dev/
        mod.rs
        echo.rs
        rl.rs
      kv/
        mod.rs
        get.rs
        put.rs
        delete.rs
    layers/
      mod.rs
      corr.rs
      timeouts.rs
      concurrency.rs
      body_caps.rs
      decode_guard.rs
      rate_limit.rs
      auth.rs        # (stub; optional)
      policy.rs      # (stub; optional)
    observability/
      mod.rs
      http_metrics.rs
      readiness.rs
  build.rs
  scripts/
    run_micronode.sh
    stop_micronode.sh
    bench_micronode.sh
    sanity_rate_limit.sh
    test_body_cap.sh
    decode_guard_smoke.sh
```

> Start by copying the gateway counterparts and renaming symbols/env/metrics prefixes.

---

## 11) Ready-truth model (how to adapt)

* Copy the readiness sampler scaffold.
* Initially: always **truthful** (no forced ready in prod).
* Keep `MICRONODE_DEV_READY=1` only for benches/smokes.
* Later: add *error-rate gate* (toggle-off default):

  * Window N seconds; if `4xx/5xx% >= threshold`, `/readyz` returns 503.

---

## 12) Common pitfalls to avoid (learned in gateway)

* **Clippy (pedantic) nits:** backticks in doc markdown; `#[must_use]` for pure fns; error docs on `Result` returning functions.
* **Build stamp showing zero:** ensure `[package] build = "build.rs"` and file is under the crate.
* **Mixed Tower versions:** keep everything on Tower 0.5; don’t pull older transitive deps via extra features.
* **Scripts failing on macOS:** avoid `mapfile -t` (use `readarray` or plain POSIX loops); keep `sed/grep` portable.

---

## 13) Drop-in numbers to aim for (early sanity on your i5/2019)

* `/healthz` ~50k RPS (wrk, 8T/256C, 30s).
* Guarded `/readyz` within ~5–10% of `/healthz`.
* Hot-reject routes ~50k RPS (rate-limit/body-cap).
* Hist-avg service time: **<40 µs** on fast path.

---

## 14) “Beyond beta” wishlist (Micronode-specific)

* **Auth & policy hooks** (behind env/feature gates) before write routes.
* **Per-tenant quotas** (reuse RL infra; bucket by API key or header).
* **Back-pressure dashboards**: inflight histograms, queue depth samplers, store latency buckets.
* **Pluggable storage interface** (sled by default; trait behind feature flags for SQLite/Postgres/RocksDB).
* **Structured logs** with `x-request-id` in every span and error.

---

## 15) Copy-paste launch (dev)

```
# build & run
SVC_GATEWAY_DEV_ROUTES=1 SVC_GATEWAY_DEV_METRICS=1 SVC_GATEWAY_DEV_READY=1 \
crates/svc-gateway/scripts/run_gateway.sh

# when Micronode is scaffolded:
MICRONODE_DEV_ROUTES=1 MICRONODE_DEV_METRICS=1 MICRONODE_DEV_READY=1 \
crates/micronode/scripts/run_micronode.sh
```

Bench boilerplate (adapt when micronode exists):

```
THREADS=8 CONNS=256 DUR=30s \
MICRONODE_DEV_ROUTES=1 MICRONODE_DEV_METRICS=1 MICRONODE_DEV_READY=1 \
crates/micronode/benches/bench_micronode.sh
```

---

### END NOTE - NOVEMBER 5 2025 - 11:30 CST


### BEGIN NOTE - NOVEMBER 5 2025 - 15:23 CST

# Micronode — Carry-Over Notes (Beta Track)

## 1) Current status (as of Nov 5, 2025)

**Build/quality gates**

* `cargo fmt -p micronode` ✅
* `cargo clippy -p micronode --no-deps -D warnings` ✅ (zero warnings after latest fixes)
* `MICRONODE_DEV_ROUTES=1 cargo run -p micronode` ✅

**Admin plane**

* `/healthz` → 200 ✅
* `/readyz` → 200 with body:

  * `{ ready: true, probes: { listeners_bound:true, cfg_loaded:true, metrics_bound:true, deps_ok:false }, mode:"truthful" }`
  * Dev override enabled by env; truthful probe structure present.
* `/version` → `{ name:"micronode", version:"0.1.0", built_at_unix: <ts> }` ✅
* `/metrics` → served by the exporter (reachable; earlier shell “000” was a zsh pipeline typo) ✅

**Dev/feature routes (foundation)**

* `/v1/ping` → `{ "pong": true }` ✅
* `/dev/echo` (JSON echo) → `{ "message": "hi" }` ✅

**Guards & layers (per-route)**

* `BodyCapLayer` — requires explicit `Content-Length` and caps body size; hot 413/411 rejects ✅
* `DecodeGuard` — 415 on *any* `Content-Encoding` and stacked encodings ✅
* `ConcurrencyLayer` — non-blocking per-route semaphore with 429 shed path ✅
* HTTP request metrics middleware (per-route) ✅

**Recent fixes (important)**

* Replaced `from_fn_with_state` approach with first-class `Layer + Service` types (Axum 0.7-correct) ✅
* Eliminated `futures-util` dependency by using `Pin<Box<dyn Future + Send>>` ✅
* Fixed lifetime in `ConcurrencyLayer` with `try_acquire_owned()` ✅
* Cleaned clippy: identity-op, needless-return, suspicious-map, unused-mut, trait bounds (Clone) ✅
* Readiness: `ReadyProbes::default()` implemented to satisfy `new_without_default` ✅

---

## 2) What we validated (smoke tests) and how to run

**Start server**

```
MICRONODE_DEV_ROUTES=1 cargo run -p micronode
```

**Admin smokes**

```
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:5310/healthz
curl -s http://127.0.0.1:5310/readyz | jq .
curl -s http://127.0.0.1:5310/version | jq .
curl -s http://127.0.0.1:5310/metrics | head -n 20
```

**Dev/feature smokes**

```
curl -s http://127.0.0.1:5310/v1/ping | jq .
curl -s -X POST http://127.0.0.1:5310/dev/echo \
  -H 'content-type: application/json' \
  --data-binary '{"message":"hi"}' | jq .
```

**Guard behavior**

```
# Reject compressed
curl -s -o /dev/null -w "%{http_code}\n" -X POST http://127.0.0.1:5310/dev/echo \
  -H 'content-type: application/json' -H 'content-encoding: gzip' \
  -H 'content-length: 18' --data-binary '{"message":"hi"}'   # 415

# Happy path requires correct Content-Length (don't hand-set unless exact)
curl -s -X POST http://127.0.0.1:5310/dev/echo \
  -H 'content-type: application/json' --data-binary '{"message":"hi"}' | jq .
```

**Tip (Content-Length)**

* If you manually set `Content-Length` and it doesn’t match the payload, Hyper will wait for missing bytes and your terminal will appear to “hang.” Prefer `--data-binary` without manually setting length.

---

## 3) Files/modules we touched (baseline you now have)

* `src/layers/body_cap.rs` — custom `BodyCapLayer` + `BodyCapService` (Clone, Send, 'static futures)
* `src/layers/concurrency.rs` — `ConcurrencyLayer` with `try_acquire_owned()` (no lifetime pitfalls)
* `src/layers/decode_guard.rs` — policy: 415 on any encoding and stacked encodings
* `src/http/routes.rs` — stable `/v1/ping` + `/dev/echo` JSON extractor (no hangs)
* `src/observability/ready.rs` — `ReadyProbes::default()` implemented

(Everything compiles clean against Axum 0.7 / Tower 0.5 / tower-http 0.6.)

---

## 4) Definition of “Micronode Beta” (working contract)

**Must-have for Beta**

1. **Admin plane complete**: `/healthz`, `/readyz` (truthful), `/metrics`, `/version`.
2. **Ingress invariants enforced** on feature routes:

   * Body cap + explicit `Content-Length`
   * Decode guard policy
   * Per-route concurrency cap
   * Correlation + http metrics middleware
3. **Minimal feature surface**:

   * KV/Document API v1 (sled default): `PUT/GET/DELETE /kv/{bucket}/{key}`
   * Optional: `/search` stub with TODO instrumentation
4. **Observability**:

   * Per-route counters + latency histograms
   * Domain counters (kv ops/bytes)
   * Ready sampler tied to actual deps (store open, metrics bound, config loaded)
5. **Config**:

   * File + env overlays; validated schema; sensible defaults; hot-reload stub
6. **Tests**:

   * Admin truth tests + property tests (oap vectors wiring optional here)
   * Guard behavior tests (body cap, decode guard, concurrency 429)
   * Basic KV round-trip tests
7. **Benches**:

   * Fast path (`/healthz`)
   * Guarded path (`/readyz` with cap)
   * Hot-reject path (decode/body cap)
8. **Docs**:

   * README quickstart
   * CONFIG, RUNBOOK, SECURITY checklists updated for Micronode reality

---

## 5) Completion estimate (Beta)

* Admin plane: **100%**
* Core route guards (body cap, decode guard, concurrency): **100%**
* HTTP metrics middleware: **100%**
* Readiness skeleton (truthful + probes): **80%** (deps_ok still stubbed)
* Config loaders/validation/hot-reload: **30%** (scaffold exists per tree; code TODO)
* Storage adapters (sled default) + KV routes: **0%** (next chunk)
* Security (tls_rustls, macaroons, amnesia, PQ toggles): **0–10%** (files scaffolded)
* Adapters (index/mailbox/storage/overlay/policy): **0%** (post-beta or thin stubs)
* Tests (unit/integration/property/chaos): **20%** (we have smokes; formal tests pending)
* Benches: **10%** (middleware ping soon; add oap_frame_perf/readiness_walk)

**Overall Beta completion:** **~40%** (foundation is solid; main remaining work is KV API + config + readiness truth wiring + tests/benches).

---

## 6) Next incremental steps (small, high-signal PRs)

### Step A — Config & readiness truth (Deps OK)

* Implement `config::load` (file + env overlay + validation).
* In `ReadyProbes`, flip:

  * `cfg_loaded` after config parse.
  * `metrics_bound` once exporter bind succeeds.
  * `listeners_bound` after Axum listener bind.
  * `deps_ok`: wire to storage open success.
* Add `/readyz` mode: `"truthful"` (default) and `"forced"` if `MICRONODE_DEV_READY=1`.

**Commands**

```
cargo test -p micronode -- tests/admin_parity.rs
```

### Step B — Storage trait + sled impl + KV routes

* Define `Storage` trait (`get/put/del`, bytes in/out, bucket+key).
* `sled` default feature, open path from config; wire metrics (`micronode_kv_ops_total{op}`, bytes).
* Routes:

  * `PUT /kv/{bucket}/{key}` → 201/204
  * `GET /kv/{bucket}/{key}` → 200/404
  * `DELETE /kv/{bucket}/{key}` → 204/404
* Apply `BodyCapLayer` and `ConcurrencyLayer` to PUT/DELETE; `DecodeGuard` to all.

**Smokes**

```
curl -s -o /dev/null -w "%{http_code}\n" -X PUT \
  http://127.0.0.1:5310/kv/a/k -H 'content-type: application/octet-stream' \
  --data-binary 'hello'
curl -s http://127.0.0.1:5310/kv/a/k
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE http://127.0.0.1:5310/kv/a/k
```

### Step C — Tests

* Unit tests for guards (cap/decode/concurrency).
* Integration tests: KV roundtrip; readiness truth flips on storage fail/ok.
* Property tests: tiny payloads vs cap boundaries; malformed headers.
* Chaos test (optional): small `Semaphore::new(1)` then parallel PUTs expecting 429.

**Run**

```
cargo test -p micronode
```

### Step D — Benches

* Criterion bench for `/healthz` (in-process service call).
* Add `/kv put/get` latency histograms (small payload).
* Document RPS + service time in `docs/PERFORMANCE.md`.

**Run**

```
cargo bench -p micronode
```

---

## 7) Observability & metrics (what to prewarm/track)

* `micronode_http_requests_total{method,route,status}`
* `micronode_request_latency_seconds{method,route}` (hist)
* `micronode_rejections_total{reason="body_cap|decode|concurrency"}`
* `micronode_ready_*` gauges (`inflight_current`, `queue_saturated` later)
* Domain:

  * `micronode_kv_ops_total{op="put|get|del"}`
  * `micronode_kv_bytes_total{dir="in|out"}`

---

## 8) Security & policy (beta scope)

* Keep `DecodeGuard` strict (no transparent decompression).
* Body cap before deserialization.
* Optional (behind feature flag for post-beta):

  * `tls_rustls` listener
  * Macaroon auth stub (cap tokens)
  * Amnesia mode toggles (no persistence profile)

---

## 9) Common pitfalls (what we already avoided / keep avoiding)

* Mixing Tower 0.4/0.5 or axum middleware generics: **we’re clean** with Layer+Service.
* `Content-Length` mismatches causing hangs (client-side) — **documented**.
* `from_fn_with_state` type bounds on `Router::layer` — **moved off it**.
* Non-'static futures in services — **fixed** via owned permit + boxed futures.

---

## 10) Handy dev commands

```
# run (dev routes on)
MICRONODE_DEV_ROUTES=1 cargo run -p micronode

# quick curl set
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:5310/healthz
curl -s http://127.0.0.1:5310/readyz | jq .
curl -s http://127.0.0.1:5310/version | jq .
curl -s http://127.0.0.1:5310/metrics | head -n 20
curl -s http://127.0.0.1:5310/v1/ping | jq .
curl -s -X POST http://127.0.0.1:5310/dev/echo \
  -H 'content-type: application/json' \
  --data-binary '{"message":"hi"}' | jq .

# clippy strict
cargo clippy -p micronode --no-deps -- -D warnings
```

---

## 11) Proposed PR sequence (small, reviewable)

1. **config+ready truth** (wire probes; docs: CONFIG.md, RUNBOOK.md updates)
2. **storage trait + sled impl** (metrics; errors.rs taxonomy)
3. **kv routes** (+ guard stacks; integration tests)
4. **tests** (guards/kv/admin)
5. **benches** (healthz, kv)
6. **docs polish** (README quickstart, SECURITY, PERFORMANCE with baseline numbers)

---


### END NOTE - NOVEMBER 5 2025 - 15:23 CST







### BEGIN NOTE - NOVEMBER 5 2025 - 16:03 CST
TLDR: We basically have a foundation built for micronode that we will come back to and finish. We paused because we need to complete other crates first in order to ship a full proper micronode profile.

# Micronode — Carry-Over Notes (Pause at Beta-Track Checkpoint)

## 1) Snapshot (Nov 5, 2025)

**State of the crate**

* Server boots and serves: `/healthz`, `/readyz` (truthful), `/version`, `/metrics`.
* Dev routes gated by env (`MICRONODE_DEV_ROUTES=1`): `/dev/echo`.
* Feature route foundation: `/v1/ping`.
* **KV v1** implemented with in-memory store:

  * `PUT /kv/{bucket}/{key}` → 201
  * `GET /kv/{bucket}/{key}` → 200/404
  * `DELETE /kv/{bucket}/{key}` → 204/404
* Ingress invariants enforced on dev + KV:

  * `DecodeGuard`: rejects any `Content-Encoding` (415).
  * `BodyCapLayer`: caps `Content-Length`, requires it for POST/PUT/PATCH, **not** for GET/DELETE (413/411 as appropriate).
  * `ConcurrencyLayer`: per-route semaphore with 429 shed path.
* Readiness probes:

  * `listeners_bound:true`, `cfg_loaded:true`, `metrics_bound:true`.
  * `deps_ok:true` (truthful for in-mem).
* Build hygiene:

  * `fmt` and `clippy -D warnings` clean.
  * Dev run is green.

**Confirmed sanity run (no comments in commands)**

```
cargo fmt -p micronode
cargo clippy -p micronode --no-deps -- -D warnings
MICRONODE_DEV_ROUTES=1 cargo run -p micronode
```

**Quick smokes**

```
curl -s http://127.0.0.1:5310/readyz | jq .
curl -s -o /dev/null -w "%{http_code}\n" -X PUT http://127.0.0.1:5310/kv/a/k -H 'content-type: application/octet-stream' --data-binary 'hello'
curl -s http://127.0.0.1:5310/kv/a/k
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE http://127.0.0.1:5310/kv/a/k
curl -s -o /dev/null -w "%{http_code}\n" -X PUT http://127.0.0.1:5310/kv/a/enc -H 'content-type: application/octet-stream' -H 'content-encoding: gzip' --data-binary 'z'
```

Expected: `readyz.ready=true`, `deps_ok=true`, `PUT→201`, `GET→hello`, `DELETE→204`, encoded PUT → `415`, large body over cap → `413`.

---

## 2) What’s implemented (files & responsibilities)

* `src/app.rs` — Router assembly; admin/dev/api nests; guard stacks; flips `deps_ok(true)` for in-mem.
* `src/http/admin.rs` — `/healthz`, `/readyz`, `/version`, `/metrics`.
* `src/http/routes.rs` — `/v1/ping` and `dev::echo`.
* `src/http/kv.rs` — `put_kv`, `get_kv`, `del_kv`.
* `src/layers/body_cap.rs` — content-length policy + cap (POST/PUT/PATCH require length).
* `src/layers/decode_guard.rs` — 415 on any `Content-Encoding` (strict).
* `src/layers/concurrency.rs` — per-route semaphore; 429 on shed.
* `src/observability/ready.rs` — `ReadyProbes` truth model.
* `src/state.rs` — `AppState` with `cfg`, `metrics`, `health`, `probes`, `storage`.
* `src/storage/mod.rs` — `Storage` trait, `MemStore` impl, `DynStorage`.
* `src/storage/sled_store.rs` — sled adapter stub behind `sled-store` feature (not wired).

---

## 3) Invariants we’re enforcing (keep them)

1. **No transparent decompression** (strict 415 on any `Content-Encoding`).
2. **Body caps always**; explicit `Content-Length` required for POST/PUT/PATCH; GET/DELETE allowed without length.
3. **Per-route concurrency bounds** (semaphore) with non-blocking 429 rejection path.
4. **Truthful readiness**: `deps_ok` reflects actual store availability (currently true for in-mem; will depend on sled open success later).
5. **Zero `unsafe`**; no locks held across `.await`.

---

## 4) Minimal API contract (Beta scope)

* `PUT /kv/{bucket}/{key}`

  * Req: `content-type: application/octet-stream`, `Content-Length` present.
  * Resp: `201 Created`, empty body.
* `GET /kv/{bucket}/{key}`

  * Resp: `200 OK` with raw bytes; `content-type: application/octet-stream`. `404` if missing.
* `DELETE /kv/{bucket}/{key}`

  * Resp: `204 No Content` or `404`.

**Rejection contract**

* `415 Unsupported Media Type` if `Content-Encoding` present.
* `411 Length Required` for POST/PUT/PATCH without length header.
* `413 Payload Too Large` if declared length exceeds configured cap.
* `429 Too Many Requests` on semaphore saturation.

---

## 5) Metrics to keep/extend

* `micronode_http_requests_total{method,route,status}`
* `micronode_request_latency_seconds{method,route}` histogram
* `micronode_rejections_total{reason="body_cap|decode|concurrency"}`
* `micronode_ready_{listeners_bound,cfg_loaded,metrics_bound,deps_ok}` gauges
* **KV domain (to add with sled wiring)**:

  * `micronode_kv_ops_total{op="put|get|del"}`
  * `micronode_kv_bytes_total{dir="in|out"}`

---

## 6) Config surface we’ll add when we resume

**`Config` additions**

* `storage.engine`: `"mem"` | `"sled"`
* `storage.path`: string (used when engine = "sled")
* `server.dev_routes`: bool (already present)

**Behavior**

* On startup:

  * If `engine=mem`: construct `MemStore`, set `deps_ok(true)`.
  * If `engine=sled`: open DB at `storage.path`; on success set `deps_ok(true)`, else leave `false` and return 500s for KV until healthy or exit fast (decision below).

**Decision to ratify on resume**

* **Fail-fast** on sled open error (preferred for prod) vs. **serve unhealthy** with `deps_ok:false`. Pick one and codify.

---

## 7) Tests & benches we plan to add (exact files)

**Integration tests**

* `tests/kv_roundtrip.rs`

  * PUT→GET→DELETE success paths; 404 on missing; binary roundtrip correctness.
* `tests/guard_behavior.rs`

  * 415 on any `Content-Encoding`; 411 for PUT with no length; 413 over cap.
* `tests/ready_truth.rs`

  * `deps_ok` reflects in-mem (true) and sled (true/false on open result).

**Property tests (optional, small)**

* `tests/prop_caps.rs`

  * Random payload sizes around cap boundaries; random headers present/absent.

**Benches (Criterion)**

* `benches/healthz.rs` — in-process service call latency baseline.
* `benches/kv_small.rs` — PUT/GET 128–1024B payloads (in-mem, then sled).
* Record baseline and document in `docs/PERFORMANCE.md`.

**Run**

```
cargo test -p micronode
cargo bench -p micronode
```

---

## 8) Sled adapter wiring plan (bite-size sequence)

**PR A — Features & deps**

* `Cargo.toml`:

  * `sled = { version = "0.34", optional = true }`
  * `[features]`

    * `default = ["mem-store"]`
    * `mem-store = []`
    * `sled-store = ["sled"]`

**PR B — Config surface**

* Extend `config::schema` with:

  ```rust
  #[derive(Debug, Clone, Deserialize)]
  pub struct StorageCfg { pub engine: String, pub path: Option<String> }
  ```
* Defaults: `engine="mem"`, `path=None`.

**PR C — AppState storage selection**

* In `AppState::new(cfg)`:

  * If `engine="sled"` and `sled-store` enabled: `SledStore::open(path?)`.
  * Set `probes.set_deps_ok(true)` only on success.
  * If engine=sled but feature disabled: log and fallback to mem (or fail-fast—decide).

**PR D — Domain metrics**

* Wrap `Storage` calls to increment ops/bytes counters.
* Add simple error taxonomy mapping to 500 (beta).

**PR E — Tests for sled**

* Conditional compile (`#[cfg(feature="sled-store")]`) for sled tests.
* `KV` tests with a temp dir.

---

## 9) Security/auth seam (reserved for Beta+1)

* `DecodeGuard` stays strict (no gzip, brotli, deflate).
* Add capability check hook (no-op for now) at KV handlers:

  * Extraction of `Passport`/`Macaroon` headers (placeholder types).
  * `ron-policy` evaluator call (stub pass-through).
* Toggle via feature flag `caps` or via config `auth.mode="none|cap"`.
* Later: `ron-kms` for local key ops; `ron-auth` for token issuance/verification.

---

## 10) Operational posture (RUNBOOK hooks to finish later)

* Readiness gates to tie:

  * `listeners_bound` → set after bind.
  * `metrics_bound` → set after exporter route online.
  * `cfg_loaded` → set after config parsed.
  * `deps_ok` → storage open success (sled) or true (mem).
* Health: always 200 unless we add self-diagnostics.
* Shutdown: drain accept loops first, then cancel per-route semaphores cleanly.

---

## 11) Error taxonomy (Beta scope)

* Keep it minimal for now:

  * `Error::Internal` → 500.
  * `Error::NotFound` → 404 (only if we decide to use typed errors in KV; currently we branch on `Option`).
* Map rejections to stable codes as already implemented (411/413/415/429).

---

## 12) “Definition of Done” checklists

**Beta DoD (HTTP+KV)**

* Truthful admin plane (done).
* KV v1 (done for mem; sled wired; domain metrics added).
* Config loader (storage.engine/path), validated defaults.
* Guard tests + KV tests + ready truth tests.
* Perf baselines recorded for `/healthz` and KV.

**Full Micronode DoD (preview)**

* Auth/caps on write/read (policy hooks).
* Key management (kms), token verification (auth).
* Audit trail (audit).
* Optional non-HTTP transports (ron-transport) if needed.
* SDK-ready semantics and examples.

---

## 13) Known pitfalls we already solved (keep these patterns)

* Axum 0.7 generic `NewError` on `.layer(...)` for `MethodRouter` — solved by annotating `.layer::<_, Infallible>(...)`.
* `BodyCapLayer` requiring content-length on DELETE — fixed: only POST/PUT/PATCH require it.
* Avoiding `futures_core` dependency in layers — use `Pin<Box<dyn Future + Send>>`.
* Avoid mixing Tower 0.4/0.5; we’re on Tower 0.5 across the board.
* No manual `Content-Length` in curl unless exact; use `--data-binary`.

---

## 14) Re-entry quickstart (when we resume)

**Clean sweep**

```
cargo fmt -p micronode
cargo clippy -p micronode --no-deps -- -D warnings
cargo test -p micronode
MICRONODE_DEV_ROUTES=1 cargo run -p micronode
```

**KV sanity**

```
curl -s -o /dev/null -w "%{http_code}\n" -X PUT http://127.0.0.1:5310/kv/a/k -H 'content-type: application/octet-stream' --data-binary 'hello'
curl -s http://127.0.0.1:5310/kv/a/k
curl -s -o /dev/null -w "%{http_code}\n" -X DELETE http://127.0.0.1:5310/kv/a/k
```

**Next action (first PR on return):** PR A from §8 (Cargo features + sled dep).

---

## 15) Dependency posture (workspace crates)

**Currently used:** `ron-kernel`, `ron-proto`, `oap`.
**Likely by Beta:** add `ron-policy` (stub) only if we want an immediate capability seam; otherwise defer to Full.
**Full Micronode:** + `ron-auth`, `ron-kms`, `ron-audit` (and optional `svc-registry`, `svc-edge`, `ron-app-sdk`, `ron-transport` as needs emerge).

---

## 16) File stubs to keep in tree (harmless until wired)

* `src/storage/sled_store.rs` (present, feature-gated).
* `examples/quickstart.rs` (compiles; placeholder main).
* Optional placeholders (empty modules are fine; compile-gated if needed):

  * `src/auth/` (capability, passport, revocation) behind `caps` feature.
  * `src/policy/` evaluator shims calling `ron-policy` (future).

---


### END NOTE - NOVEMBER 5 2025 - 16:03 CST




### BEGIN NOTE - NOVEMBER 16 2025 - 11:27 CST


This is looking **really** good now — Micronode is actually doing real work and proving it under tests.

You’ve got:

* ✅ Admin plane (`/healthz`, `/readyz`, `/version`, `/metrics`)
* ✅ Ingress invariants (decode guard + body cap + concurrency)
* ✅ KV v1 over HTTP (`PUT/GET/DELETE /v1/kv/{bucket}/{key}`)
* ✅ Tests for admin parity, KV roundtrip, and guard behavior all green

Given your earlier request to *“use God tier levels of profoundly deep analysis”* and that we just landed the big missing tests, the best next move is to **capture where Micronode stands now** and what’s left for “true” Micronode beta vs “full profile” — so future-you (and future instances) don’t have to reconstruct the current state from git archaeology.

Below is a **drop-in NOTE** you can paste at the bottom of:

> `crates/micronode/NOTES.MD`

---

### BEGIN NOTE – NOVEMBER 16 2025 – 16:30 CST

# Micronode — Beta Track Checkpoint (Post-Guard + KV Tests)

## 0) TL;DR

Micronode is now a **real, test-validated micro-node**:

* Admin surface is truthful and observable.
* Ingress invariants (decode + body cap + concurrency) are enforced and tested.
* KV v1 over HTTP works end-to-end (in-memory engine) and is tested.
* HTTP metrics are wired and checked via `/metrics` parity test.

We’re very close to a **“Beta (HTTP + in-mem KV)”** definition of done. The main remaining “strict” Beta items are:

* Wiring **storage config surface** (engine selection, path) + sled engine.
* Domain metrics for KV (ops + bytes).
* A first pass at **perf benches** with documented baselines.
* Optional: minimal PQ/amnesia toggles surfaced in config (already sketched via configs).

---

## 1) Current Micronode Status (Code + Behavior)

**Build / Quality**

* `cargo fmt -p micronode` ✅
* `cargo clippy -p micronode --no-deps -D warnings` ✅
* `cargo test -p micronode` ✅

**Admin plane**

* `/healthz` — 200, liveness only (no self-diagnostics yet).
* `/readyz` — JSON body with truthful probes:

  * `listeners_bound`
  * `cfg_loaded`
  * `metrics_bound`
  * `deps_ok` (currently true for in-mem store when booted via `main`, and flipped manually to true in tests)
* `/version` — `{ name:"micronode", version:"0.1.0", built_at_unix:<ts> }`.
* `/metrics` — Prometheus exporter via `ron-kernel::Metrics`, plus Micronode HTTP metrics.

**Feature & dev routes**

* `/v1/ping` — simple `{ "pong": true }` sanity route.
* `/v1/kv/{bucket}/{key}`:

  * `PUT` → 201, stores bytes under `(bucket, key)` in **in-memory** store.
  * `GET` → 200 with raw bytes (`application/octet-stream`) or 404.
  * `DELETE` → 204 or 404.
* `/dev/echo` — JSON echo route (behind `MICRONODE_DEV_ROUTES=1`) used as a **guard test surface**.

**Ingress invariants (per-route)**

* `DecodeGuard`:

  * Rejects **any** `Content-Encoding` with 415 on guarded routes.
  * No transparent decompression; no gzip/brotli/deflate magic.
* `BodyCapLayer`:

  * Enforces `HTTP_BODY_CAP_BYTES`.
  * Returns 413 on payloads over cap.
  * Requires `Content-Length` for write verbs (POST/PUT/PATCH) while allowing GET/DELETE without it.
* `ConcurrencyLayer`:

  * Per-route semaphore.
  * Returns 429 on saturation.
* HTTP metrics middleware:

  * `micronode_http_requests_total{method,route,status}`
  * `micronode_request_latency_seconds`

**Readiness & observability**

* `ReadyProbes` is wired to `AppState` and flipped in:

  * `main` (normal boot).
  * Tests (in-process Axum server bootstrap).
* `http_metrics` middleware:

  * Uses `OnceLock` + global registry to register `micronode_http_requests_total` and `micronode_request_latency_seconds`.
  * Handles both success and error paths, recording:

    * `method` (`GET`, `PUT`, etc.)
    * `route` (path string, e.g. `/v1/kv/a/k`)
    * `status` (“200”, “201”, “404”, “500”…)
  * Does **not** propagate handler errors (Error = `Infallible`), instead logs and returns a 500 while still updating metrics.
* `prewarm()` is invoked so that `/metrics` always exposes the HTTP metric families even before the first request.

---

## 2) Test Suite Snapshot (What We Have Locked In)

**1) `tests/admin_parity.rs`**

* Boots Micronode (via the real `main` path) and asserts:

  * `/healthz` == 200.
  * `/readyz.ready == true` and the probes are sensibly set.
  * `/version` returns the expected fields.
  * `/metrics` **contains** `micronode_http_requests_total` and `micronode_request_latency_seconds`.

**2) `tests/kv_roundtrip.rs`**

* Spins up an in-process Axum server using `build_router(cfg)` and a TCP listener on `127.0.0.1:0`.
* Manually flips readiness probes:

  * `cfg_loaded = true`
  * `listeners_bound = true`
  * `metrics_bound = true`
  * `deps_ok = true` (truthful for in-mem).
* Exercises the KV API:

  * `PUT /v1/kv/a/k` with “hello”:

    * Expects 201.
  * `GET /v1/kv/a/k`:

    * Expects 200 and body == `b"hello"`.
  * `DELETE /v1/kv/a/k`:

    * Expects 204.
  * `GET /v1/kv/a/k` again:

    * Expects 404.

This locks in the **KV v1 contract** and proves the **in-memory storage engine** behavior.

**3) `tests/guard_behavior.rs`**

Two tests, both use `/dev/echo` with dev routes enabled:

1. `decode_guard_rejects_any_content_encoding`:

   * Sends `POST /dev/echo` with:

     * `content-type: application/json`
     * `content-encoding: gzip`
   * Expects 415 (Unsupported Media Type).
   * Verifies that any `Content-Encoding` on guarded routes is rejected.

2. `body_cap_enforces_max_payload_size`:

   * Constructs payload of `HTTP_BODY_CAP_BYTES + 1` bytes.
   * `POST /dev/echo` with `content-type: application/json`.
   * Expects 413 (Payload Too Large).
   * Verifies that `BodyCapLayer` enforces the configured size cap in practice.

**4) Other *_test shells present**

* `amnesia_proof.rs`, `backpressure.rs`, `facets_proxy.rs`, `oap_limits.rs`, `pq_fallback.rs`, `pq_modes.rs` currently have 0 tests each — they act as **future hooks** for:

  * Amnesia profiles (no persistent storage / RAM-only).
  * Backpressure behavior (inflight counts, rates).
  * Facet proxy semantics (Micronode as a “mini gateway”).
  * OAP frame size limits over Micronode endpoints.
  * PQ cipher modes and fallback behavior.

These are explicitly **Beta+** / “full Micronode profile” targets, not immediate blockers.

---

## 3) Current Invariants & Guarantees (What Micronode Promises Today)

1. **Admin plane is truthful and observable**

   * `ReadyProbes` reflect config + listener + metrics + deps (in-mem storage).
   * `/readyz.mode == "truthful"` by default; dev overrides exist but are not required in tests.

2. **Ingress invariants on key routes**

   * No transparent decompression on guarded routes (currently dev routes + KV routes).
   * Body size caps applied **before** any heavy parse/deserialize.
   * Concurrency limits guard service health (429 instead of queue explosions).

3. **KV semantics (in-mem engine)**

   * `(bucket, key)` → opaque byte value.
   * Overwrite semantics on PUT (last write wins).
   * 404 is the only way to observe missing keys (no leakage of internal error details).
   * No persistent storage (Micronode currently behaves as a RAM KV, suitable for amnesia-mode or dev).

4. **Observability across the stack**

   * Per-request counters + latency histograms for HTTP plane.
   * Admin parity test ensures HTTP metrics families are actually visible at `/metrics`.

5. **Zero `unsafe` & lock discipline**

   * Guards / middleware do not hold blocking locks across `.await`.
   * Storage engine for in-mem is a simple `RwLock<HashMap<..>>`, used in short critical sections.

---

## 4) Beta Completion Estimate (Updated)

Using the **“HTTP + KV + guards + observability”** lens for Micronode Beta:

* Admin plane: **100%**
* HTTP metrics middleware: **100%**
* Guard layers (decode/body-cap/concurrency): **100%**
* KV v1 (in-mem engine): **100%**
* Tests:

  * Admin parity: **100%**
  * KV roundtrip: **100%**
  * Guard behavior: **100%**
* Config loader / schema:

  * Basic server config (bind + dev routes): **present and used**.
  * Storage config (engine/path) and PQ/amnesia toggles: **partially scaffolded in configs, not fully wired in code**.
* Storage engines:

  * In-mem engine: **100%**
  * Sled engine: **stub exists, not wired** (**~20–30%**)
* Domain metrics:

  * HTTP plane: **100%**
  * KV domain (ops/bytes): **0–10%** (design agreed in notes, not yet implemented)
* Benches:

  * Bench harness files exist (oap frame perf, PQ overhead, readiness walk) but **not yet updated to latest router/limits wiring** and no canonical baselines recorded in notes.

**Overall Micronode Beta completion (HTTP + in-mem KV profile): ~80–85%.**
If we define a strict Beta that *requires* sled engine + KV domain metrics + perf baselines, then we’re closer to **~70–75%**.

---

## 5) Next Steps Toward “Strict” Micronode Beta

### Step 1 — Storage Config + Engine Selection

**Goal:** Allow config to choose between `engine = "mem"` and `engine = "sled"`.

* Extend `Config::storage` schema:

  * `engine: String` (`"mem"` | `"sled"`)
  * `path: Option<String>` (used when `"sled"`)
* In `AppState::new(cfg)` (or equivalent boot path):

  * If `engine = "mem"`:

    * Use in-mem store, set `deps_ok = true`.
  * If `engine = "sled"` (and feature is enabled):

    * Open `SledStore::open(path)`.
    * On success: set `deps_ok = true`.
    * On failure: **fail-fast** (preferred) vs “serve unhealthy but alive” (alternative; decide and encode).
* Tests:

  * Sled-only tests behind `#[cfg(feature = "sled-store")]` using a temp dir.
  * `ready_truth` style tests for `deps_ok` reflecting sled open success/failure.

### Step 2 — KV Domain Metrics

**Goal:** Add:

* `micronode_kv_ops_total{op="put|get|del"}`
* `micronode_kv_bytes_total{dir="in|out"}`

**Options:**

1. Instrument at **storage trait** level:

   * Wrap `Storage` in a `MetricsStorage<S>` that increments counters based on method and byte length.
   * Keeps KV metrics aligned with any future engine (mem/sled/other).
2. Instrument in **HTTP handlers** (simpler, more coupled):

   * On `PUT`, increment `ops{op="put"}` and `bytes{dir="in"}`.
   * On `GET`, if 200, increment `ops{op="get"}` and `bytes{dir="out"}`.
   * On `DELETE`, increment `ops{op="del"}`.

Given Micronode’s goal of being a “God-tier dev UX node,” option (1) is cleaner, but option (2) is acceptable for Beta if we want faster delivery.

### Step 3 — Bench Harness + Baselines

**Goal:** Lock in **RPS + latency** expectations on your 2019 MBP i5 baseline.

* Update existing benches:

  * `benches/healthz.rs` (or equivalent) — fast path latency (`/healthz`).
  * `benches/readiness_walk.rs` — readiness behavior under light load.
  * KV bench (new) for small payloads (128–1024B) on `/v1/kv`.
* Capture and record in `docs/PERFORMANCE.md`:

  * RPS for `/healthz` and `/v1/kv`.
  * Average/median latency (using histograms).
  * Notes about power mode (plugged in vs battery / Low Power Mode).

### Step 4 — (Optional for Beta) Amnesia + PQ Tuning Hooks

These can be Beta+ if needed, but worth sketching:

* Amnesia:

  * `amnesia.mode = "on|off"` plus possibly “hybrid” for sled+tmpfs.
  * When `amnesia = on`, disallow sled engine and log on boot.
* PQ:

  * Use configs already present under `configs/micronode.pq*.toml`.
  * Expose a logical `pq.mode = "off|preferred|required"` that later ties into ron-transport / overlay decisions.

---

## 6) Interop & Evolution (How Micronode Fits Into RON-CORE)

* **Today**, Micronode offers:

  * A small, HTTP-only micro-node with strong ingress invariants and observability.
  * An in-memory KV surface suitable for:

    * Dev/testing.
    * Amnesia-mode toy deployments.
    * Future facet demos (e.g., front-end apps talking directly to Micronode).
* **Tomorrow**, Micronode will:

  * Gain sled persistence + amnesia toggles, making it a **developer-friendly, minimal RON-CORE node** you can run next to an app.
  * Integrate with:

    * `ron-auth` / `ron-kms` (capabilities, tokens, key ops).
    * `ron-policy` (policy checks).
    * `ron-app-sdk` (DX-first SDKs exposing Micronode KV and other APIs as first-class “planes” to apps).
  * Act as the **single-tenant profile** in the broader Micronode/Macronode story:

    * Micronode = local, small, nearly stateless, easy to host.
    * Macronode = full overlay/DHT/reg+edge stack.

---

## 7) Handy Dev Commands (Reconfirmed)

For reference when re-entering Micronode work later:

```bash
# Format + clippy strict
cargo fmt -p micronode
cargo clippy -p micronode --no-deps -- -D warnings

# Run full test suite
cargo test -p micronode

# Run key focused tests
cargo test -p micronode --test admin_parity
cargo test -p micronode --test kv_roundtrip
cargo test -p micronode --test guard_behavior

# Dev run with dev routes enabled
MICRONODE_DEV_ROUTES=1 cargo run -p micronode
```

---

## 8) Summary

Micronode is now **far past “toy”**:

* It boots cleanly, reports truthful readiness, enforces ingress invariants, and exposes a practical KV surface.
* It has **real tests** for admin parity, KV behavior, and guard behavior.
* HTTP metrics are wired and validated.

The remaining work to reach “strict HTTP + KV Beta” is **well-scoped and incremental**:

1. Storage config + sled engine wiring.
2. KV domain metrics.
3. Perf benches + baselines.
4. (Optional) PQ/amnesia toggles wired to config.

From here, future sessions can either:

* Push these remaining items to call Micronode “Beta” with persistence, or
* Start integrating Micronode with `ron-app-sdk` and the facet story as **the default single-tenant node**.


### END NOTE - NOVEMBER 16 2025 - 11:27 CST