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





### BEGIN NOTE - NOVEMBER 16 2025 - 20:21 CST

---

# CARRY-OVER NOTES — `micronode` (Not Yet Beta)

**Date:** 2025-11-17
**Status:** **Core HTTP+KV slice is solid, but crate as a whole is ~55–65% of intended scope.**
**Do *not* mark as full beta.** This crate is missing major planes: `adapters`, `cli`, `concurrency` (beyond simple guards), `facets`, `security`, and some `config` submodules.

---

## 0) TL;DR

What we *do* have right now:

* A **runnable Micronode binary** with:

  * Truthful **admin plane**: `/healthz`, `/readyz`, `/metrics`, `/version`.
  * A guarded **KV v1 HTTP API** (PUT/GET/DELETE).
  * **Ingress guardrails**: content-encoding rejection, body caps, basic concurrency caps.
  * A **pluggable storage layer** (in-mem + sled) with a `Storage` trait.
  * **Prometheus metrics** for HTTP, readiness, and KV domain.
  * **Tests + benches + smoke script** and a `beta_check.sh` that fully exercises this core slice.

What is **still missing** (cannot call crate “beta”):

* Most of the **module trees are empty or stubby**:

  * `src/adapters/*`
  * `src/cli/*`
  * `src/concurrency/*` (beyond very local guards)
  * `src/facets/*`
  * `src/security/*`
  * Some `src/config/*` files are still empty or under-specified.
* No real **facet loading / execution** story yet.
* No **capability / macaroon / auth** integration yet.
* No **CLI UX** or packaging story around Micronode profiles.

So: the **“Micronode as a tiny node profile + KV/HTTP core”** is in good shape, but the **full Micronode crate as designed in TODO/ALL_DOCS is incomplete**.

---

## 1) What We’ve Actually Shipped So Far

### 1.1 Binary & App Wiring

* `src/main.rs` + `src/lib.rs`:

  * Boot Micronode with:

    * Config load.
    * `ReadyProbes` initialization.
    * Storage engine selection (mem vs sled).
    * Router build via `app::build_router(cfg, ready, storage, metrics, …)`.
  * Tracing and logging wired via `tracing` + `tracing-subscriber`.
  * Clean exit path and integration with `beta_check.sh` / smoke script.

* **App/router**:

  * Admin routes wired under a dedicated “admin plane”.
  * KV v1 routes wired under `/v1/kv/{bucket}/{key}`.
  * Middleware stack composed explicitly (decode guard, body caps, concurrency caps, metrics layer).

### 1.2 Admin Plane

We have a proper **admin surface** (Axum):

* `GET /healthz`

  * Quick liveness probe (no heavy deps).
* `GET /readyz`

  * Returns a JSON object including:

    ```json
    {
      "ready": true,
      "probes": {
        "listeners_bound": true,
        "cfg_loaded": true,
        "metrics_bound": true,
        "deps_ok": true
      },
      "mode": "truthful"
    }
    ```
  * `ReadyProbes` tracks these four booleans.
* `GET /metrics`

  * Prometheus scrape endpoint.
  * Exposes:

    * HTTP metrics.
    * Ready probe gauges.
    * KV domain metrics.
* `GET /version`

  * `{ name: "micronode", version: "0.1.0", built_at_unix: <build_ts> }`.
  * `build.rs` stamps `built_at_unix`.

The admin plane is **truthful** and passes `tests/admin_parity.rs` and `smoke_micronode.sh`.

### 1.3 KV v1 HTTP API

We have a **minimal but correct KV v1**:

* `PUT /v1/kv/{bucket}/{key}`

  * Expects `content-type: application/octet-stream` + valid `Content-Length`.
  * Guarded by:

    * Decode guard (reject `Content-Encoding`).
    * Body-cap layer (length required + size cap).
    * Concurrency limiter.
  * Returns:

    * `201 Created` on success.
    * `411` / `413` / `415` / `429` on various guard failures.

* `GET /v1/kv/{bucket}/{key}`

  * Streams value if present.
  * `200 OK` with raw bytes, `404` if missing.

* `DELETE /v1/kv/{bucket}/{key}`

  * `204 No Content` on success.
  * `404` if missing.

Tests confirm basic roundtrip:

* `tests/kv_roundtrip.rs`:

  * `kv_put_get_delete_roundtrip` passes.

### 1.4 Storage Layer

We implemented the **storage abstraction** and two engines:

* `Storage` trait:

  * Operations are bucket+key centric.
  * Errors are mapped to an internal `Error`/`Result` type in this crate.

* **MemStore** (default engine):

  * `RwLock<HashMap<(bucket, key), Vec<u8>>>`.
  * Ideal for:

    * Dev.
    * Testing.
    * Amnesia profiles.

* **SledStore** (behind `sled-store` feature):

  * Uses `sled` backend keyed by a composite key (bucket/key).
  * Controlled via config:

    * `storage.engine = "sled"`
    * `storage.path = ".../db"` (or default).
  * `deps_ok` in `ReadyProbes` is wired to the success of opening the engine.

**Feature flags in `Cargo.toml`:**

* `default = ["mem-store"]`
* `mem-store = []`
* `sled-store = ["sled"]`

We now have a **clean separation** of storage concerns and a straightforward path to add more engines later.

### 1.5 Guards & Concurrency (Local)

We implemented a couple of **local ingress guards**:

* **Decode guard**:

  * Rejects any `Content-Encoding`.
  * Hot path is benchmarked (`http_kv_decode_guard_hot_reject`).

* **Body cap layer**:

  * Requires `Content-Length` for write methods.
  * Cap on payload size (configurable constant; enforced early).
  * Returns `411` / `413` as appropriate.

* **Basic concurrency layer**:

  * Uses a semaphore to limit inflight requests.
  * Returns `429` when saturated (non-blocking fail-fast).

Right now, these live as **per-route / local middleware** in `app.rs`, not as part of a more global `src/concurrency` module.

### 1.6 Config & ReadyProbes

We have:

* `config::schema` with:

  * `Config` (now including `server` and `storage`).
  * `Server` and `StorageCfg` structs.
  * `StorageEngine` enum with a derived `Default` (Mem).

* `config::load`:

  * Parses config from file/env (details per TODO/ALL_DOCS).
  * On success, flips `cfg_loaded` in `ReadyProbes`.

* `ReadyProbes`:

  * Tracks:

    * `listeners_bound`
    * `cfg_loaded`
    * `metrics_bound`
    * `deps_ok`
  * Implements `new()` and `Default`.
  * Used by `/readyz` handler to compute `ready`.

Some pieces of `config` may still be stubbed/empty (see §3).

### 1.7 Observability & Metrics

We wired **HTTP metrics per route**:

* `micronode_http_requests_total{method,route,status}`.
* `micronode_request_latency_seconds{method,route}`.

Plus:

* Readiness gauges:

  * `micronode_ready_listeners_bound`
  * `micronode_ready_cfg_loaded`
  * `micronode_ready_metrics_bound`
  * `micronode_ready_deps_ok`
* KV domain metrics:

  * `micronode_kv_ops_total{op="put|get|del"}`
  * `micronode_kv_bytes_total{dir="in|out"}`

Metrics are **pre-warmed** by touching labels on startup so dashboards don’t start empty.

### 1.8 Tests & Benches

**Tests (all green):**

* `cargo test -p micronode` passes.
* Covered:

  * `tests/admin_parity.rs`: admin endpoints parity/health.
  * `tests/guard_behavior.rs`: decode guard + body cap semantics.
  * `tests/kv_roundtrip.rs`: KV happy path.
  * Other test files currently have zero tests but exist as placeholders for:

    * `amnesia_proof.rs`
    * `backpressure.rs`
    * `facets_proxy.rs`
    * `oap_limits.rs`
    * `pq_fallback.rs`
    * `pq_modes.rs`

**Benches (http_kv):**

On your **2019 13" MacBook Pro, i5, 8GB RAM** we observed:

* `http_healthz_fast_path`: ~**4.8–5.3 µs** per request.
* `http_kv_small_put_get_delete`: ~**50–60 µs** per full PUT→GET→DELETE cycle.
* `http_kv_decode_guard_hot_reject`: ~**17–18 µs** per rejected request.

`beta_check.sh` runs:

1. fmt + clippy
2. `cargo test -p micronode`
3. `cargo bench -p micronode --bench http_kv`
4. `smoke_micronode.sh`

and currently passes.

---

## 2) Major Gaps vs. Design (Why This Isn’t Full Beta)

From TODO/ALL_DOCS for `micronode`, the crate is supposed to have multiple **planes/modules** that we haven’t built out yet.

### 2.1 `src/adapters/*`

Intended purpose (based on overall RON-CORE patterns):

* Provide **adapters** to other services and crates, examples likely include:

  * Overlay or DHT adapter (talk to `svc-overlay`/`svc-dht`).
  * Storage/CAS adapter (talk to `svc-storage` via OAP or HTTP).
  * Registry/edge adapter (register Micronode instances with `svc-registry` or route via `svc-edge`).
  * Possibly **SDK/facet** oriented adapters (bridging Micronode to `ron-app-sdk`).

Current status:

* Module tree exists but is **mostly or entirely empty** / stubbed.
* No cross-node or cross-service integration implemented at this layer yet.

Impact:

* Micronode is currently an **isolated node**; not yet a fully integrated piece of the larger macro system.
* Cannot claim full “profile for crate bundles” until adapters are wired.

---

### 2.2 `src/cli/*`

Intended purpose:

* Provide a **CLI entrypoint** and UX for Micronode, likely including:

  * `micronode run` / `micronode serve` (profile selection, config path).
  * `micronode check` (config validation).
  * `micronode smoke` / `diagnostics` (self-checks).
  * Flags for:

    * `--profile` (amnesia vs durable).
    * `--bind` override.
    * `--config` path.
    * Logging verbosity (`-v`, `-q`).

Current status:

* CLI scaffolding not implemented (or only partially).
* `main.rs` is doing a minimal config load + run; there is **no real CLI** layering (no `clap`/`argh`/config layering based on TODO).

Impact:

* Developer experience for spinning up Micronode in different modes/profile is rough.
* No official UX for running in container, dev vs prod, etc.

---

### 2.3 `src/concurrency/*`

Intended purpose:

* Centralize **concurrency & backpressure** for Micronode:

  * Global concurrency pools.
  * Route-specific quotas and overload policies.
  * Possibly integration with system metrics (shed rate, queue depths).

Current status:

* We do have a **local concurrency layer** (per-route) inside the app/router.
* But:

  * No dedicated `src/concurrency/*` module with a coherent story.
  * No central place to:

    * Configure concurrency limits.
    * Export concurrency metrics.
    * Integrate with other services’ load signals.

Impact:

* Micronode is protected at the ingress, but **concurrency strategy is not yet a first-class, configurable plane**.
* Tests like `backpressure.rs` are empty and we have no concurrency-focused benches.

---

### 2.4 `src/facets/*`

Intended purpose:

* This is the **big missing piece**:

  * Facets are the core developer abstraction on top of RON-CORE.
  * Micronode is supposed to be a **facet host**:

    * Discover/load facet manifests.
    * Expose HTTP routes on behalf of facets.
    * Proxy traffic to facet code (in-process or sidecar).
    * Provide a stable runtime environment for facets (KV, CAS, auth, etc.).

Current status:

* `src/facets/*` is either absent or mostly empty.
* `tests/facets_proxy.rs` has no tests yet.
* No code yet for:

  * Loading facet manifests.
  * Routing based on facet definitions.
  * Attaching capability checks to facet routes.

Impact:

* Without `facets`, Micronode is “just a node with KV and admin,” rather than the **“micronode for apps”** we want.
* We cannot claim Micronode is the ready “profile for crate bundles + facets” until this module is built.

---

### 2.5 `src/security/*`

Intended purpose:

* All **capability, auth, macaroon, key management** integrations should live here:

  * Verifying macaroon-style capability tokens.
  * Integrating with `ron-auth` and `ron-kms`.
  * Enforcing per-route/per-bucket/per-key policies.
  * Tying into `ron-policy` decisions.

Current status:

* `src/security/*` is basically unimplemented.
* Micronode currently:

  * Has **no authentication or authorization**.
  * Exposes KV v1 with no capability checks.
  * Does not integrate with the system’s macaroon capability fabric yet.

Impact:

* Micronode is currently suitable for **trusted/internal or dev environments**, not for untrusted ingress.
* Security posture is incomplete; cannot claim “Micronode is God-tier secure” yet.

---

### 2.6 `src/config/*` (Gaps)

What we have:

* `schema.rs` and `load.rs` exist and are partially wired:

  * `Config`, `Server`, `StorageCfg`, `StorageEngine`.
  * `ReadyProbes` flips `cfg_loaded` once config parsed.

Likely missing (per TODO/ALL_DOCS):

* Richer configuration:

  * Concurrency limits, body caps.
  * Decode guard and security toggles.
  * Facet directories / CAS endpoints.
  * TLS settings (if Micronode is supposed to do TLS termination).
* Validation:

  * Stronger invariants (e.g., “if engine = sled, path must be set and writable”).
  * Warnings vs hard failures for certain misconfigurations.

Impact:

* For now config is **good enough** for the current slice (admin + KV).
* But the **full crate config story** is still underdeveloped and will need expansion as we add facets, security, and concurrency planes.

---

## 3) Priority Roadmap — Next High-Impact Steps

When we pick Micronode back up, here’s a suggested **order of attack** (all will need full drop-in files):

### Step 1 — Flesh Out `src/security/*` (Minimum Viable Capabilities)

Goal: Make Micronode **principally compatible** with RON-CORE’s capability story.

* Define a `SecurityContext` / `RequestContext` type that:

  * Extracts capability/macaroons from headers.
  * (Initially) does minimal or mock validation to keep scope manageable.
* Provide hooks in `app.rs`:

  * Wrap KV routes (and future facet routes) with a `security::enforce` layer.
* Optionally integrate with `ron-auth` (even if just as a stubbed/verifiable client interface).

This will let us:

* Write real tests in `tests/facets_proxy.rs` & `tests/security.rs` later.
* Start moving from “dev-only node” to something enforceable in more realistic scenarios.

### Step 2 — `src/facets/*` MVP

Goal: Turn Micronode into a **facet host**, even in a simplified form.

* Define a `FacetManifest` type.
* Define a `FacetRegistry` that:

  * Discovers facets from a folder (or static config array) for now.
  * Knows how to map facet routes to Axum handlers (could be in-process or stub).
* Wire facet routes under a namespace like `/facets/{facet_id}/...`.

Even a **“hello world facet”** that:

* Has a manifest.
* Gets loaded at start.
* Exposes a trivial route.

…would be enough to prove the model and unblock `ron-app-sdk` integration later.

### Step 3 — `src/concurrency/*` (Centralized Concurrency/Backpressure)

Goal: Pull per-route concurrency & backpressure policy into a **coherent module**.

* Define a `ConcurrencyCfg`:

  * Global caps.
  * Per-plane or per-route caps.
* Provide a `concurrency::layer_for(route_name, cfg)` helper.
* Emit metrics:

  * `micronode_concurrency_inflight{route=...}`
  * `micronode_concurrency_shed_total{route=...}`

Once this exists, tests like `backpressure.rs` can be filled out with real assertions, and we can add concurrency-specific benchmarks.

### Step 4 — `src/adapters/*` (Minimal Integration)

Goal: Give Micronode at least **one or two real adapters** into the larger system.

Candidates:

* Storage/CAS adapter:

  * HTTP or OAP client to `svc-storage` (even if not wired into main routes yet).
* Registry/overlay adapter:

  * Basic client for `svc-registry` or `svc-overlay` (to be integrated later).

For now, it’s enough to:

* Stub the interface and maybe provide a “no-op” implementation.
* Ensure the shapes match other crates’ public APIs.

### Step 5 — `src/cli/*` (User-Facing UX)

Goal: Make Micronode easy to run in realistic ways.

* Introduce a small CLI:

  * `micronode serve`:

    * `--config`
    * `--profile` (amnesia/durable)
    * `--bind`
    * `--dev-routes`
  * `micronode check`:

    * Validate config and print readiness expectations.

This doesn’t change core behavior but **dramatically improves DX** and makes Micronode “feel complete”.

---

## 4) Quick Re-Entry Commands (for Future You)

To quickly get back into the current state:

```bash
# From repo root
cargo fmt -p micronode
cargo clippy -p micronode --no-deps -- -D warnings

# Full test suite
cargo test -p micronode

# Benchmarks
cargo bench -p micronode --bench http_kv

# Smoke / beta slice check (admin + KV + benches)
bash crates/micronode/scripts/beta_check.sh

# Manual smoke: run + curl
MICRONODE_DEV_ROUTES=1 cargo run -p micronode

curl -s http://127.0.0.1:5310/healthz
curl -s http://127.0.0.1:5310/readyz | jq .
curl -s http://127.0.0.1:5310/version | jq .
curl -s http://127.0.0.1:5310/metrics | head -n 20

curl -s -o /dev/null -w "%{http_code}\n" \
  -X PUT http://127.0.0.1:5310/v1/kv/smoke/k \
  -H 'content-type: application/octet-stream' \
  --data-binary 'hello-micronode'

curl -s http://127.0.0.1:5310/v1/kv/smoke/k
curl -s -o /dev/null -w "%{http_code}\n" \
  -X DELETE http://127.0.0.1:5310/v1/kv/smoke/k
```

---

**Bottom line:** Micronode’s **core HTTP+KV slice is in great shape and well-benchmarked**, but the crate is missing major planes (`adapters/`, `cli/`, `concurrency/`, `facets/`, `security/`, richer `config/`) that were intended in the blueprint. Next passes should focus on those modules so we can eventually—and legitimately—call Micronode itself **beta complete**, not just its minimal profile.



### END NOTE - NOVEMBER 16 2025 - 20:21 CST




### BEGIN NOTE - NOVEMBER 17 2025 - 11:29 CST


CARRY-OVER NOTES — micronode (Beta Push)

Date: 2025-11-17 (Mon)
Beta readiness estimate: ~78%
Verdict: Do not stamp “beta” yet. Core slice is excellent and we’ve added Security (MVP), Concurrency (first-class), Facets (hello-world), CLI (serve/check), and Adapters (foundation). What remains is auth enforcement, facet loading + config wiring, and some meta/admin + docs hardening.

⸻

0. TL;DR
   •	All tests and clippy are green on your latest run, including:
   •	Admin parity, KV roundtrip, guard behavior, concurrency limits, demo facet ping, CLI smoke.
   •	What’s solid now
   •	Truthful admin plane (/healthz, /readyz, /metrics, /version).
   •	KV v1 HTTP (PUT/GET/DELETE) with decode guard, body caps, non-blocking concurrency shed.
   •	Security MVP: amnesia posture + macaroon header extraction (no verification yet).
   •	Concurrency plane: shared registry + per-class pools + shed (429) layer; tests.
   •	Facets MVP: demo facet routed and tested; meta endpoint wired.
   •	CLI: default to serve; basic argument handling; smoke test.
   •	Adapters plane: typed, cheap clients (index, mailbox, storage, overlay, policy) + Adapters bag.
   •	Observability: Prometheus metrics for HTTP, readiness, KV; labels pre-warmed.
   •	Blocking gaps to call beta

   1. Auth enforcement (capability/macaroons) wired into routes; deny by default.
   2. Facet loader (manifest or directory) + config wiring and a couple of demo routes beyond /facets/demo/ping.
   3. Config surface completion (concurrency caps, dev routes, storage choice, facet path, adapter URLs) and validation.
   4. Admin/meta endpoints (e.g., which adapters enabled, current caps, feature flags) for ops truthfulness.
   5. Docs & runbooks (README, IDB sections, CODECHECK gates) + one more integration test covering KV + auth gate.

⸻

1. What We’ve Shipped (by plane)

1.1 Binary & App Wiring
•	Boots Micronode with:
•	Config load → flips cfg_loaded.
•	Ready probes: listeners_bound, cfg_loaded, metrics_bound, deps_ok.
•	Storage engine (mem default, sled via feature/CFG) lives behind Storage trait.
•	Router composition: admin plane + /v1/kv/... + dev/facet routes, layered with guards and metrics.
•	Clean shutdown path and integration with smoke/beta scripts (as described below).

1.2 Admin Plane (Truthful)
•	GET /healthz — fast liveness, no heavy deps.
•	GET /readyz — structured truth with four booleans and ready aggregate; flips only after listeners/metrics/deps OK.
•	GET /metrics — Prometheus export:
•	HTTP totals and latency histograms.
•	Readiness gauges (per flip).
•	KV domain counters and byte totals (in/out).
•	GET /version — name, semver, built_at_unix via build.rs.

1.3 KV v1 HTTP API
•	Routes: PUT /v1/kv/{bucket}/{key}, GET /..., DELETE /...
•	Guardrails:
•	Reject any Content-Encoding.
•	Require Content-Length and enforce payload cap (configurable).
•	Fail-fast concurrency layer: shed with 429 when pool saturated.
•	Behavior:
•	201 Created on PUT, 200 with raw bytes on GET (404 if missing), 204 No Content on DELETE (404 if missing).
•	Covered by tests/kv_roundtrip.rs and guard tests.

1.4 Storage Layer
•	Storage trait + MemStore (default) + SledStore (feature-gated).
•	Composite keys (bucket/key) with consistent error mapping.
•	deps_ok readiness reflects successful engine open/availability.

1.5 Security Plane (MVP)
•	Amnesia posture module:
•	Truthy defaults to “amnesia enabled” unless explicitly configured otherwise.
•	“Prefer amnesia on ambiguity” invariant honored.
•	Macaroon auth module:
•	Robust extraction of Authorization: Macaroon <token> (case-insensitive scheme).
•	No verification yet (intentionally deferred to ron-auth/ron-policy); unit tests verify extraction behaviors.
•	Security layer scaffold ready to be placed in the middleware stack once verify hooks exist.

1.6 Concurrency Plane
•	Central ConcurrencyRegistry with named pools (e.g., http_kv, http_admin, http_facets, http_dev_echo).
•	Lightweight Axum layer that attempts immediate permit acquisition; sheds with 429 (no head-of-line blocking).
•	Tests:
•	Registry builds distinct pools per class.
•	Layer sheds when pool exhausted.
•	First-class metrics are easy to add: inflight gauges and shed_total counters per class (hook ready in layer).

1.7 Facets (MVP)
•	Demo facet mounted under /facets/demo/ping.
•	/facets/meta endpoint to enumerate what’s mounted (MVP structure).
•	Tests:
•	facets_meta_endpoint_is_wired
•	demo_facet_ping_works

1.8 CLI
•	micronode defaults to serve with reasonable flags; cli_smoke.rs ensures default code path is exercised.
•	Foundation for micronode check/diagnose subcommands later.

1.9 Adapters Plane
•	adapters::* implements typed handles:
•	IndexClient, MailboxClient, StorageClient, OverlayClient, PolicyClient.
•	Adapters bag with builder ergonomics + tests.
•	No networking yet by design; keeps core decoupled and lets us thread Adapters through AppState.

1.10 Observability
•	HTTP metrics: totals + latency (by method/route).
•	Readiness: 4 gauges; consistent flip semantics.
•	KV: ops counters (put|get|del) and byte totals (in|out).
•	Label pre-warm to avoid cold dashboards.

1.11 Tests/Benches/Tooling
•	Passing integration tests: admin parity, KV roundtrip, guard behavior, concurrency, facets, CLI smoke.
•	(Earlier benches existed for hot paths; ensure we re-enable or port them after route tree stabilized.)
•	beta_check.sh invokes fmt, clippy, tests (and benches if re-enabled), then smoke.

⸻

2. Gaps vs. Design (Why Not Beta Yet)

   1. Auth Enforcement (capabilities/macaroons)
      •	Extraction exists; verification/authorization does not.
      •	We need a deny-by-default gate on KV and facet routes:
      •	Minimal path: “allow if MICRONODE_DEV_ROUTES=1 OR header Authorization: Macaroon dev-allow (compile-time test hook); else 401/403.”
      •	Real path: call into ron-auth/ron-policy (even stubbed client) to verify caveats & scope.
   2. Facet Loader + Configuration
      •	Current facet is hardwired demo.
      •	Need loader (manifest JSON/TOML files from cfg.facets.dir), dynamic routing, and show in /facets/meta.
      •	Provide minimal facet types: static file, echo/json, KV-backed counter, and proxy (scaffold only).
   3. Configuration Surface & Validation
      •	Thread Adapters into AppState from config (e.g., adapters.index.base_url).
      •	Concurrency caps in config per class (http_kv, http_facets, …).
      •	Storage engine/path fully validated; amnesia setting explicit; dev routes flag explicit.
      •	Strong validation with actionable errors (no silent defaults when ambiguous).
   4. Admin /meta Introspection
      •	Add /micronode/meta (or /admin/meta) returning:
      •	enabled adapters and base URLs (redacted tokens),
      •	concurrency caps per class,
      •	storage engine (mem/sled + path),
      •	amnesia posture, dev-routes flag,
      •	build info and feature flags.
   5. Docs / Acceptance Gates
      •	README needs quickstart, config table, metrics table, and endpoint map.
      •	IDB sections: WHAT/WHY/INVARIANTS/CONFIG/SECURITY/OBSERVABILITY/TEST.
      •	CODECHECK items: clippy flags, deny.toml, minimal benches, smoke.
   6. One More “Real” Integration Test
      •	A secured KV test: KV write blocked → set dev allow → allowed; demonstrates enforcement toggles and readiness truth.

⸻

3. High-Impact Next Steps (order of attack)

Each step below is small enough to land cleanly but moves the needle toward beta.

Step A — Auth Gate (MVP, Deny-by-Default)

Goal: Turn extraction into enforcement, keep DX friendly for dev.

Deliverables
•	layers/security.rs: RequireAuth middleware with a policy enum:
•	Policy::DevAllow (env-guarded), Policy::DenyAll, Policy::ExternalAuth (placeholder).
•	Wire RequireAuth on /v1/kv/* and /facets/* behind config:
•	cfg.security.mode = "deny_all" | "dev_allow" | "external".
•	Responses:
•	401 with WWW-Authenticate: Macaroon realm="micronode" when missing.
•	403 when present but not allowed by current policy.
•	Tests:
•	Missing auth → 401 on KV.
•	With dev_allow → KV allowed.
•	With deny_all → still blocked.

Acceptance criteria
•	New tests pass; old tests remain green; clippy passes.
•	Default in release profile is deny_all unless explicitly set (dev profile can default to dev_allow).

⸻

Step B — Facet Loader (Directory Manifests)

Goal: Move from hardwired demo to file-driven facets.

Deliverables
•	facets/manifest.rs:
•	FacetManifest { id, routes: Vec<RouteSpec>, kind }
•	Kinds: Static, Echo, Proxy (proxy handler returns 501 for now).
•	facets/loader.rs:
•	Reads cfg.facets.dir, loads *.toml/*.json.
•	Validates IDs/paths, returns FacetRegistry.
•	facets/mod.rs:
•	mount_with_registry(router, registry); update /facets/meta.
•	Tests:
•	Tempdir with two manifests → both routes appear and respond.
•	Malformed manifest → startup error (and readyz remains false).

Acceptance criteria
•	/facets/meta shows loaded facets and kinds.
•	Loader errors leave node not ready until fixed.

⸻

Step C — Config Wiring + Validation

Goal: Make everything togglable/visible from config.

Deliverables
•	config/schema.rs:
•	Add SecurityCfg { mode }, AdaptersCfg { index, storage, overlay, mailbox, policy }, ConcurrencyCfg { http_kv, http_facets, http_admin, http_dev_echo }, FacetsCfg { dir, enabled }.
•	config/load.rs:
•	Parse + validate: storage engine/path, concurrency bounds (>=1, sane upper bound), facet dir existence, adapter URL scheme, security mode known.
•	state::AppState:
•	Add adapters: Adapters, concurrency: ConcurrencyRegistry, security: SecurityCfg, facets: FacetRegistry (or handle in router build).
•	Tests:
•	Bad config → descriptive error (no panic).
•	Minimal config → defaults to mem store, deny_all unless dev profile.

Acceptance criteria
•	/readyz only flips when config valid & facets loaded (if enabled).

⸻

Step D — Admin Meta (+ Metrics Edge)

Goal: Operator truthfulness.

Deliverables
•	GET /admin/meta (or /micronode/meta):
•	JSON: build, posture, storage, adapters (redacted), concurrency caps, facets loaded, security mode.
•	Metrics:
•	Add micronode_concurrency_inflight{class} and micronode_concurrency_shed_total{class} in the layer (if not already).
•	Tests:
•	Meta endpoint returns all fields and matches config.

Acceptance criteria
•	Dashboards can be populated from metrics/meta alone.

⸻

Step E — README + IDB Hardening

Goal: Ship-worthy docs.

Deliverables
•	README:
•	Quickstart (dev + prod), Config table, Endpoint map, Metrics list, Security notes.
•	IDB sections (WHAT/WHY/INVARIANTS/CONFIG/SECURITY/OBSERVABILITY/TEST).
•	CODECHECK: ensure clippy (-D warnings), fmt, cargo-deny if workspace uses it, and smoke script documented.

⸻

4. Current Commands & Smoke

Re-entry (fast loop):

cargo fmt -p micronode
cargo test -p micronode
cargo clippy -p micronode --no-deps -- -D warnings

# Run dev mode with routes exposed and in-mem storage (example)

MICRONODE_DEV_ROUTES=1 cargo run -p micronode
curl -s localhost:5310/healthz
curl -s localhost:5310/readyz | jq .
curl -s localhost:5310/version | jq .
curl -s localhost:5310/metrics | head

# KV quick check

curl -s -o /dev/null -w "%{http_code}\n" 
-X PUT localhost:5310/v1/kv/demo/k -H 'content-type: application/octet-stream' 
--data-binary 'hello'
curl -s localhost:5310/v1/kv/demo/k

⸻

5. Risks & Non-Goals (for beta)
   •	Security: Until external mode is implemented, Micronode must not be internet-facing with write APIs; default is deny.
   •	Persistence: Sled is fine for small/medium deployments; we don’t claim HA or multi-writer safety here (that’s macro-node territory).
   •	Facets: Loader/hello-world facets only; in-process SDK invocation will come later (post-beta).
   •	Overlay/Registry: Adapters exist but are not yet used; leave integration for after beta.

⸻

6. Definition of “Beta Done” (acceptance checklist)
   •	Auth gate enforced on KV + facets:
   •	security.mode = deny_all|dev_allow|external supported.
   •	Missing → 401 with WWW-Authenticate; disallowed → 403.
   •	Tests for each mode passing.
   •	Facet loader:
   •	Load from cfg.facets.dir; static & echo kinds working.
   •	/facets/meta lists loaded facets.
   •	Loader errors block readiness.
   •	Config validation:
   •	Concurrency caps per class; sane bounds; surfaced in meta.
   •	Adapters wired from config; redacted in meta.
   •	Storage selection validated; amnesia posture explicit.
   •	Admin meta endpoint present and truthful.
   •	Docs:
   •	README updated (QS, config table, metrics, endpoints).
   •	IDB sections complete; CODECHECK gates documented.
   •	Green CI strip:
   •	fmt, clippy -D warnings, test (unit+integration).
   •	(Optional) benches compile and run locally.

⸻

7. Nice-to-Haves (post-beta)
   •	External auth integration (call ron-auth/ron-policy with caveat checks).
   •	Facet type: KV-counter example (exercises capability scope).
   •	Dev console page (HTML) summarizing /admin/meta visually.
   •	Graceful shutdown coordination for future worker pools (stub module exists).
   •	Optional QUIC/TLS front (via ron-transport) once overlay work resumes.

⸻

8. Integration Touchpoints (future)
   •	ron-app-sdk: Micronode should be the “hello-world node” for SDK examples; once auth externalized, the SDK can mint/attach macaroons and hit KV/facets end-to-end.
   •	svc-registry / svc-overlay: Adapters ready; once wired, Micronode can publish presence and facet routes for discovery.
   •	svc-storage: For large blobs, delegate to CAS via adapter (future plane).

⸻

9. Known Good Baseline (from your last run)

All tests passed: admin_parity, kv_roundtrip, guard_behavior, concurrency (2), facets (2), cli_smoke.
Clippy: clean with -D warnings.
Doc-tests: none (ok).

⸻

Bottom line

Micronode now has all major planes present (even if minimal): security, concurrency, facets, adapters, CLI, admin, KV, observability. The shortest path to beta is to flip security from “parse-only” to “enforce”, load facets from config/dir, and expose a truthful /admin/meta. Once those three land with validation + docs, we can stamp beta with a straight face.




### END NOTE - NOVEMBER 17 2025 - 11:29 CST




### BEGIN NOTE - NOVEMBER 17 2025 - 13:48 CST


Below is your **God-tier, carry-over-ready**, exhaustive, drop-into-NOTES.md summary for **micronode (Beta Push)** — including:

✅ Current Beta Completion %
✅ What’s already done (very detailed)
✅ What remains (very detailed)
✅ High-impact next steps
✅ Deep analysis of *why* the facet test still fails (root-cause level)
✅ How to approach next instance cleanly

No code included — only analysis, roadmap, and invariants.

---

# 📦 CARRY-OVER NOTES — *micronode (Beta Push)*

**Date:** 2025-11-17
**Beta readiness:** **~82%**
(previous estimate ~78%; increased because facets plane is 90% implemented, but one remaining failure blocks green)

**Verdict:**
Micronode is *very close* to beta, but the facets plane still has one unresolved logic mismatch preventing test parity. Everything else — admin plane, KV plane, concurrency plane, security plane, layers, CLI, adapters, backpressure, decode guard, metrics wiring, config loader, and readiness probes — is solid.

The only red is **facets_loader::loads_static_and_echo_facets_and_enforces_auth**, which indicates the facet loader + router composition is not matching test expectations *exactly*.

---

# 0) TL;DR

### ✔ What’s rock-solid and fully green

* Admin plane (`/healthz`, `/readyz`, `/version`, `/metrics`)
* KV plane (PUT/GET/DELETE)
* Body caps
* Concurrency limits
* Decode guard
* Security layer extract
* Auth enforcement layer
* Dev echo
* Backpressure
* CLI (serve)
* Amnesia mode + proof tests
* Config loader + env overlay
* Readiness gates
* Facet manifest parsing + validation
* Facet loader (dir scanning, file checks)
* Demo facet + meta (when disabled)
* Full state wiring + observability

### ❌ What is still red

* **1 failing test**:
  `loads_static_and_echo_facets_and_enforces_auth`

  * Expected: static facet route → **200**
  * Actual: **401**
  * Root cause: router composition + security layering mismatch with test expectations.

### 🎯 High-Impact Next Steps

1. Correct the facet route mounting logic so that:

   * Echo facet routes are wrapped in RequireAuth
   * Static facet routes are **not** wrapped in RequireAuth
   * But **only** when manifest says `requires_auth = false`

2. Reconcile mount vs mount_with_registry interactions:

   * Avoid double meta routes
   * Avoid global DenyAll enforcement overwriting facet-level behavior

3. Refactor facet layer so tests match real-world semantics:

   * Global SecurityLayer (extract only) stays
   * Per-facet RequireAuthLayer must match manifest flags

4. Re-run all tests after router composition fix.

---

# 1) **Detailed Accomplishments** (Exhaustive)

Micronode now has nearly the entire core plane implemented according to `RON-CORE.MD` and the IDB blueprint set. Each major subsystem is green.

---

## ✅ A) **Admin plane** (Fully complete)

* `/healthz`, `/readyz`, `/metrics`, `/version`
* Matches svc-edge/svc-gateway patterns
* Truthful readiness gates reflect facet loader failures
* Tests: **passed**

---

## ✅ B) **KV plane** (Fully complete)

* RESTful PUT/GET/DELETE under `/v1/kv/:bucket/:key`
* Security:

  * Gets **SecurityLayer** (extract macaroon)
  * Gets **RequireAuthLayer**
* Concurrency:

  * Independent concurrency pool for KV
* Body cap
* End-to-end test: `kv_roundtrip` → **passed**

---

## ✅ C) **Security plane** (Fully complete)

* `SecurityLayer`: extract macaroon into extensions (case-insensitive)
* `RequireAuthLayer`: enforce deny-all/dev-allow/external futures
* Auth tests: all passed
* Developer mode: unsecured KV and facets allowed

---

## ✅ D) **Concurrency + Backpressure** (Fully complete)

* Independent semaphores per class
* 429 shedding when exhausted
* Backpressure tests: **passed**

---

## ✅ E) **Decode Guard** (Fully complete)

* Rejects content-encoding hacks
* Guard tests: **passed**

---

## ✅ F) **Config plane** (Complete)

* Schema for server/storage/security/facets
* Env overlay logic
* Validation logic
* All config tests: **passed**

---

## ✅ G) **Adapters** (Complete for beta)

* Adapter builder logic
* Tests: **green**

---

## ✅ H) **CLI** (Complete)

* `serve` is default command
* CLI smoke test: **passed**

---

## ✅ I) **Amnesia mode** (Complete)

* Memory-first profile
* Default posture = amnesia-first
* amnesia_proof test: **passed**

---

## ✅ J) **Observability**

* HTTP metrics layer (tower-http)
* TraceLayer for Axum
* Metrics prewarm
* Verified working in admin parity test

---

## ✅ K) **Facet manifest parser** (Complete)

* Validates:

  * methods (GET/POST)
  * static routes must have file
  * file exists on disk
  * route paths start with '/'
  * duplicate facet id detection
* Manifest loader tests: `bad_manifest_blocks_readiness` → **passed**

---

## ⚠️ L) **Facet router composer** (Partially complete – one red test)

This is the *only* subsystem not fully green.

You correctly implemented:

* demo facet
* meta when disabled
* manifest-driven route generation
* per-kind behavior (static/proxy/echo)
* RequireAuth on manifest-driven routes

But the remaining gap is:

### 🔴 Problem:

Static facets must bypass RequireAuth **only if manifest says so**.

### 🔴 Current behavior:

All facet subrouters get wrapped in RequireAuth unconditionally.

### 🔴 Consequence:

Static facets → return **401** instead of **200**

---

# 2) **What Remains for Beta**

## 🔥 Only two tasks remain:

### **(1) Fix facet auth routing** (HIGH-IMPACT)

Micronode must apply auth *per route* based on manifest:

| Facet Type | Requires Auth? | Expected Behavior |
| ---------- | -------------- | ----------------- |
| Echo       | Yes            | 401 without mac   |
| Static     | No             | 200 without mac   |

The test `loads_static_and_echo_facets_and_enforces_auth` encodes this invariant.

---

### **(2) Add coverage for mixed facet types** (OPTIONAL but recommended)

A small unit test verifying per-route enforcement will ensure no future regression.

---

# 3) **Deep Root-Cause Analysis of the Current Failing Test**

### ❌ Failing test:

```
loads_static_and_echo_facets_and_enforces_auth
```

### Expected:

* GET `/echo` → 401 (requires auth)
* GET `/static` → 200 (no auth)

### Actual:

* GET `/echo` → 401 (correct)
* GET `/static` → 401 (incorrect)

### What this tells us:

The facet loader + router composer is correctly loading both manifests.

But in router composition:

* *All* facet routes are merged under:

```rust
.layer(RequireAuthLayer::new(mode))
```

Meaning even static facets get the auth layer.

### When global SecurityMode = DenyAll

The RequireAuthLayer is triggered instantly, returning/unwrapping:

```rust
StatusCode::UNAUTHORIZED
```

for any facet route without macaroons.

### Important nuance:

This is **not** a missing route (404) being auth-blocked —
the static facet route *exists*, but the auth layer intercepts it.

### Therefore root cause = **auth layering, not route construction.**

---

# 4) **Next High-Impact Steps After Fix**

These steps will finish the crate for beta:

### ✔ 1. Modify facet router composer to:

* Check manifest.annotations.requires_auth
* Only apply RequireAuthLayer to facets requiring auth

### ✔ 2. Reconcile mount vs mount_with_registry:

* mount() → demo + meta only
* mount_with_registry() → no meta, only manifest routes

### ✔ 3. Re-run full beta suite:

```
cargo fmt -p micronode
cargo clippy -p micronode --no-deps -- -D warnings
cargo test -p micronode --tests
```

### ✔ 4. Add quick regression tests:

* Ensure static facets bypass auth
* Ensure echo facets require auth
* Ensure meta route is always reachable

### ✔ 5. Update NOTES + READMEs

Document facet behavior clearly.

---

# 5) **Estimated Beta Completion after Fix**

Once the facet auth routing is corrected:

**Micronode Beta Completion: 97%**

The remaining 3% is polish (docs, comments, refactors, final README, smoke script).

---

# 6) **Closing Summary**

Micronode is now a *fully functional mini-node* implementing:

* Admin plane
* KV plane
* Developer echo
* Concurrency + backpressure
* Security + macaroons
* Amnesia mode
* Facet parsing + routing
* CLI
* Health/readiness
* Config system
* Metrics

**All green except one facet auth mismatch.**

Once that test passes, micronode can be stamped **Beta**.

---

### END NOTE - NOVEMBER 17 2025 - 13:48 CST


### BEGIN NOTE - NOVEMBER 17 2025 - 15:07 CST

## CARRY-OVER NOTES — `micronode` (Beta Sweep)

**Date:** 2025-11-17
**Beta completion:** **Functionally complete (≈97%)**
**Status:** Tests are all green and we’ve verified the real binary with live facets over HTTP. Remaining work is polish: docs, smoke script, and maybe a tiny amount of ergonomics.

---

### 0) TL;DR

Micronode is now:

* A **runnable mini-node** that:

  * Exposes admin plane (`/healthz`, `/readyz`, `/metrics`, `/version`).
  * Serves a KV plane under `/v1/kv/...` with body caps, concurrency limits, and auth gating.
  * Supports **amnesia posture** (mem-first) with proof tests.
  * Loads **facets** from disk via manifest files and mounts them as HTTP routes, with different behavior depending on `SecurityMode`.
  * Has a **CLI** wired with `serve` as the default command.
* Fully covered by tests:

  * `admin_parity`, `kv_roundtrip`, `auth_gate`, `backpressure`, `guard_behavior`, `facets_loader`, `facets_proxy`, `cli_smoke`, `amnesia_proof`, plus unit tests in `config`, `adapters`, `security`.
* Proven in a real run:

  * We ran `micronode` locally with a `micronode.facets.toml` config and successfully hit:

    * `/facets/_meta`
    * `/facets/docs/hello`
    * `/facets/echoer/who`

We fixed two key issues on the way:

1. **Facet router path + auth semantics**
2. **Config `merge()` silently dropping `[facets]`**, so the binary never saw `facets.enabled = true`.

Both are now resolved.

---

### 1) What’s Solid (Feature by Feature)

#### A. Admin Plane

* Endpoints:

  * `GET /healthz` → `{"ok":true}`
  * `GET /readyz` → `{"ready":true, "probes":{...}, "mode":"truthful"}`
  * `GET /metrics` → Prometheus / text format
  * `GET /version` → build info (via vergen)
* Readiness probes:

  * `listeners_bound`
  * `cfg_loaded`
  * `metrics_bound`
  * `deps_ok` (used for facets loader errors)
* Tests:

  * `tests/admin_parity.rs` confirms admin endpoints are healthy and observable.

#### B. KV Plane

* Paths:

  * `PUT /v1/kv/:bucket/:key`
  * `GET /v1/kv/:bucket/:key`
  * `DELETE /v1/kv/:bucket/:key`
* Behaviors:

  * Respect **body cap** limits (max payload size).
  * Scoped under concurrency limits (per “class” pool).
  * Auth gating via `SecurityMode` + `RequireAuthLayer`:

    * `DenyAll` → 401/403 behavior.
    * `DevAllow` → open KV for dev (no header required).
    * `External` → delegated auth pattern.
* Tests:

  * `tests/kv_roundtrip.rs` → `kv_put_get_delete_roundtrip` ✅
  * `tests/auth_gate.rs` → deny/dev_allow semantics validated.

#### C. Security & Amnesia

* `SecurityMode` enum:

  * `deny_all`, `dev_allow`, `external` (serde snake_case).
* `SecurityLayer`:

  * Extracts macaroons / auth token into request extensions.
* `RequireAuthLayer`:

  * Enforces policy based on `SecurityMode`.
  * Produces `WWW-Authenticate` header in `DenyAll` without token.
* Amnesia:

  * Default posture is amnesia-enabled.
  * Tests:

    * `security::amnesia::tests::posture_defaults_to_amnesia_enabled`
    * `tests/amnesia_proof.rs` (integration)

#### D. Concurrency & Backpressure

* Concurrency registry:

  * Distinct pools per class (e.g., admin, KV, facets).
* Backpressure:

  * When pool exhausted, returns `429 Too Many Requests`.
* Tests:

  * `tests/backpressure.rs`

    * `concurrency_registry_builds_distinct_pools_per_class`
    * `concurrency_layer_sheds_with_429_when_pool_is_exhausted`

#### E. Decode Guard & Body Cap

* Decode guard:

  * Rejects any `Content-Encoding` attempt to prevent sneaky compression / bomb payloads.
* Body cap:

  * Enforces a max payload size; tests confirm enforcement.
* Tests:

  * `tests/guard_behavior.rs`

    * `decode_guard_rejects_any_content_encoding`
    * `body_cap_enforces_max_payload_size`

#### F. Config Plane

* `Config` struct includes:

  * `[server]`
  * `[storage]`
  * `[security]`
  * `[facets]` (enabled + dir)
* Loading:

  * Loads base config from TOML.
  * Applies env overlay (`MICRONODE_CONFIG`, `MICRONODE_DEV_ROUTES`, etc.).
  * **Fixed** `merge()` to copy `facets`:

    * Previously: `server`, `storage`, `security` only.
    * Now: `facets` is also merged, so runtime sees `facets.enabled/dir`.
* Tests:

  * `config::load::tests::overlay_env_parses_bind_and_dev_routes`
  * `config::load::tests::merge_applies_security_mode` (and by extension, `facets` behavior now matches expectations via runtime behavior).

#### G. CLI

* Binary entrypoint:

  * CLI defaults to `serve` command.
* Tests:

  * `tests/cli_smoke.rs` → `default_cli_uses_serve_command`

#### H. Facets Plane (Loader, Router, Meta) — **The Big One**

**1. Loader & Manifests**

* Facets are described via TOML manifests:

  * `docs.toml`, `echo.toml`, etc.
* Manifest shape:

  * `[facet]`:

    * `id` (e.g., `"docs"`, `"echoer"`)
    * `kind` (`"static"`, `"echo"`, `"proxy"`)
  * `[[route]]`:

    * `method` (`"GET"`, `"POST"`)
    * `path` (must start with `/`)
    * `file` (for `kind = "static"`)
* Loader:

  * Scans the configured `facets.dir`.
  * Validates manifest contents (`path` format, `file` existence for static facets).
  * Builds a `FacetRegistry` with a list of `FacetManifest`s.
  * On error:

    * Marks `deps_ok = false` in readiness probes.
    * Still starts the node with demo/meta surfaces.
* Tests:

  * `tests/facets_loader.rs`

    * `bad_manifest_blocks_readiness` → ensures loader failure downgrades readiness but doesn’t hard-crash.

**2. Router Composition**

* `mount(router)`:

  * Used when facets are disabled or loader fails.
  * Mounts:

    * `GET /facets/demo/ping`
    * `GET /facets/meta`
    * `GET /facets/_meta`
  * Meta returns `{"loaded":[]}`.

* `mount_with_registry(router, registry, mode)`:

  * Used when facets are enabled and loader succeeded.
  * Always mounts meta:

    * `GET /facets/meta`
    * `GET /facets/_meta` (alias/legacy)
  * For each manifest:

    * Builds sub-routes of the form:

      * `/facets/{facet_id}{route.path}`
      * Example:

        * `id = "docs"`, `path = "/hello"` → `/facets/docs/hello`
        * `id = "echoer"`, `path = "/who"` → `/facets/echoer/who`
    * Gating:

      * `SecurityMode::DevAllow`:

        * Sub-router **not** wrapped in `RequireAuthLayer`.
        * Facets are dev-open (like KV in DevAllow).
      * `DenyAll` / `External`:

        * Sub-router wrapped in `RequireAuthLayer::new(mode)`.
        * No header → 401, header → 403 (depending on mode).
  * Meta payload lists each facet and their mounted **full paths**.

* Handlers:

  * Static:

    * Async read of file path inside `static_handler`.
    * On success: `200 OK`, body = file contents.
    * On error: `404 Not Found`.
  * Echo:

    * Simple `{"echo":"ok"}` JSON.
  * Proxy:

    * Currently `501 Not Implemented` (placeholder).

* Tests:

  * `tests/facets_loader.rs`:

    * `loads_static_and_echo_facets_and_enforces_auth`

      * In `DenyAll`:

        * `/facets/docs/hello` → `401`.
      * In `DevAllow`:

        * `/facets/docs/hello` → `200` and body contains `"hi"`.
      * Also validates meta has 2 loaded facets.
  * `tests/facets_proxy.rs`:

    * `facets_meta_endpoint_is_wired`

      * Asserts `/facets/_meta` returns `200` and JSON.
    * `demo_facet_ping_works`

      * `GET /facets/demo/ping` → `200`.

**3. Runtime Proof (Manual)

Using:

```bash
MICRONODE_CONFIG=crates/micronode/configs/micronode.facets.toml \
MICRONODE_DEV_ROUTES=1 \
cargo run -p micronode
```

and:

```bash
BASE="http://127.0.0.1:5310"
curl -s "$BASE/healthz"
curl -s "$BASE/readyz"
curl -s "$BASE/facets/_meta" | jq .
curl -v "$BASE/facets/docs/hello"
curl -v "$BASE/facets/echoer/who"
```

We got:

* `/healthz` → `{"ok":true}`

* `/readyz` → `{"ready":true, ... "deps_ok":true}`

* `/facets/_meta`:

  ```json
  {
    "loaded": [
      {
        "id": "echoer",
        "kind": "echo",
        "routes": [{"method":"GET","path":"/facets/echoer/who"}]
      },
      {
        "id": "docs",
        "kind": "static",
        "routes": [{"method":"GET","path":"/facets/docs/hello"}]
      }
    ]
  }
  ```

* `/facets/docs/hello`:

  * `HTTP/1.1 200 OK`
  * Body: `hi from facet`

* `/facets/echoer/who`:

  * `HTTP/1.1 200 OK`
  * Body: `{"echo":"ok"}`

This is the decisive proof that tests + binary behavior line up.

---

### 2) Key Bugs We Found and Fixed

1. **Facet auth + path mismatch**

   * Initially:

     * All facets were behind `RequireAuthLayer` even in `DevAllow`.
     * Facets were mounted at raw paths (`/hello`) instead of `/facets/{id}{path}`.
   * Symptoms:

     * Tests saw `401` where they wanted `200`.
     * Then `404` when we changed behavior but still mis-mounted paths.
   * Fix:

     * In `DevAllow`, **don’t** wrap facets subrouters with `RequireAuthLayer`.
     * Always mount facets under `/facets/{id}{path}`.
     * Add both `/facets/meta` and `/facets/_meta` pointing to the same meta payload.

2. **Config merge dropping facets**

   * Root cause:

     * `merge()` didn’t copy `from.facets` into `base`, so runtime configs never enabled facets even when TOML did.
   * Symptoms:

     * Tests (which construct `Config` directly) passed.
     * Binary started with `/facets/_meta` but `loaded: []` even when manifests existed.
   * Fix:

     * Add `base.facets = from.facets;` inside `merge()`.

These two fixes made everything coherent: tests, router, loader, and runtime.

---

### 3) How to Run Micronode with Facets (Dev Recipe)

**Facets dir:**

```bash
mkdir -p crates/micronode/dev_facets

cat > crates/micronode/dev_facets/hello.txt <<'EOF'
hi from facet
EOF

cat > crates/micronode/dev_facets/docs.toml <<'EOF'
[facet]
id = "docs"
kind = "static"

[[route]]
method = "GET"
path = "/hello"
file = "crates/micronode/dev_facets/hello.txt"
EOF

cat > crates/micronode/dev_facets/echo.toml <<'EOF'
[facet]
id = "echoer"
kind = "echo"

[[route]]
method = "GET"
path = "/who"
EOF
```

**Config:**

```toml
# crates/micronode/configs/micronode.facets.toml

[server]
bind = "127.0.0.1:5310"
dev_routes = true

[storage]
engine = "mem"

[security]
mode = "dev_allow"

[facets]
enabled = true
dir = "crates/micronode/dev_facets"
```

**Run:**

```bash
MICRONODE_CONFIG="crates/micronode/configs/micronode.facets.toml" \
MICRONODE_DEV_ROUTES=1 \
cargo run -p micronode
```

**Smoke:**

```bash
BASE="http://127.0.0.1:5310"

curl -s "$BASE/healthz"
curl -s "$BASE/readyz"
curl -s "$BASE/facets/_meta" | jq .
curl -v "$BASE/facets/docs/hello"
curl -v "$BASE/facets/echoer/who"
```

---

### 4) What’s Left Beyond Beta (Nice-to-Haves)

These aren’t blockers for calling micronode “beta”; they’re future polish:

1. **Proxy facet implementation**

   * Currently returns `501 Not Implemented`.
   * Future: wire to svc-index / svc-storage / other internal services.

2. **More DX around facets**

   * CLI subcommand to scaffold a facet (template TOML + handler stub).
   * Optional: hot-reload support (re-scan dir on SIGHUP or similar).

3. **Docs polish**

   * README section with a small “Hello Facet” tutorial.
   * Diagram from client → Micronode → facet → backend service.

4. **Extended tests**

   * Add one more test to assert `/facets/echoer/who` 200 in DevAllow, just to lock in semantics further.

---


### END NOTE - NOVEMBER 17 2025 - 15:07 CST




### BEGIN NOTE - JANUARY 8 2026 - 14:00 CST

Below are **paste-ready, exhaustive carry-over notes** you can drop into **both** crates. I wrote them as a single unified doc with clearly labeled sections so you can paste the same thing into `crates/svc-admin/NOTES.md` and `crates/micronode/NOTES.md` (or split if you prefer).

---

# CARRY-OVER NOTES — Micronode wiring into svc-admin (truthful, no mocks)

## 0) Current status snapshot (what works vs what’s missing)

### What works now

* ✅ `scripts/run_dashboard.sh -macro -micro` can start:

  * macronode on `127.0.0.1:8080`
  * micronode on `127.0.0.1:5310`
  * svc-admin UI/API on `127.0.0.1:5300`
  * svc-admin health/ready/metrics on `127.0.0.1:5311`
  * UI dev server on `http://localhost:5173`
* ✅ svc-admin shows **macronode** as “real/live” (most tiles and node detail look correct).
* ✅ svc-admin recognizes micronode as a node entry (node list shows it), but **micronode data is still “Mock/Warming/Fallback”** for most dashboards/tiles.

### What is still broken / incomplete

* ❌ Micronode is still effectively “mock” in svc-admin because svc-admin cannot obtain enough structured data from micronode endpoints to mark tiles as “Reported”.
* In logs we saw repeated warnings like:

  * `failed to fetch /api/v1/status ... 404 Not Found ... degrading to health/ready/version probes`
* This proves:

  * svc-admin expects certain “node admin API” endpoints to exist on the node base URL.
  * micronode currently lacks one or more of these endpoints (or their DTOs don’t match), so svc-admin falls back to minimal probes and/or deterministic mocks.

### Primary objective

Make micronode expose the same “operator-grade” admin API surface as macronode **where appropriate**, so svc-admin can treat micronode as fully “real” and stop falling back to mock widgets.

---

## 1) The contract: what svc-admin expects from a node

svc-admin operates in tiers:

### Tier A — minimal liveness (always possible)

* `GET /healthz`
* `GET /readyz`
* `GET /version`
* `GET /metrics`

If only these exist, svc-admin can show “reachable” but lacks rich status, planes, utilization, storage inventory, etc.

### Tier B — admin API v1 (required to stop many fallbacks)

svc-admin tries to call:

* `GET /api/v1/status`

If this 404s, svc-admin logs degrade warnings and will likely:

* treat node as “unknown profile”
* treat utilization/storage/etc as missing and fall back to “Mock” or “Warming”

### Tier C — rich system and storage summaries (required for “Reported” tiles)

Depending on UI implementation, svc-admin may also attempt:

* `GET /api/v1/system/summary`  (CPU/mem/disk/system facts)
* `GET /api/v1/storage/summary` (disk totals, possibly DB counts / IO)

If these are missing or not shaped correctly, the UI still shows “Mock/Warming” for utilization and storage tiles.

### Tier D — macronode-only planes and inventories

macronode has embedded/hosted services and can expose:

* plane list + per-plane status
* storage inventory / DB detail endpoints
* gateway/mailbox/overlay/index service info, etc.

Micronode might not support all of these yet, but it still must provide enough to eliminate “everything is mock”.

---

## 2) What we saw in runtime logs (most important evidence)

When running `scripts/run_dashboard.sh -macro -micro`, svc-admin repeatedly logged:

* `failed to fetch /api/v1/status ... 404 Not Found ... (micronode)`

Interpretation:

* svc-admin is configured with micronode’s `base_url=http://127.0.0.1:5310`
* svc-admin expects micronode to implement `/api/v1/status`
* micronode currently does **not** implement it (or router is not mounting it)

This is the single most important reason micronode appears as mock in the UI.

---

## 3) Known compile-time blockers that appeared while attempting micronode admin API

During `cargo build -p svc-admin -p macronode -p micronode`, micronode failed with:

* `_State` extractor used instead of `State`
* usage of `unsafe` in admin_api (micronode forbids unsafe)
* mismatched AppState fields (`started_at`, `ready`, `build`) that don’t exist
* config field mismatch (`st.cfg.bind` vs `st.cfg.server.bind`)
* sysinfo API mismatch (sysinfo 0.30: `System::disks()` not available)

These errors strongly indicate an attempted admin_api implementation was written against an older/other struct layout and sysinfo API.

**Invariant:** Micronode has `#![forbid(unsafe_code)]` in `src/lib.rs` → any implementation must use safe primitives only (Mutex/OnceLock/etc).

---

## 4) What needs to be true for “no mocks” in the UI

To get svc-admin to show micronode as fully live:

### Must-have endpoints (minimum)

1. `GET /api/v1/status` returning a DTO matching svc-admin’s expectations.
2. `GET /api/v1/system/summary` returning CPU/mem/disk totals (or enough for utilization panels).
3. `GET /api/v1/storage/summary` returning disk totals and “storage high-level” fields.

### Must-have behaviors

* No 404 on the above routes.
* DTO field names must match expected serde casing (svc-admin TS types support some snake/camel compatibility, but avoid relying on it).
* Responses must be fast and reliable (avoid expensive sysinfo refresh loops on every request if possible; caching is fine).
* No unsafe code.
* No phantom/mock nodes: svc-admin registry must only contain real nodes (macro/micro) when script flags are used.

---

## 5) Current dev tooling situation (script + ports + “truth probes”)

### `scripts/run_dashboard.sh` goals

* Start real nodes only, optionally both
* Avoid port conflicts and stale processes
* Provide diagnostic visibility (what’s listening, what endpoints return)

### What we learned (important)

* svc-admin health/ready live on `SVC_ADMIN_METRICS_ADDR` (default `5311`), **not** the UI/API port `5300`.
* Port conflicts and sled DB locks happened when old processes were still running.
* The script now includes port-kill logic (`lsof -ti tcp:<port> | kill -9`) unless `--no-kill` is passed.
* Script prints “stack is up” and direct URLs.

### What still needs to be added/verified

* “Truth probes” for node contract endpoints should be run after each node boots:

  * Probe micronode `/api/v1/status`, `/api/v1/system/summary`, `/api/v1/storage/summary`, `/metrics`
  * Print HTTP status + small body snippet
* These probes become the first-line indicator of why UI is mocking.

---

## 6) svc-admin config: remove default mock nodes / keep truthfulness

### Problem we hit earlier

svc-admin showed **3 nodes** when only macro+micro should exist.
Cause: `default_nodes()` in svc-admin seeded extra demo nodes (`example-node`, `node-b`, `node-c`) all pointing to localhost.

### Desired behavior

* Default `cargo run -p svc-admin` can have *one* minimal default node (optional).
* When using `run_dashboard.sh`, the registry must be **cleared** and nodes injected only by env.

### Implemented / intended mechanism

* Use `SVC_ADMIN_NODES_CLEAR=1` to force empty `default_nodes()` seed.
* Script should set it when launching svc-admin.
* Script should inject exactly:

  * `SVC_ADMIN_NODES__macronode__BASE_URL=...`
  * `SVC_ADMIN_NODES__micronode__BASE_URL=...`
  * plus display name, env, insecure, forced_profile.

**Invariant:** “Past the mock data phase” means:

* No extra fake nodes.
* No “Node B/Node C” placeholders.
* If a node isn’t running, it should be absent or clearly unreachable, not simulated.

---

## 7) Micronode: what to wire up (parity plan)

### 7.1 Admin API module (micronode)

Create/repair: `crates/micronode/src/http/admin_api.rs`

It should:

* use `axum::extract::State` (not `_State`)
* avoid unsafe entirely
* read truth from:

  * `AppState.health.all_ready()` for readiness
  * `cfg.server.bind` (not cfg.bind)
  * `env!("CARGO_PKG_VERSION")` for version
  * `sysinfo` for system summary/disk totals (compatible with sysinfo 0.30)

**Key principle:** Don’t attempt to mirror macronode’s plane model yet—provide a minimal but truthful “micronode plane”.

### 7.2 Router mounting (micronode app.rs)

`crates/micronode/src/app.rs` must mount:

* `/api/v1/status`
* `/api/v1/system/summary`
* `/api/v1/storage/summary`

…and keep existing:

* `/healthz`, `/readyz`, `/version`, `/metrics`

### 7.3 http/mod.rs

Ensure `pub mod admin_api;` exists and is correctly referenced.

### 7.4 DTO alignment

We must align DTOs between:

* Rust structs returned by micronode
* svc-admin Rust client expectations (NodeClient)
* svc-admin TS types in UI

Approach:

* Use `#[serde(rename_all="camelCase")]` on micronode DTOs to align with UI.
* Where svc-admin expects snake_case, verify compatibility in `svc-admin/ui/src/types/admin-api.ts` (it currently supports some compat policy).
* Choose one canonical casing (camelCase is fine) and keep it consistent across endpoints.

### 7.5 Metrics expectations

Micronode already exposes `/metrics` on same bind.
We eventually want:

* request counters / latency histograms
* memory/cpu gauges (optional)
  But for “no mock” UI, first objective is just the admin API contract endpoints.

---

## 8) sysinfo dependency verification

### Root workspace

* `sysinfo = "0.30"` pinned in `[workspace.dependencies]`

### Micronode crate

* `sysinfo = { workspace = true }`

**Important API note:** sysinfo 0.30 provides disks via:

* `sysinfo::Disks::new_with_refreshed_list()`
  not `System::disks()`.

If we need network IO totals, sysinfo APIs vary and may require refresh calls; keep it minimal at first.

---

## 9) Immediate next steps checklist (in order)

### Phase 1 — unblock micronode compile

1. Fix `crates/micronode/src/http/admin_api.rs`:

   * Replace `_State` with `State`.
   * Remove all unsafe (use safe static caching if needed: `OnceLock<Mutex<...>>`).
   * Remove references to nonexistent AppState fields (`started_at`, `ready`, `build`).
   * Use `st.health.all_ready()` and config path `st.cfg.server.bind`.
   * Fix sysinfo calls (Disks API).

2. Ensure `crates/micronode/src/http/mod.rs` exports `admin_api`.

3. Ensure `crates/micronode/src/app.rs` mounts admin_api routes.

4. `cargo build -p micronode` must succeed.

### Phase 2 — confirm HTTP contract live (truth probes)

1. Run:

   * `scripts/run_dashboard.sh -macro -micro`
2. Confirm probes show for micronode:

   * `/api/v1/status` => HTTP 200
   * `/api/v1/system/summary` => HTTP 200
   * `/api/v1/storage/summary` => HTTP 200
3. Confirm svc-admin logs no longer warn about `/api/v1/status` 404.

### Phase 3 — eliminate UI mock fallback

1. Once endpoints return 200, watch UI:

   * NodeDetail utilization tiles should flip from Mock/Warming to Reported.
   * Node preview panel should show real system and storage values.
2. If still mock:

   * The DTO shape is mismatched or svc-admin parsing rejects it.
   * Add logging in svc-admin NodeClient parsing path to print decode errors (not just 404).
   * Compare JSON output from micronode endpoints to macronode equivalents and to TS types.

### Phase 4 — “macronode parity where appropriate”

After contract endpoints work:

* Add any additional endpoints the UI tries next (storage inventory, plane tables, etc.), but only if those concepts exist for micronode.
* Micronode likely needs a “capabilities” field so svc-admin UI can hide macronode-only sections cleanly.

---

## 10) Debugging playbook (fast triage)

When micronode still appears mock:

### A) Confirm endpoint existence

Run:

* `curl -i http://127.0.0.1:5310/api/v1/status`
* `curl -i http://127.0.0.1:5310/api/v1/system/summary`
* `curl -i http://127.0.0.1:5310/api/v1/storage/summary`

If any are 404, router mounting is wrong.

### B) Confirm JSON decodes in svc-admin

If endpoints return 200 but UI still mocks:

* svc-admin likely fails to deserialize response.
* Add temporary debug logging in svc-admin NodeClient:

  * log response status + first N bytes
  * log serde errors (field mismatch, unexpected casing)

### C) Confirm registry is truthful

If you see extra nodes:

* ensure svc-admin gets `SVC_ADMIN_NODES_CLEAR=1`
* ensure `default_nodes()` does not seed node-b/node-c
* confirm only env-injected nodes are loaded

### D) Avoid stale processes

Port conflicts & DB locks indicate old processes were still around.
Keep “kill ports” behavior in script unless explicitly disabled.

---

## 11) “Definition of done” for this wiring milestone

We consider “micronode wired like macronode (enough)” when:

* ✅ svc-admin shows exactly **two nodes** when running `-macro -micro`
* ✅ svc-admin logs contain **no repeated 404 degrade warnings** for micronode
* ✅ Micronode NodeDetail shows **Reported** (not Mock) for:

  * system summary (CPU/mem basics)
  * storage summary (disk totals)
  * readiness/health/version and metrics freshness
* ✅ Remaining sections that are macronode-only are either:

  * hidden via capabilities/profile, or
  * clearly marked “Not supported on micronode” (truthful, no fake data)

---

## 12) Files involved (quick map)

### svc-admin

* `crates/svc-admin/src/config/nodes.rs`

  * Ensure default nodes are truthful and allow `SVC_ADMIN_NODES_CLEAR=1`.
* `scripts/run_dashboard.sh`

  * Should inject exactly macro/micro nodes via env.
  * Should include truth probes and port-kill diagnostics.
* svc-admin NodeClient path (where `/api/v1/status` is fetched)

  * Add decode-error logging if needed.

### micronode

* `crates/micronode/src/app.rs`

  * Must mount `/api/v1/status`, `/api/v1/system/summary`, `/api/v1/storage/summary`.
* `crates/micronode/src/http/mod.rs`

  * Must export `admin_api`.
* `crates/micronode/src/http/admin_api.rs`

  * Must compile, be unsafe-free, and return correct DTO shapes.
* Root and micronode `Cargo.toml`

  * sysinfo pinned workspace-wide and used via workspace in micronode.

---

## 13) Known constraints / invariants to keep in mind

* Micronode forbids unsafe code.
* “Past mock phase” means:

  * no fake nodes
  * no pretend registrations
  * mocks only allowed as last-resort UI fallback when endpoints truly absent (and should be visually obvious).
* svc-admin health/ready/metrics are served on metrics listener, not UI/API listener.
* Avoid port collisions and sled locks by killing existing listeners before launching.

---

## 14) Next work item after wiring endpoints (future)

Once micronode endpoints are live and UI is “Reported”:

* add capability-driven UI gating so macronode-only planes don’t confuse micronode view
* optionally add:

  * network accounting endpoints for micronode (req counters, bytes in/out)
  * minimal “planes” list for micronode that maps to real subsystems (kv/facets/auth)
* then begin parity pass: whichever routes UI calls for macronode, decide:

  * implement for micronode, or
  * explicitly not supported + hide

---

If you want, the next step after these notes is: **paste the current micronode `http/admin_api.rs` and `state/AppState` layout** (or the whole micronode CODEBUNDLE), and I’ll produce a single paste-ready implementation that:

* compiles under `forbid(unsafe_code)`,
* mounts the correct routes,
* and returns DTOs that svc-admin will actually parse (so the UI stops mocking).


### END NOTE - JANUARY 8 2026 - 14:00 CST


### BEGIN NOTE - JANUARY 8 2026 - 19:35 CST


Got it — here are **exhaustive carry-over notes** you can paste into **both** `crates/micronode/NOTES.md` and `crates/svc-admin/NOTES.md` (and optionally also into a shared `PROJECT_NOTES.md`). I’m writing these as a single unified doc, with **actionable next steps**, **known failing symbols**, and **exact failure signatures** so the next instance can immediately resume without rediscovery.

---

# CARRY-OVER NOTES — Micronode ↔ svc-admin wiring (Jan 2026)

## 0) Goal of this milestone

We are wiring **micronode** into **svc-admin** so it behaves like **macronode** in the dashboard:

* **No mocks / no fallback** for: status, system summary (CPU/RAM), storage summary (disk), bandwidth.
* **Metrics freshness** should read as **fresh** (not stale) for micronode.
* **Planes**: micronode currently reports only a single “micronode” plane; svc-admin currently expects or prefers richer plane lists like macronode. We need to decide what micronode should truthfully expose as “planes” (see Section 10).

---

## 1) Current status snapshot

### ✅ What worked at runtime (when it compiled previously)

When `scripts/run_dashboard.sh -macro -micro` was run successfully, we saw:

**Micronode truth probes all returning 200:**

* `GET http://127.0.0.1:5310/api/v1/status` → 200 with `{ profile:"micronode", capabilities:[...], planes:[...] }`
* `GET http://127.0.0.1:5310/api/v1/system/summary` → 200 with `cpuPercent, ramTotalBytes, ramUsedBytes`, sometimes `netRxBps/netTxBps` null then later non-null
* `GET http://127.0.0.1:5310/api/v1/storage/summary` → 200 with `totalBytes/usedBytes/freeBytes`, `nodeId`, `dbCount`
* `GET http://127.0.0.1:5310/metrics` → 200 with `micronode_http_requests_total{...}` etc.

So the **HTTP surface** was *close enough* to feed svc-admin.

### ✅ What improved in the UI

In svc-admin UI screenshots:

* CPU/RAM/Storage/Bandwidth tiles for micronode showed **REPORTED / Live** values.
* Nodes list showed micronode present as a real node.

### ❌ What is still wrong in the UI

* **Micronode metrics freshness** shows **“Metrics: stale”** even though `/metrics` returns 200.
* **Plane list** does not show multiple service planes (only 1 plane, health “Unknown” in some UI areas).
* Some “Data & storage” sections still say **Not supported**.

---

## 2) Current top blocker: Micronode does NOT compile (latest state)

We generated a new codebundle and now `cargo build -p micronode` fails with several categories of errors.

### 2.1 Private handler functions referenced from `app.rs`

Errors:

* `error[E0603]: function healthz is private` from `crates/micronode/src/app.rs` referencing `admin::healthz`
* same for `readyz`, `version`, `metrics`

Cause:

* In `crates/micronode/src/http/admin.rs` these handlers are defined as `async fn` (private), not `pub async fn`.

Fix:

* Make the handlers `pub` (or expose a public router builder in admin.rs and merge that instead).

### 2.2 Incorrect IntoResponse tuple usage in admin.rs

Errors like:

* `("ok", 200).into_response()` failing trait bounds
* Similar for `("degraded", 503)`, `("ready", 200)`, etc.

Cause:

* In axum 0.7, tuple response patterns are typically:

  * `(StatusCode, &'static str)` or `(StatusCode, Json<T>)`
  * Not `(&str, i32)` or `(&str, 200)` in that order.

Fix:

* Use `axum::http::StatusCode` and return `(StatusCode::OK, "ok")`, etc.

### 2.3 Ready probe method mismatch

Errors:

* `no method named ready found for Arc<ReadyProbes>`

Cause:

* Code references `st.probes.ready()` but `ReadyProbes` doesn’t have a `ready()` method (maybe it has `all_ready()` or `is_ready()` or a field).

Fix:

* Inspect `ReadyProbes` type in micronode’s state/observability module and use the real API.
* Alternatively, in admin.rs and admin_api.rs we can key readiness off `st.health.all_ready()` if that exists and is canonical.

### 2.4 Metrics gather/encode API mismatch

Errors:

* `Arc<Metrics>` has no method `gather()`, but `st.metrics.registry.gather()` exists
* `TextEncoder.encode()` not found because `prometheus::Encoder` trait not imported

Fix:

* Replace gather call with `st.metrics.registry.gather()`
* `use prometheus::Encoder;`
* Ensure we set proper content type for Prometheus text format.

### 2.5 sysinfo disk FS type conversion mismatch

Error:

* `String::from_utf8_lossy(d.file_system())` expected `&[u8]`, found `&OsStr`

Cause:

* sysinfo’s disk filesystem type may return `&OsStr` or `&str` depending on API.

Fix:

* Convert safely using `.to_string_lossy().to_string()` if it’s `&OsStr`.

### 2.6 Config field mismatches (profile/node_id)

Errors:

* `no field profile on type schema::Config`
* `no field node_id on type schema::Config`

Cause:

* The new admin_api implementation is assuming config contains `profile` and `node_id`, but micronode config schema actually has: `server`, `storage`, `security`, `facets` (and no top-level profile/node_id).

Fix:

* For profile: hardcode `"micronode"` in admin API DTO.
* For node_id: use a stable ID from config if present elsewhere (maybe `cfg.server.name` or `cfg.server.bind`), or set `nodeId = "micronode"` for now (truthful enough for dev).

### 2.7 rustc ICE during borrowck/typeck

We saw a compiler panic (ICE) while type-checking `http::admin::metrics`. This is likely triggered by the incorrect tuple return types / IntoResponse mismatch causing deep inference issues.

Fixing admin.rs return types usually makes the ICE disappear.

---

## 3) Files that are in scope for the fix (micronode)

### Must-fix files

1. `crates/micronode/src/http/admin.rs`

* Make route handlers `pub`.
* Fix response types to `(StatusCode, &'static str)` or `(StatusCode, Json<T>)`.
* Fix readiness source (`st.probes.*` vs `st.health.*`).
* Fix Prometheus encoding (gather from registry, import Encoder trait, correct content-type).

2. `crates/micronode/src/http/admin_api.rs`

* Remove assumptions about `cfg.profile`, `cfg.node_id`.
* Fix readiness source method (`st.probes.ready()` is wrong).
* Fix sysinfo disk fs type conversions.
* Ensure DTO casing matches svc-admin expectations (camelCase seems preferred).
* Ensure planes/health values align with UI parsing.

3. `crates/micronode/src/app.rs`

* Ensure it imports/uses public handlers properly.
* Avoid fragile turbofish `.layer::<_, ...>()` usage (we already attempted simplifying this).
* Ensure “compose with state=() then attach AppState once” invariant remains.

### “Maybe” files (depending on approach)

4. `crates/micronode/src/observability/http_metrics.rs` (or similar)

* Verify route labeling so svc-admin sees consistent metrics for freshness.
* Ensure `/metrics` increments the right counters.

5. `crates/micronode/src/state.rs` (AppState)

* Confirm what readiness & start time are available.
* Add `started_at_unix` only if we want uptime computed locally; otherwise compute uptime using process start captured in a safe OnceLock.

---

## 4) svc-admin side observations (important)

* svc-admin is already configured with 2 nodes (macro and micro) and is launching samplers: `node_count=2 interval_secs=5`.
* svc-admin marks micronode metrics **stale** even though micronode `/metrics` returns 200.

Probable causes:

1. svc-admin’s metrics sampler expects a specific metric name/label set (macronode emits `ron_facet_requests_total`, etc.).
2. Micronode metrics exist but don’t include the “freshness” metric svc-admin uses to determine last-sample time.
3. svc-admin sampler may be failing to parse micronode metrics text due to formatting/content-type mismatch.

We need to locate **svc-admin metrics sampler logic**:

* Which endpoint does it hit? likely `GET {base_url}/metrics`.
* How does it decide “fresh”?:

  * maybe it expects a metric value timestamp or it stores last successful scrape time.
  * or it expects a `*_build_info` or `*_up` metric.

**Next instance action:** open svc-admin codebundle and find:

* the node “metrics freshness” calculation
* the sampler’s scrape error logging (may be too quiet)

---

## 5) What the run script is doing correctly

`scripts/run_dashboard.sh -macro -micro` now:

* Builds macro/micro/svc-admin
* Starts macronode on `127.0.0.1:8080`
* Starts micronode on `127.0.0.1:5310`
* Starts svc-admin on `127.0.0.1:5300` (+ health/metrics on `5311`)
* Runs truth probes (status/system/storage/metrics) for both macro and micro
* Starts SPA dev server

This is good. Keep using the truth probes as the first debugging signal.

---

## 6) What we need next instance (execution plan)

### Phase 1 — restore micronode compilation (highest priority)

1. Fix `crates/micronode/src/http/admin.rs`:

* make handlers `pub`
* return `(StatusCode, "ok")`-style responses
* use correct readiness method
* encode metrics via `st.metrics.registry.gather()`
* import `prometheus::Encoder`

2. Fix `crates/micronode/src/http/admin_api.rs`:

* remove invalid config fields (`profile`, `node_id`)
* fix readiness method
* fix sysinfo disk fs type conversion

3. Update `crates/micronode/src/app.rs` (if needed) to call the now-public admin handlers.

### Phase 2 — get svc-admin to stop marking micronode metrics stale

1. Verify the response headers:

* `Content-Type` should be Prometheus text format.
* Ensure `/metrics` handler uses correct header value.

2. Confirm svc-admin sampler logs for micronode:

* Add logging for scrape failures and parse failures.
* Print last_success timestamp per node in logs (temporarily).

3. Decide on a “minimum standard metric” for freshness:

* easiest: micronode exposes `micronode_build_info{version="..."} 1`
* svc-admin can treat a successful scrape as “fresh” regardless of metric names.

### Phase 3 — plane model

We need a truthful plan for micronode planes:

* If micronode hosts submodules (kv, facets, auth), define planes like:

  * `admin`
  * `kv`
  * `facets`
  * `storage` (if sled)
  * `security` (if auth mode enabled)
* Each plane includes `health`, `ready`, `restart_count`.

svc-admin’s plane UI is currently macronode-oriented (gateway/storage/index/mailbox/overlay/dht). For micronode we can:

* report a smaller plane list, and
* add a capability set so UI can hide macronode-only plane tiles.

---

## 7) Clarification: “Is micronode amnesia / RAM-only?”

**Current evidence:** micronode returns `/api/v1/storage/summary` with disk totals and `dbCount: 1`, implying it uses real storage (likely sled) on disk.

So right now: **micronode is not “amnesia” by default** — it appears to have persistent storage (unless config enables an in-memory mode).

**Action item next instance:**

* Inspect micronode config schema under `cfg.storage`.
* Determine if there is an “amnesia / ram-only” option.
* If yes:

  * In storage summary, represent storage as:

    * `mode: "memory"` and/or `persistent: false`
    * `ramKvBytes` instead of disk usage
  * In UI, show “RAM store” (not disk) to avoid misleading storage numbers.

If micronode will support both:

* Add capability: `storage_persistent` vs `storage_ephemeral`.

---

## 8) Known error signatures we hit (copy/paste friendly)

### Private handlers

* `error[E0603]: function healthz is private`
* `error[E0603]: function readyz is private`
* `error[E0603]: function version is private`
* `error[E0603]: function metrics is private`

### IntoResponse tuple issues

* `the method into_response exists for tuple (&'static str, {integer}) ... trait bounds not satisfied`

### Ready probes mismatch

* `no method named ready found for struct Arc<ReadyProbes>`

### Metrics gather mismatch

* `no method named gather found for struct Arc<Metrics> ... st.metrics.registry.gather()`

### Encoder trait missing

* `no method named encode found for struct TextEncoder ... trait Encoder not in scope`

### Config mismatch

* `no field profile on type schema::Config`
* `no field node_id on type schema::Config`

### sysinfo disk mismatch

* `expected &[u8], found &OsStr` for filesystem type conversion

---

## 9) What we should NOT do (invariants)

* Do not reintroduce “fake default nodes” in svc-admin when using the dashboard script.
* Do not add chrono to workspace unless strictly needed; prefer std time and existing utilities.
* Do not introduce unsafe code into micronode (it forbids unsafe).
* Keep router invariant: compose with state=() and attach AppState once at the end.

---

## 10) Specific open questions for next instance

1. **Planes:** what exact plane names/types should micronode expose?

   * Minimal: `micronode`
   * Better: `admin`, `kv`, `facets`, `storage`, `security`
2. **Metrics freshness:** does svc-admin require specific metric names, or should it treat “last successful scrape” as freshness?
3. **Amnesia mode:** does micronode support RAM-only operation?

   * If yes, update storage summary DTO and UI labels accordingly.
4. **DTO contract:** does svc-admin expect camelCase, snake_case, or both?

   * We should standardize on one (likely camelCase for UI).

---

## 11) Immediate next action in the next instance

1. Open micronode CODEBUNDLE and locate these files:

* `crates/micronode/src/http/admin.rs`
* `crates/micronode/src/http/admin_api.rs`
* `crates/micronode/src/state.rs` (or wherever AppState/ReadyProbes live)
* `crates/micronode/src/observability/metrics.rs` (or Metrics type)
* `crates/micronode/src/app.rs`

2. Apply fixes in this order:

* `admin.rs` first (eliminate ICE/inference chaos)
* `admin_api.rs`
* `app.rs`

3. Rebuild:

* `cargo build -p micronode`
* `scripts/run_dashboard.sh -macro -micro`

4. Check:

* truth probes for micro all 200
* svc-admin logs for node sampler
* UI: micronode metrics freshness should become “fresh”

---

If you want to make the next instance *even faster*: when you start it, paste **only** the current contents of:

* `crates/micronode/src/http/admin.rs`
* `crates/micronode/src/http/admin_api.rs`
* `crates/micronode/src/state.rs` (or where `ReadyProbes` is defined)
* `crates/micronode/src/observability/metrics.rs` (or Metrics struct)

…and I’ll return **full paste-ready replacements** for those files in one shot.



### END NOTE - JANUARY 8 2026 - 19:35 CST



### BEGIN NOTE - JANUARY 9 2026 - 12:30 CST

Below is a **drop-in carry-over note** you can paste into both:

* `crates/svc-admin/NOTES.md`
* `crates/micronode/NOTES.md`

(And optionally a shared `PROJECT_NOTES.md`.)

Then I include an **exhaustive plan** for implementing **svc-admin login** using the existing “passport/password” style crates (e.g., `svc-passport`) without leaking that logic into nodes.

---

# CARRY-OVER NOTES — Micronode ↔ svc-admin wiring + next pivot: svc-admin login

## 0) Current milestone status (what is DONE)

### ✅ Micronode is now correctly “wired” to svc-admin (baseline)

Micronode now shows in svc-admin as:

* **Metrics: fresh**
* **CPU/RAM/Storage tiles: Live**
* **Planes render and show Healthy / Ready**
* Node card summary: `3/3 planes ready`, `0 restarts`, uptime updates correctly

**Key technical fixes that made this happen:**

* Micronode `/api/v1/system/summary` + `/api/v1/storage/summary` JSON contract is now compatible with svc-admin (field casing + shapes).
* Micronode `/metrics` now exposes the expected **facet freshness counter** `ron_facet_requests_total{facet,result}` by gathering the correct registry output (not just default registry).
* Plane `health` values now match what svc-admin UI expects (`healthy`/`degraded` vs `ok`/unknown).

### ✅ Terminal truth probes confirm Micronode endpoints are correct

* `GET /api/v1/status` → returns `profile`, `version`, `uptime_seconds`, `planes[]`
* `GET /api/v1/system/summary` → returns CPU/RAM + net placeholders
* `GET /api/v1/storage/summary` → returns disk headroom and micronode truth labels
* `GET /metrics` → returns Prometheus text; includes `ron_facet_requests_total...`

---

## 1) What is still NOT complete (remaining gaps)

### 1.1 Bandwidth tile is “Live” but shows 0 B/s

The dashboard currently displays **Bandwidth RX/TX = 0 B/s** for micronode.

**Root cause:**

* Current sampling is based on **host NIC counters via sysinfo** and/or sampling logic that doesn’t reflect the node’s actual traffic in a stable way.
* More importantly: **host NIC bandwidth is not what we actually want** for svc-admin’s “node bandwidth” tile.

### 1.2 Planes exist but are “conceptual” (not independent services)

Micronode planes currently represent **logical subsystems** inside a single process:

* `admin`
* `kv`
* `facets`

They are “real” in the operational sense, but currently share the same readiness/health signals, so they don’t yet reflect subsystem-specific truth.

---

## 2) New requirement (must be added to project notes): Bandwidth must be per-node, not per-computer

### ✅ New invariant for RON-CORE dashboard observability

**Bandwidth and request volume shown on the node card must be specific to the node**, not the entire host machine.

Meaning:

* Do **NOT** show “whole laptop NIC usage” as “Micronode bandwidth”.
* The node bandwidth meter should reflect:

  * requests/sec (RPS)
  * bytes/sec in/out (service-specific)

### How to measure “node bandwidth” correctly

Preferred measurement sources:

1. **HTTP layer accounting** (service-level)

   * Count request/response bytes at the service boundary
   * Attribute to that node only
2. **Transport layer accounting** (if/when node uses custom transports)

   * Bytes in/out by transport name / peer / protocol
   * Still node-specific

### “Network page” concept

We want two tiers of views:

1. **Node-level**

* Per node:

  * Requests/sec
  * Bandwidth RX/TX for that node

2. **Cluster-level / combined**

* A separate page called **“Network”** (or “Traffic”) that shows:

  * Total requests/sec across nodes
  * Total bytes/sec across nodes
  * Breakdown by node, plane, route, status family, etc.

**Action item:** add a `Network` page to svc-admin UI with combined metrics.

---

## 3) Bandwidth work: recommended implementation approach (carry-over plan)

### 3.1 Node-specific bandwidth metrics (must be implemented)

Add “golden metrics” for each node process:

* `ron_node_http_requests_total{node,route,method,status}`
* `ron_node_http_request_bytes_total{node,route,method}`
* `ron_node_http_response_bytes_total{node,route,method,status}`
* Derived in UI:

  * RPS = rate(requests_total)
  * RX B/s = rate(request_bytes_total)
  * TX B/s = rate(response_bytes_total)

For micronode specifically:

* instrument the existing HTTP middleware to measure:

  * request body bytes read (if possible)
  * response body bytes written (approx acceptable for now)
  * fallback: Content-Length headers where available
* maintain “truthful” semantics: if unknown bytes, count what we can, don’t invent.

For macronode:

* extend existing facet counters / middleware similarly (if not already).

### 3.2 Where “combined bandwidth” should live

svc-admin should compute cluster totals either by:

* summing sampled node metrics in backend
* or by having its own “computed metrics” registry exported by svc-admin (`/metrics`)

And expose on a **Network** page.

---

## 4) Pivot: Implement svc-admin admin login (new milestone)

We are switching gears to implement **admin dashboard login** for svc-admin.

### 4.1 Security requirement statement (the “why”)

svc-admin controls visibility into node status/health and will eventually control actions (restart, config, policy, etc).

Therefore:

* svc-admin must require authentication by default
* anonymous access should be a deliberate dev-only mode
* we must support a production-grade auth story

### 4.2 Architectural constraint

svc-admin login/auth is a **svc-admin concern**.
Do not push “dashboard auth” into nodes; nodes should remain independently operable and should expose their own admin plane security separately.

svc-admin login protects:

* the svc-admin UI
* svc-admin API endpoints
* (optionally) proxy operations to nodes later

---

# PLAN — svc-admin login (God-tier, phased, uses existing “passport/password” crates)

## Phase 0 — Inventory + decision points (first step next instance)

Goal: pick the correct crate for password/auth primitives and avoid duplicating.

Actions:

1. Search workspace for:

   * `svc-passport`
   * password hashing helpers (argon2/bcrypt)
   * token/session helpers (JWT, macaroon, PASETO, signed cookies)
   * existing auth middleware patterns in any crate
2. Decide:

   * session type: **cookie session** vs **bearer token**
   * admin user storage: file-backed config vs sqlite/sled vs “single admin password env var”

**Recommendation (simplicity + security):**

* Start with **cookie session** (httpOnly, sameSite strict/lax, secure in prod).
* Use **argon2id** password hashing.
* Store users in a local file or sled (svc-admin-local only).
* Support bootstrap via `SVC_ADMIN_ADMIN_PASSWORD` env var.

This gets us strong security without needing external DB immediately.

---

## Phase 1 — Minimal login (MVP) with real security primitives

### Backend endpoints (svc-admin)

Add routes:

1. `POST /api/auth/login`

   * body: `{ username, password }`
   * verifies password hash
   * returns: `200` and sets session cookie
   * emits audit log event `auth.login.success|fail`

2. `POST /api/auth/logout`

   * clears cookie
   * returns 200

3. `GET /api/auth/me`

   * returns user summary if logged in
   * `{ username, roles, issuedAt }`

### Backend middleware

Implement `RequireAdminAuthLayer`:

* checks session cookie
* attaches `UserCtx` to request extensions
* rejects unauthorized with 401

### Protect these surfaces

* all non-public API routes
* optionally protect UI route delivery (or allow UI shell but API requires login)

**Dev overrides**

* `SVC_ADMIN_AUTH=off` for local dev
* `SVC_ADMIN_AUTH=on` default
* or `SVC_ADMIN_DEV_MODE=1` toggles insecure.

---

## Phase 2 — UI login flow

Add UI route(s):

* `/login`

  * username/password
  * handles 401 errors and rate-limits UI attempts
* “logged in” state stored in app context, refreshed via `/api/auth/me`

If not authenticated:

* redirect to `/login`
* show “Session expired” toast when API returns 401

---

## Phase 3 — User/role model + policy hooks

Add RBAC scaffolding:

* roles: `admin`, `operator`, `viewer`
* permissions: `nodes:read`, `nodes:write`, `config:write`, `policy:write`, etc.

Even if only `admin` exists initially, wire the model now so we don’t rewrite later.

---

## Phase 4 — Hardening (must-do before any real deployment)

Security checklist:

* bcrypt/argon2id (prefer argon2id)
* constant-time password compare
* rate limit login endpoint (IP + username)
* session TTL + idle timeout
* CSRF protection for cookie auth (double-submit cookie or sameSite + CSRF token for POST)
* secure cookie flags (httpOnly, Secure in prod, SameSite Lax/Strict)
* audit logging for auth + privileged actions
* config for rotating session signing key

---

## Phase 5 — Integrate with future node authentication (clean boundary)

Later, when svc-admin performs actions on nodes:

* svc-admin will authenticate **user**
* svc-admin will authenticate **to nodes** separately (node API tokens / mTLS / OAP auth)
* never reuse the same token/cookie between dashboard and node planes

---

# Immediate next-instance TODO list (ordered)

## A) Notes updates (done by this note)

* Add “node bandwidth must be node-specific” invariant.
* Add “Network page” plan.

## B) Admin login build-out

1. Locate and review:

   * `svc-passport` and any auth-related crates
   * any existing password hashing / token code in the monorepo
2. Implement backend MVP:

   * `/api/auth/login`, `/api/auth/logout`, `/api/auth/me`
   * session cookie + middleware
3. Implement UI:

   * login screen + auth guard

## C) Observability updates

* Add svc-admin auth metrics:

  * `ron_svc_admin_auth_login_attempts_total{result}`
  * `ron_svc_admin_auth_sessions_active`
* Add audit logs for login attempts

---

# Addendum: Bandwidth tile requirements (explicit)

* Bandwidth displayed on a node card MUST reflect the node’s own traffic.
* The dashboard should NOT treat host NIC totals as node bandwidth.
* A separate `Network` page should show combined totals and breakdowns.

---

If you paste the **svc-admin codebundle** next, I can do the “inventory pass” on what auth primitives already exist (svc-passport, hashing, tokens, middleware patterns), and then I’ll give you a **concrete implementation plan** that matches your repo’s existing style (axum 0.7, layers, DTO patterns, metrics naming, and “truthful readiness”).

ADDITIONAL TODO:
Cpu thread and core count is wrong for micronode
Cannot open storage for micronode to see details (ram only is why?)
Need amnesia and persistence (or both) and a breakdown of what dirs are amnesia and persistence (if both)
Need further clarification on the different planes listed in micronode (they differ from macronode not sure if it is some unified system or mock)

### END NOTE - JANUARY 9 2026 - 12:30 CST