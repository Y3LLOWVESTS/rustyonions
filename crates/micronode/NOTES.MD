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