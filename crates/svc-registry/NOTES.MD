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