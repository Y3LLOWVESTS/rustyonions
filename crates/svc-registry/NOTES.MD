### BEGIN NOTE - NOVEMBER 10 2025 - 12:17 CST

**Estimated beta completion for `svc-registry`: ~55–60%** (I’ll call it **58%**).
Foundation is in, admin plane is solid, basic API + metrics + SSE heartbeat are working, pins are clean, and we’ve got a repeatable smoke script. The remaining work is mostly storage + write-paths + readiness wiring + a few invariants/tests and hardening passes.

---

# CARRY-OVER NOTES — `svc-registry` (Beta Push)

## 0) TL;DR (where we are)

* **Build/Run**: clean with workspace-pinned `axum 0.7.9` / `tower 0.5.2` / `tower-http 0.6.6`.
* **Admin plane**: `/healthz` 200, `/readyz` 503 (degraded by design until storage init), `/version` JSON, `/metrics` Prometheus with per-route histograms + counters.
* **API (foundation)**: `GET /registry/head` returns a versioned head (in-mem store), `GET /registry/stream` emits SSE **heartbeats**.
* **Metrics**: request histograms + `requests_total` labeled by `method`, `route`, `status`. Verified to advance under load.
* **Middleware stack**: correlation-id, CORS (permissive for now), timeouts, request size limits, metrics layer, error mapping.
* **Scripts**: `crates/svc-registry/scripts/smoke_registry.sh` drives admin checks, API check, metrics movement, SSE smoke.
* **Tests**: HTTP contract + chaos/readiness scaffolds pass; loom scaffold passes.

**Next milestone**: storage init + minimal **commit** write-path → flip readiness → SSE “commit” events → golden tests.

---

## 1) What’s working (proof)

* **Service boots** and logs:

  * `svc-registry listening admin_addr=127.0.0.1:9909 api_addr=127.0.0.1:9444`
* **Admin plane**:

  * `/healthz` → 200 (empty body)
  * `/readyz` → 503 with `{ degraded:true, ready:false, write_ready:false }` (correct default)
  * `/version` → includes `service`, `version`, `commit`, schema map
  * `/metrics` → contains:

    * `request_latency_seconds_bucket{route="/registry/head",...}`
    * `requests_total{method="GET",route="/registry/head",status="200"}`
* **API**:

  * `GET /registry/head` → `{version:0, payload_b3:"b3:0", committed_at:null}`
* **SSE**:

  * `GET /registry/stream` → periodic `data: heartbeat:<n>`
* **Metrics advance** when curling `/registry/head` repeatedly (seen histogram + counter increments).
* **Smoke script**:

  * Waits for /healthz
  * Asserts `/version` service name
  * Reads `/registry/head`
  * Asserts metrics increased by >=N
  * Reads a couple SSE lines
  * Prints “✅ svc-registry smoke OK”

---

## 2) Code map (high level)

```
crates/svc-registry/
  src/
    main.rs                        # entrypoint, pins middleware stack, binds listeners
    build_info.rs                  # commit/time capture (via build.rs)
    http/
      routes.rs                    # /registry/head, /registry/stream
      middleware/
        corr_id.rs                 # ULID correlation-id header + span fields
        metrics.rs                 # per-route latency histogram + requests_total
        limits.rs                  # request body limits
        timeouts.rs                # request timeouts + mapping to 408
    observability/
      endpoints.rs                 # /metrics /healthz /readyz /version
      logging.rs                   # json logs w/ EnvFilter
      metrics.rs                   # RegistryMetrics (Prometheus collectors)
    storage/
      inmem.rs                     # InMemoryStore: Head struct, version counter, SSE taps
      trait.rs (RegistryStore)     # (if split) async API for head/commit/subscribe
    config/
      model.rs                     # Config { bind_addr, metrics_addr, ... }
      load.rs                      # load_config(None) with TOML + env overrides
    shutdown.rs                    # graceful_shutdown()
  scripts/
    smoke_registry.sh              # smoke runner used above
  build.rs                         # sets env! for GIT_COMMIT_HASH, BUILD_TIME_RFC3339
  Cargo.toml                       # features aligned to workspace pins
```

---

## 3) Invariants (current + planned)

* **I-1 Ready gates**: service not “ready” until **storage initialized** and **SSE broadcaster alive**.
* **I-2 Monotonic head**: `version` strictly increases on each successful commit.
* **I-3 Idempotent read**: `GET /registry/head` is pure, fast, and cheap (no side effects).
* **I-4 Commit visibility**: commit returns only after the new head is **durably recorded** (for persistent backends) and **broadcast enqueued**.
* **I-5 Telemetry**: every HTTP request carries `corr_id`; metrics label cardinality remains bounded (route names, not raw URLs).
* **I-6 Backpressure**: SSE stream drops slow consumers and never stalls writers.
* **I-7 Limits & timeouts**: reject bodies > limit; latency buckets capture tail; timeouts mapped to 408.

---

## 4) Middleware stack (current behavior)

Order (outer → inner):

1. **MetricsLayer** — time whole request, label by matched route
2. **limits_layer** — cap request size (keep minimal for read-heavy paths)
3. **timeouts_layer** — overall per-request timeout; mapped to 408 with `HandleErrorLayer`
4. **CorsLayer::permissive** — dev-friendly (tighten for prod)
5. **CorrLayer** — generate/propagate `x-corr-id`, add to tracing span

Notes:

* We deliberately wrapped the **whole router** (not per route) so Axum’s `Router::layer` error bounds are satisfied (we map errors before Axum expects `Infallible`).

---

## 5) Metrics (names + intent)

* `request_latency_seconds` (histogram) with labels `{route}`; bucket defaults tuned for sub-ms → seconds.
* `requests_total` (counter) with `{method, route, status}`.
* **Planned** (beta):

  * `registry_commits_total{result}`: `ok`/`error`
  * `registry_sse_clients{state}`: `connected`/`dropped`
  * `registry_head_version` (gauge) — current head for dashboards
  * `registry_store_ops_total{op}` — `get_head`, `commit`, `subscribe`
  * `registry_event_queue_depth` (gauge)

---

## 6) Config (today vs. beta)

* **Today**: `bind_addr`, `metrics_addr` from `Config`, loaded via `load_config(None)`.
* **Beta add**:

  * `store.backend = "inmem" | "sled" | "fs" | "pg" | "kv"` (start with `inmem`, add `sled` later)
  * `limits.max_request_bytes`
  * `timeouts.request_ms`
  * `sse.heartbeat_ms`, `sse.max_clients`, `sse.drop_policy`
  * `cors.allowed_origins` (tighten)

---

## 7) Storage plan & API shape

### Trait (conceptual)

```rust
#[async_trait]
pub trait RegistryStore: Send + Sync {
    async fn head(&self) -> Head;                        // fast read
    async fn commit(&self, payload_b3: String) -> Head;  // monotonic bump
    fn subscribe(&self) -> BroadcastStream<RegistryEvent>; // SSE source
}
```

* **InMemoryStore**: already backs `head()`; add `commit()` to bump version + push event.
* **Future backends** (post-beta or late-beta optional): `sled` (simple), `fs` (append-only log), `pg` (durable).

---

## 8) API: current & upcoming

* ✅ `GET /registry/head` → `{version, payload_b3, committed_at}`
* ✅ `GET /registry/stream` → SSE `heartbeat:<n>` (commit events soon)
* ⏳ **POST `/registry/commit`** (beta must-have):

  * Request: `{ payload_b3: "b3:<...>" }`
  * Response: new `Head`
  * Emits SSE event: `event: commit\ndata: {"version":..., "payload_b3":"...", "committed_at":...}`
  * Idempotency: if same current head, either no-op or re-emit (decide policy; simplest: no-op).

Optional nice-to-haves:

* `GET /registry/history?limit=N` (in-mem ring or best-effort)
* `GET /registry/stats` (head + counts, mostly mirrors metrics)

---

## 9) Readiness wiring (flip `/readyz` green)

* On boot:

  * init store → set `services_ok = true`
  * start SSE broadcaster → set `queues_ok = true`
* `/readyz`: green when `services_ok && queues_ok` (and later any store/write gates).

---

## 10) Tests (present & to add)

**Present (passing)**

* `tests/http_contract.rs`: `healthz_ok`, `readyz_degraded_by_default`, `version_shape`, `registry_head_shape`
* `tests/chaos_ready.rs`: scaffold
* `tests/concurrency_loom.rs`: scaffold
* `tests/invariants.rs`: scaffold

**Add for beta**

* `commit_happy_path`: POST commit → head.version += 1 → SSE commit event observed
* `monotonic_version_property`: many commits; assert strictly increasing
* `sse_multi_clients`: spawn multiple listeners; assert all see the commit (within timeout)
* `limits_enforced`: giant body → 413
* `timeouts_mapped`: injected delay → 408
* `corr_id_propagates`: ensure header → logs/metrics (integration with tracing test capture)
* `readiness_flip`: before storage init 503 → after init 200

---

## 11) Observability & logs

* JSON logs with `tracing-subscriber` (`EnvFilter`) + `tracing-serde`.
* Every request includes `corr_id` (header and span).
* Add structured fields on commit: `version`, `payload_len`, `dur_ms`.

---

## 12) Security & policy (beta scope)

* Keep **CORS permissive** only in dev; add config to restrict origins.
* Define **max request size** and **timeout** defaults sane for LAN dev.
* No auth at this layer (will rely on `svc-passport/ron-auth` when integrated). Leave hooks on write paths to insert auth layer later (tower layer spot reserved).

---

## 13) Performance expectations

* **Read path** (`GET /registry/head`): sub-millisecond on dev laptop is expected (observed buckets < 5ms).
* **SSE**: heartbeats cheap; ensure per-client buffers small; drop slow clients.
* **Write path** (in-mem): expect micro- to sub-millisecond updates + broadcast to M clients.
* Beta perf gate idea:

  * P50 head: < 1 ms, P99 < 5 ms
  * Commit throughput (in-mem): > 50k/s single thread (local), mostly bounded by broadcast fan-out.

---

## 14) Interop with other crates

* **ron-kernel**: HealthState + wait_for_ctrl_c (already used)
* **ron-metrics**: Prometheus registry & HTTP export (used through endpoints module)
* **ron-bus / svc-overlay**: future: announce head changes onto bus (optional)
* **svc-passport / ron-auth**: future: add write-path auth (macaroons/passports)
* **ron-policy**: future: authorizers on commit (validate payload_b3 rules)

---

## 15) Developer ergonomics

### One-shot run

```bash
cargo run -p svc-registry
```

### Smoke (spawns, waits, asserts)

```bash
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

### Handy curls

```bash
curl -i http://127.0.0.1:9909/healthz
curl -s http://127.0.0.1:9909/version | jq .
curl -s http://127.0.0.1:9909/metrics | head
curl -s http://127.0.0.1:9444/registry/head | jq .
curl -N http://127.0.0.1:9444/registry/stream | head -n 3
```

---

## 16) What’s left for **Beta**

### Must-have

* [ ] **Storage init gate** + flip `/readyz` once active (set `services_ok`, `queues_ok`).
* [ ] **POST `/registry/commit`** write-path in in-mem store (monotonic version; capture `committed_at`).
* [ ] **SSE commit events** (named `commit`) including version + payload.
* [ ] **Golden tests** for commit/read/SSE/readiness/limits/timeouts.
* [ ] **Metrics**: `registry_commits_total{result}`, `registry_sse_clients{state}`, `registry_head_version`.
* [ ] **Tighten CORS via config** (dev permissive, prod restrictive).
* [ ] **Docs**: README/IDB/RUNBOOK for this crate (how to run, configure, smoke, and expected outputs).

### Nice-to-have (if time allows; otherwise post-beta)

* [ ] Persistent backend option (**sled**) behind a feature flag (`--features sled-store`).
* [ ] `GET /registry/history?limit=N` (ring in memory).
* [ ] Bus emission (publish commit summaries onto `ron-bus` channel).
* [ ] Simple auth hook (accept bearer and log; enforcement later).
* [ ] Basic benchmark script for read path and commit throughput (criterion or wrk-like with hyper client).

---

## 17) Risks / gotchas

* Axum router `.layer(...)` type bounds are strict — **always** map layer errors to `Infallible` before calling `Router::layer`.
* Keep **metrics label cardinality** low (route templates only).
* SSE can leak memory if per-client buffers aren’t bounded—verify drop policy.
* Don’t flip `/readyz` green until both **store** and **broadcaster** are up.

---

## 18) Next steps (recommended order)

1. Implement **commit** in `InMemoryStore` + route `POST /registry/commit`.
2. Wire **SSE commit events** (keep heartbeats).
3. Flip **readiness** once store+broadcaster active.
4. Add **golden tests** (commit path + SSE) and extend `smoke_registry.sh` to POST and assert version++ and SSE commit line appears.
5. (Optional) `sled` backend under feature flag.
6. Tighten **CORS** via config and finalize README.

---

If you want, I can drop in paste-ready files next for:

* `storage/trait.rs`, `storage/inmem.rs` (commit + broadcaster),
* `http/routes.rs` (POST commit handler),
* and an updated `smoke_registry.sh` that performs a commit and validates SSE.

### END NOTE - NOVEMBER 10 2025 - 12:17 CST






### BEGIN NOTE - NOVEMBER 10 2025 - 13:12 CST
---

# CARRY-OVER NOTES — `svc-registry` (Beta Push)

**Date:** 2025-11-10 (Mon) 12:55 CST
**Beta completion estimate:** **~72%** (previous: ~58%).
**Why the bump:** write-path (`POST /registry/commit`) works end-to-end; SSE emits `event: commit`; readiness gates now flip `/readyz` to 200 after init; registry metrics added and integrated; smoke script covers the full happy path.

---

## 0) TL;DR

* **Build/Run:** green on workspace pins (`axum 0.7.9`, `tower 0.5.2`, `tower-http 0.6.6`, `tokio-stream` with `sync`).
* **Admin plane:** `/healthz` 200; `/readyz` 200 after store+broadcaster init; `/version` JSON; `/metrics` Prometheus.
* **API:**

  * `GET /registry/head` → versioned head (in-mem).
  * `POST /registry/commit` → monotonic bump + SSE commit event.
  * `GET /registry/stream` (SSE) → heartbeats + `event: commit` with JSON head.
* **Metrics:**

  * HTTP: `request_latency_seconds{route}`, `requests_total{method,route,status}`.
  * Registry: `registry_commits_total{result}`, `registry_head_version`, `registry_sse_clients_connected_total`, `registry_sse_clients_disconnected_total`.
* **Scripts:** `scripts/smoke_registry.sh` supports **two-terminal** and `--spawn`; asserts metrics moves, commit bump, SSE commit event observed.
* **Clippy:** clean with `deny` gates (we locally allowed `expect_used` inside metrics registration only).

---

## 1) What’s demonstrably working (proof)

* **Boot + bind:** service prints `svc-registry listening` with `admin_addr=127.0.0.1:9909` and `api_addr=127.0.0.1:9444`.
* **Admin URLs:**

  * `/healthz` → 200 (empty body).
  * `/readyz` → 200 after storage constructed and broadcaster probe (`store.subscribe()`) succeeds. Before these flip, it returns 503 with truthful flags.
  * `/version` → JSON: `{service, version, commit, schema:{"registry":"1.0.0"}, deprecations:[]}`.
  * `/metrics` → exposes both **HTTP** and **registry** metrics.
* **Read plane:** `GET /registry/head` shows `{version: <u64>, payload_b3: "b3:...", committed_at: RFC3339 or null}`.
* **Write plane:** `POST /registry/commit` with `{"payload_b3":"b3:<base64>"}` bumps `version` by +1, sets `committed_at`, and returns the new head.
* **SSE:** `GET /registry/stream` yields periodic `event: heartbeat` lines plus `event: commit` with JSON head on write.
* **Smoke script:** verifies:

  * `/healthz` OK
  * `/readyz` (now 200)
  * `/version` shape
  * HTTP `requests_total` advanced for `/registry/head`
  * Commit path: version increments by +1 and `/registry/head` matches the committed version
  * `event: commit` appears in the SSE log

---

## 2) /// REDACTED

## 3) API contract (beta)

### `GET /registry/head`

* **Response:** `200 OK`

```json
{
  "version": 42,
  "payload_b3": "b3:SGVsbG8=",
  "committed_at": "2025-11-10T18:43:24.854241Z"
}
```

### `POST /registry/commit`

* **Request:** `{"payload_b3":"b3:<base64>"}` (must start with `b3:`)
* **Responses:**

  * `200 OK` with new head (monotonic `version += 1`)
  * `400 Bad Request` with `{"error":{"code":"invalid_payload","message":...}}`
  * `500 Internal Server Error` with `{"error":{"code":"commit_failed","message":...}}`

### `GET /registry/stream` (SSE)

* **Events:**

  * `event: heartbeat` with `data: <counter>`
  * `event: commit` with `data: <JSON Head>`

**Idempotency:** currently **not** deduplicating same-value commits; client decides policy. (We can add a no-op guard keyed by exact payload match if needed.)

---

## 4) Readiness & invariants

* **Readiness gates (truthful):**

  * `services_ok = true` once store constructed.
  * `queues_ok = true` once a `subscribe()` probe succeeds (broadcaster alive).
  * `/readyz` → 200 only when `services_ok && queues_ok`.

* **Invariants:**

  * **I-1 Monotonic version:** head.version strictly increases on successful commits.
  * **I-2 Commit visibility:** response is returned only after head updated and commit broadcast enqueued (in-mem).
  * **I-3 SSE non-blocking:** slow consumers dropped by broadcast semantics (laggers don’t backpressure writers).
  * **I-4 Bounded label cardinality:** metrics labeled by **route templates** (not raw paths).

---

## 5) Metrics (names → intent)

### HTTP (via middleware)

* `requests_total{method,route,status}` — request counts (all routes).
* `request_latency_seconds_bucket{route,...}` — latency histogram per route.

### Registry (new)

* `registry_commits_total{result="ok|error"}` — write-path outcomes.
* `registry_head_version` (gauge) — current head (for dashboards/alerts).
* `registry_sse_clients_connected_total` / `registry_sse_clients_disconnected_total` — lifecycle counters.

> Note: SSE “disconnect” is currently best-effort. Axum 0.7’s `Sse` lacks `on_close`; we will add a small channel-close hook if we need stronger accounting.

---

## 6) Smoke + Runbook

### One-terminal (spawns service)

```
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

* Kills holders on ports, runs `cargo run -p svc-registry`, waits for `/healthz`, performs checks, and tears down.
* Logs: `/tmp/svc-registry.log`.

### Two-terminal

Terminal A:

```
cargo run -p svc-registry
```

Terminal B:

```
crates/svc-registry/scripts/smoke_registry.sh
```

**Expected highlights during smoke:**

* `/readyz` returns **200** (now green).
* `COMMIT RESP` shows `version: initial+1`.
* SSE log contains `event: commit` with matching JSON head.
* Metrics: `requests_total` advances by ≥5 for `/registry/head`.

---

## 7) Testing plan (beta “golden” set)

**Already covered by smoke** (manual/integration):

* commit happy-path (bump + SSE event).
* HTTP metrics movement for `/registry/head`.

**Add as automated tests** (`tests/*.rs`):

1. **`commit_happy_path`**: POST commit → `version += 1`, then GET head equals committed head; assert `registry_commits_total{ok}` increments.
2. **`monotonic_version_property`**: N commits → strictly increasing; no duplicates.
3. **`sse_multi_clients`**: spawn M SSE readers; one commit; all observe `event: commit` within timeout.
4. **`limits_enforced`**: POST body > limit → `413 Payload Too Large`.
5. **`timeouts_mapped`**: inject handler delay beyond per-request timeout → `408 Request Timeout`.
6. **`readiness_flip`**: before init → `/readyz` 503; after gates set → `/readyz` 200.
7. **`corr_id_propagates`**: header present → echoed to logs/trace span (capture test sink).
8. **`metrics_shape`**: `/metrics` contains the four registry collectors plus HTTP collectors.

---

## 8) Config (today vs. beta)

**Today (implicit defaults):**

* `admin_addr = 127.0.0.1:9909`
* `api_addr = 127.0.0.1:9444`

**Beta (recommended keys):**

```toml
[bind]
admin = "127.0.0.1:9909"
api   = "127.0.0.1:9444"

[limits]
max_request_bytes = 65536

[timeouts]
request_ms = 5000

[sse]
heartbeat_ms = 5000
max_clients  = 8192
drop_policy  = "lag-drop"   # informative; behavior is broadcast drop

[cors]
allowed_origins = ["http://localhost:*"]  # tighten in prod
```

* Env overrides respected (e.g., `REGISTRY_BIND__ADMIN`, `REGISTRY_SSE__HEARTBEAT_MS`) if you keep our standard loader pattern.

---

## 9) Performance expectations & gates

* **Read path** (`GET /registry/head`): sub-ms on dev laptop; P99 < 5ms.
* **Write path (in-mem)**: micro- to sub-ms for head update + enqueue; throughput mostly bounded by SSE fan-out.
* **Beta perf gates (dev laptop):**

  * `request_latency_seconds{route="/registry/head"}` P50 < 1ms, P99 < 5ms (wrk-like 1–2 threads OK).
  * Commit throughput: ≥ 50k/s single thread with no SSE clients; with 100 SSE clients, ≥ 10k/s observed.

**Post-beta** (optional benches):

* Criterion microbench for commit loop.
* `wrk`/`oha` profiles for `/registry/head` and `/registry/commit`.
* Histogram SLO alerts on dashboards.

---

## 10) Security & policy (beta scope)

* **CORS**: dev-permissive; add config to tighten origins in prod.
* **Auth**: none yet on write path; hook point reserved (tower layer) to integrate `svc-passport`/`ron-auth` macaroons.
* **Input validation**: enforce `payload_b3` prefix; consider size cap (already via limits) and optional base64 check later.
* **Observability**: structured logs; correlation ID (`x-corr-id`) set/propagated.

---

## 11) Interop with RON-CORE crates

* **ron-kernel**: `HealthState` + `wait_for_ctrl_c()` (used for readiness and shutdown).
* **ron-metrics**: Prometheus crates and patterns; we export via a local `/metrics` handler against the default registry.
* **ron-bus/svc-overlay** (post-beta): optional publish of commit summaries to the bus for cross-service awareness.
* **svc-passport/ron-auth**: future write-path authorization via bearer/macaroons; plug as `tower::Layer` before handlers.
* **ron-policy**: optional policy check on `commit` (e.g., schema/rules on payload).

---

## 12) Risks / watch-outs

* **SSE disconnect accounting:** without `on_close`, totals are best-effort; not a blocker for beta. We can add a tiny probe (drop-notify channel) to track disconnects precisely.
* **Metrics duplication:** multiple constructors would re-register collectors; we guard by constructing once at boot (panic on duplicates is acceptable).
* **Label cardinality:** keep `route` labels to match templates (avoid embedding variable segments).
* **Backpressure:** default `broadcast` lag behavior drops slow consumers — by design. Document this for clients.

---

## 13) What’s left for **Beta**

**Must-have**

* [ ] Automated tests (list in §7).
* [ ] Config loader wire-up (binds, limits, timeouts, SSE, CORS) with env overrides.
* [ ] README / IDB / RUNBOOK updates to reflect final endpoints, config keys, and smoke steps.
* [ ] Tighten CORS via config (prod profile).
* [ ] Add `registry_event_queue_depth` (gauge) and/or a lagged-events counter if we want visibility on dropped SSE events.

**Nice-to-have (post-beta or late-beta)**

* [ ] Persistent store feature flag (e.g., `sled-store`) with append-only log + replay to build head.
* [ ] `GET /registry/history?limit=N` from in-mem ring (debugging).
* [ ] Bus emission (publish commit summaries onto `ron-bus`).
* [ ] Auth layer integration (macaroon verification via `svc-passport`/`ron-auth`).

---

## 14) Quick commands

**Dev loop**

```
cargo fmt -p svc-registry
cargo clippy -p svc-registry --no-deps -- -D warnings
cargo run -p svc-registry
```

**Smoke (spawn)**

```
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

**Handy curls**

```
curl -i http://127.0.0.1:9909/healthz
curl -s http://127.0.0.1:9909/readyz | jq .
curl -s http://127.0.0.1:9909/version | jq .
curl -s http://127.0.0.1:9909/metrics | head
curl -s http://127.0.0.1:9444/registry/head | jq .
curl -N http://127.0.0.1:9444/registry/stream | head -n 4
curl -s -X POST http://127.0.0.1:9444/registry/commit \
  -H 'content-type: application/json' \
  --data '{"payload_b3":"b3:SGVsbG8="}' | jq .
```

---

## 15) Troubleshooting

* **`tokio_stream::wrappers::BroadcastStream` missing**
  Ensure `tokio-stream = { version="0.1.17", features=["sync"] }` in `Cargo.toml`.

* **`/readyz` stuck at 503**
  Confirm gates are flipped: `set_services_ok(&health, true)` after store init and `set_queues_ok(&health, true)` after a `store.subscribe()` probe.

* **SSE “commit not observed” in smoke**
  The script now uses `curl -sN` and robust grep; if it still fails, inspect the temp file path printed by the script tail.

* **Clippy `new_without_default`**
  We implemented `Default` and allow `expect_used` inside metrics registration only.

---

## 16) Definition of Done (Beta)

* ✅ Endpoints: `/healthz`, `/readyz`, `/version`, `/metrics`, `/registry/head`, `/registry/commit`, `/registry/stream`.
* ✅ Readiness truthful and flips green post-init.
* ✅ HTTP + registry metrics visible and advancing in smoke.
* ✅ Smoke script green in both modes.
* ⏳ Automated test suite (golden list in §7) checked into CI; `cargo test -p svc-registry` green.
* ⏳ Config loader active; README/IDB/RUNBOOK updated.
* ⏳ CORS tightened via config for prod profile.

---

## 17) Next steps (recommended order)

1. Implement **config loader** (binds, limits, timeouts, SSE, CORS) + env overrides.
2. Add the **automated tests** from §7.
3. Wire `registry_event_queue_depth` gauge (optional).
4. Finalize **docs** (README/IDB/RUNBOOK), include example dashboards/panels for key metrics.
5. (Optional) Feature-gate a **sled** persistent store and add a simple replay on boot.

---

### END NOTE - NOVEMBER 10 2025 - 13:12 CST


### BEGIN NOTE — NOV 10, 2025 — 13:55 CST (svc-registry smoke locked)

**Status:** ✅ Smoke green in both modes
**Build:** `cargo fmt && cargo clippy -D warnings` clean
**Script:** `crates/svc-registry/scripts/smoke_registry.sh` (two-terminal + `--spawn`)
**Highlights:**

* `/healthz` 200; truthful `/readyz` 200 post-init.
* `/version` JSON as expected.
* `/metrics`: counters/gauges registered; `requests_total` increments; `registry_head_version` visible.
* Read path: `GET /registry/head` returns `{version:0,payload_b3:"b3:0",committed_at:null}` on fresh boot.
* Write path: `POST /registry/commit` bumps `version` by +1 and reflects on subsequent `GET /registry/head`.
* SSE: `event: commit` observed on stream; heartbeat line present.
* Smoke assertions:

  * `requests_total{route="/registry/head"}` value delta ≥ 5 after five GETs.
  * Head bump verified: `version -> version+1` and HEAD matches.
  * SSE commit event captured and logged.

**What changed since last note:**

* Metrics module: added `Default`, scoped allow for `expect_used` only during collector registration.
* Added `sse_client_connected()`; silenced `sse_client_disconnected()` until close-hook lands.
* Smoke script hardened:

  * Spawn mode now waits on “listening” log or `/healthz` probe.
  * Metrics delta uses counter **value** (not line count).
  * macOS-safe port cleanup (`lsof -ti tcp:$PORT`), tidy PID teardown.

**Next (beta DoD items):**

1. Automated tests (golden set):

   * `commit_happy_path` (bump + head equality + `registry_commits_total{ok}` inc).
   * `monotonic_version_property` (N commits strictly increasing).
   * `sse_multi_clients` (M readers observe `event: commit`).
   * `limits_enforced` → 413 on oversized body.
   * `timeouts_mapped` → 408 on forced handler delay.
   * `readiness_flip` (pre-init 503 → post-init 200).
   * `metrics_shape` (collectors present).
2. Config loader wire-up w/ env overrides:

   * `[bind] admin, api`, `[limits] max_request_bytes`, `[timeouts] request_ms`,
     `[sse] heartbeat_ms, max_clients, drop_policy`, `[cors] allowed_origins`.
3. Docs polish:

   * README/IDB/RUNBOOK: final endpoints, config keys, and the new smoke flow (both modes).
4. (Optional late-beta) add `registry_event_queue_depth` gauge for visibility on drop/lag.

**Quick commands:**

```
# split terminal
cargo run -p svc-registry
crates/svc-registry/scripts/smoke_registry.sh

# one terminal
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

**Gotchas to remember:**

* If `/metrics` appears stale during the delta check, give a brief `sleep 0.2` (already in script).
* `jq` is required by the smoke script. Install if missing: brew install jq.

### END NOTE — NOV 10, 2025 — 13:55 CST






### BEGIN NOTE - NOVEMBER 10 2025 - 16:37 CST

# CARRY-OVER NOTES — `svc-registry` (Beta Complete)

**Date:** 2025-11-10
**Scope:** Minimal but truthful registry with monotonic head, commit API, SSE stream, metrics + admin plane, file-backed config defaults, build info.

---

## 0) TL;DR (state of the world)

* ✅ **Beta complete** for agreed scope. Build, smoke, and tests are green.
* **Public API (beta):**

  * `GET /registry/head` → current head `{version, payload_b3, committed_at}`
  * `POST /registry/commit` → increments head; returns new head
  * `GET /registry/stream` (SSE) → `event: heartbeat` and `event: commit` lines
* **Admin plane:**

  * `GET /healthz`, `GET /readyz`, `GET /version`, `GET /metrics`
  * Readiness gates flip to 200 once services + queues are OK
* **Storage:** in-memory reference impl via `RegistryStore` trait
* **Metrics:** HTTP totals + latencies, commit outcomes, head gauge, SSE lifecycle
* **Config:** file loader + sane defaults (env overrides are deferred)
* **Build info:** commit SHA + build RFC3339 exposed at `/version`

---

## 1) What we shipped (beta accomplishments)

### Functional

* Monotonic head with atomic version bump on `POST /registry/commit`.
* Stable JSON DTOs; predictable field names and types.
* SSE stream with periodic heartbeat (keeps clients alive) and commit events.
* Truthful readiness: `readyz` returns 200 only when `services_ok && queues_ok` gates are true.

### Observability

* Prometheus counters/histograms with bounded cardinality:

  * `requests_total{method,route,status}`
  * `request_latency_seconds{route}`
  * `registry_commits_total{result="ok|error"}`
  * `registry_head_version`
  * `registry_sse_clients_connected_total`, `registry_sse_clients_disconnected_total`
* `/version` contains `{ service, version, commit, schema, deprecations }`.

### DX / Structure

* Clean module layout: `config/`, `http/`, `observability/`, `storage/`.
* `RegistryStore` trait decouples HTTP from storage impl.
* Smoke script: exercises admin plane, metrics movement, SSE, and commit flow.

---

## 2) How to build, run, smoke

### Build (dev or release)

```bash
# dev
cargo build -p svc-registry

# release
cargo build -p svc-registry --release
```

### Run (split-terminal recommended)

**Terminal A**

```bash
cargo run -p svc-registry
# logs: {"message":"listening","admin_addr":"127.0.0.1:9909","api_addr":"127.0.0.1:9444"}
```

**Terminal B**

```bash
crates/svc-registry/scripts/smoke_registry.sh
```

### Run with auto-spawn (one terminal)

```bash
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

Notes:

* `--spawn` starts the binary and stops it at the end.
* If you previously had a server running, kill it first to avoid port conflicts.

---

## 3) How to run tests

### All tests

```bash
cargo test -p svc-registry
```

### Focused tests

```bash
cargo test -p svc-registry --test api_commit
cargo test -p svc-registry --test http_contract
cargo test -p svc-registry --test metrics_text
cargo test -p svc-registry --test storage_monotonic
```

**What they cover**

* `api_commit`: round-trip commit and body shape; head reflection.
* `http_contract`: admin plane contracts; readiness flips to 200 with gates.
* `metrics_text`: `/metrics` includes registry counters; text exposition OK.
* `storage_monotonic`: head is monotonic; event stream fires on commit.

---

## 4) API contracts (beta)

### `GET /registry/head`

**200 OK**

```json
{
  "version": 1,
  "payload_b3": "b3:....",
  "committed_at": "2025-11-10T19:52:04.008654Z"
}
```

* `version`: u64 (monotonic; starts at 0)
* `payload_b3`: `b3:<base64url or token-like>` (opaque b3 string the caller provides)
* `committed_at`: RFC3339 or `null` when `version=0`

### `POST /registry/commit`

**Request**

```json
{ "payload_b3": "b3:your-payload" }
```

**Response 200**

```json
{
  "version": 2,
  "payload_b3": "b3:your-payload",
  "committed_at": "2025-11-10T21:30:25.380255Z"
}
```

* On success increments head by +1 and timestamps.

### `GET /registry/stream` (SSE)

* Emits:

  * `event: heartbeat` (periodic)
  * `event: commit` with `data: {"version":N,...}`

**Example curl**

```bash
curl -N http://127.0.0.1:9444/registry/stream
```

---

## 5) Admin plane contracts

### `GET /healthz`

* Always `200 OK` (liveness).

### `GET /readyz`

* **200** only when `services_ok && queues_ok` gates are true (we flip them during bootstrap after storage + a subscribe probe).
* Body:

```json
{ "ready": true, "degraded": false, "services_ok": true, "queues_ok": true }
```

### `GET /version`

* Example:

```json
{
  "service": "svc-registry",
  "version": "0.1.0",
  "commit": "557a122b7f9efa398fef2b14de80aded47d80135",
  "schema": { "registry": "1.0.0" },
  "deprecations": []
}
```

### `GET /metrics`

* Prometheus text exposition.

---

## 6) Configuration

### Files

* `crates/svc-registry/src/config/`

  * `model.rs` — `Config` struct + defaults
  * `load.rs` — file loader, precedence: explicit file → defaults (env overrides deferred)
  * `validate.rs` — guardrails
  * `reload.rs` — stub (for future hot-reload)

### Defaults (important)

* `api` bind: `127.0.0.1:9444`
* `admin` bind: `127.0.0.1:9909`
* Timeouts: read/write `5s`, idle `60s`
* Storage: `in-memory` for beta

### Loading

* Code calls `AppConfig::load()` (wrapper around `config::load::load_config(None)` in current wiring)
* **Future:** SVCR_* env overrides + hot-reload.

---

## 7) Storage & events

### Trait

`storage::RegistryStore`:

* `async fn head(&self) -> Head`
* `async fn commit(&self, payload_b3: String) -> anyhow::Result<Head>`
* `fn subscribe(&self) -> broadcast::Receiver<RegistryEvent>`

### Events

`RegistryEvent::Commit(Head)` currently (we kept names tight and simple for beta).

### Current impl

* `storage::inmem::InMemoryStore`:

  * Holds `Head { version, payload_b3, committed_at }`
  * Broadcasts on commit; used by SSE handler and tests.

**Future plug-ins:** sled/sqlite backends behind the same trait (keep DTO stable).

---

## 8) Metrics (names & when they move)

* `requests_total{method,route,status}` → every HTTP request (both planes).
* `request_latency_seconds{route}` → observed per matched route template.
* `registry_commits_total{result}` → increments `ok` on success; `error` on failure.
* `registry_head_version` → gauge set on commit and when head is read.
* `registry_sse_clients_connected_total` → increments on new SSE connection.
* `registry_sse_clients_disconnected_total` → reserved; to be hooked on close.

**Test note:** We avoided “AlreadyReg” panics by registering once per process and using the default Prometheus registry consistently. Tests construct routers carefully to avoid duplicate registration in the same process.

---

## 9) Using `svc-registry` with other services

* **Read path (polling):** downstream service can `GET /registry/head` periodically to detect changes.
* **Event path (push):** prefer `GET /registry/stream` SSE to react to `commit` in near real-time (no thundering herd).
* **Write path:** only permitted writers should call `POST /registry/commit` (authorization/policy is future work).
* **Integration sketch:**

  * On boot, a consumer connects to SSE; on `commit`, refetch head and apply.
  * For idempotent pipelines, consider comparing the observed `version` with a cached `last_applied_version`.

---

## 10) Operational runbook (short)

**Start**

```bash
RUST_LOG=info ./target/release/svc-registry
```

**Health**

```bash
curl -sS http://127.0.0.1:9909/healthz -i
curl -sS http://127.0.0.1:9909/readyz  -i
curl -sS http://127.0.0.1:9909/version | jq
```

**Metrics**

```bash
curl -sS http://127.0.0.1:9909/metrics | head -n 50
```

**Commit**

```bash
curl -sS -XPOST http://127.0.0.1:9444/registry/commit \
  -H 'content-type: application/json' \
  -d '{"payload_b3":"b3:example"}' | jq
```

**Stream**

```bash
curl -N http://127.0.0.1:9444/registry/stream
```

**Stop**
Ctrl-C (graceful via `wait_for_ctrl_c()`).

---

## 11) Invariants & guardrails

* **Monotonicity:** `version` strictly increases on successful commits.
* **Atomicity:** each commit produces a single new `Head`.
* **Truthful readiness:** 200 only when `services_ok && queues_ok`.
* **Stable labels:** metrics label sets are fixed (no unbounded cardinality).
* **DTO stability:** field names/types remain compatible; additions must be backward-compatible.

---

## 12) Deferred “nice-to-have” backlog (post-beta)

* **Persistence backends:** `sled` (embedded) and `sqlite` (via `rusqlite`), behind `RegistryStore`.
* **History endpoints:** `GET /registry/history?from=..&to=..`, `GET /registry/at/:version`.
* **Diff/Patch:** simple semantic diffs between heads (format TBD).
* **Policy/quotas:** integrate `ron-policy`; enforce authN/Z for write path.
* **SSE disconnect accounting:** hook on-close to increment `disconnected_total`.
* **Env overrides:** `SVCR_*` → layered precedence (env > file > defaults).
* **Hot reload:** watch config file; atomically swap safe fields.
* **Structured logs:** correlation IDs, request IDs, JSON logs by default.
* **Benchmarks:** wrk profile for read path (head polling) & SSE fanout.

---

## 13) Troubleshooting (greatest hits)

* **Smoke `--spawn` can’t reach `/healthz`:** a prior instance may be holding ports; kill it, then rerun.
* **Prometheus “AlreadyReg” in tests:** ensure single metrics registry per process; don’t instantiate `RegistryMetrics` in multiple test modules without process isolation. Our current tests follow this.
* **SSE seems quiet:** verify server is running, and post a commit; heartbeats should appear even without commits.

---

## 14) Release hygiene

```bash
git tag -a svc-registry-beta -m "svc-registry beta"
git push --tags
cargo build -p svc-registry --release
crates/svc-registry/scripts/smoke_registry.sh --spawn
```

---

## 15) Quick embed example (other crate)

```rust
use std::sync::Arc;
use axum::Router;
use svc_registry::{
    config::model::Config,
    http::routes::registry_routes_with_cfg,
    observability::metrics::RegistryMetrics,
    storage::inmem::InMemoryStore,
};

fn build_registry_router() -> Router {
    let metrics = RegistryMetrics::default();
    let store   = Arc::new(InMemoryStore::new());
    let cfg     = Config::default();
    registry_routes_with_cfg(metrics, store, &cfg)
}
```

---

**Status:** Beta ✅. Ready for downstream consumers (read via SSE/poll, write via commit).
**Next up:** persistence, policy, and history when we move to the post-beta “nice-to-haves.”


### END NOTE - NOVEMBER 10 2025 - 16:37 CST


### BEGIN NOTE - JULY 10 2026 - 12:55 CST

Grounded in the latest Rust-only codebundle, BUILD_PLAN_Z, and the Phase 5 terminal/notes. `svc-registry` is the registry authority for service-node descriptors, reward-recipient bindings, eligibility/probation/quarantine state, and epoch registry roots; Phase 5 specifically added the reward binding registry, payout guard, self-pay rejection, and registry-backed binding/rotation intake.   

# BUILD_PLAN_Z — svc-registry Change Notes

## Exhaustive Carryover Notes for `crates/svc-registry`

## 0. Current svc-registry Status

Current crate:

```text id="9fq4xk"
crates/svc-registry
```

Current BUILD_PLAN_Z role:

```text id="huctr0"
Registry authority for service-node descriptors, reward-recipient bindings, recipient resolution, eligibility/probation/quarantine state, and future epoch registry roots.
```

Current status:

```text id="o2hf51"
svc-registry — GREEN / PARKED for Phase 5 reward-recipient binding, payout guard, and request intake.
```

Known green Phase 5 checks:

```bash id="5g89r9"
cargo test -p svc-registry --test reward_binding_registry
cargo test -p svc-registry --test reward_payout_guard
cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
cargo test -p svc-registry --test reward_binding_request_intake
cargo check -p svc-registry
cargo clippy -p svc-registry --all-targets --no-deps -- -D warnings
```

Known Phase 5 closeout:

```bash id="ec84w3"
bash scripts/check_phase5_reward_recipient_binding_closeout.sh
```

Known final closeout result:

```text id="5hve8o"
Phase 5 reward-recipient binding closeout passed.
Phase 5A/5B/5C are green.
svc-registry can intake strict binding/rotation requests and reject payout override/self-pay shortcuts.
No wallet mutation, ledger mutation, confirmed ROC, evidence payout override, or fake payout finality is exposed.
```

The terminal output confirms the Phase 5 closeout script ran the registry tests, payout guard tests, feature-gated self-issuance fixture test, `cargo check -p svc-registry`, focused Clippy, and workspace check. 

---

## 1. svc-registry’s Role in BUILD_PLAN_Z

BUILD_PLAN_Z assigns `svc-registry` to the economic identity/registry side of the service-node reward path.

Its role is:

```text id="qpsesn"
service-node descriptors
reward recipient bindings
recipient resolution
eligibility state
probation state
quarantine state
registry roots for epochs
future service-node registry authority
```

It is the place where a service node’s identity and reward-recipient binding become registry-resolved truth.

The global reward doctrine says:

```text id="yioo94"
No node mints alone.
No node self-pays.
No arbitrary payout address in service evidence.
No fake balances.
No fake receipts.
No fake finality.
Service-node rewards require service_node_id, accepted evidence, registry identity, bound reward recipient account, reward plan, policy validation, quorum acceptance, wallet execution, and ledger receipt.
```

That means `svc-registry` is not the wallet and not the ledger. It resolves identity and recipient binding, then other crates use that resolved account in later economic stages.

---

## 2. Main BUILD_PLAN_Z Phase Affecting svc-registry

The primary phase that changed `svc-registry` was:

```text id="83tk95"
Phase 5 — Node Operator Reward Recipient Binding
```

Phase 5’s goal:

```text id="8u5vex"
Allow service-node operators to receive ROC through a bound CrabLink/RON account, while preventing service evidence from redirecting payout and preventing node self-pay shortcuts.
```

Phase 5 work in `svc-registry`:

```text id="jvyefw"
store service-node descriptor
store reward recipient binding
resolve service_node_id to reward_recipient_account_id
support binding rotation
expose registry root for future epoch transitions
reject service evidence payout override
reject self-pay by default
provide registry-backed binding/rotation request intake
```

The build plan explicitly states the reward binding work in `svc-registry`: store service-node descriptors, reward recipient bindings, resolve `service_node_id` to `reward_recipient_account_id`, support rotation, and expose registry roots for epoch transitions. 

---

## 3. Main Files / Areas Changed

Primary source areas:

```text id="8u55pp"
crates/svc-registry/src/rewards/mod.rs
crates/svc-registry/src/rewards/issuance_guard.rs
crates/svc-registry/src/rewards/request_intake.rs
```

Primary tests:

```text id="8aj4cl"
crates/svc-registry/tests/reward_binding_registry.rs
crates/svc-registry/tests/reward_payout_guard.rs
crates/svc-registry/tests/reward_binding_request_intake.rs
```

The newer codebundle confirms these source and test files exist under `svc-registry`. 

---

## 4. Phase 5A — Reward Binding Registry Model

## 4.1 What changed

Added the registry model for service-node reward-recipient binding.

Primary module:

```text id="ul079g"
crates/svc-registry/src/rewards/mod.rs
```

Primary test:

```text id="6hjbsn"
crates/svc-registry/tests/reward_binding_registry.rs
```

## 4.2 Core responsibility

The registry model tracks:

```text id="w2kogs"
service_node_id
current reward binding
reward recipient account id
reward recipient display address
binding id
binding effective epoch
binding expiry
pending rotation
rotation nonce
rotation effective epoch
```

## 4.3 Important behavior

The registry can:

```text id="yadzt2"
insert a binding
resolve a service node to its bound reward recipient
return unbound state
return expired state without recipient smuggling
schedule a future rotation
apply pending rotation only after effective epoch
reject duplicate binding ids
reject duplicate rotation nonces
reject a second current binding for the same service node
reject rotation for missing/mismatched binding
no-op apply rotation when no pending rotation exists
```

## 4.4 Registry resolution contract

The registry’s key output is:

```text id="7a481k"
service_node_id -> bound reward_recipient_account_id
```

The key invariant is:

```text id="0lm9ah"
Downstream reward planning must use the registry-resolved reward recipient account, not an arbitrary payout address embedded in service evidence.
```

## 4.5 Test coverage

Known test file:

```text id="qnfjo9"
crates/svc-registry/tests/reward_binding_registry.rs
```

Known green tests:

```text id="i80udc"
registry_resolves_service_node_to_bound_reward_recipient
registry_returns_unbound_without_smuggling_recipient_fields
registry_reports_expired_binding_without_recipient_account
registry_schedules_pending_rotation_with_nonce_replay_rejection
registry_applies_pending_rotation_only_after_effective_epoch
registry_apply_rotation_is_noop_without_pending_rotation
registry_rejects_duplicate_binding_id_and_duplicate_nonce
registry_rejects_second_current_binding_for_same_service_node
registry_rejects_rotation_for_missing_or_mismatched_binding
```

Known green result:

```text id="90517m"
reward_binding_registry: 9 passed
```

The terminal output confirms all 9 `reward_binding_registry` tests passed. 

---

## 5. Phase 5A — Payout Authorization Preview Guard

## 5.1 What changed

Added payout preview guard logic.

Primary module:

```text id="6wzzhn"
crates/svc-registry/src/rewards/issuance_guard.rs
```

Primary test:

```text id="ywphts"
crates/svc-registry/tests/reward_payout_guard.rs
```

## 5.2 Core responsibility

The payout guard previews whether a service-node payout would be authorized based on registry-resolved recipient truth.

It is a guard/preview, not a wallet or ledger mutation.

## 5.3 Important types

Added or used types include:

```text id="250v9y"
preview_reward_payout_authorization
RewardPayoutAuthorizationError
RewardPayoutAuthorizationPreview
SelfIssuanceMode
```

## 5.4 What the guard rejects

The payout guard rejects:

```text id="i1bog0"
evidence_payout_override
unresolved recipient
beneficiary mismatch
self-issued payout by default
self-issuance fixture mode unless explicitly feature-gated
```

## 5.5 What the guard allows

The payout guard allows:

```text id="2yfp4j"
independent issuer can preview resolved bound recipient
```

This means a legitimate non-self issuer/reward flow can proceed to later reward planning/wallet stages only after registry resolution.

## 5.6 Evidence payout override rejection

Critical invariant:

```text id="oyz85x"
Service evidence cannot redirect payout.
```

If evidence includes an override recipient, the guard rejects it even if the registry has a valid bound recipient.

Known test:

```text id="tvoe7q"
evidence_payout_override_is_rejected_even_when_registry_resolves
```

## 5.7 Unresolved recipient rejection

Critical invariant:

```text id="qimkzm"
Unbound or unresolved service nodes cannot produce payout authorization preview.
```

Known test:

```text id="9y1teg"
unresolved_recipient_is_rejected_before_any_payout_preview
```

## 5.8 Beneficiary mismatch rejection

Critical invariant:

```text id="dq353n"
The candidate beneficiary must match the registry-resolved reward recipient.
```

Known test:

```text id="qsa9zx"
beneficiary_mismatch_is_rejected
```

## 5.9 Self-pay rejection

Critical invariant:

```text id="36s00x"
A node must not self-pay by default.
```

Known test:

```text id="ds1cse"
self_issued_payout_to_bound_recipient_is_rejected_by_default
```

This matches the user’s project-level anti-self-pay rule.

## 5.10 Feature-gated fixture mode

There is a test-profile / feature-gated self-issuance fixture mode.

Feature:

```text id="95l5pw"
test-self-issuance-fixtures
```

Important behavior:

```text id="ao909g"
Even fixture self-issuance mode still requires the explicit feature gate.
```

Known test:

```text id="vzgcx9"
self_issuance_fixture_mode_still_requires_feature_gate
```

Known commands:

```bash id="236w5a"
cargo test -p svc-registry --test reward_payout_guard
cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
```

Known result:

```text id="so5wvt"
reward_payout_guard: 6 passed default
reward_payout_guard: 6 passed with test-self-issuance-fixtures
```

The terminal output confirms both default and feature-gated payout guard test runs passed 6/6. 

---

## 6. Phase 5C — Registry-Backed Binding / Rotation Request Intake

## 6.1 What changed

Phase 5C moved beyond macronode-local reward display/request state and added registry-backed request intake.

Primary module:

```text id="zv7g52"
crates/svc-registry/src/rewards/request_intake.rs
```

Wired through:

```text id="053kam"
crates/svc-registry/src/rewards/mod.rs
```

Primary test:

```text id="0xjbc3"
crates/svc-registry/tests/reward_binding_request_intake.rs
```

The notes explicitly identify this module, wiring, and test as the Phase 5C implementation surface. 

## 6.2 New types added

Added types:

```text id="z9jwcf"
ResolvedRewardRecipientForBinding
RegistryRewardBindingRequestV1
RegistryRewardBindingRotationRequestV1
RegistryRewardBindingRequestReceipt
RewardBindingRequestError
RegistryRewardBindingRequestIntake
```

These were recorded in the Phase 5C notes. 

## 6.3 Binding request behavior

Registry-backed binding request intake:

```text id="6zzlkc"
accepts binding request
constructs strict ron-proto ServiceNodeRewardBindingV1
inserts through RewardBindingRegistry
resolves bound recipient
returns non-authoritative receipt
rejects evidence_payout_override before registry insert
never mutates wallet
never mutates ledger
never claims confirmed ROC
```

## 6.4 Rotation request behavior

Registry-backed rotation request intake:

```text id="rf0cy2"
accepts rotation request
constructs strict ron-proto ServiceNodeRewardBindingRotationV1
schedules rotation through RewardBindingRegistry
rejects evidence_payout_override before registry schedule
rejects rotation for unbound node through registry rules
returns non-authoritative receipt
never mutates wallet
never mutates ledger
never claims confirmed ROC
```

## 6.5 Non-authoritative receipt

The request intake returns a receipt, but this receipt is not:

```text id="dwylc8"
wallet receipt
ledger receipt
confirmed ROC receipt
payout receipt
quorum finality
```

It is only:

```text id="xrovp6"
registry request/intake acknowledgement
```

## 6.6 Test coverage

Known tests:

```text id="l1cxic"
binding_request_inserts_registry_binding_and_resolves_recipient_without_mutation
binding_request_rejects_evidence_payout_override_before_registry_insert
rotation_request_schedules_pending_rotation_without_mutation_or_finality
rotation_request_rejects_evidence_payout_override_before_registry_schedule
rotation_request_for_unbound_node_fails_through_registry_rules
```

Known green result:

```text id="kmidlv"
reward_binding_request_intake: 5 passed
```

The terminal output confirms all 5 request-intake tests passed. 

---

## 7. Relationship to ron-proto

`svc-registry` consumes strict DTOs from `ron-proto`.

`ron-proto` owns:

```text id="y4mqp1"
ServiceNodeRewardBindingV1
ServiceNodeRewardBindingRotationV1
RewardRecipientResolutionV1
NodeRewardRecipientStatusV1
```

`svc-registry` owns:

```text id="vcnivf"
registry storage/model
binding insertion
recipient resolution
rotation scheduling/application
payout preview guard
request intake
self-pay rejection
evidence payout override rejection
```

Critical split:

```text id="fkfqwi"
ron-proto validates DTO shape.
svc-registry applies registry rules.
svc-wallet and ron-ledger eventually execute and confirm value movement.
```

Do not move registry state mutation into `ron-proto`.

Do not move wallet/ledger mutation into `svc-registry`.

---

## 8. Relationship to macronode / crabnode

`macronode` and `crabnode` provide operator-facing reward bind/show/rotate surfaces.

But macronode state is currently:

```text id="i5qar8"
runtime-local request/display state only
```

`svc-registry` is the stronger registry-backed layer.

Relationship:

```text id="2aixab"
crabnode/macronode = operator request/display UX
svc-registry = authoritative binding/recipient-resolution model
ron-proto = strict DTO wire contract
svc-wallet/ron-ledger = future payout/receipt truth
```

Phase 5C specifically moved beyond macronode-local request display state into registry-backed request intake. 

---

## 9. Relationship to Future Economic Crates

BUILD_PLAN_Z places `svc-registry` among the economic crates but not as the wallet or ledger.

Relevant division:

```text id="mg2nqz"
ron-accounting = snapshots/reports only, no balance mutation
svc-rewarder = capped deterministic planning, references service_node_id not arbitrary payout address
ron-policy = declarative gates only, no ledger mutation
svc-wallet = mutation front-door, pays canonical account resolved from registry/binding
ron-ledger = durable balance/receipt truth
svc-registry = service-node descriptors, reward recipient bindings, eligibility/probation/quarantine state, registry roots
ron-audit = audit trail/proofs
```

This division is spelled out in BUILD_PLAN_Z. 

Important consequence:

```text id="ug9kuy"
svc-registry resolves who should be paid; it does not itself pay.
```

---

## 10. Economic / Security Invariants Locked by svc-registry

The `svc-registry` reward-binding work locks these invariants:

```text id="148m0g"
No service evidence can redirect payout.
No node self-pay shortcut exists.
No arbitrary payout address in service evidence.
Unbound service nodes do not resolve to hidden recipient fields.
Expired bindings do not smuggle recipient account data.
Duplicate binding ids are rejected.
Duplicate rotation nonces are rejected.
Second current binding for same service node is rejected.
Rotation requires matching existing binding.
Rotation applies only after its effective epoch.
Payout preview requires registry-resolved recipient.
Payout preview rejects beneficiary mismatch.
Payout preview rejects self-issued payout by default.
Fixture self-issuance requires explicit feature gate.
Request intake never mutates wallet.
Request intake never mutates ledger.
Request intake never claims confirmed ROC.
```

The closeout script explicitly checked doctrine markers “No service evidence can redirect payout” and “No node self-pay shortcut exists.” 

---

## 11. Scripts Added / Used

## 11.1 Phase 5C script

Created:

```text id="fkmkl0"
scripts/check_phase5c_registry_reward_binding_intake.sh
```

Purpose:

```text id="1r9e9m"
Locks registry-backed binding/rotation intake over strict ron-proto DTOs.
```

Known result:

```text id="4pypkt"
Phase 5C registry reward binding intake preflight passed.
```

## 11.2 Full Phase 5 closeout script

Created:

```text id="dck4dt"
scripts/check_phase5_reward_recipient_binding_closeout.sh
```

Purpose:

```text id="7c40j1"
Chains Phase 5A + Phase 5B + Phase 5C into one green checkpoint.
Runs focused tests/checks.
Runs focused Clippy.
Runs workspace check.
Confirms the full Phase 5 reward-recipient binding surface is green.
```

The notes show this script runs the ron-proto service-node binding test, svc-registry request-intake test, svc-registry binding registry test, svc-registry payout guard test, macronode reward tests, `cargo check -p svc-registry`, Clippy for `svc-registry`, and `cargo check --workspace`. 

---

## 12. Important Test Files and What They Prove

## 12.1 `reward_binding_registry.rs`

Command:

```bash id="xh821t"
cargo test -p svc-registry --test reward_binding_registry
```

Proves:

```text id="lue2ll"
service node can resolve to bound reward recipient
unbound registry state does not smuggle recipient fields
expired binding does not expose recipient account
pending rotation can be scheduled
pending rotation applies only after effective epoch
apply rotation is no-op without pending rotation
duplicate binding id rejected
duplicate rotation nonce rejected
second current binding rejected
missing/mismatched rotation rejected
```

Known result:

```text id="trzcva"
9 passed
```

## 12.2 `reward_payout_guard.rs`

Command:

```bash id="ks6o8b"
cargo test -p svc-registry --test reward_payout_guard
```

Proves:

```text id="4w8sv3"
evidence payout override rejected
unresolved recipient rejected
beneficiary mismatch rejected
self-issued payout rejected by default
fixture mode requires feature gate
independent issuer can preview resolved bound recipient
```

Known result:

```text id="v8u6rp"
6 passed
```

## 12.3 `reward_payout_guard.rs` with fixture feature

Command:

```bash id="2xw0gk"
cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
```

Proves:

```text id="aridxm"
test fixture self-issuance behavior remains explicitly feature-gated
default anti-self-pay posture is not silently disabled
```

Known result:

```text id="t80qkg"
6 passed
```

## 12.4 `reward_binding_request_intake.rs`

Command:

```bash id="3870xu"
cargo test -p svc-registry --test reward_binding_request_intake
```

Proves:

```text id="ge6379"
binding request inserts registry binding and resolves recipient without mutation
binding request rejects evidence payout override before registry insert
rotation request schedules pending rotation without mutation or finality
rotation request rejects evidence payout override before registry schedule
rotation request for unbound node fails through registry rules
```

Known result:

```text id="dwnmai"
5 passed
```

---

## 13. What svc-registry Does NOT Do Yet

`svc-registry` does not yet provide:

```text id="msmnbo"
production durable service-node descriptor lifecycle
full service-node eligibility/probation/quarantine state machine
epoch registry root publication
quorum epoch acceptance
wallet payout execution
ledger mutation
ledger receipt creation
confirmed ROC finality
Passport ownership proof verification
full policy bundle validation
fraud/challenge lifecycle
moderation/pruning registry state
public registry HTTP reward-binding API hardening
full persistence backend path for reward bindings
```

Some of these may have existing registry infrastructure in the crate, but BUILD_PLAN_Z Phase 5 only greened the reward-binding registry/intake/payout guard surface.

---

## 14. Non-Regression Checklist

Future `svc-registry` patches must preserve:

```text id="ldfy7o"
service_node_id is the reward evidence binding key
reward_recipient_account_id comes from registry resolution
service evidence cannot carry payout override
evidence_payout_override is rejected before insert/schedule/payout preview
unbound resolution cannot smuggle recipient fields
expired binding cannot expose recipient account as payable
duplicate binding_id rejected
duplicate rotation_nonce rejected
second current binding rejected
rotation requires existing matching binding
rotation applies only after effective epoch
payout preview rejects unresolved recipient
payout preview rejects beneficiary mismatch
payout preview rejects self-issued payout by default
fixture self-issuance requires explicit feature
request intake returns non-authoritative receipt only
request intake never mutates wallet
request intake never mutates ledger
request intake never claims confirmed ROC
```

Future `svc-registry` patches must not add:

```text id="lzsuhz"
wallet_mutation = true
ledger_mutation = true
confirmed_roc
direct payout execution
arbitrary payout address in evidence
self-pay bypass
unilateral local mint
fake ledger receipt
fake balance
fake finality
```

---

## 15. Important Files to Watch

Primary files:

```text id="y6ha6w"
crates/svc-registry/src/rewards/mod.rs
crates/svc-registry/src/rewards/issuance_guard.rs
crates/svc-registry/src/rewards/request_intake.rs
crates/svc-registry/tests/reward_binding_registry.rs
crates/svc-registry/tests/reward_payout_guard.rs
crates/svc-registry/tests/reward_binding_request_intake.rs
```

Adjacent registry files to watch when Phase 8+ expands:

```text id="sfp82y"
crates/svc-registry/src/http/routes.rs
crates/svc-registry/src/storage/
crates/svc-registry/src/pipeline/
crates/svc-registry/src/governance/
crates/svc-registry/src/readiness/gate.rs
```

Do not touch broader registry/governance/storage paths for reward binding unless the next phase needs durable API/storage integration.

---

## 16. Recommended svc-registry Baseline Commands

Before touching `svc-registry` again:

```bash id="twmfqd"
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-registry --test reward_binding_registry
cargo test -p svc-registry --test reward_payout_guard
cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
cargo test -p svc-registry --test reward_binding_request_intake
cargo check -p svc-registry
cargo clippy -p svc-registry --all-targets --no-deps -- -D warnings
```

If changing any Phase 5 reward-binding surface, run:

```bash id="yemcbb"
bash scripts/check_phase5_reward_recipient_binding_closeout.sh
```

Then:

```bash id="8jzvzz"
cargo check --workspace
```

---

## 17. How svc-registry Fits Future Phases

For the next official phase:

```text id="yc06rq"
Phase 8 — Service Node Foundation
```

`svc-registry` probably should not be the first crate touched unless macronode needs registry-backed status.

Likely Phase 8 role for `svc-registry`:

```text id="bqi75k"
provide service-node reward binding status to macronode/svc-admin
eventually expose service-node descriptor readiness
eventually expose eligibility/probation/quarantine status
remain the source for bound reward recipient account resolution
avoid wallet/ledger mutation
avoid confirmed ROC claims
```

Recommended Phase 8 sequencing:

```text id="7i7gio"
Start in macronode.
Expose service-node foundation/readiness truth there.
Only touch svc-registry if macronode needs real registry-backed reward binding or service-node descriptor status.
Preserve all Phase 5 reward-binding tests.
```

Potential later `svc-registry` work:

```text id="z3m9il"
durable reward binding storage
HTTP API for registry-backed binding request intake
service-node descriptor registry
operator ownership proof verification integration
eligibility/probation/quarantine state
registry root generation for epochs
registry event/audit feed
macronode/svc-admin reward binding status integration
```

---

## 18. Good svc-registry Phase Marker to Carry Forward

Use this marker:

```text id="q9esla"
svc-registry — BUILD_PLAN_Z status

Completed:
- reward binding registry model
- service_node_id -> reward_recipient_account_id resolution
- unbound state without recipient smuggling
- expired binding state without recipient account exposure
- duplicate binding id rejection
- duplicate rotation nonce rejection
- second current binding rejection
- pending rotation scheduling
- pending rotation application only after effective epoch
- rotation mismatch/missing binding rejection
- payout authorization preview guard
- evidence payout override rejection
- unresolved recipient rejection
- beneficiary mismatch rejection
- self-issued payout rejection by default
- feature-gated test self-issuance fixture mode
- registry-backed binding request intake
- registry-backed rotation request intake
- non-authoritative request receipts
- no wallet mutation
- no ledger mutation
- no confirmed ROC
- no fake payout finality

Known green:
cargo test -p svc-registry --test reward_binding_registry
cargo test -p svc-registry --test reward_payout_guard
cargo test -p svc-registry --features test-self-issuance-fixtures --test reward_payout_guard
cargo test -p svc-registry --test reward_binding_request_intake
cargo check -p svc-registry
cargo clippy -p svc-registry --all-targets --no-deps -- -D warnings
bash scripts/check_phase5_reward_recipient_binding_closeout.sh
cargo check --workspace
```

---

## 19. Short Summary

```text id="7chfvd"
Since starting BUILD_PLAN_Z, svc-registry has become the registry-backed authority for service-node reward recipient binding and payout-resolution safety. It now owns a binding registry model, recipient resolution, pending rotation scheduling/application, duplicate/replay rejection, payout authorization preview, evidence payout override rejection, default self-pay rejection, and registry-backed binding/rotation request intake over strict ron-proto DTOs. It returns only non-authoritative intake/preview results and deliberately does not mutate wallet, mutate ledger, claim confirmed ROC, or provide fake payout finality.
```


### END NOTE - JULY 10 2026 - 12:55 CST



### BEGIN NOTE - JULY 14 2026 - 01:00 CST


Next is the `svc-registry` changelog. These notes cover the Phase 19 containment custody, replay protection, canonical lifecycle application, appeal visibility, and cross-crate enforcement work completed this session. The final full-crate and workspace closeout passed successfully. 

# `svc-registry` Changelog Notes — Phase 19 Enforcement Custody and Containment

## Summary

This session extended `svc-registry` with the canonical registry-owned enforcement surface required for BUILD_PLAN_Z Phase 19, **Bad Node Detection and Quarantine**.

The crate now accepts validated Service Node violation reports, evaluates them through the canonical `ron-policy` enforcement policy, applies the resulting containment state to the existing Phase 18 Service Node lifecycle descriptor, stores visible enforcement status, rejects replayed evidence, and records pending appeals without restoring eligibility.

The implementation preserves the existing ownership model:

* `ron-proto` owns shared violation and enforcement DTOs;
* `ron-policy` owns deterministic violation review;
* `svc-registry` owns canonical lifecycle custody and transition application;
* `svc-rewarder` owns reward-plan filtering and denial records.

No duplicate registry, quarantine model, trusted-node list, or shadow lifecycle state machine was created.

---

# Files Added

## `crates/svc-registry/src/enforcement.rs`

Added the Phase 19 enforcement registry.

This module owns process-local custody for:

* accepted violation reports;
* consumed report IDs;
* consumed evidence roots;
* last accepted violation epoch per Service Node;
* current visible enforcement status;
* submitted appeal IDs.

It also coordinates the real Phase 19 flow:

1. validate violation evidence;
2. reject replay;
3. load the canonical Service Node descriptor;
4. call `ron-policy`;
5. verify the policy decision matches the report and descriptor;
6. atomically apply the reviewed containment state;
7. store evidence and enforcement status;
8. expose a non-authoritative result to callers.

## `crates/svc-registry/tests/internal_roc_beta_phase19_enforcement_registry.rs`

Added the primary Phase 19 registry and cross-crate test suite.

The suite initially covered registry custody and appeal behavior, then was extended with complete policy-to-registry-to-rewarder integration tests.

---

# Files Updated

## `crates/svc-registry/src/eligibility.rs`

Added a narrow crate-private containment application method:

* `apply_reviewed_containment`

This method allows the enforcement module to apply a reviewed containment decision without:

* replacing the complete descriptor;
* mutating private registry maps directly;
* inventing a second lifecycle implementation;
* bypassing the existing descriptor validation rules.

The method accepts only:

* `Degraded`
* `Quarantined`
* `Blocked`

It rejects:

* `Candidate`
* `Probation`
* `Eligible`

It also verifies:

* the registry’s current state matches the state reviewed by policy;
* no invalid state-effective epoch is supplied;
* no stale containment transition is applied;
* the updated descriptor still passes canonical validation.

## `crates/svc-registry/src/lib.rs`

Registered the new enforcement module through the crate’s public module surface.

This made the registry enforcement types available to:

* tests;
* future service intake;
* administrative inspection;
* later durable persistence work;
* reward review consumers.

---

# Canonical Registry Enforcement Model

Added `ServiceNodeEnforcementRegistry`.

This registry is separate from the reward-binding registry and eligibility registry in responsibility, but it does not replace either one.

It maintains focused Phase 19 custody for:

* violation evidence;
* replay markers;
* current enforcement status;
* appeal submissions.

The existing `ServiceNodeEligibilityRegistry` remains the canonical source of lifecycle truth.

The enforcement registry cannot independently claim that a node is:

* eligible;
* part of quorum;
* reward eligible;
* restored after an appeal.

Every containment result must be applied to the existing canonical lifecycle descriptor.

---

# Violation Intake and Application

Added:

* `evaluate_and_apply_violation`

This method performs the complete reviewed registry transition for one violation report.

## Processing sequence

The method:

1. validates the violation report;
2. rejects a previously used report ID;
3. rejects a previously consumed evidence root;
4. rejects a report epoch that does not advance the node’s accepted violation history;
5. loads the Service Node’s canonical eligibility descriptor;
6. evaluates the descriptor and report through `ServiceNodeViolationPolicyV1`;
7. validates that the policy decision exactly matches its inputs;
8. constructs the next visible enforcement status;
9. applies the reviewed state to the canonical descriptor;
10. records the report ID;
11. records the evidence root;
12. records the latest violation epoch;
13. stores the current enforcement status;
14. returns the complete transition result.

All fallible validation and policy review occur before replay markers or enforcement records are consumed.

This ensures rejected reports do not poison future corrected submissions.

---

# Atomic Canonical Containment

The enforcement flow applies containment through the existing Phase 18 descriptor.

The resulting canonical state becomes one of:

* `Degraded`
* `Quarantined`
* `Blocked`

The transition preserves:

* Service Node identity;
* registry entry identity;
* reward-binding reference;
* node public key;
* registration epoch;
* service-history root;
* challenge-history root.

Only the reviewed lifecycle state and its valid effective epoch change.

This avoids replacing the descriptor with an incomplete enforcement-specific object.

---

# Reviewed Policy Decision Verification

Before applying a policy decision, `svc-registry` independently checks that it matches the exact descriptor and report supplied to policy.

The registry verifies:

* report ID;
* Service Node ID;
* violation kind;
* evidence root;
* previous lifecycle state;
* evaluation epoch;
* reward-review posture;
* quorum posture;
* selected containment state.

The decision must:

* deny rewards;
* exclude the node from quorum;
* select `Degraded`, `Quarantined`, or `Blocked`.

If any reviewed field is inconsistent, the registry rejects the decision as:

* `InvalidPolicyDecision`

This prevents an accidental or compromised caller from attaching a valid-looking policy result to different evidence.

---

# Report-ID Replay Protection

The registry records every accepted violation report ID.

A later submission with the same report ID is rejected as:

* `ReportReplay`

This prevents the same named report from being applied twice.

The replay marker is stored only after successful policy review and descriptor application.

A malformed or mismatched report does not consume its report ID.

---

# Evidence-Root Replay Protection

The registry separately records every accepted evidence root.

A new report with a different report ID but the same evidence root is rejected as:

* `EvidenceReplay`

The error identifies:

* the replayed evidence root;
* the original report that consumed it.

This prevents trivial replay evasion by renaming a report while reusing the same objective evidence.

---

# Per-Node Violation Epoch Protection

The registry tracks the latest accepted violation epoch for each Service Node.

A later report must have a strictly greater `detected_epoch`.

The registry rejects:

* identical epochs;
* older epochs;
* rollback attempts.

The error records:

* Service Node ID;
* proposed epoch;
* last accepted epoch.

This adds another replay and ordering boundary beyond report IDs and evidence roots.

---

# Failure Atomicity

Tests prove that rejected intake does not partially mutate registry state.

When a report fails because of:

* missing descriptor;
* mismatched Service Node identity;
* repeated report ID;
* repeated evidence root;
* repeated node epoch;
* invalid policy result;

the system does not partially consume unrelated state.

The following remain unchanged after rejection:

* canonical descriptor;
* current enforcement status;
* accepted report count;
* replay maps, except for previously accepted evidence.

This allows a corrected report to be submitted after an invalid attempt without being falsely classified as replayed.

---

# Stronger Containment Preservation

The registry consumes the policy rule that containment cannot be downgraded.

A later weaker violation can still be recorded for evidence history, but it cannot weaken the canonical node state.

For example:

1. a denylist violation blocks the node;
2. a later provider-spam report is accepted as additional evidence;
3. the node remains blocked;
4. the original stronger enforcement reason remains visible;
5. the original enforcement-status identity remains unchanged.

The later report is retained without hiding or replacing the reason for stronger containment.

---

# Enforcement Status Custody

The registry stores `ServiceNodeEnforcementStatusV1` by Service Node ID.

The status records:

* status ID;
* Service Node ID;
* containment state;
* violation reason;
* evidence root;
* effective epoch;
* appeal posture.

The visible status is updated when:

* a new state transition occurs;
* the node receives its first enforcement status;
* an appeal is submitted.

A later report that does not change or strengthen state does not automatically overwrite a stronger existing enforcement reason.

---

# Enforcement Transition Result

Added `ServiceNodeEnforcementTransitionV1`.

The transition result contains:

* accepted violation report;
* deterministic policy decision;
* canonical descriptor after application;
* current enforcement status;
* whether lifecycle state changed;
* whether visible enforcement status changed.

It also exposes conservative helpers confirming that:

* the resulting node does not count toward quorum;
* rewards are denied;
* no economic mutation is authorized;
* no appeal resolution is authorized.

The transition is a registry result, not a wallet, ledger, payout, receipt, or finality artifact.

---

# Pending Appeal Submission

Added:

* `submit_appeal`

This records a visible pending appeal for a currently contained Service Node.

A valid submission requires:

* an existing enforcement status;
* a new appeal ID;
* no existing appeal posture;
* a submission epoch at or after the containment epoch.

The resulting appeal status is:

* `Pending`

It stores:

* appeal ID;
* submission epoch.

It does not store a resolution epoch or resolution evidence until a separate future reviewed resolution flow exists.

---

# Appeal Replay Protection

The registry tracks appeal IDs independently.

A reused appeal ID is rejected as:

* `AppealIdReplay`

The error identifies:

* the replayed appeal ID;
* the Service Node that originally consumed it.

This prevents the same appeal identity from being reused:

* for the same node;
* for another node;
* after an earlier successful submission.

---

# Duplicate Appeal Protection

A Service Node with an existing appeal posture cannot submit another appeal through this intake path.

The registry rejects another submission as:

* `AppealAlreadyExists`

This applies when the current appeal state is:

* `Pending`
* `Accepted`
* `Rejected`

The existing appeal must be handled through an explicit future review or lifecycle process rather than overwritten.

---

# Appeal Epoch Validation

An appeal cannot predate the containment it challenges.

If the appeal submission epoch is earlier than the enforcement effective epoch, the registry rejects it as:

* `AppealPredatesContainment`

This prevents an appeal record from claiming to challenge a decision before that decision existed.

---

# Appeals Do Not Restore Eligibility

Submitting an appeal changes only the visible appeal posture.

It does not modify the canonical Service Node descriptor.

A quarantined node with a pending appeal remains:

* `Quarantined`;
* excluded from quorum;
* denied reward planning.

A blocked node with a pending appeal remains:

* `Blocked`;
* excluded from quorum;
* denied rewards.

The registry does not treat appeal submission or appeal acceptance as automatic restoration.

A later explicit recovery transition remains required.

---

# Quorum Exclusion

Because the registry applies containment to the canonical Phase 18 descriptor, existing quorum logic automatically observes the new state.

Tests confirm that a contained node:

* does not project as eligible;
* does not count toward quorum;
* cannot contribute quorum weight;
* cannot re-enter quorum through an appeal-status field.

No parallel quorum denylist was required.

The existing lifecycle state remains the source of truth.

---

# Reward-Path Integration

The Phase 19 registry tests were extended to call the real `svc-rewarder` enforcement-aware reward review.

This proves that the output of:

1. `ron-proto`;
2. `ron-policy`;
3. `svc-registry`;

flows directly into reward denial.

The cross-crate test does not manually fabricate a quarantined descriptor after policy review.

It uses the actual descriptor and enforcement status returned by registry containment.

---

# Hash-Mismatch Cross-Crate Flow

The cross-crate tests prove:

1. an eligible Service Node receives a `HashMismatch` report;
2. `ron-policy` recommends `Quarantined`;
3. `svc-registry` atomically applies `Quarantined`;
4. the descriptor no longer counts toward quorum;
5. the descriptor and status are supplied to `svc-rewarder`;
6. the node receives a denial record;
7. no reward plan is created for that node;
8. no economic mutation authority is granted.

---

# Invalid Epoch Signature Cross-Crate Flow

The same end-to-end path is tested for:

* `InvalidEpochSignature`

The node is:

* quarantined;
* excluded from quorum;
* excluded from reward planning.

This proves invalid epoch signers cannot remain economically active after containment.

---

# Denylist Violation Cross-Crate Flow

The cross-crate suite proves:

1. a denylist violation is submitted;
2. policy recommends immediate `Blocked`;
3. registry applies `Blocked`;
4. the node is excluded from quorum;
5. rewarder creates a denial-only review;
6. no reward allocation is produced.

This directly satisfies the Phase 19 requirement that a denylist violator is blocked and earns no rewards.

---

# Privacy Leak and Appeal Cross-Crate Flow

The cross-crate suite also proves:

1. a privacy leak report quarantines the node;
2. the operator submits a pending appeal;
3. the registry exposes the appeal visibly;
4. the canonical descriptor remains quarantined;
5. the pending appeal is passed into reward review;
6. the node remains reward-denied;
7. no reward plan is created;
8. no economic mutation authority exists.

This proves appeal visibility and reward denial operate together correctly.

---

# Reward-Recipient Binding Abuse Support

Although the direct denial calculation is implemented in `svc-rewarder`, registry custody now supports the canonical enforcement status required to carry:

* `RewardRecipientBindingAbuse`

through the containment and reward-review path.

The registry does not:

* rewrite the payout recipient;
* redirect rewards;
* approve an alternate account;
* mutate a reward binding.

The safe response is containment plus downstream reward denial.

---

# Error Model

Added `ServiceNodeEnforcementRegistryError`.

The error taxonomy includes:

* invalid enforcement artifact;
* missing canonical descriptor;
* report replay;
* evidence replay;
* violation epoch replay;
* inconsistent policy decision;
* policy evaluation failure;
* eligibility transition failure;
* missing enforcement status;
* appeal ID replay;
* existing appeal;
* appeal predating containment.

This gives callers deterministic failure categories for:

* CLI display;
* future HTTP intake;
* audit logs;
* operator consoles;
* tests.

---

# Eligibility Transition Errors Added

The Phase 18 eligibility transition error model was extended with enforcement-specific failures:

## `ReviewedContainmentStateMismatch`

Returned when registry custody has moved to a different state than the state reviewed by policy.

This prevents stale decisions from being applied after another transition.

## `InvalidReviewedContainmentState`

Returned when enforcement attempts to apply a state outside:

* `Degraded`
* `Quarantined`
* `Blocked`

This prevents the enforcement module from promoting nodes to:

* `Probation`
* `Eligible`

## Existing state-effective epoch validation

Containment application also reuses the existing invalid decision epoch protection.

---

# Tests Added and Extended

The final Phase 19 registry suite contains ten focused tests.

## `bad_hash_report_atomically_quarantines_canonical_descriptor`

Proves that a hash mismatch:

* produces quarantine;
* updates the canonical descriptor;
* creates visible enforcement status;
* excludes the node from quorum;
* denies rewards;
* records the report exactly once;
* grants no economic or appeal-resolution authority.

## `denylist_violation_blocks_node_immediately`

Proves that a denylist violation moves an eligible node directly to `Blocked`.

## `report_id_evidence_and_node_epoch_replays_are_rejected`

Proves independent replay rejection for:

* report ID;
* evidence root;
* node violation epoch.

It also proves rejected replay attempts do not change descriptor or status truth.

## `rejected_policy_input_does_not_consume_report_identity`

Proves that malformed or mismatched intake does not consume a report ID or evidence root.

A corrected report can subsequently succeed.

## `later_weaker_report_cannot_downgrade_or_hide_block_reason`

Proves that:

* a blocked node remains blocked;
* a weaker report can be retained as evidence;
* the stronger original enforcement reason remains visible;
* the original status ID remains stable.

## `pending_appeal_is_visible_but_does_not_restore_eligibility`

Proves that:

* a pending appeal is stored and visible;
* the canonical descriptor remains quarantined;
* quorum remains denied;
* reward planning remains denied;
* replayed or duplicate appeals reject.

## `appeal_requires_existing_containment_and_valid_epoch`

Proves that:

* appeals without an enforcement status reject;
* appeals predating containment reject.

## `phase19_cross_crate_containment_denies_hash_and_epoch_signer_rewards`

Proves the complete policy-registry-rewarder flow for:

* hash mismatch;
* invalid epoch signature.

Both violations produce quarantine, quorum exclusion, and denial-only reward reviews.

## `phase19_cross_crate_denylist_block_produces_no_reward_plan`

Proves that a registry-blocked denylist violator produces no reward plan.

## `phase19_cross_crate_pending_appeal_stays_visible_and_reward_denied`

Proves that a pending privacy-leak appeal remains visible through reward review without restoring reward eligibility.

---

# Phase 18 Regression Protection

The new enforcement registry was tested alongside the existing Phase 18 registry and quorum suites.

Regression coverage confirmed that Phase 19 did not break:

* candidate registration;
* exact reward-binding verification;
* objective history-root updates;
* candidate-to-probation transitions;
* probation-to-eligible transitions;
* challenge-based quarantine;
* replay-safe lifecycle evaluations;
* weighted quorum behavior;
* fresh-node minority caps;
* candidate quorum exclusion;
* quarantined-signer rejection;
* founder/manual-trust field rejection.

The Phase 18 cross-crate Sybil tests also remained green.

---

# Verification Results

The focused Phase 19 registry suite passed:

* 10 passed
* 0 failed

The focused Phase 18 registry regressions passed:

* eligibility transition: 7/7;
* quorum weighting: 8/8;
* cross-crate Sybil resistance: 4/4.

The crate also passed:

* `cargo check -p svc-registry`;
* strict Clippy with `--all-targets --no-deps -- -D warnings`;
* `cargo test -p svc-registry --all-targets`;
* the final workspace compile gate.

The final full-crate run also confirmed all existing `svc-registry` test families remained green, including:

* reward-binding registry;
* reward-binding request intake;
* reward payout guard;
* API commit;
* HTTP contract;
* concurrency;
* storage monotonicity;
* metrics;
* invariant tests;
* existing Phase 18 lifecycle tests.

The final Phase 19 closeout completed with:

`PHASE19_FINAL_CLOSEOUT_STATUS=0`

---

# Resulting `svc-registry` Responsibilities

After this session, `svc-registry` now owns:

* canonical custody of accepted violation reports;
* report-ID replay protection;
* evidence-root replay protection;
* per-node violation epoch monotonicity;
* verified consumption of `ron-policy` decisions;
* atomic application of containment to Phase 18 descriptors;
* current enforcement-status custody;
* preservation of stronger containment;
* pending appeal submission;
* appeal-ID replay protection;
* appeal visibility without restoration;
* canonical quorum exclusion through lifecycle state;
* cross-crate handoff into reward denial.

---

# Authority Boundary Preserved

The registry enforcement implementation does not:

* independently invent violation severity;
* inspect live peer traffic;
* expose peer IP addresses;
* approve or redirect payouts;
* compute reward amounts;
* create reward allocations;
* issue ROC;
* mutate a wallet;
* mutate the ledger;
* create payout receipts;
* mint or burn ROC;
* mint or burn ROX;
* submit Solana transactions;
* resolve bridge settlement;
* claim finality.

It applies reviewed containment to registry-owned lifecycle truth and exposes the result to the next correct subsystem.

---

# Final Outcome

`svc-registry` now provides the canonical enforcement custody layer connecting:

1. objective violation evidence from `ron-proto`;
2. deterministic containment review from `ron-policy`;
3. the existing Phase 18 Service Node lifecycle;
4. downstream reward denial in `svc-rewarder`.

Bad nodes can now be quarantined or blocked exactly once, excluded from quorum through canonical state, denied rewards, and shown a visible appeal posture without weakening the project’s authority, replay, privacy, or decentralization boundaries.

The final crate changed in this session is `svc-rewarder`.



### END NOTE - JULY 14 2026 - 01:00 CST


### BEGIN NOTE - JULY 15 2026 - 18:30 CST


## svc-registry — Session Changelog

* Added Phase 23 mid-epoch reward-binding rotation attack coverage.
* Rejected replacement rotations without overwriting the accepted pending rotation.
* Preserved the current recipient until the approved future epoch.
* Prevented rejected rotations from consuming replay nonces or redirecting payouts.
* Confirmed quarantine, eligibility, quorum weighting, and Sybil-resistance rules.
* Completed focused tests, strict Clippy, and acceptance checks.



### END NOTE - JULY 15 2026 - 18:30 CST


### BEGIN NOTE - JULY 15 2026 - 18:40 CST

These notes consolidate the Phase 22G implementation record, the later CrabLink and Phase 23 hardening work, the complete Phase 24 acceptance output, and the sanctions-compliance requirement recorded in `BUILD_PLAN_Z`. The final gate executed 211 focused tests, strict Clippy, workspace checks, and CrabLink Tauri checks before producing `PHASE24_FINAL_STATUS=GREEN_PARKED`.   

The sanctioned-jurisdiction IP blocking requirement was recorded as a **mandatory future compliance gate**, not completed implementation. The plan defines the intended TOML files and enforcement boundaries but explicitly schedules implementation before public production access, production node admission, or production ROC participation.  

# RustyOnions / CrabLink BUILD_PLAN_Z — Final Session Wrap-Up and Carryover Notes

**Date:** July 15, 2026
**Repositories:** RustyOnions and CrabLink
**Primary plan:** `BUILD_PLAN_Z`
**Session coverage:** Closing Phase 22 work, Phase 23 hardening and chaos validation, Phase 24 private-beta readiness, and final BUILD_PLAN_Z parking
**Final status:** `COMPLETE / GREEN / PARKED`

---

# 1. Executive summary

This session completed the final implementation, hardening, client integration, documentation, and acceptance work for `BUILD_PLAN_Z`.

The session began with the real Phase 22 local reward loop already substantially implemented and focused on:

1. Completing and verifying the Phase 22G end-to-end ROC reward loop.
2. Exporting and validating receipt-only confirmed ROC in CrabLink.
3. Completing the real CrabLink-to-User-Node object-verification path.
4. Completing Service Node Operator Mode and local Service Node attachment.
5. Adding Phase 23 fail-closed, privacy, replay, cache, rotation, and dependency-outage tests.
6. Aligning private-beta runbooks with the actual two-node product.
7. Creating a reproducible final Phase 24 acceptance runner.
8. Running focused tests, strict Clippy, full workspace checks, and CrabLink Tauri checks.
9. Updating the readiness records to declare Phase 24 and `BUILD_PLAN_Z` complete and parked.

The final acceptance output was:

```text
Phase 24 private-beta readiness acceptance passed.
Phase 24 is complete, green, and parked.
PHASE24_FINAL_STATUS=GREEN_PARKED
```

`BUILD_PLAN_Z` is now complete through Phase 24.

This is a private-beta and local-development readiness result. It is not a claim of unrestricted public production readiness, production Solana bridge readiness, staking readiness, exchange readiness, or production mint/burn readiness.

---

# 2. Final BUILD_PLAN_Z status

The closing phases are now:

```text
Phase 21 — CrabLink Service Node Operator Mode:
GREEN / PARKED

Phase 22 — Local Two-Node Reward Loop:
GREEN / PARKED

Phase 23 — Hardening, Chaos, and Fail-Closed Validation:
GREEN / PARKED

Phase 24 — Private Beta Readiness:
COMPLETE / GREEN / PARKED
```

Final project marker:

```text
PHASE24_FINAL_STATUS=GREEN_PARKED
```

The Phase 24 checklist, readiness status, park record, and authoritative private-beta runbook were updated to consistently contain:

```text
COMPLETE / GREEN / PARKED
```

The final acceptance script now verifies the parked status before printing the final marker.

---

# 3. Final product architecture preserved

The final product continues to expose only two public node products:

```text
CrabLink User Node
CrabLink Service Node
```

There is no separate public “Economic Node” product.

Economic services remain internal authority components and are not presented to ordinary users or Service Node operators as another node product.

## 3.1 CrabLink User Node

The User Node remains:

```text
private
passive
outbound-first
loopback-only
CrabLink-managed
amnesia-first
resource-bounded
non-serving by default
not a public provider
not a user-administered server
```

Normal users do not manage:

```text
public ports
provider routes
server dashboards
RPC endpoints
reward issuance
wallet mutation
ledger mutation
quorum membership
```

The User Node verifies content and economic replay but does not create economic authority.

## 3.2 CrabLink Service Node

The Service Node remains:

```text
headless-first
CLI-manageable
public content-serving where configured
b3/OAP based
moderation-aware
persistence-policy-aware
evidence-producing
reward-recipient bound
registry eligible
quorum participating when eligible
unable to mint alone
```

A Service Node may be operated entirely through `crabnode`.

The optional admin UI and CrabLink Operator Mode are convenience surfaces and are not required for the node daemon to continue operating.

## 3.3 Optional administrator UI

`svc-admin` remains:

```text
optional
disabled by default
loopback-only by default
authenticated after bootstrap
not required for headless operation
not required for CLI operation
not wallet authority
not ledger authority
not registry authority
not finality authority
```

There is no centralized password reset.

If local administrator credentials are lost, the operator recreates the local administrator state and bootstraps a new local administrator.

The separately registered reward-recipient binding and durable ROC are not lost merely because local admin credentials are recreated.

## 3.4 CrabLink client

CrabLink remains:

```text
user experience
user intent
User Node lifecycle manager
optional Service Node controller
native privilege boundary
display of backend-derived truth
```

CrabLink must not become:

```text
wallet authority
ledger authority
reward-plan authority
registry authority
quorum authority
moderation-policy authority
persistence truth
provider truth
receipt issuer
finality authority
Service Node daemon container
```

---

# 4. Actual code-bearing crates and components changed during this session

Actual Rust or client implementation/test code was changed in seven code-bearing components:

```text
micronode
svc-rewarder
macronode
svc-dht
svc-storage
svc-registry
crablink-tauri
```

Other crates were exercised by tests, Clippy, and workspace compilation but did not receive direct Rust source changes during this session.

---

# 5. `micronode` changes

## 5.1 Real bounded object-verification queue

Added the real User Node object-verification queue.

The queue:

* accepts bounded verification work;
* records pending verification outcomes;
* enforces replay and idempotency rules;
* enforces finite capacity;
* exposes truthful queue state through User Node status;
* rejects work when verification is disabled or the node is paused;
* does not create economic authority.

## 5.2 Pending evidence behavior

Valid object verification may create pending evidence.

Corrupt or invalid input may create challenge evidence where appropriate.

Pending evidence remains non-authoritative.

It does not automatically create:

```text
accounting snapshot
reward plan
wallet request
wallet balance
ledger operation
ledger receipt
confirmed ROC
finality
```

## 5.3 Replay and queue-capacity rejection

Added and verified:

* verification nonce replay rejection;
* idempotency replay rejection;
* queue-capacity rejection;
* deterministic behavior for duplicate work;
* no silent eviction of pending challenge material;
* no fake success on rejected work.

## 5.4 Pause and resume semantics

The paused User Node:

* rejects new verification writes;
* preserves read-only pending evidence;
* does not consume replay identity for work rejected because the node is paused;
* resumes safely;
* does not invent accounting, rewards, wallet state, ledger state, receipts, or confirmed ROC.

The Phase 23 test prints a conclusive marker proving this behavior.

## 5.5 Passive runtime and privacy

Verified:

* loopback-only administration;
* no public inbound configuration;
* bounded resource budgets;
* private passive runtime status;
* no public serving role;
* no residential-IP publication;
* truthful pending-evidence status without reward claims.

## 5.6 Epoch replay and challenge behavior

The User Node epoch-replay suite verifies:

* valid structural replay acceptance;
* duplicate payout detection;
* cap mismatch detection;
* replay-total mismatch detection;
* supply-conservation mismatch detection;
* economics-configuration hash mismatch detection;
* invalid or noneligible quorum signer detection;
* real Ed25519 quorum signature verification;
* deterministic challenge evidence;
* stable primary finding precedence;
* duplicate challenge suppression;
* queued retry after failed challenge transport;
* no fake challenge acceptance.

---

# 6. `svc-rewarder` changes

## 6.1 Complete Phase 22G reward-loop integration

Added the complete local Phase 22G reward-loop integration test.

The test covers:

```text
canonical Service Node evidence
canonical User Node evidence
accounting classification
accounting epoch snapshot
deterministic accounting-to-rewarder handoff
deterministic capped reward plan
registry-derived reward recipient
self-issuance rejection
eligible quorum review
real Ed25519 signatures
two-of-three quorum
single-node mint rejection
svc-wallet execution
ron-ledger durable receipt
exact retry without double issuance
ledger replay
supply conservation
User Node replay acceptance
deterministic tamper challenge
```

The test proves the full local reward loop without introducing a fake wallet, fake ledger, fake receipt, or fake finality layer.

## 6.2 Focused Phase 22 reward-loop runner

Added:

```text
scripts/check-phase22-local-reward-loop.sh
```

The runner covers:

* complete Phase 22G integration;
* accounting snapshot regressions;
* accounting handoff regressions;
* Service Node reward-plan regressions;
* reward-binding registry regressions;
* payout-guard regressions;
* wallet quorum execution;
* User Node epoch replay;
* workspace compilation.

The Phase 22G runner completed green.

## 6.3 Receipt-only confirmed-ROC projection export

Extended the real Phase 22G test so it can optionally export a confirmed-ROC projection when:

```text
PHASE22_CONFIRMED_ROC_PROJECTION_PATH
```

is supplied.

The projection is derived from the actual completed reward-loop execution.

It contains:

* schema and version;
* epoch ID;
* account ID;
* confirmed ROC minor units;
* receipt-only source label;
* receipt count;
* operation IDs;
* last ledger sequence;
* last ledger root;
* transition hash;
* economics-configuration hash;
* wallet receipt confirmation;
* ledger replay confirmation;
* User Node replay acceptance;
* pending-evidence posture;
* display-only posture;
* explicit client non-authority flags.

Required source label:

```text
wallet_ledger_receipt_only
```

## 6.4 Dependency-outage behavior

Added Phase 23 degraded-readiness coverage proving:

* liveness remains truthful;
* readiness identifies unavailable gates;
* existing manifests remain inspectable;
* existing settlement previews remain read-only;
* new computation is rejected;
* wallet emission is rejected before egress;
* no new manifest is created;
* no receipt, balance, payout, confirmed ROC, or finality is invented.

## 6.5 Determinism and replay

Verified:

* same snapshot, policy, and epoch produce the same plan;
* reordered snapshot rows do not change the plan;
* duplicate epoch replay is deduplication, not second payout authority;
* idempotency keys are retry identities, not operation identities;
* exact retry does not double issue;
* planning output does not claim ledger or wallet truth.

## 6.6 Clippy fixes

Fixed strict-Clippy findings by:

* replacing cloned single-element slices with `std::slice::from_ref`;
* removing unnecessary evidence clones;
* preserving later ownership required for evidence-root construction.

Strict Clippy passed with:

```text
--all-targets --no-deps -D warnings
```

---

# 7. `macronode` changes

## 7.1 Expired setup-token hardening

Added Phase 23 coverage proving:

* an expired setup token is rejected;
* the expired token is cleared;
* the optional administrator UI remains disabled;
* the headless Service Node remains healthy;
* no economic or finality authority is created by the failed setup attempt.

## 7.2 Setup-token TTL test configuration

Updated the affected setup-token TTL test configuration and validation behavior so the expiration path could be tested deterministically.

Strict-Clippy findings caused by the test/configuration changes were fixed.

## 7.3 Operator and CLI surfaces verified

The final acceptance gate verified:

```text
crabnode_cli
crabnode_policy_cli
crabnode_prune_cli
crabnode_rewards_cli
crabnode_setup_cli
crabnode_persistence_cli
operator_admin
```

Verified behavior includes:

* status through loopback admin;
* explicit non-mutating dry run;
* short-lived setup-token handling;
* local admin user creation;
* policy block/unblock;
* quarantine/release;
* global-deny precedence;
* exact local prune behavior;
* no network-wide deletion claims;
* reward-recipient binding;
* future-epoch reward-recipient rotation;
* persistence review and pin/unpin paths;
* optional admin UI lifecycle;
* headless daemon independence.

## 7.4 Phase 22 local attachment support

The Service Node was used by the local Phase 22 topology and CrabLink attachment tests.

The attachment path verified:

* Service Node connection;
* unauthorized access remains unauthorized;
* correct authentication succeeds;
* canonical operator status is returned;
* reward binding is backend-derived;
* refresh remains attached to the configured node;
* no client process-control or economic authority is created.

---

# 8. `svc-dht` changes

## 8.1 Phase 23 provider-discovery chaos test

Added:

```text
phase23_provider_discovery_chaos.rs
```

The test proves:

* stale DHT records do not produce fake lookup success;
* lookup degrades only to a fresh canonical alternate;
* raw IP provider identities are rejected;
* socket and transport-route identities are rejected;
* privacy review detects leaking provider data.

## 8.2 Canonical provider identity

The only canonical public node identity remains:

```text
crab://node/<node-id>
```

Rejected as canonical provider identity:

```text
raw IPv4
raw IPv6
host:port
socket address
TCP route
UDP route
relay://
onion://
service://
```

## 8.3 Stale and unavailable provider behavior

Verified:

* expired records are excluded;
* unavailable records do not become available merely through stale readvertisement;
* status-aware selection is deterministic and bounded;
* missing or invalid local status updates are rejected;
* no usable provider returns explicit failure rather than fake success.

## 8.4 IP privacy

Verified that provider find, debug, and JSON output do not expose:

```text
residential IP
LAN IP
socket field
raw transport route
hidden IP under another field name
```

This privacy work is distinct from the future sanctions IP-blocking requirement.

The provider privacy work prevents disclosure of user addresses. It does not implement country or sanctions-based access blocking.

---

# 9. `svc-storage` changes

## 9.1 Hostile-content and cache-chaos coverage

Added:

```text
phase23_hostile_content_chaos.rs
```

The test proves:

* corrupt provider bytes fail complete BLAKE3 verification;
* cache eviction produces truthful not-found behavior;
* global deny defeats local allow;
* owner tombstone defeats local allow;
* bytes may remain resident while serving is refused;
* oversized objects are rejected without evicting valid cached content;
* hostile content does not create successful delivery or economic/finality authority.

## 9.2 Complete BLAKE3 verification

Verified that content is served only when returned bytes fully match the canonical B3 identifier.

Rejected:

* wrong bytes stored under an object ID;
* corrupt stream bytes;
* malformed B3 identifiers;
* oversized OAP frames;
* oversized OAP chunks;
* object larger than finite cache capacity.

## 9.3 Moderation precedence

Verified effective refusal precedence:

```text
global deny defeats local allow
owner tombstone defeats local allow
local block defeats local allow
quarantine defeats local allow
```

Moderation runs before storage lookup and before serving.

Alternate HTTP, OAP, cache, and storage paths cannot bypass the moderation decision.

## 9.4 Amnesia-first storage

Verified:

* default storage is finite and ephemeral;
* a new backend instance contains no previous ephemeral bytes;
* eviction produces not found;
* exact replacement keeps truthful accounting;
* prune updates accounting;
* oversized objects reject without mutation.

Cache presence is not proof of:

```text
persistence
moderation approval
paid access
reward eligibility
receipt truth
confirmed ROC
```

## 9.5 Non-authority boundary

Verified that `svc-storage` does not expose direct wallet or ledger mutation.

Metering and evidence remain non-authoritative.

Explicit settlement adapters do not turn storage into balance or receipt truth.

---

# 10. `svc-registry` changes

## 10.1 Mid-epoch reward-binding replacement attack

Extended reward-binding coverage for a second rotation attempt during an already pending future-epoch rotation.

Verified:

* the second replacement is rejected;
* the accepted pending rotation is not overwritten;
* the current epoch keeps the original recipient;
* the accepted rotation applies only at its effective epoch;
* the rejected request does not consume its replay nonce;
* the rejected request cannot redirect payout;
* no wallet, ledger, receipt, confirmed ROC, or finality authority is created.

## 10.2 Reward-recipient resolution

Verified the canonical path:

```text
service_node_id
→ active registry binding
→ reward_recipient_account_id
```

Evidence cannot supply or override its own payout address.

Unbound, expired, or mismatched nodes cannot receive a valid payout preview.

## 10.3 Self-payment protection

Verified:

* a Service Node cannot issue to its own bound operator recipient by default;
* beneficiary mismatch is rejected;
* evidence payout override is rejected;
* fixture behavior still requires an explicit feature gate;
* unresolved recipients reject before payout preview.

## 10.4 Eligibility and Sybil resistance

Verified:

* new registry identities cannot enter as eligible or manually trusted;
* candidate nodes have no quorum or reward authority;
* probation nodes are excluded from quorum and capped in reward participation;
* quarantined nodes are excluded from quorum and reward planning;
* fresh-node aggregate weight remains a strict minority;
* many fresh identities cannot gain control by count;
* mature weighted quorum remains deterministic;
* founder-weight or unsafe policy fields are rejected;
* there is no permanent manually trusted production-node flag.

## 10.5 Eligibility transitions

Verified:

* candidate-to-probation transition;
* probation-to-eligible transition based on committed service history;
* quarantine after upheld challenges;
* stale history-root rejection;
* epoch-regression rejection;
* invalid-policy rejection without consuming evaluation state;
* no-op reviews are recorded and replay protected.

---

# 11. CrabLink / `crablink-tauri` changes

## 11.1 Service Node Operator Mode

Completed the optional Service Node Operator surface under:

```text
crab://operator
```

Operator Mode supports:

* local and remote Service Node connection posture;
* canonical operator status;
* signed reward-recipient binding;
* reward-recipient rotation;
* moderation review;
* persistence review;
* receipt-derived confirmed ROC;
* truthful disconnected and unauthorized states.

Operator Mode is disabled by default and is not required for normal CrabLink use or headless Service Node operation.

## 11.2 Native Operator command family

Added and registered native command modules for:

```text
operator_node
operator_reward_binding
operator_moderation_review
operator_persistence_review
```

Command family includes:

```text
service_node_operator_status
service_node_operator_bind_reward_recipient
service_node_operator_moderation_pending
service_node_operator_moderation_decide
service_node_operator_persistence_pending
service_node_operator_persistence_decide
```

The React layer does not receive unrestricted browser-based administrative authority.

## 11.3 Memory-only administrator credentials

Administrator credentials remain route-local and memory-only.

They are not written to:

```text
localStorage
sessionStorage
persistent settings
URLs
logs
confirmed-ROC projections
receipt caches
```

## 11.4 Native OAP object retrieval

Added native bounded OAP retrieval.

The native path:

1. uses the configured privacy-compatible route;
2. requests the canonical B3 object;
3. applies bounded response parsing;
4. rejects malformed, truncated, oversized, or corrupt bytes;
5. performs complete BLAKE3 verification;
6. submits only authenticated bytes to the real User Node queue.

The React layer cannot certify object integrity.

## 11.5 Real User Node submission boundary

Added the Tauri-to-User-Node submission path.

The submission requires:

```text
canonical User Node target
configured privacy route
complete BLAKE3-authenticated bytes
bounded verification request
explicit false authority fields
```

The response remains pending evidence.

Required false authority fields include:

```text
accounting authority
reward authority
wallet mutation
ledger mutation
receipt truth
confirmed ROC
finality
```

## 11.6 Confirmed-ROC native validation

Added:

```text
apps/crablink-tauri/src-tauri/src/confirmed_roc.rs
```

The module validates the real Phase 22 reward-loop projection.

It rejects:

* pending evidence;
* reward-plan-only state;
* fake source labels;
* zero or malformed amounts;
* missing receipts;
* malformed operation IDs;
* duplicate operation IDs;
* missing ledger sequence;
* malformed BLAKE3 roots or hashes;
* wallet mutation claims;
* ledger mutation claims;
* finality claims.

Confirmed ROC may be displayed only after:

```text
wallet receipt exists
ledger replay succeeds
User Node replay accepts the epoch
projection is display-only
client mutation and finality flags remain false
```

## 11.7 Confirmed-ROC frontend normalization

Added frontend normalization for the native projection.

The UI preserves the distinction between:

```text
pending evidence
accounting state
reward plan
wallet request candidate
durable confirmed ROC
```

The UI does not calculate an authoritative account balance from local client state.

## 11.8 Privacy-route-unavailable test

Added:

```text
phase23_privacy_relay_unavailable.rs
```

The test proves:

* one configured OAP attempt is made;
* route failure is returned truthfully;
* no direct or public provider fallback occurs;
* the User Node queue is not contacted;
* no evidence is created;
* no reward state is created;
* no wallet or ledger state is created;
* no receipt is created;
* no confirmed ROC is displayed;
* no finality is claimed.

Privacy failure is not permission to expose the user through a direct route.

## 11.9 Lifecycle isolation

Verified that:

* CrabLink stopping does not redefine Service Node truth;
* Service Node loss affects Operator status only;
* User Node loss affects User Node status only;
* the optional UI is not the Service Node runtime;
* attaching Operator Mode does not merge client and node authority;
* no lifecycle event creates economic authority.

## 11.10 Tauri-wide mouse defect

Investigated and corrected the Tauri-wide intermittent mouse interaction defect.

Previously observed symptoms included:

* only some clicks registering;
* controls requiring Tab navigation;
* inconsistent input interaction;
* scrollbar clicks failing.

The defect was treated as a real hit-testing/event-capture problem, not a cosmetic issue.

After the correction and rebuild, mouse clicking, controls, and scrolling were confirmed to behave normally before Phase 22 continued.

## 11.11 Boundary scanners and acceptance tooling

Added or updated focused scanners for:

```text
Operator boundary
Operator UI boundary
signed reward binding
moderation review
persistence review
OAP object boundary
User Node verification boundary
confirmed-ROC projection boundary
```

The scanners verify that CrabLink cannot silently acquire:

```text
daemon authority
wallet authority
ledger authority
receipt authority
registry authority
policy authority
persistence truth
finality authority
```

---

# 12. Phase 22G completed behavior

The Phase 22G end-to-end reward loop now proves:

```text
verified contribution
→ canonical accounting snapshot
→ deterministic rewarder handoff
→ capped registry-bound plan
→ policy validation
→ eligible quorum
→ svc-wallet execution
→ ron-ledger durable receipt
→ ledger replay
→ independent User Node replay
→ confirmed ROC projection
```

The loop rejects:

```text
self-issuance
arbitrary payout recipient
single-node minting
duplicate payout
economics hash mismatch
policy mismatch
registry-root mismatch
reward-binding-root mismatch
recipient mismatch
invalid quorum signature
noneligible signer
tampered replay
supply mismatch
```

Exact accepted retry returns the same durable receipt without a second issue.

---

# 13. Phase 22H projection and integration repairs

Several failures occurred in new integration tooling and were repaired without weakening product boundaries.

## 13.1 Rust module-layout mismatch

The first patch expected:

```rust
pub mod commands;
```

The real application used:

```rust
mod commands;
```

The patch stopped before all Phase 22H files were created.

A repair patch aligned the new module registration with the real private module layout and completed the confirmed-ROC files.

## 13.2 macOS `mktemp` failure

The first projection runner used a temporary-file template with a suffix after `XXXXXX`.

On macOS, that template failed.

The result was:

* no temporary projection file;
* an empty projection path;
* optional projection export failure.

The repair changed the workflow to create a temporary directory and then place a normal projection filename inside it.

The underlying reward calculation, wallet execution, ledger receipt, and replay logic were not the cause of the failure.

## 13.3 Test-target name mismatch

The first Phase 24 acceptance script referenced old macronode test names:

```text
policy_cli
prune_cli
rewards_cli
setup_cli
persistence_cli
```

The current targets were:

```text
crabnode_policy_cli
crabnode_prune_cli
crabnode_rewards_cli
crabnode_setup_cli
crabnode_persistence_cli
```

The acceptance script was corrected to use the current target names.

No production code failure was involved.

## 13.4 Markdown line-wrap checker failures

The Phase 24D documentation checker initially expected full sentences that crossed Markdown line breaks.

It falsely reported missing text for:

```text
Tor/onion product-contract rejection
privacy-route direct-fallback rejection
```

The checker was corrected to validate line-contained fragments.

The runbook content itself did not need changing.

---

# 14. Phase 23 hardening and chaos coverage

Phase 23 added adversarial and outage coverage rather than new authority.

## 14.1 Phase 23A — expired setup token

Proved:

* expired setup token rejects;
* optional admin stays disabled;
* headless Service Node stays healthy;
* no economic or finality authority is created.

## 14.2 Phase 23B — paused User Node

Proved:

* paused node rejects new verification;
* rejected pause-time work does not consume replay identity;
* read-only pending evidence remains visible;
* resume is safe;
* no accounting, reward, wallet, ledger, receipt, or confirmed-ROC authority is created.

## 14.3 Phase 23C — provider discovery and privacy

Proved:

* stale providers do not create fake success;
* lookup uses only a fresh canonical alternate;
* raw IP and transport identities reject before insertion;
* privacy review catches public output leaks.

## 14.4 Phase 23D — hostile content

Proved:

* corrupt provider bytes fail complete BLAKE3 verification;
* cache eviction returns truthful not found;
* global deny and owner tombstone defeat local allow;
* oversized objects do not evict valid content;
* hostile states create no delivery or economic/finality authority.

## 14.5 Phase 23E — reward-binding rotation attack

Proved:

* second pending rotation is rejected;
* accepted rotation is preserved;
* rejected nonce is not consumed;
* current epoch keeps original recipient;
* rotation applies only at the effective epoch;
* no payout redirection occurs.

## 14.6 Phase 23G — rewarder dependency outage

Proved:

* readiness degrades truthfully;
* liveness remains truthful;
* read-only inspection remains available;
* compute and wallet emission fail closed;
* no new manifest or payout truth is created.

## 14.7 Phase 23J — CrabLink privacy-route failure

Proved:

* privacy-route outage fails before User Node submission;
* no direct/public fallback;
* no evidence or economic state;
* no confirmed ROC;
* no finality.

## 14.8 Final Phase 23 status

The broader Phase 23 acceptance had already been parked green before Phase 24 documentation work began.

Phase 24 reran the most important Phase 23 behaviors as part of the final aggregate gate.

---

# 15. Phase 24 documentation work

Phase 24 created an authoritative private-beta operator and product record and aligned legacy runbooks with current behavior.

## 15.1 Authoritative guide

Created:

```text
docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md
```

The guide documents:

* two-node product posture;
* CrabLink User Node UX;
* Service Node quickstart;
* `crabnode` CLI;
* optional admin UI;
* first-run setup;
* reward-recipient binding;
* ROC earning and reward explanation;
* Service Node quorum;
* IP privacy;
* moderation and pruning;
* persistence policy;
* policy and denylist operations;
* incident response;
* known limitations;
* acceptance evidence.

## 15.2 Phase 24A checker

Created:

```text
scripts/check-phase24-private-beta-node-runbook.sh
```

This validates that the authoritative guide contains the required product, operator, privacy, reward, recovery, and limitations sections.

## 15.3 Node runbook alignment

Updated documentation in:

```text
crates/micronode/docs/RUNBOOK.MD
crates/macronode/docs/RUNBOOK.MD
crates/svc-admin/docs/RUNBOOK.MD
```

These now prominently defer to the Phase 24 private-beta posture.

Created:

```text
scripts/check-phase24-node-runbook-alignment.sh
```

## 15.4 Economic runbook alignment

Updated documentation in:

```text
crates/svc-registry/docs/RUNBOOK.MD
crates/svc-rewarder/docs/RUNBOOK.MD
crates/svc-wallet/docs/RUNBOOK.MD
crates/ron-ledger/docs/RUNBOOK.MD
```

The documented economic path is now:

```text
registry binding and eligibility
→ deterministic capped reward planning
→ wallet mutation front door
→ durable ledger receipt and balance truth
```

Created:

```text
scripts/check-phase24-economic-runbook-alignment.sh
```

## 15.5 Content and privacy runbook alignment

Updated documentation in:

```text
crates/svc-storage/docs/RUNBOOK.MD
crates/svc-dht/docs/RUNBOOK.MD
```

The runbooks now document:

* complete BLAKE3/OAP verification;
* moderation precedence;
* amnesia-first cache;
* persistence eligibility;
* truthful local pruning;
* canonical `crab://` identity;
* stale-provider rejection;
* residential-IP privacy;
* no Tor/onion product contract.

Created:

```text
scripts/check-phase24-content-privacy-runbook-alignment.sh
```

## 15.6 Final readiness and park status records

Updated:

```text
docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md
docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md
docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md
docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md
```

Created:

```text
scripts/check-phase24-private-beta-park-status.sh
```

The final readiness checkbox is now complete.

---

# 16. Final Phase 24 acceptance runner

Created:

```text
scripts/dev-phase24-private-beta-readiness-park.sh
```

The runner verifies:

1. Phase 24 documentation contracts.
2. Rust formatting.
3. User Node behavior.
4. Service Node CLI and operator controls.
5. Provider privacy and stale discovery.
6. Content integrity, moderation, amnesia, and pruning.
7. Reward binding, eligibility, and Sybil resistance.
8. Deterministic reward planning and dependency failure.
9. Wallet quorum, mutation, and duplicate protection.
10. Ledger durability, replay, and conservation.
11. Strict Clippy.
12. RustyOnions workspace compilation.
13. CrabLink User Node and privacy boundaries.
14. Final Phase 24 parked status.

The script now ends only after the park-status checker succeeds.

Final output:

```text
Phase 24 private-beta park-status check passed.
Phase 24 private-beta readiness acceptance passed.
Phase 24 is complete, green, and parked.
PHASE24_FINAL_STATUS=GREEN_PARKED
```

---

# 17. Final acceptance test totals

The final Phase 24 acceptance gate executed **211 focused tests**.

## 17.1 User Node

```text
micronode: 27 tests
```

Covered:

* admin parity;
* epoch replay;
* object verification;
* passive runtime.

## 17.2 Service Node

```text
macronode: 41 tests
```

Covered:

* base `crabnode` CLI;
* policy CLI;
* prune CLI;
* rewards CLI;
* setup CLI;
* persistence CLI;
* operator admin.

## 17.3 Provider discovery

```text
svc-dht: 10 tests
```

Covered:

* discovery chaos;
* privacy review;
* provider lookup;
* provider selection.

## 17.4 Content storage

```text
svc-storage: 31 tests
```

Covered:

* amnesia storage;
* legacy HTTP moderation;
* OAP HTTP transport;
* OAP object fetch;
* hostile-content chaos;
* no direct mutation.

## 17.5 Registry and reward binding

```text
svc-registry: 48 tests
```

Covered:

* cross-crate Sybil resistance;
* eligibility registry;
* eligibility transitions;
* quorum weighting;
* reward-binding registry;
* binding request intake;
* payout guard.

## 17.6 Reward planning

```text
svc-rewarder: 22 tests
```

Covered:

* integration and readiness;
* complete local reward loop;
* no direct mutation;
* replay and no double issue.

The two displayed panics in the confirmed-ROC projection tests were expected `#[should_panic]` tests and passed normally.

## 17.7 Wallet

```text
svc-wallet: 20 tests
```

Covered:

* quorum execution;
* paid-action idempotency;
* receipt lookup after replay;
* approved-payout execution boundary.

## 17.8 Ledger

```text
ron-ledger: 9 tests
```

Covered:

* paid-content replay;
* replay conservation;
* approved-payout replay.

## 17.9 CrabLink Tauri

```text
crablink-tauri: 3 Rust tests
```

Covered:

* privacy-route unavailable;
* required false authority fields;
* User Node target and privacy-route validation.

The native boundary scanner also passed.

Total:

```text
27 + 41 + 10 + 31 + 48 + 22 + 20 + 9 + 3 = 211
```

---

# 18. Compilation and lint verification

The final gate completed:

```text
cargo fmt --check
```

for the affected RustyOnions crates.

Strict Clippy completed for:

```text
macronode
micronode
svc-admin
svc-dht
svc-storage
svc-registry
svc-rewarder
svc-wallet
ron-ledger
```

using:

```text
--all-targets --no-deps -D warnings
```

The `--no-deps` posture remains intentional so unrelated `ron-kernel` lint debt does not block these crate surfaces.

The following completed green:

```text
cargo check --workspace
cargo check -p ron-ledger --features quickchain-preflight
```

CrabLink completed:

```text
cargo fmt --check
cargo test
native boundary scanner
cargo clippy --all-targets --no-deps -D warnings
cargo check
```

---

# 19. Crates tested but not directly code-modified in this session

These crates were exercised by the closing reward loop, acceptance tests, strict Clippy, or workspace checks but did not receive direct Rust source changes during this session:

```text
svc-wallet
ron-ledger
ron-accounting
ron-policy
ron-proto
ron-kms
svc-admin
svc-overlay
svc-gateway
omnigate
```

Important distinction:

* `svc-admin`, `svc-wallet`, and `ron-ledger` did receive Phase 24 runbook/documentation updates.
* They did not receive direct Rust behavior changes in this session.
* Their existing behavior was heavily verified by the final acceptance gate.

---

# 20. Files and scripts added or updated

Important project-level additions and updates include:

```text
scripts/check-phase22-local-reward-loop.sh
scripts/check-phase24-private-beta-node-runbook.sh
scripts/check-phase24-node-runbook-alignment.sh
scripts/check-phase24-economic-runbook-alignment.sh
scripts/check-phase24-content-privacy-runbook-alignment.sh
scripts/check-phase24-private-beta-park-status.sh
scripts/dev-phase24-private-beta-readiness-park.sh
```

Documentation:

```text
docs/internal-roc/PRIVATE_BETA_NODE_RUNBOOK.md
docs/internal-roc/PRODUCT_BETA_READINESS_STATUS.md
docs/internal-roc/PRODUCT_BETA_READINESS_CHECKLIST.md
docs/internal-roc/PRODUCT_BETA_READINESS_PARK.md
```

Runbooks updated:

```text
crates/micronode/docs/RUNBOOK.MD
crates/macronode/docs/RUNBOOK.MD
crates/svc-admin/docs/RUNBOOK.MD
crates/svc-registry/docs/RUNBOOK.MD
crates/svc-rewarder/docs/RUNBOOK.MD
crates/svc-wallet/docs/RUNBOOK.MD
crates/ron-ledger/docs/RUNBOOK.MD
crates/svc-storage/docs/RUNBOOK.MD
crates/svc-dht/docs/RUNBOOK.MD
```

---

# 21. Economic and authority invariants preserved

The final implementation preserves the following path:

```text
verified contribution
→ accounting snapshot
→ deterministic reward plan
→ policy review
→ registry-bound recipient
→ eligible quorum
→ svc-wallet execution
→ ron-ledger durable receipt
→ confirmed ROC display
```

## 21.1 No arbitrary payout recipient

Evidence cannot choose its payout recipient.

The recipient must come from the active registry binding.

## 21.2 No self-pay shortcut

A Service Node cannot pay its own operator recipient by default.

Test-only behavior remains explicitly gated.

## 21.3 No single-node mint

One Service Node cannot:

```text
prepare a valid quorum payout
execute an epoch alone
mint ROC alone
claim finality alone
```

## 21.4 Pending state is not confirmed ROC

The following are not confirmed ROC:

```text
verification evidence
accounting snapshot
reward plan
policy approval
registry binding
quorum proposal
wallet request candidate
client cache
```

Confirmed ROC requires a durable accepted ledger receipt and accepted replay.

## 21.5 No fake truth

The final code and documentation continue to prohibit:

```text
fake balance
fake receipt
fake payout
fake wallet execution
fake ledger mutation
fake confirmed ROC
fake quorum
fake eligibility
fake provider success
fake deletion
fake persistence
fake finality
```

---

# 22. Privacy invariants preserved

User Nodes must not expose:

```text
residential IP
LAN IP
socket address
direct user-to-user route
peer-IP display
geolocation inferred from public UI output
```

Public provider identity remains:

```text
crab://node/<node-id>
```

CrabLink must not fall back to a direct public provider route when its configured privacy-compatible route is unavailable.

Debug output, JSON output, status output, and provider DTOs are all part of the privacy boundary.

---

# 23. Content and persistence invariants preserved

Canonical content identity remains:

```text
b3:<64 lowercase hexadecimal characters>
```

Content is served only after complete BLAKE3 validation.

Unvetted content remains amnesia-first.

Persistence approval means eligibility for durable treatment. It does not prove:

```text
durable write completed
replica exists
archive exists
network-wide pin exists
checkpoint exists
```

Local pruning does not claim:

```text
network-wide deletion
remote deletion
global tombstone publication
successful provider withdrawal when unavailable
successful index invalidation when unavailable
```

Moderation and pruning do not mutate wallet or ledger state.

---

# 24. Sanctions-compliance and IP-ban TOML status

## 24.1 Clear status

The sanctioned-country and sanctioned-IP blocking system was **not implemented in code during this session**.

Based on the work performed and the supplied project records:

```text
configs/sanctions-compliance.toml
configs/sanctions-compliance.dev.toml
```

were defined as required future canonical paths, but this session did not create and integrate the complete sanctions-compliance system.

The requirement was added to `BUILD_PLAN_Z` as:

```text
Future Compliance Gate — Sanctioned-Jurisdiction IP Range Blocking
```

It remains mandatory future work.

## 24.2 What was recorded

The build plan now specifies that the future policy must support:

* schema and version;
* canonical and development profiles;
* explicit enablement posture;
* blocked ISO country codes;
* blocked region or territory identifiers;
* IPv4 CIDR ranges;
* IPv6 CIDR ranges;
* source name and revision;
* effective and review timestamps;
* lookup-failure posture;
* per-boundary enforcement flags;
* reviewed development exemptions.

Planned canonical files:

```text
configs/sanctions-compliance.toml
configs/sanctions-compliance.dev.toml
```

## 24.3 What was not implemented or verified

This session did not implement or verify:

```text
canonical production sanctions TOML file
development sanctions TOML file
shared Rust loader
shared typed sanctions policy object
IPv4 CIDR evaluator
IPv6 CIDR evaluator
country-code evaluator
region-code evaluator
trusted-proxy remote-address resolution
CrabLink bootstrap enforcement
public API ingress enforcement
gateway ingress enforcement
account/passport registration enforcement
Service Node registration enforcement
peer-admission enforcement
runtime participation enforcement
reward-evidence enforcement
reward-eligibility enforcement
sanctions decision metrics
privacy-preserving sanctions audit records
appeal or error-correction workflow
authoritative sanctions-source update process
```

No claim should be made that sanctioned countries or sanctioned IP ranges are currently blocked across CrabLink and RustyOnions.

## 24.4 Important distinction from completed IP privacy work

The completed DHT and CrabLink work protects users from IP disclosure and direct-route fallback.

That completed work includes:

```text
no residential IP in provider output
no raw socket provider identity
canonical crab:// node identity
no direct public fallback on privacy-route failure
```

This is **not** the same as:

```text
country geolocation blocking
sanctions jurisdiction blocking
IPv4 sanctions CIDR blocking
IPv6 sanctions CIDR blocking
```

The privacy boundary is implemented.

The sanctions/IP-ban enforcement boundary remains future work.

## 24.5 Required future implementation timing

The sanctions-compliance gate must be completed before:

```text
public production access
production node admission
production runtime participation
production ROC reward eligibility
```

It should not block continued local development or controlled private-beta testing, but public production must not proceed without it.

---

# 25. Recommended sanctions-compliance implementation sequence

This is the most important near-term follow-up after the BUILD_PLAN_Z wrap-up.

## Step 1 — inspect for partial prior work

Before patching, inspect the current repository for any existing partial sanctions files, schemas, or policy types.

Do not assume the example in `BUILD_PLAN_Z` is already present as a real file.

## Step 2 — add canonical configuration files

Create:

```text
configs/sanctions-compliance.toml
configs/sanctions-compliance.dev.toml
```

The production file must not contain an invented or unreviewed authoritative sanctions list.

The development profile must be explicitly labeled and must not silently weaken production posture.

## Step 3 — choose one shared owner

Prefer:

```text
ron-policy
```

or a narrowly scoped dedicated compliance crate.

Only one owner should:

* parse TOML;
* normalize country and region identifiers;
* parse IPv4 and IPv6 CIDRs;
* validate metadata;
* calculate a stable policy identity/hash;
* return typed decisions.

Consuming crates must not independently parse the TOML or maintain separate sanctions lists.

## Step 4 — validate fail-closed behavior

Validation should reject:

* malformed CIDR;
* duplicate or overlapping malformed policy entries where prohibited;
* invalid country codes;
* invalid region identifiers;
* invalid schema or version;
* missing source metadata;
* invalid timestamps;
* unsafe production exemptions;
* unknown enforcement keys;
* production policy with disabled required boundaries.

## Step 5 — trusted remote-address derivation

The evaluator must obtain the original remote address only from a trusted ingress boundary.

`Forwarded` and `X-Forwarded-For` must be honored only from explicitly configured trusted proxies.

An untrusted client must not be able to spoof the address used for compliance evaluation.

## Step 6 — apply consistent enforcement

The same policy should be enforced at:

```text
CrabLink frontend/session bootstrap
public API ingress
gateway ingress
account or passport registration
Service Node registration
peer admission
node runtime participation
reward-evidence acceptance
reward eligibility
administrator and creator product surfaces
```

Frontend denial alone is insufficient.

Server-side enforcement is required.

## Step 7 — preserve authority boundaries

The sanctions policy may allow, deny, or defer later processing.

It must not:

```text
create ROC
modify ROC balance
write ledger receipts
create payout authority
create quorum authority
mint or burn ROC
mint or burn ROX
claim legal finality
claim universal compliance
```

## Step 8 — add focused tests

Required tests should include:

* blocked IPv4 CIDR denied;
* blocked IPv6 CIDR denied;
* allowed addresses continue to later checks;
* malformed CIDR rejects policy loading;
* invalid country code rejects loading;
* client forwarding-header spoof cannot bypass ingress resolution;
* frontend denial mirrored server-side;
* blocked node cannot register;
* blocked node cannot join runtime;
* blocked node cannot enter reward planning;
* lookup failure follows configured production posture;
* development fixture requires development profile;
* consuming crates contain no hardcoded sanctions list;
* sanctions decision creates no wallet, ledger, payout, quorum, or finality state.

## Step 9 — logging and privacy

Avoid unnecessary storage of full IP addresses.

Prefer:

```text
stable bounded audit ID
normalized decision code
policy revision
privacy-preserving prefix where sufficient
low-cardinality metrics
```

Do not expose the full sanctions block list to untrusted clients.

---

# 26. Work explicitly not completed or enabled

The following remain outside the completed BUILD_PLAN_Z private-beta scope:

```text
sanctions/IP-ban TOML implementation
public production deployment
public permissionless node admission
production ROC reward participation
live wallet calls
live ledger mutation outside intended local tests
production minting
production burning
ROX/Solana bridge runtime
production bridge settlement
staking
liquidity
exchange-facing behavior
external redemption
public chain finality
```

ROX Anchor remains a separate project and is not activated by this BUILD_PLAN_Z completion.

---

# 27. Operational cautions going forward

## 27.1 Do not reopen green phases casually

Phases 21–24 are parked.

Reopen only for:

* confirmed regression;
* new private-beta requirement;
* security defect;
* sanctions-compliance implementation;
* production-readiness work explicitly outside BUILD_PLAN_Z.

## 27.2 Preserve the acceptance runner

Use:

```text
scripts/dev-phase24-private-beta-readiness-park.sh
```

as the reproducible aggregate regression gate when making changes that touch the completed node/reward/client path.

## 27.3 Preserve the two-node model

Do not create a third public Economic Node product.

## 27.4 Preserve no-fake-authority rules

Do not add client-side balances, fake receipts, fake payout status, fake finality, or fake provider success to simplify demos.

## 27.5 Preserve headless operation

The Service Node must remain CLI-operable and independent of CrabLink Operator Mode and `svc-admin`.

## 27.6 Preserve no centralized password reset

Lost node-admin credentials require local admin-state recreation.

Durable rewards remain tied to the external reward-recipient account.

## 27.7 Preserve no Git mutation instruction

Do not add, commit, push, or otherwise mutate Git state unless the user explicitly reverses the current instruction.

Continue using ordinary filesystem inspection and paste-ready patches.

---

# 28. Recommended next-session opener

Use the following opener when beginning the post-BUILD_PLAN_Z work:

```text
We are continuing RustyOnions after the final BUILD_PLAN_Z closeout.

BUILD_PLAN_Z Phase 24 is COMPLETE / GREEN / PARKED, and
scripts/dev-phase24-private-beta-readiness-park.sh ends with:

PHASE24_FINAL_STATUS=GREEN_PARKED

Do not reopen the completed node, reward-loop, privacy, or private-beta phases
unless a real regression is found.

The next near-term requirement is the centralized sanctions-compliance and
sanctioned-jurisdiction IP-blocking gate recorded in BUILD_PLAN_Z.

First inspect the repository for any existing partial implementation of:

configs/sanctions-compliance.toml
configs/sanctions-compliance.dev.toml

Then implement one shared typed loader/evaluator, likely owned by ron-policy or
a dedicated compliance crate, before wiring it into CrabLink bootstrap, gateway
ingress, Service Node registration, peer/runtime admission, and reward
eligibility.

Use focused tests, fail closed for protected production operations, do not
hardcode sanctions lists across crates, preserve IP privacy, and do not create
wallet, ledger, reward, quorum, or finality authority.

No Git commands or Git mutations.
```

Recommended baseline command before new implementation:

```bash
cd /Users/mymac/Desktop/RustyOnions

bash scripts/dev-phase24-private-beta-readiness-park.sh
```

Only begin new work after the baseline still ends with:

```text
PHASE24_FINAL_STATUS=GREEN_PARKED
```

---

# 29. Final session conclusion

This session finished the complete `BUILD_PLAN_Z` implementation arc.

The resulting system now has:

```text
private CrabLink-managed User Node
headless CLI-managed Service Node
canonical crab:// node identity
bounded OAP retrieval
complete BLAKE3 verification
real pending verification evidence
replay rejection
resource and queue bounds
moderation enforcement
amnesia-first storage
persistence eligibility
truthful local pruning
provider discovery and stale-record rejection
residential-IP privacy
Service Node reward-recipient binding
objective eligibility and quarantine
Sybil-resistant quorum weighting
deterministic capped reward planning
single-node mint rejection
self-payment rejection
wallet mutation through svc-wallet
durable receipt truth through ron-ledger
exact retry without double issue
ledger replay and conservation
independent User Node replay
deterministic challenge behavior
receipt-only confirmed ROC in CrabLink
optional Service Node Operator Mode
fail-closed dependency and privacy-route behavior
complete private-beta operator documentation
reproducible final acceptance tooling
```

Final state:

```text
RustyOnions / CrabLink
BUILD_PLAN_Z
COMPLETE / GREEN / PARKED
```

Known next major requirement:

```text
Centralized sanctions-compliance TOML policy and sanctioned-jurisdiction
IP-range enforcement — NOT YET IMPLEMENTED.
```

That requirement should be addressed in the near future and must be complete before public production access, production node admission, or production ROC reward participation.

These notes are ready to place in the RustyOnions repository as the final `BUILD_PLAN_Z` wrap-up record.


### END NOTE - JULY 15 2026 - 18:40 CST