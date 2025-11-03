### BEGIN NOTE - OCTOBER 30 2025 - 17:00 CST
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
IMPORTANT INFORMATION FROM OTHER CRATES WHICH WILL BE HELPFUL IN BUILDING OMNIGATE:
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!


Awesome—here’s a tight, carry-over “Omnigate Integration Notes” pack distilled from the uploaded code bundles for **ron-transport**, **oap**, **svc-overlay**, and **ron-policy**. It’s organized so you can paste sections into `crates/omnigate/NOTES.md` (and reference in your IDB/TODO).

# Omnigate Integration Notes (carry-over)

## 0) What Omnigate must be

Ingress that speaks HTTP/Web transports on the edge, validates capability/policy, enforces platform ceilings, then forwards into the mesh (OAP/1 over ron-transport → overlay/DHT/storage/index). Core invariants: no ambient auth, deterministic envelopes, amnesia-friendly, truthful readiness, bounded labels, and fixed wire limits (frame ≤ **1 MiB**, chunk ≈ **64 KiB**).  

---

## 1) Wire & Envelope (OAP/1 → Transport) — what Omnigate must honor

* **Deterministic envelopes + size caps:** OAP/1 enforces 1 MiB frame and ~64 KiB chunk sizing; DTOs use strict serde to avoid drift. This is a **platform bound** that policy can only tighten, never relax. Omnigate must reject earlier than downstream if a request would exceed caps.  

* **Transport invariants:** single-writer per connection; no locks across `.await`; enforce max frame before allocation. These assumptions appear in `ron-transport`’s public surface and helpers (e.g., `reserve_capped`). Omnigate’s ingress writers should follow the same pattern.  

* **Cancellation & graceful close:** rely on `Cancel` (tokio-util `CancellationToken`) and attempt protocol-graceful shutdown (e.g., TLS `close_notify`). Ingress request tasks should spawn with a child token and always attempt graceful writer shutdown.  

* **Read/Write/Idle deadlines:** use the shared timeout helpers (`with_timeout`, `IdleGuard`) for bounded operations and idle detection. Map Omnigate’s per-route policy to these timeouts.  

---

## 2) Transport Hooks Omnigate will call (ron-transport)

* **Public API modules:** `config`, `conn`, `limits`, `metrics`, `tcp`/`tls` (feature-gated), readiness gate, error taxonomy, and a stable TLS type alias. Plan to consume `spawn_transport(...)` + bus/health plumbing exactly as used by examples.  

* **TLS server config** (rustls 0.26.x wrapper) is provided behind a feature with helper that loads certs/keys and builds `ServerConfig`. Keep Omnigate’s TLS load path identical for consistency.  

* **Transport events on the kernel bus:** `Connected/Disconnected {peer, name, reason}`. Omnigate should forward these to its observability stream and include `name` = "omnigate". 

* **Metrics counters (writer path):** `bytes_out` per transport name; Omnigate should mirror the pattern for ingress egress bytes. 

* **Smoke/bench scripts** to reuse as patterns for Omnigate’s own `scripts/local/*`: echo smoke, tls smoke, etc., including CI “sanitize env” script.  

---

## 3) Overlay & Discovery boundaries (svc-overlay / svc-dht)

* **Overlay vs DHT split:** Overlay focuses on sessions, routing, gossip, peer health—DHT (svc-dht) handles provider discovery. Omnigate should not embed DHT semantics—only call into index/dht services over OAP when needed. 

* **Acceptance targets (discovery path):** DHT lookups **p99 ≤ 5 hops**; Omnigate’s retries should respect bounded backoff/jitter and surface clean errors. 

---

## 4) Policy & Governance (ron-policy) — what Omnigate must consult

* **Library, not service:** `ron-policy` is a pure lib; evaluation is deterministic & side-effect-free; callers provide the clock (`now_ms`). Omnigate integrates it directly in the request path (PDP co-located with ingress).  

* **Stable public surface:** consume `PolicyBundle`, `evaluate(bundle, &RequestCtx, now_ms) -> Decision`, and validation helpers. Keep DTO shapes strict (`deny_unknown_fields`).  

* **Tighten-only vs platform bounds:** Policy may only **tighten** Omnigate’s global limits (frame/body, decompression ratio). Omnigate must treat platform caps as non-negotiable pre-checks. 

* **Performance intent:** With ≤1k rules on common paths, eval must be **p95 < 1 ms** on modest cores. Your recent bench (~1.17 µs op) is far under budget; carry that SLO into Omnigate’s perf gates. 

* **Reason codes:** keep stable strings for allow/deny and validation causes; these feed structured errors/logs in Omnigate. 

---

## 5) Omnigate HTTP surface (copy these operational behaviors)

* **Observability endpoints:** `/metrics`, `/healthz`, `/readyz`, `/version` with **truthful readiness** (don’t 200 `/readyz` until bundles parsed, transport bound, and caches warmed). 

* **Error taxonomy & status mapping:** `BadRequest→400`, `NotFound→404` (cacheable negative), `Forbidden→403`, `Conflict→409` (bundle updates), `Internal→500`. Add a metric label `err_kind` for cardinality-controlled observability. 

* **400 vs 404 discipline:** malformed input is **400**; a miss that is well-formed is **404**; explicit denials are **403**. Preserve body shape across 200/404 to avoid client churn. 

* **Admin plane header:** support `X-Admin-Token` (off by default) for admin endpoints; PDP path may accept `Authorization: Bearer …` under a feature gate. 

* **Label discipline:** bound metric labels (e.g., `tenant`, `route`, `decision`), no per-user labels. 

---

## 6) Config surface alignment

* **Transport features:** default `tcp+tls`; optional `arti`, `quic`. Omnigate should mirror feature gates and keep type parity (notably `tokio_rustls::rustls::ServerConfig` via the alias).  

* **Runtime knobs:** bind addr, timeouts (read/write/idle), max frame/body caps, and readiness gates must line up with `ron-transport`’s `TransportConfig`/timeouts helpers.  

---

## 7) CI & Tooling carry-over

* **Perf & TLA placeholders:** replicate `perf.yml` and `tla.yml` scaffolds so Omnigate has bench artifacts and a future TLA+ hook ready. Also include `env_sanitize.sh`.   

* **Local smokes:** create `scripts/local/ingress_smoke.sh` by cloning the `tcp_echo` pattern: spin server, wait for “listening on …”, curl/nc probes, and clean shutdown.  

---

## 8) Reference flows Omnigate must enable (end-to-end acceptance)

* **Store-and-forward:** cap-checked ingress → mailbox SEND/RECV/ACK with idempotency; **p95 enqueue+dequeue < 50 ms (local)**. Omnigate must enforce DRR/quotas on ingress. 

* **Edge assets:** range requests, strong caching; **p95 hit < 40 ms (local)**. Ensure Omnigate supports byte-ranges and cache headers. 

* **Content lookup & fetch:** index→dht→storage with hop and retry bounds, surfaced through Omnigate. 

* **Cap issuance/use:** no ambient trust; short TTLs; rotations w/o downtime—Omnigate must treat caps as first-class and fast-reject missing/expired. 

---

## 9) “Done for Beta” checklist (Omnigate)

1. **Ingress server bound** (TCP/TLS), truthful `/readyz`. (Reuse ReadyGate pattern.) 
2. **Policy eval in hot path** with cached `PolicyBundle`; deterministic reasons in responses/logs.  
3. **Caps enforced**: frame/body/decompress ceilings pre-check; timeouts set via helpers.  
4. **Error taxonomy + metrics** aligned (latency histogram, decision counter, bounded labels). 
5. **Admin plane guard** (`X-Admin-Token`) behind a feature; trace + body-limit middleware. 
6. **Local smoke & CI scaffolds** (perf/TLA placeholders) in `scripts/` and `.github/workflows/`.  

---

## 10) Post-beta (quick wins)

* **QUIC/Arti features**: enable behind features to match transport story (optional in beta). 
* **Policy lifecycle hooks**: track `PolicyChanged{version,id_b3}` bus event and hot-swap bundles. 
* **Schema emission (optional)**: feature-gated JSON schema endpoint for clients. 

---

## 11) Concrete “how to run” seeds (copy/adapt)

* **Transport TLS smoke (pattern for Omnigate):** per `spawn_transport(...)` + rustls loader; print “listening on …”, then `curl`/`nc` like the echo smoke script.  

* **Bench/Perf artifacts:** replicate `perf.yml` to archive Criterion results so we can diff Omnigate hot path regressions later. 

---

Got it—here’s a tight, drop-in carryover pack for **omnigate** that pulls the critical contracts, invariants, and “don’t-break-this” behaviors from **ron-kernel**, **ron-bus**, **ron-proto**, and **ron-metrics**.

# Omnigate Carryover Notes (from kernel/bus/proto/metrics)

## 1) Wire & envelope contracts (ron-proto)

* **OAP/1 envelopes** (HELLO/START/DATA/END/ERROR) are the interop contract. Respect the enum set and frame size guard: `MAX_FRAME_BYTES = 1 MiB`. Feature growth is via stable tokens (e.g., `Hello.features: Vec<String>`).  
* **HELLO** carries the protocol (`"OAP/1"`), version (mirrors `PROTO_VERSION`), and a features vector; unknown fields are rejected. Default uses owned `String`.  
* **START** sets negotiated limits: `seq_start`, `max_frame_bytes` (must be ≤ 1 MiB), optional meta. Hosts enforce the limit. 
* **END/ERROR** finalize flows and carry stable error kinds. Keep error messages and optional details wire-stable.  
* **Validation**: use `Validate` with `Limits` and reject DATA frames exceeding negotiated `max_frame_bytes`. The tests show the exact shape (8-byte OK vs 9-byte reject). 

**Implication for omnigate**

* On inbound connections: do `HELLO → START` handshake, persist negotiated `max_frame_bytes` per connection/stream, and enforce on all DATA frames.
* Keep feature tokens string-stable; don’t gate logic on ad-hoc strings without a registry.
* Treat OAP errors as telemetry-worthy, with typed `Kind` surfaced to metrics/logs.

---

## 2) Kernel surfaces & lifecycle (ron-kernel)

* **Frozen public API** omnigate should rely on: `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`. These are semver-guarded. 
* **Readiness contract**: `/readyz` must return **503 until** both `config_loaded` and “services healthy” are true; avoid locks across `.await`, use bounded channels, and one-receiver-per-task patterns. 
* **Metrics exporter** exposes `/metrics`, `/healthz`, `/readyz`; optional thread-local buffering (`metrics_buf`) removes atomics on the hot path and surfaces bus counters (publish, drop, lag, etc.) and MOG counters.  
* **Demo wiring** shows env→labels, `HealthState` seeds, server bind, and help text with endpoints (copy this shape for omnigate’s admin plane).  
* **Example service wiring** also shows combining app routes with metrics router and middlewares (latency/status). This is the canonical Axum integration. 

**Implication for omnigate**

* Mount the **admin plane** exactly like kernel/metrics examples: fuse business routes + metrics/health/ready + latency/status middlewares.
* Follow readiness semantics rigidly; don’t flip `/readyz` to 200 until **at least** `config_loaded=true` and your core deps are healthy.
* If performance warrants, enable the `metrics_buf` feature in the build matrix for the gateway hot path.

---

## 3) In-process eventing (ron-bus)

* **Bus**: bounded Tokio broadcast wrapper; lossy for **lagging receivers**; capacity fixed at construction. Sender is cloneable; **one receiver per task** to avoid sharing/races.  
* **Capacity**: configured via `BusConfig` (`capacity`, overflow warn throttle, namespace, amnesia label). Marked `#[non_exhaustive]`; use builder, not struct literals.   
* **Events set** (non-exhaustive): `Health{service, ok}`, `ConfigUpdated{version}`, `ServiceCrashed{service, reason}`, `Shutdown`. This mirrors kernel’s canonical events. 
* **Integration tests** show expected usage and **capacity cutover** (drop old bus, create new with new capacity; there is no runtime resize).  

**Implication for omnigate**

* Use a **dedicated receiver per task** for gateway subsystems (e.g., connection manager, config watcher). Expect `RecvError::Lagged(n)` in stress; decide whether to raise capacity or tolerate drop.
* If you ever need to resize, perform a **cutover** (A→drop→B) like the test demonstrates.
* Publish `KernelEvent::Health` and `ServiceCrashed` to make ops behavior uniform across services.

---

## 4) Metrics, health, latency (ron-metrics)

* **Attach to Axum**: merge metrics router and attach `axum_latency` + `axum_status` middlewares so `/metrics` reflects both counts and latency histograms; tests assert 2xx counters and latency counts appear.  
* **Labels**: always provide `BaseLabels { service, instance, build_version, amnesia }`. Health keys (e.g., `config_loaded`, `db`, `cache`) drive `/healthz` and `/readyz`.  
* **Standalone exposer** pattern (bind addr from `RON_METRICS_METRICS_ADDR`, print URLs) is the recommended “boring defaults” snippet.  

**Implication for omnigate**

* Mirror the **metrics wiring** from examples to keep instrumentation consistent. Export a small, stable set of gateway counters (requests by class, inflight, rejects due to backpressure), and rely on the shared latency histogram for RED metrics.
* Use `HealthState` keys (“config_loaded”, “overlay”, “policy”, “dht”, “index”, “kms”, etc.) to make `/readyz` reflect omnigate’s real deps.

---

## 5) Concurrency & backpressure (SDK/gateway perspective)

* The **Crate Map** notes the model we want: a small async connection pool; per-connection `inflight_limit`; global semaphore; **wait or fail-fast** policy when saturated; timeouts/retries for *idempotent* ops only (GET/HELLO). PUT retries require idempotent server semantics (e.g., content-hash PUT-if-absent). 
* Kernel forbids cross-await locks; prefer short, contained critical sections and bounded channels. 

**Implication for omnigate**

* Implement **global + per-conn** inflight guards around OAP I/O. Respect idempotency rules for retries.
* Never hold a lock across `.await`; prefer channels and atomic state for routing (e.g., response `corr_id` fan-out).

---

## 6) Operational invariants to preserve

* **Readiness gate** policy (503 until ready) is part of our SLO story; do not relax it. 
* **Bus semantics**: lossy for laggers; one receiver per task; no runtime capacity change—do cutover.  
* **OAP limits**: enforce negotiated `max_frame_bytes` and the absolute 1 MiB cap.  
* **Metrics shape**: keep base labels + latency/status middlewares; ensure `/metrics` shows request status class and latency counts.  

---

## 7) Quick integration checklist for omnigate

1. **Admin plane**

   * Merge metrics router; attach `axum_latency` + `axum_status`. Export `/metrics`, `/healthz`, `/readyz`. Seed `HealthState` keys per dependency. 
2. **Handshake**

   * Perform `HELLO` (protocol/version/features) and validate; send `START` with agreed `max_frame_bytes` (≤ 1 MiB). Enforce for all DATA.   
3. **Concurrency**

   * Use per-conn inflight + global semaphore; no lock across `.await`. Retries only for idempotent ops.  
4. **Event bus**

   * Allocate a `Bus` for omnigate system events; one receiver per task; publish `Health`, `ConfigUpdated`, `ServiceCrashed`, `Shutdown`. Use capacity cutover if resizing.   
5. **Telemetry**

   * Keep base labels; ensure request status class counters and latency histogram are visible under load tests. 

---

## 8) “Paper cuts” to avoid

* Don’t pattern-match on raw feature strings from `Hello.features` without a registry—treat them as **tokens** (future-proof). 
* Don’t grow frames above negotiated `max_frame_bytes` even if you “know” the peer—treat it as a hard invariant. 
* Don’t try to “resize” the bus in place—cut over like the test shows. 

---

## 9) Known-good code shapes to copy

* **Metrics+HTTP template** (labels, bind, printed endpoints): use the `exposer.rs` and `app+metrics` example patterns.   
* **Bus API surface** (new → sender → subscribe → recv → Shutdown): mirror the `api_surface_basic` test. 

---


Awesome—here’s a tight, drop-in carryover pack for **svc-omnigate** based on the codebundles for **svc-index, svc-storage, ron-naming, svc-dht** (and the shared blueprints). I focused on contracts, limits, readiness, observability, and the little gotchas that will keep Omnigate “drift-proof.”

# Carryover Notes → svc-omnigate

## 1) HTTP contracts & error taxonomy to mirror

* **Status discipline (client vs. miss vs. hit).** Keep the 400/404/200 split you used in svc-index; Omnigate should forward the same semantics when brokering to services, and add 403 for explicit policy denials when ron-policy is in the path. Also preserve **stable DTO shapes even on 404** to avoid frontend churn. 
* **One mapping layer for errors.** Reuse the compact enum→HTTP mapping pattern (labels suitable for metrics). Expect: `bad_request→400`, `not_found→404`, `forbidden→403`, `conflict→409`, `internal→500`. Add `err_kind` labels. 
* **Concrete example in svc-index** shows structured JSON body and labeled codes—copy that shape for Omnigate’s gateway errors. 

## 2) Hardening defaults & limits (copy as-is)

* **Global limits** to apply at the edge: 5s timeouts; 512 inflight; 500 rps; **1 MiB body cap**; decompression ≤10× + absolute cap. Enforce through a small Tower layer and apply consistently on every route. Expose `/metrics`, `/healthz`, `/readyz`, `/version`. 
* svc-index already declares constants aligned with these defaults (e.g., 1 MiB `MAX_BODY_BYTES`, OAP frame caps); Omnigate should centralize these so backends don’t drift. 

## 3) Readiness & lifecycle

* **Truthful readiness:** Don’t report ready until upstream dependencies are open/warmed (e.g., policy bundle compiled, routing table preheated). svc-index/ron-policy notes call out “warm before `/readyz`=200.” Use the same gate in Omnigate. 
* **Supervisor pattern:** svc-dht uses a bootstrap supervisor which opens the “ready gate” only after minimum fill—reuse this gate+signal pattern in Omnigate to coordinate admin plane readiness. 

## 4) Observability (names, labels, guardrails)

* **Endpoints:** `/healthz`, `/readyz`, `/version`, `/metrics`. **Metrics:** reuse `request_latency_seconds` histogram; add `inflight_requests` gauge and per-decision counters where Omnigate invokes policy (`decision="allow|deny|error"`). Keep label cardinality bounded (tenant, route, decision). 
* **Correlation IDs:** svc-index injects `x-corr-id` via a helper; Omnigate should do the same at ingress so all services share the same header.  

## 5) AuthN/Z handoff patterns

* **Admin/auth layers:** svc-index pattern (`X-Admin-Token`) generalizes well. For Omnigate: admin plane stays header-guarded; data plane prefers `Authorization: Bearer <jwt>` (feature-gated, minimal deps) and punts policy to ron-policy. 

## 6) Body limits, decompression, and trace

* **Body caps** and **decompression guards** exist as placeholders in svc-index—implement the real Tower layers in Omnigate and enforce across all proxied routes (prevents “zip bombs” and jumbo payloads). 

## 7) Naming & addressing (don’t re-invent)

* **ron-naming is authoritative** for address parsing/normalization (BLAKE3 CAS ids, IDNA, unicode normalization). Omnigate must **call into ron-naming** rather than parsing on its own; this prevents format drift between clients, gateway, and backends.  
* CLI/test scaffolds and benches in ron-naming show the exact encode/normalize contract—mirror these when validating incoming Omnigate routes that carry addresses in path/query.  

## 8) Index/DHT/Storage brokering rules

* **Index first, DHT as hint:** svc-index provides a DHT client stub and ranking heuristics; Omnigate should consult svc-index for resolution, and only reach into DHT paths for control-plane flows or as a fallback/hint (hedged lookups later).  
* **DHT concurrency controls:** svc-dht exposes a simple, global semaphore limiter for in-flight “legs.” If Omnigate ever fans out lookups (e.g., multi-backend reads), reuse this limiter pattern to cap concurrency and backpressure at the edge. 
* **Storage contract touchpoints:**

  * `PUT` returns `{ cid }` (JSON); Omnigate should pass this unchanged. 
  * **Range/ETag path** and **streaming slices** are zero-copy oriented; preserve range headers and avoid buffering >1 MiB chunks when proxying. 
  * Respect **amnesia mode** and profile knobs (micronode/macronode) when deciding persistence and retries. 

## 9) Config surface to keep consistent

* Keep **env var names** consistent with svc-index/storage (binds, timeouts, max body bytes). Example storage profile shows `max_body_bytes = 1_048_576`, timeouts 5 s—clone those defaults to Omnigate and allow override in a single `svc-omnigate.example.toml`. 
* Ensure **DB/socket paths** align across services to avoid “phantom 404s” (index DB path mismatch was a prior footgun). 

## 10) Admin plane & security plumbing

* **UDS posture & peercred**: storage’s UDS/peercred scaffolds indicate the posture we want whenever Omnigate speaks locally—use `0700` dir, `0600` sockets, and **SO_PEERCRED** allow-lists for privileged admin ops.  

## 11) CI/quality gates to inherit

* **Phase gates**: Bronze→Gold flow (Clippy, grep bans, TSan, Loom for kernel-adjacent logic, structure-aware fuzz before RC). Ensure Omnigate’s CI inherits these exact gates. 
* **No unbounded buffers**, streaming I/O, memory ceilings—explicitly test for these with perf/chaos jobs (svc-index already has perf/chaos workflows).  

## 12) “Do this exactly” checklist for Omnigate MVP

1. **Ingress stack:** `TraceLayer` → `BodyLimit(1 MiB)` → `DecompressionGuard(≤10×)` → `Admin/Auth` → `CorrId` → router. (Names/limits from above.)   
2. **Readiness DAG:** block `/readyz` until policy bundle parsed, index channel healthy, and transport bound—mirroring the svc-dht supervisor “ready gate.”  
3. **Error/metrics:** adopt svc-index JSON error body and label scheme; publish `request_latency_seconds` + `inflight_requests`, and policy `decision_total`.  
4. **Naming gate:** run all inbound addresses through **ron-naming** normalization (no ad-hoc parsing). 
5. **Proxy behavior:** stream bodies; don’t buffer beyond chunk hints (64 KiB); preserve `Range`, `ETag`, and `x-corr-id` end-to-end.   
6. **Profiles:** respect micronode/macronode toggles for amnesia, retries, and persistence defaults. 
7. **Concurrency/backpressure:** if Omnigate does any fan-out (e.g., to multiple backends), cap with a semaphore limiter like svc-dht’s `Limiter`. 

---

### END NOTE - OCTOBER 30 2025 - 17:00 CST


### BEGIN NOTE - OCTOBER 31 2025 - 11:44 CST

# Status snapshot

**What’s working now**

* **Binary boots & binds**: API on `127.0.0.1:5305`, admin on `127.0.0.1:9605`.
  `GET /healthz → {"omnigate":true}`; `/metrics` exports baseline gauges/counters.
* **Middleware stack assembled** (order: cheap → expensive): fair-queue, quotas, corr-id, classify, decompress guard, body caps, slow-loris placeholder.
* **Admission tests**: `tiny_quota_allows_then_429s`, `quota_when_exhausted_429` ✅.
* **Hardening tests**: body/decompression guards (415/413 variants) ✅.
* **Smoke script**: `crates/omnigate/scripts/smoke_omnigate.sh` runs fmt+clippy, boots, hits `/healthz`+`/metrics`, checks 404 on `/`.
* **Bench** (in-process / no sockets): `~10.8–10.9 µs` per GET `/ping` with full middleware → ~**92k req/s** single-stream. Good.

**My estimate**

* **~70% to Beta** (RON-CORE beta bar: boots clean, guards on, basic admission/routing, health/ready/metrics solid, benches and smoke scripted).
* **~35% to Gold** (production polish: policy integration, overload/readiness gating, structured error surface, fuzz/property tests, perf ablation + docs).

---

# What’s next (Beta checklist)

1. **Config surface for admission** (make it real & TOML-driven)

   * Quotas: global QPS/burst; optional per-IP/per-CIDR buckets.
   * Fair queue: weights for classes (anon/auth/admin), max in-flight.
   * Body caps: `max_content_length`, `reject_on_missing_length`.
   * Decompression guard: allowlist (`identity`, `gzip`), hard-deny stacked encodings.
   * **Deliverables**: `Config` struct + `From<toml>`; defaulted `Default::default()`; validate at startup; doc in `CONFIG.md`.
   * Add a paste-ready `omnigate.toml` example.

2. **Metrics you actually need**

   * `admission_quota_exhausted_total` (labels: reason=global|ip|token)
   * `admission_fair_queue_enqueued_total` / `_dropped_total` / `_inflight`
   * `http_requests_total` (method, route, status) and `request_latency_seconds` histogram
   * `body_reject_total` (oversize/missing_len) and `decompress_reject_total` (unknown/stacked)
   * Wire them into `/metrics`; add a **metrics contract test** with regex asserts.

3. **Readiness / overload gate**

   * Flip **/readyz** to **degrade** when:

     * in-flight > threshold, or
     * sustained 429/503 rate > threshold over N seconds, or
     * admission queues saturated.
   * Expose gauges in `/metrics`; add a `readyz_overload.rs` test that drives load until **not ready**.

4. **Structured error surface**

   * Map admission failures to stable JSON problem docs:
     `429 Too Many Requests` → `{ code:"RATE_LIMITED", retry_after:ms, scope:"global|ip|token" }`
     `413 Payload Too Large` → `{ code:"PAYLOAD_TOO_LARGE", limit:bytes }`
     `415 Unsupported Media Type` → `{ code:"UNSUPPORTED_ENCODING", encoding:"br,gzip", hint:"identity only" }`
   * Ensure `x-request-id`/`x-correlation-id` always present (you already set corr-id).
   * Test both JSON and empty-body fallbacks.

5. **Ablation/perf notes (fast)**

   * Run bench variants to attribute cost per layer; record a tiny table in `PERFORMANCE.md`.
   * Add `profile.bench` tweaks (LTO thin, cgu=1) and disable tracing sinks in bench.

6. **Developer UX**

   * `README.md`: quick start + smoke + bench commands + config sample.
   * `RUNBOOK.md`: common 4xx/5xx, how to interpret metrics, how /readyz flips.

---

# Stretch items (Gold polish / near-term)

* **Policy hook (ron-policy)**: pluggable `PolicyEngine` trait; middleware stub that can `Allow/Deny/Mutate` with reason; counters for allow/deny.
* **Per-route admission**: allow `Router` extensions to tag routes with classes/limits; demonstrate on `/admin/*` vs `/api/*`.
* **Fuzz/property tests**: header parsing (encoding, ranges), large header sets, random content-length inconsistencies.
* **Load gen script** (wrk2 or bombardier) to validate overload & readiness flip behavior end-to-end (sockets).
* **Observability polish**: sampling tracing spans for slow requests (`> p99`), include corr-id.

---

# Concrete tasks you can do immediately

1. **Config & metrics scaffolds**

   * Add fields to `Config` for quotas/fair-queue/body_caps/decompress.
   * Thread config into middleware `layer()` constructors.
   * Add Prometheus counters/histograms listed above; export.

2. **Ready gate logic**

   * In your health/ready module, watch moving counters (429/503) + in-flight gauge; if thresholds trip for N seconds → `ready=false`.

3. **Error mappers**

   * Centralize `IntoResponse` mappers for admission errors to JSON problems (include `x-request-id` and `retry-after` when relevant).

4. **Docs**

   * Write/commit `CONFIG.md`, `PERFORMANCE.md` (paste your bench numbers), update `README.md` quick-start.

---

# Updated progress bar (my call)

* ✅ Boot/admin plane/metrics: **Done**
* ✅ Admission core (quotas/fair-queue) + tests: **Done**
* ✅ Body/decompress guards + tests: **Done**
* ✅ Smoke + bench harness: **Done**
* ▶️ Config surface (TOML) for admission: **In progress**
* ▶️ Metrics granularity: **In progress**
* ▶️ Readiness degradation on overload: **In progress**
* ▶️ Structured error JSON: **In progress**
* ◻️ Policy hooks / per-route admission: **Planned**
* ◻️ Fuzz/property/load tests: **Planned**
* ◻️ Docs polish (CONFIG/RUNBOOK/PERF): **Planned**

**Beta ETA confidence:** with the checklist above, you’re roughly **70% complete** toward Omnigate Beta. Knock out config + metrics + readiness + error surface, and you can stamp it. After that, we can thread in ron-policy and do the perf ablation write-up.


### END NOTE - OCTOBER 31 2025 - 11:44 CST






### BEGIN NOTE - OCTOBER 31 2025 - 16:15 CST
---

# Omnigate — Carry-Over Notes (Beta push)

## Status snapshot (today)

* **Binary + binds**

  * API: `127.0.0.1:5305`
  * Admin/metrics: `127.0.0.1:9605`
  * Root and ops aliases:

    * `/healthz` and `/readyz` at **root** (API port)
    * `/ops/healthz`, `/ops/readyz`, `/ops/version` on API port (namespaced)
* **Health**: `GET /healthz → {"omnigate":true}` ✅
* **Readiness**:

  * Truthful gate returns `503` until gates are set (conservative default) ✅
  * **Dev override** implemented (reads `OMNIGATE_DEV_READY` at startup; returns `200`), see “Readiness” below
* **Metrics**: `/metrics` exporting baseline + kernel series (e.g., `amnesia_mode`, bus histos) ✅
* **Middleware stack (ingress)** (cheap → expensive):

  * corr-id → classify → **decompress guard** → **body caps** → slow-loris placeholder
  * quota/fair-queue scaffolding in place (compile-clean), basic global quota guard available
* **Error surface**

  * `crate::errors::{GateError, Problem}` → stable JSON problem docs
  * Admission guards and policy stubs map to correct status codes
* **Downstream (egress)**

  * `downstream/` filled with compiling clients and shared helpers:

    * `index_client.rs`, `storage_client.rs`, `mailbox_client.rs`, `dht_client.rs`
    * `retry.rs` (full-jitter backoff), `error.rs` (small taxonomy), `latency.rs`, `hedge.rs`
    * All use `reqwest` (rustls native roots), finite timeouts, and simple retry policy
* **Bench (in-process, no sockets)**

  * `GET /v1/ping` through full middleware stack:

    * **10.63–10.73 µs**/req ⇒ **~93–94k rps** single stream ✅
    * After root-level bench profile (`lto=thin`, `codegen-units=1`) ✅
* **Smoke**

  * `crates/omnigate/scripts/smoke_omnigate.sh`: boots, checks `/healthz`, `/metrics`, `/`=404, `/readyz`=200|503
  * Currently shows `503` on `/readyz` (truthful default) and “ok” ✅
* **Config**

  * Strongly-typed `Config` with `server`, `oap`, `admission`, `policy`, `readiness` structs
  * Env overrides for binds; TOML loader with defaults and validation; 1 MiB OAP frame cap enforced
* **Observability**

  * HTTP tracing layer + Prometheus exporter
  * Admin plane served via `ron-kernel::Metrics::serve(...)`
* **Tests**

  * Admission/guards tests compile; metrics contract test expects fixture (add file to pass)
* **Drifts fixed this session**

  * Replaced stale `cfg.admin.bind`/`cfg.bootstrap` with `cfg.server.metrics_addr`
  * Added missing error mappers; re-export path normalized (`crate::errors::GateError`)
  * Fixed `GlobalLimiter::new(...)` call site
  * Root `/readyz` + `/healthz` routes added; shared handlers for root and ops
  * Workspace `Cargo.toml` profiles moved to **root** (bench perf improved)

---

## What we’ve validated with commands

* Build/test/bench:

  ```
  cargo build -p omnigate
  cargo test  -p omnigate
  cargo bench -p omnigate --bench middleware_ping
  ```
* Smoke:

  ```
  crates/omnigate/scripts/smoke_omnigate.sh
  ```
* Bench profile confirmed active from **workspace root** (`[profile.bench]`).

---

## Readiness behavior & the dev-override

* **Truthful** readiness (`/readyz`) is **503** until all “ready gates” are flipped (listeners bound, queues ok, shed rate ok, etc.). This is deliberate for production safety.
* **Dev override**:

  * Implemented in `App::build`: reads `OMNIGATE_DEV_READY` **once at startup** and carries it in `AdminState`.
  * If `true`, both **`/readyz`** and **`/ops/readyz`** return **`200`** with body `ready (dev override)`.

> Your smoke output still shows `503` despite `OMNIGATE_DEV_READY=1 ...`. This means the env var likely isn’t reaching the spawned binary from the script (not exported, or script spawns the binary without inheriting env). See **Fix: smoke script** below.

---

## Performance snapshot (record these)

* **After root profile tuning**: 10.63–10.73 µs per request, ~93–94k rps (in-process).
* Normal variance: ±0.3–0.6 µs depending on background load.
* Next: sockets path (wrk/bombardier), overload flip validation.

---

## What’s left for **Beta** (actionable checklist)

1. **Finish config surface & validation**

   * Quotas: global and per-IP (enable flag, qps, burst).
   * Fair-queue weights (anon/auth/admin), max-inflight.
   * Body caps (`max_content_length`, `reject_on_missing_length`).
   * Decompression guard (allowlist; deny stacked encodings).
   * Example `configs/omnigate.toml` + `CONFIG.md` documenting all keys.

2. **Metrics you act on**

   * `http_requests_total{method,route,status}`
   * `request_latency_seconds{method,route}` histogram
   * `admission_quota_exhausted_total{scope=global|ip|token}`
   * `admission_fair_queue_events_total{event=enqueued|dropped}`
   * `body_reject_total{reason=oversize|missing_len}`
   * `decompress_reject_total{reason=unknown|stacked}`
   * **Contract test** asserting presence/format (fixture or live scrape).

3. **Readiness / overload gate wiring**

   * Flip to **not ready** when:

     * `in_flight > threshold` (gauge),
     * sustained `429|503` rate > threshold over N seconds,
     * admission queues saturated.
   * Expose gauges/counters backing these decisions; add a test that drives load until `503`, then recovers.

4. **Structured JSON errors everywhere**

   * Ensure admission & guard failures respond with JSON problem docs (current `GateError` covers it).
   * Always include `x-request-id` / `x-correlation-id` propagation.
   * Add small tests for 413/415/429 shapes.

5. **Docs & DX**

   * `README.md`: quick start, config sample, smoke/bench commands.
   * `PERFORMANCE.md`: current bench numbers, ablation notes (see Stretch).
   * `RUNBOOK.md`: what 4xx/5xx mean, how readiness flips, what metrics to watch.

6. **Tests**

   * Add missing metrics fixture to satisfy `metrics_contract.rs`.
   * Admission quota unit tests (happy path, exhausted).
   * Decompression + body cap matrix tests.

> With the above, Omnigate is **Beta-complete** for RON-CORE.

---

## Stretch / Gold polish (near-term)

* **Policy middleware hook** (`ron-policy`)

  * `PolicyEngine` trait with `Allow | Deny {reason,status} | Mutate`.
  * `policy_middleware_shortcircuits_total{status}` counter.
* **Per-route admission classes**

  * Tag routes with class/weights/limits; different limits for `/ops/*` vs `/api/*`.
* **Fuzz/property testing**

  * Header parsing (encodings), content-length inconsistencies, large header sets.
* **Sockets load harness**

  * `bombardier`/`wrk` script to validate overload → readiness 503, then recover.
* **Observability**

  * Sampled spans for slow requests (`>p99`), corr-id in every event.

---

## Known pitfalls & how we fixed them

* ❌ **Wrong config paths** (`cfg.admin` / `cfg.bootstrap`) → ✅ replaced with `cfg.server.metrics_addr` and removed deprecated fields.
* ❌ **Moved value use** (`cfg.server` moved into serve, then logged) → ✅ take local `server_cfg`, copy `metrics_addr` before move.
* ❌ **Missing `IntoResponse` import** for tuple responses → ✅ added `use axum::response::IntoResponse`.
* ❌ **`GlobalLimiter.new` typo** → ✅ `GlobalLimiter::new`.
* ❌ **Profiles in crate Cargo.toml** → ✅ moved to **workspace root**.
* ❌ **Metrics contract test missing fixture** → ✅ add `testing/fixtures/metrics.sample.txt` (below).

---

## Files to add (tests & smoke)

### 1) Add the metrics fixture so the test passes

**Create:** `crates/omnigate/testing/fixtures/metrics.sample.txt`

```
# HELP http_requests_total Requests by route/method/status
# TYPE http_requests_total counter
http_requests_total{route="/ping",method="GET",status="200"} 42

# HELP request_latency_seconds Request latency by route/method
# TYPE request_latency_seconds histogram
request_latency_seconds_bucket{route="/ping",method="GET",le="0.005"} 10
request_latency_seconds_bucket{route="/ping",method="GET",le="0.01"} 20
request_latency_seconds_bucket{route="/ping",method="GET",le="0.025"} 30
request_latency_seconds_sum{route="/ping",method="GET"} 0.123
request_latency_seconds_count{route="/ping",method="GET"} 30

# HELP admission_quota_exhausted_total Quota rejections by scope
# TYPE admission_quota_exhausted_total counter
admission_quota_exhausted_total{scope="global"} 0

# HELP admission_fair_queue_events_total Fair queue events by type
# TYPE admission_fair_queue_events_total counter
admission_fair_queue_events_total{event="enqueued"} 0

# HELP body_reject_total Body rejections by reason
# TYPE body_reject_total counter
body_reject_total{reason="oversize"} 0

# HELP decompress_reject_total Decompression guard rejections
# TYPE decompress_reject_total counter
decompress_reject_total{reason="unknown"} 0

# HELP policy_middleware_shortcircuits_total Requests denied by policy middleware
# TYPE policy_middleware_shortcircuits_total counter
policy_middleware_shortcircuits_total{status="403"} 0
```

Run:

```
cargo test -p omnigate
```

---

### 2) Fix: smoke script didn’t propagate env to the child process

Even when you run `OMNIGATE_DEV_READY=1 crates/omnigate/scripts/smoke_omnigate.sh`, some shells don’t export that to subshells unless the script itself exports it before spawning the binary. Let’s make the script bullet-proof and also try both `/readyz` and `/ops/readyz`.

**Paste-ready replacement:** `crates/omnigate/scripts/smoke_omnigate.sh`

```
#!/usr/bin/env bash
set -euo pipefail

# Colors
G="\033[0;32m"; Y="\033[1;33m"; R="\033[0;31m"; Z="\033[0m"

echo -e "${Y}fmt + clippy + build…${Z}"
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo build -p omnigate

API_ADDR="${API_ADDR:-127.0.0.1:5305}"
LOG="/tmp/omnigate.log"

# Ensure the env var is exported for the spawned process
export OMNIGATE_DEV_READY="${OMNIGATE_DEV_READY:-}"

echo -e "${Y}starting omnigate at ${API_ADDR} (logs: ${LOG})…${Z}"
BIN="target/debug/omnigate"
# Start in background with inherited environment
"${BIN}" > "${LOG}" 2>&1 &
PID=$!

cleanup() {
  echo -e "${Y}stopping omnigate (pid=${PID})…${Z}"
  kill "${PID}" >/dev/null 2>&1 || true
  wait "${PID}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Wait for /healthz
echo "waiting for /healthz…"
set +e
for i in {1..50}; do
  curl -fsS "http://${API_ADDR}/healthz" >/dev/null && break
  sleep 0.2
done
set -e

echo "-- /healthz"
curl -fsS "http://${API_ADDR}/healthz"
echo

echo "-- /metrics (head)"
curl -fsS "http://${API_ADDR}/metrics" | head -n 20

echo "-- / (expect 404)"
CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://${API_ADDR}/")
if [ "${CODE}" = "404" ]; then
  echo -e "✅ 404 ok"
else
  echo -e "❌ expected 404, got ${CODE}"
  exit 1
fi

check_ready() {
  local path="$1"
  local body status
  status=$(curl -s -o /dev/stderr -w "%{http_code}" "http://${API_ADDR}${path}")
  body=$(curl -s "http://${API_ADDR}${path}" || true)
  echo "readyz status (${path}): ${status}"
  if [ "${status}" = "200" ]; then
    echo -e "${G}✅ ready (body: ${body})${Z}"
    return 0
  else
    echo -e "${Y}ℹ️ not ready (status ${status}, body: ${body})${Z}"
    return 1
  fi
}

echo "-- /readyz (expect 200 with OMNIGATE_DEV_READY=1, else 503)"
if ! check_ready "/readyz"; then
  echo "-- trying /ops/readyz as fallback"
  check_ready "/ops/readyz" || true
fi

echo -e "${G}✅ smoke ok${Z}"
```

Run:

```
chmod +x crates/omnigate/scripts/smoke_omnigate.sh
OMNIGATE_DEV_READY=1 crates/omnigate/scripts/smoke_omnigate.sh
```

You should now see `readyz status: 200` with the “ready (dev override)” body. If it’s still 503, the binary isn’t seeing the env (very rare after this change). In that case we can add a temporary CLI flag (e.g., `--dev-ready`) read by `main.rs`, but the `export` above usually resolves it.

---

## What’s next (fast path to Beta)

1. **Commit**: all changes above + fixture + updated smoke script.
2. **Finish metrics series** + contract test regexes.
3. **Wire readiness thresholds** and flip logic; add a quick test that drives the flip.
4. **Docs**: `CONFIG.md`, `RUNBOOK.md`, and `PERFORMANCE.md` with today’s numbers.
5. **Optional**: sockets bench harness (`brew install bombardier` or `wrk`).



### END NOTE - OCTOBER 31 2025 - 16:15 CST





### BEGIN NOTE - OCTOBER 31 2025 - 18:42 CST

# Omnigate — Carry-Over Notes (Beta Push)

## 1) Current Snapshot (Oct 31, 2025, America/Chicago)

* **Status:** Omnigate compiles most of the way; HTTP server + admin plane boot and respond; readiness truth test passes when config is provided. Runtime (supervisor/worker) scaffolding exists and mostly wires up, but the **DTO ↔ route** mismatch is now the primary build blocker.
* **Latest blocking errors (from your log):**

  * `routes/ops.rs` expects `VersionResponse { name, version: &str, commit }` but our DTO has `{ version: String, git: Option<String> }`.
  * `routes/v1/mod.rs` expects `PingResponse { pong, ts_ms }` but our DTO is `{ ok }`.
  * Minor type issue on `version` expects `String` not `&str`.
* **What already works (previous runs):**

  * `ready_truth` test: ✅ passed after we fixed `--config` handling + readiness gating.
  * `smoke_omnigate.sh`: ✅ reached `/healthz`, `/readyz`, `/ops/readyz`, returned 404 on `/`, and surfaced Prometheus metrics (e.g., `amnesia_mode`).
  * Config gating: readiness flips once `--config` is supplied (verified in both test + smoke).

---

## 2) Accomplishments to Date

1. **Admin/API surfaces:**

   * `/healthz` (admin truth), `/readyz` (API truth), `/ops/readyz` alias.
   * 404 on `/` confirmed.
   * Prometheus metrics endpoint serving (`amnesia_mode`, bus histograms placeholders).

2. **Truthful readiness gate:**

   * `ready_truth` integration test verifies readiness flips only after config is loaded.
   * Smoke script launches the binary with `--config` so the **config gate** trips correctly.

3. **Config wiring:**

   * TOML at `crates/omnigate/configs/omnigate.toml`.
   * Addresses observed:

     * `bind = "127.0.0.1:5305"` (API)
     * `metrics_addr = "127.0.0.1:9605"` (admin/metrics)

4. **Runtime scaffolding in place:**

   * `runtime/` folder with `channels.rs`, `shutdown.rs`, `supervisor.rs`, `worker.rs`, `sample.rs`.
   * Broadcast/mpsc channels factory; graceful shutdown token; supervisor loop; spawn worker pattern; sample `TickWorker` stub.

5. **DTO scaffolding + tests introduced:**

   * `types/dto.rs` with `VersionResponse { version:String, git:Option<String> }` and `PingResponse { ok:bool }`.
   * Added serialization round-trips in `dto_serialization.rs` (blocked now only because route structs didn’t match DTOs).

6. **Observability:**

   * Tracing init.
   * Prometheus metrics served.
   * Basic metrics names present (placeholders already visible in output).

---

## 3) How to Run (Commands)

### A. Fast local build & smoke

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo build -p omnigate

# Spins up the binary with --config, waits for /healthz, /metrics, /readyz
crates/omnigate/scripts/smoke_omnigate.sh
```

**What “good” looks like (seen before):**

* `/healthz` JSON: `{"config":true,"omnigate":true}`
* `/readyz` JSON: `{"ready":true}`
* `/` returns 404 (expected)
* `/metrics` includes `# HELP amnesia_mode ...` etc.

### B. Tests (when the DTO mismatch is fixed)

```bash
cargo test -p omnigate --test ready_truth --test dto_serialization --test zk_receipts
```

**Notes:**

* `ready_truth` already passed previously once config handling was fixed.
* `dto_serialization` will pass after we align routes ↔ DTOs (see Fix Plan below).
* `zk_receipts` is a placeholder; fine to keep as stub with minimal assertions until zk code lands.

---

## 4) Known Pitfalls We Already Solved (Keep!)

* **“spawn omnigate: NotFound”** in `ready_truth` → we updated test to resolve bin path and pass `--config` so readiness flips.
* **`cfg(build_git_hash)` unexpected cfg** → either use a Cargo feature or emit `cargo:rustc-check-cfg` in `build.rs`. We moved to the simple optional `git` field rather than conditional cfgs in the handler.
* **Missing `async_trait`** and dyn-compat errors → we added `async-trait` dependency and adjusted trait objects to not require async in trait vtables (run method remains async but trait is used behind `Arc<dyn Worker>` via `async_trait`).
* **Broadcast/mpsc constructor mismatch** → supervisor now calls `mk_supervisor_bus(worker_backlog)` and receives `(tx_cmd, _rx_cmd, up_tx, up_rx)` as designed.
* **Shutdown API shape** → moved from `shutdown.shutdown()` to `shutdown.cancel()` and awaiting `shutdown.cancelled()` in tasks.

---

## 5) Current Blocking Issues & Minimal Fix Plan

You have **two consistent “shape” mismatches** between handlers and DTOs. Pick **Option A (recommended)** to keep our clean DTOs and update the routes; or **Option B** to expand DTOs to match the current handlers.

### Option A — Change the routes to match existing DTOs (smaller change)

1. **`routes/ops.rs` – version endpoint**
   Use our DTO:

```rust
use crate::types::VersionResponse;

// ...
let commit: Option<String> = option_env!("GIT_COMMIT").map(|s| s.to_string());

Json(VersionResponse {
    version: env!("CARGO_PKG_VERSION").to_string(),
    git: commit, // previously `commit`
})
```

Remove non-existent fields (`name`, `commit`), convert `&str` → `String`.

2. **`routes/v1/mod.rs` – ping endpoint**
   Use our DTO:

```rust
use crate::types::PingResponse;

// ...
Json(PingResponse { ok: true })
```

Remove `pong` and `ts_ms` fields (or, if you want `ts_ms`, add it to DTO below).

**Pros:** small diff; keeps DTOs tight.
**Cons:** if you *wanted* `name/commit/pong/ts_ms`, you lose them unless you expand DTOs.

### Option B — Expand DTOs to include fields handlers want

If you prefer the richer wire schema:

**`types/dto.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionResponse {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    // keep legacy alias for build metadata if you still want it:
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PingResponse {
    pub pong: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts_ms: Option<u64>,
}
```

Then adjust handlers to fill `name = "omnigate"`, `version = CARGO_PKG_VERSION.to_string()`, `commit = option_env!("GIT_COMMIT").map(str::to_string)`, and `ts_ms = Some(now_ms)`.

**Pros:** richer API, matches earlier expectation.
**Cons:** Slightly bigger change set; must update `dto_serialization` test strings.

**Either option** will unblock the five compile errors you hit.

---

## 6) Runtime Track — What’s left to reach “Beta”

* [ ] **Finish runtime glue** (lightweight, production-safe):

  * `channels.rs`: DONE (mk_supervisor_bus returning `(broadcast_tx, broadcast_rx, mpsc_tx, mpsc_rx)`).
  * `shutdown.rs`: DONE (token + trigger; `cancel()`; `cancelled().await`).
  * `worker.rs`: trait + `spawn_worker()` wrapper (already there; confirm `async_trait` import, no needless borrows).
  * `supervisor.rs`: DONE (spawns workers, relays Stop, drains joins).
  * `sample.rs` (`TickWorker`): keep minimal; ensure it compiles and responds to Stop + shutdown token.

* [ ] **Policy/zk stubs (keep minimal for beta)**

  * `zk/receipts.rs`, `zk/no_mutate.rs`: expose DTOs/traits but gated behind feature flags like `zk` so base build stays slim. Tests can be smoke-level only (e.g., type compiles, simple serialization).

* [ ] **DTO test alignment**

  * `dto_serialization.rs`: update JSON fixtures to the chosen schema (Option A or B).
  * Add round-trip serde tests for `VersionResponse` and `PingResponse`.

* [ ] **Observability**

  * Ensure `/metrics` exposes basic HTTP timing histograms for `/healthz`, `/readyz`, `/ops/*`.
  * Wire `amnesia_mode` gauge to actual config.

* [ ] **Readiness truth table**

  * Already proven for config gate. Add future gates TODOs (e.g., listener bound, bus ready, policy loaded). For beta, config + listener bound is enough.

* [ ] **Docs & scripts**

  * `README.md`: quickstart, routes table, ports, env vars.
  * `RUNBOOK.md`: smoke, rotate logs path, graceful shutdown.
  * `CONFIG.md`: document `bind`, `metrics_addr`, and how `--config` flips readiness.
  * Ensure `scripts/smoke_omnigate.sh` is in CI.

* [ ] **CI Gating**

  * `cargo fmt`, `clippy -D warnings`, `cargo test` for the three tests.
  * Optionally add `cargo deny` and `cargo public-api` if you’ve standardized those across the workspace.

---

## 7) Benchmarks (none required for Beta, nice to have)

Omnigate isn’t a hot path crate (it’s a gateway/edge). If you want a quick perf pulse:

* Add a tiny `criterion` route bench (in-process) for `GET /ping` using axum router, similar to your omnigate middleware bench you ran earlier for `/ping`. Track **p50/p90 latency** and allocations. This is optional for Beta.

---

## 8) Acceptance Checklist for **Omnigate Beta**

* [ ] **Builds cleanly** with `-D warnings`.
* [ ] **Smoke passes**: `/healthz`, `/readyz`, `/ops/readyz`, `/`→404, `/metrics`.
* [ ] **Tests green**: `ready_truth`, `dto_serialization` (updated), `zk_receipts` (stub ok).
* [ ] **Config gate** proven (already done).
* [ ] **Graceful shutdown**: ctrl-C → Stop broadcast → workers exit → server aborted/closed.
* [ ] **Docs**: README, CONFIG, RUNBOOK updated.
* [ ] **CI**: fmt, clippy, test in pipeline.

---

## 9) Exact Next Edits to Unblock You (short todo)

Choose one of these paths and apply:

**Path A (short): Update handlers to match DTOs**

* Edit `crates/omnigate/src/routes/ops.rs`:

  * Remove `name` and `commit` fields.
  * Set `version: env!("CARGO_PKG_VERSION").to_string()`.
  * Set `git: option_env!("GIT_COMMIT").map(str::to_string)`.

* Edit `crates/omnigate/src/routes/v1/mod.rs`:

  * Return `Json(PingResponse { ok: true })`.

* Re-run:

  ```bash
  cargo fmt -p omnigate
  cargo clippy -p omnigate --no-deps -- -D warnings
  cargo build -p omnigate
  crates/omnigate/scripts/smoke_omnigate.sh
  cargo test -p omnigate --test ready_truth --test dto_serialization --test zk_receipts
  ```

**Path B (expand DTOs)**

* Expand `VersionResponse` and `PingResponse` as described above, keep route code as-is but change `version.to_string()`, and adjust `dto_serialization.rs` fixtures.

Either path is fine; **Path A** is the fastest unblock.

---

## 10) Reference: Ports & Files

* **API bind:** `127.0.0.1:5305`
* **Admin/metrics:** `127.0.0.1:9605`
* **Config file:** `crates/omnigate/configs/omnigate.toml`
* **Logs (smoke):** `/tmp/omnigate.log`
* **Key source files:**

  * `src/routes/ops.rs`, `src/routes/v1/mod.rs`
  * `src/types/dto.rs`, `src/types/mod.rs`
  * `src/runtime/{channels.rs,shutdown.rs,supervisor.rs,worker.rs,sample.rs}`
  * `src/zk/{receipts.rs,no_mutate.rs}` (stubs OK for beta)

---

## 11) Known Nice-to-Haves (Post-Beta)

* Feature flag `runtime-sample` so `TickWorker` is dev-only.
* Add `/version` route that returns full `VersionResponse` (and optionally `name`) for UI clients.
* Emit a compact git hash at build via `build.rs` (sets `GIT_COMMIT` env var) and remove `cfg(build_git_hash)` usage entirely.
* Add a `/readyz/details` endpoint listing the gates (`config_loaded`, `listener_bound`, etc.) for debugging.
* Minimal rate-limit middleware on `/ops/*` (tower-http) if you want extra safety.



### END NOTE - OCTOBER 31 2025 - 18:42 CST



### BEGIN NOTE - OCTOBER 31 2025 - 22:53 CST


# Omnigate — Carry-Over Notes (Beta Push)

## 1) Snapshot: where we are (today)

* **Binary + App skeleton is up**: `main.rs` loads config (file/env), builds the app, starts the admin plane via `ron-kernel::Metrics::serve`, serves the Axum API listener, and runs a supervised worker (TickWorker). Health and readiness endpoints are wired truthfully (with an optional **dev override**) and **amnesia mode is plumbed through to metrics**. 
* **Admin plane**: `/healthz`, `/readyz` and Prometheus `/metrics` are served by the kernel; omnigate flips health/config gates and exposes gauges/counters for readiness and admission. 
* **Config system**: typed `Config` with defaults + TOML file loader + env-var overrides; validation stub in place; OAP limits enforced (1 MiB max frame). 
* **Middleware stack**: Corr-ID, response classify (NOP), decompression guard, body caps (preflight `Content-Length` + Axum body limit), slow-loris placeholder; logging/HTTP tracing via `tower_http::trace`. 
* **Admission**: `fair_queue` and `quotas` implemented as layers (return 503 / 429 respectively) but **not yet attached** to the top-level router (current `admission::attach` is a no-op shim). 
* **Readiness policy**: in-crate `readiness::policy::ReadyPolicy` computes readiness from inflight, 429/503 error rate and queue saturation, with anti-flap and RED gauges/counters. **Not yet wired to live sources** (fair queue/quotas need to update the gauges). 
* **Policy middleware**: present and integrated with `ron-policy` Evaluator; honors `policy.fail_mode` (`deny|allow`) and emits labeled short-circuit metrics (403/451/503). **State wiring (AppState with evaluator)** is referenced but not finalized. 
* **Downstream clients**: thin reqwest clients for **index**, **storage**, **mailbox**, **dht** with retry/hedging helpers and a registry of service endpoints (localhost defaults). 
* **Routes**: `v1::ping` is live; ops endpoints expose versionz; several v1 feature routes (objects/index/mailbox/dht/facet/*) are stubs. 
* **Metrics**: stable metric names for HTTP/RED + admission + policy; plus the **amnesia_mode** gauge we set at boot. 
* **Runtime**: supervisor/workers with cooperative shutdown primitives (CancellationToken) and a sample TickWorker. 
* **Benches & scripts**:

  * `benches/middleware_ping.rs` runs an in-process GET `/ping` microbench of the middleware stack (we saw ~10.8 µs on your box). 
  * `scripts/smoke_omnigate.sh` and **full-fat** `scripts/sanity_omnigate.sh` (does fmt+clippy+build+select tests, boots the service, asserts `/healthz`, `/readyz`, checks amnesia gauge, tries quota/413/415 paths, etc.). 

**Rough Beta completion estimate right now**: **~80–85%**. We landed amnesia + truthful readiness skeleton + admission/policy layers and clients; the remaining work is mostly wiring and filling stubs (auth, readiness wiring, attach admission, v1 routes, tests/fuzz/CI polish). (You previously called it ~90% when we were treating readiness as “dev override”; counting truthful gating + auth + routes pushes a bit of work back onto the stack.)

---

## 2) How to run & verify (smoke/sanity/benches)

### Quick smoke

```bash
crates/omnigate/scripts/smoke_omnigate.sh
# Uses configs/omnigate.toml by default, builds, starts, curls:
# - GET /healthz (expect 200)
# - GET /metrics (admin plane, expect 200)
# - GET / (expect 404)
# - GET /readyz (truthful; may be non-200 unless gates flip)
```



**Tip — dev-readiness override**
Truthful `/readyz` requires gates; for local development you can force 200:

```bash
OMNIGATE_DEV_READY=1 crates/omnigate/scripts/sanity_omnigate.sh
```

The script:

* builds + runs **selected tests** (`dto_serialization`, `ready_truth`, `zk_receipts`),
* boots omnigate with `OMNIGATE_AMNESIA=on`,
* asserts `/healthz`, `/readyz`, `/metrics`,
* checks **amnesia_mode==1** and probes 413/415 and 429/503 paths best-effort, printing related metrics. 

### Manual run

```bash
cargo run -p omnigate -- --config crates/omnigate/configs/omnigate.toml
curl -s http://127.0.0.1:5305/healthz
curl -s http://127.0.0.1:9605/metrics | head
```

Config knobs: `OMNIGATE_BIND`, `OMNIGATE_METRICS_ADDR`, `OMNIGATE_AMNESIA` env overrides are honored. 

### Microbench

```bash
cargo bench -p omnigate --bench middleware_ping
# Measures in-process GET /v1/ping across the middleware stack.
```

Interpretation: end-to-end middleware + router overhead (no sockets). Use this as perf smoke; **don’t** over-index it vs. networked p50/p95. 

---

## 3) What’s left to reach Beta (concrete, bite-sized tasks)

I grouped tasks so we can parallelize. Each item points to where to touch.

### A) Admission & Readiness wiring (truthful gates)

1. **Attach admission layers** in the top-level stack:

   * Update `middleware::apply` (or `lib.rs` pre-apply) to call:

     * `admission::quotas::attach(...)` (global 429) and
     * `admission::fair_queue::attach(...)` (503 shed + `x-omnigate-cap` header).
       Then remove the no-op in `admission::attach`. Ensure types satisfy `Router::layer` bounds. 
2. **Drive readiness gauges**:

   * When quota rejects, increment `ADMISSION_QUOTA_EXHAUSTED_TOTAL` (already done) **and** update rolling 429/503 error ratio feeding `ReadyPolicy`.
   * When fair-queue drops or saturates, set `READY_QUEUE_SATURATED=1` and reflect current in-flight in `READY_INFLIGHT_CURRENT` (the fair-queue `Gate` already tracks `in_flight`; expose the value into the policy). 
3. **Expose ReadyPolicy to /readyz**:

   * Either: integrate `ReadyPolicy::ready()` into the existing kernel readiness gate (via a small adapter), or surface a **service-local** `GET /readyz` that reads `ReadyPolicy` and returns 200/503 accordingly. The skeleton is ready; just bridge the state. 
4. **Config plumbing**:

   * Read `readiness.{max_inflight_threshold,error_rate_429_503_pct,window_secs,hold_for_secs}` from TOML into `ReadyCfg`. Instantiate one `ReadyPolicy` in `App::build` and share it via `State/Arc`. 

**Definition of Done (Admission/Ready):**

* `sanity_omnigate.sh` shows **429s** during the burst step and non-zero `admission_quota_exhausted_total`.
* Toggling queue headroom triggers `READY_QUEUE_SATURATED=1`.
* `/readyz` truthfully returns **503** when inflight > threshold or error rate ≥ budget, and flips back after hold period. 

### B) Auth / Policy surface

5. **Auth stubs**: fill `src/auth/{capability.rs,passport_client.rs,revocation.rs,mod.rs}` with minimal DTOs + a Passport verifier stub (JWT/Macaroon-ish) and an in-memory revocation cache (just shape it; no crypto yet). Expose a `Capability` extracted into `Request extensions`. 
6. **Policy wiring**:

   * Create `AppState` (tenant/region/tags, `ron_policy::Evaluator`) and attach to router via `.with_state(Arc<AppState>)`.
   * Load `configs/policy.bundle.json` at boot; bump `POLICY_BUNDLE_LOADED_TOTAL` on success.
   * Place `PolicyLayer` **after** Corr-ID and **before** business routes to short-circuit early. Honor `policy.fail_mode`. 

**DoD (Auth/Policy):**

* Supplying a deny rule in the bundle yields **403** (or **451** when `reason="LEGAL"`) and increments `policy_middleware_shortcircuits_total` with the right label.
* Missing evaluator with `fail_mode="deny"` returns 403. 

### C) v1 routes (fill the stubs)

7. **/v1/objects**: proxy a GET to `StorageClient` for a health touch (and later, range reads using `testing/vectors/omnigate/range_read.json`). Start minimal: `GET /v1/objects/healthz -> storage /healthz` passthrough. 
8. **/v1/index**: proxy a GET to `IndexClient /healthz` and a small `POST /search` echo to prove JSON path. 
9. **/v1/mailbox`&`/v1/dht`**: similar health hooks using their clients; keep shapes DTO-stable. 
10. **/v1/facet/{feed,media,graph}**: define read-only placeholder endpoints returning `{ok:true}` with tags/caps surfaced in headers for policy testing; expand later with hydration. 

**DoD (Routes):**

* All above paths respond 200 JSON; failures from downstream map to a stable envelope via `errors::http_map`. Add a tiny mapper in each client path to `GateError::{PolicyError,Degraded}` if needed. 

### D) Hydration skeleton

11. Fill `hydration/{planner.rs,compose.rs,mod.rs}` with:

* `Plan { steps: Vec<Step> }` and `compose(Plan, Clients) -> Json` that fans out GETs to index/storage and assembles a response (kept tiny; used for a bench later).
* Add `benches/hydration.rs` to measure p50/p95 under mocked clients. 

### E) Observability polish

12. Add basic fields to `observability/logging.rs` and `tracing_spans.rs` (span per request, corr-id, route label). Ensure **bounded label cardinality** (route templates, not full URLs). 
13. Emit `http_requests_total{route,method,status}` and `request_latency_seconds{route,method}` in one place (simple tower layer or per-route wrapper). Names are already reserved. 

### F) PQ hooks (optional, Beta-nice-to-have)

14. `pq/negotiate.rs` to surface a request/response header indicating PQ-hybrid support (purely informational for now). 

### G) Tests, vectors, fuzz, CI

15. **Hook the existing tests** the sanity script references: ensure they compile & pass (some are placeholders right now). If missing, add minimal versions that validate DTOs and the middleware envelopes. 
16. **Fuzz targets**: wire `fuzz_targets/{decompress_guard.rs,headers.rs,capability.rs}` to exercise header parsers and the guard decisions (just glue with `cargo-fuzz` scaffolding). 
17. CI scripts in `.github/workflows/*` are placeholders—add actual steps:

* build + clippy `-D warnings`,
* run test matrix x86_64/aarch64,
* (later) perf regression job that runs the bench and compares to `testing/performance/baselines/*.json`. 

---

## 4) Acceptance checklist for Beta

* **Truthful readiness**: `/readyz` flips to 503 under: (a) inflight > threshold, (b) 429/503 error rate ≥ budget, (c) queue saturated; recovers only after `hold_for`. Gauges/counters reflect transitions. 
* **Admission active**: global quota produces measurable **429s**; fair-queue sheds to **503** with `x-omnigate-cap` indicating caps; related counters increment. 
* **Policy enforced**: evaluator loaded once (counter incremented); deny and legal paths return 403/451 with stable JSON. `fail_mode=deny` path returns 403. 
* **Amnesia telemetry**: `amnesia_mode` gauge == `1` when `cfg.server.amnesia=true` or `OMNIGATE_AMNESIA=on`. 
* **Routes basic set**: `/v1/{ping,objects/*,index/*,mailbox/*,dht/*,facet/*}` respond with DTO-stable shapes and correct error envelopes. 
* **Observability**: `http_requests_total` and `request_latency_seconds` emit with bounded labels; logs include corr-id. 
* **Benches**: `middleware_ping` stable within a tolerance window; (optional) hydration bench lands with a baseline JSON under `testing/performance/baselines/`. 
* **CI green**: fmt, clippy `-D warnings`, unit/integration tests, deny.toml checks pass. 

---

## 5) Gotchas / common issues we already saw

* **“/readyz not 200” confusion**: By design, with **dev override OFF** (`OMNIGATE_DEV_READY` unset/0), `/readyz` will only go 200 when gates deem it ready. Use `OMNIGATE_DEV_READY=1` during local iteration or wire the readiness policy and admission first. The sanity script demonstrates both modes. 
* **Forget to pass `--config`**: The smoke/sanity scripts pass it so the “config loaded” gate flips; if you run the binary manually without `--config`, we still load defaults but you’ll want to confirm `/metrics` binds to the expected admin port. 

---

## 6) Minimal sequence to finish Beta (fast path)

1. Wire **admission layers** into the router and connect them to **ReadyPolicy** updates.
2. Instantiate **ReadyPolicy** from TOML; have `/readyz` call it (no dev override).
3. Fill **Auth** stubs + **AppState** and **PolicyLayer** state injection; load policy bundle; add a single deny rule to test 403/451.
4. Implement small **/v1** passthrough routes (health touches) for storage/index/mailbox/dht; return stable envelopes on failure.
5. Add **observability** counters/histos on requests; confirm bounded labels.
6. Land/enable the tests referenced by `sanity_omnigate.sh`; make the **429 burst** step actually produce 429s.
7. Run **smoke/sanity** clean, then **middleware_ping** bench, capture baseline.

That’s it—once those are green, we can stamp **Beta**.

---

### END NOTE - OCTOBER 31 2025 - 22:53 CST



### BEGIN NOTE - NOVEMBER 1 2025 - 16:17 CST

# Omnigate Carry-Over Notes (Buildable Plan)

## 1) Current status — what already works (✅)

* **Admin plane up**: `/healthz`, `/readyz`, `/versionz` served from the admin/metrics bind; `OMNIGATE_DEV_READY=1` forces 200 on `/readyz`.
* **Middleware stack**: `classify → corr_id → policy → body_caps → decompress_guard → slow_loris` is layered; **policy** is active but safely no-ops unless a policy **bundle** is provided.
* **Policy integration**:

  * `src/lib.rs` loads a `PolicyBundle` from `cfg.policy.bundle_path`, inserts `Extension(Arc<PolicyBundle>)`.
  * `middleware/policy.rs` builds an on-the-fly `Evaluator` from that bundle per request and enforces `Allow / Deny (403/451) / Error (503)`, with metrics via `POLICY_SHORTCIRCUITS_TOTAL`.
* **Admission guards** exist and build clean:

  * `admission/quotas.rs`: global token bucket (hardcoded params, for now).
  * `admission/fair_queue.rs`: hard cap + interactive headroom (hardcoded params, for now).
* **Config model**: `src/config/mod.rs` has a rich `Config` with `admission.{global_quota, ip_quota, fair_queue, body, decompression}` and helpers:

  * `GlobalQuota::params_u32()` → `(qps, burst)`.
  * `FairQueue::hard_and_headroom()` → `(hard, headroom)`.
  * Safe defaults via `#[serde(default)]` and `Default` impls.
* **Routing**: v1 `/ping` responds; facet/index/mailbox/objects/dht stubs compile (health endpoints exist).
* **Observability**: HTTP trace layer present; counters for policy short-circuiting; fair-queue drops and “saturated” gauge wired.

## 2) What’s left to hit “beta” (🎯)

### A. Wire **admission** from **config** (the step you paused on)

Goal: replace hardcoded limits with values from `cfg.admission`.

**Patch plan (file-by-file):**

1. **`crates/omnigate/src/admission/mod.rs`**

   * Replace the current `attach<S>(router: Router<S>) -> Router<S>` with a config-aware variant and keep a thin default wrapper for tests:

   ```rust
   // new
   pub fn attach_with_cfg<S>(router: Router<S>, cfg: &crate::config::Admission) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       // order matters: quotas first, then fair-queue
       let r = quotas::attach_with_cfg(router, &cfg.global_quota);
       let r = fair_queue::attach_with_cfg(r, &cfg.fair_queue);
       r
   }

   // optional shim for legacy/tests
   pub fn attach<S>(router: Router<S>) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       attach_with_cfg(router, &crate::config::Admission::default())
   }
   ```

2. **`crates/omnigate/src/admission/quotas.rs`**

   * Expose a cfg-aware attach and keep the existing one as a shim:

   ```rust
   pub fn attach_with_cfg<S>(router: Router<S>, gq: &crate::config::GlobalQuota) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       let (qps, burst) = gq.params_u32();
       let limiter = GlobalLimiter::new(qps, burst);
       router.layer(from_fn_with_state(limiter, quota_guard))
   }

   // keep for tests/back-compat; calls into attach_with_cfg with defaults
   pub fn attach<S>(router: Router<S>) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       let defaults = crate::config::Admission::default();
       attach_with_cfg(router, &defaults.global_quota)
   }
   ```

3. **`crates/omnigate/src/admission/fair_queue.rs`**

   * Mirror the pattern:

   ```rust
   pub fn attach_with_cfg<S>(router: Router<S>, fq: &crate::config::FairQueue) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       let (hard, headroom) = fq.hard_and_headroom();
       let gate = Arc::new(Gate::new(hard, headroom));
       router.layer(from_fn_with_state(gate, fairness_guard))
   }

   pub fn attach<S>(router: Router<S>) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       let defaults = crate::config::Admission::default();
       attach_with_cfg(router, &defaults.fair_queue)
   }
   ```

4. **`crates/omnigate/src/middleware/mod.rs`**

   * **Stop** attaching admission here (we don’t have `cfg` in this module).
   * Return the router after slow-loris:

   ```rust
   pub fn apply<S>(router: Router<S>) -> Router<S>
   where
       S: Clone + Send + Sync + 'static,
   {
       let router = router
           .layer(classify::layer())
           .layer(corr_id::layer())
           .layer(policy::layer());

       let (preflight_len_guard, default_body_limit) = body_caps::layer();
       router
           .layer(preflight_len_guard)
           .layer(default_body_limit)
           .layer(decompress_guard::layer())
           .layer(slow_loris::layer())
   }
   ```

5. **`crates/omnigate/src/lib.rs`**

   * After building the middleware stack, **now** attach admission using `cfg.admission`:

   ```rust
   // after: let app_router = middleware::apply(app_router).layer(observability::http_trace_layer());
   let app_router = crate::admission::attach_with_cfg(app_router, &cfg.admission);
   ```

> Result: limits for **global QPS/burst** and **fair-queue hard/headroom** flow from `Config`, overridable via file/env, with sane defaults preserved.

---

### B. Read admission/body/decompress from config (small follow-ups)

* **`body_caps` guard**: read `cfg.admission.body.max_content_length` and `reject_on_missing_length`.

  * If the module already returns a `layer()` pair, add `layer_with_cfg(cfg: &Admission)`.
  * If not, add an internal `const DEFAULT_MAX` and replace with values from `cfg`.
* **`decompress_guard`**: read `cfg.admission.decompression.allow` and `deny_stacked`.

  * Deny stacked encodings when configured; keep “identity/gzip” as defaults.

> Placement: These stay inside `middleware::apply` but you’ll need to pass in config. Two options:
>
> 1. Change `middleware::apply(router)` → `apply_with_cfg(router, &cfg)` and use it in `lib.rs`.
> 2. Keep `apply` simple (no cfg) and drive caps/decompress from globals/static defaults.
>    If you want real config wiring (recommended), do (1) now. The pattern is the same as admission.

---

### C. Routes polish (warnings/visibility)

* Ensure reply DTOs exposed by public handlers are **`pub struct`** (you already hit warnings earlier).

  * Files: `routes/v1/{dht.rs,index.rs,mailbox.rs,objects.rs,facet/mod.rs}`.
* Keep `/v1/ping` for smoke, plus `healthz` under each service stub for parity.

---

### D. Readiness wiring (optional now, nice before beta)

* Gate readiness with moving window of 429/503 and inflight caps using your `config.readiness`.
* You already set `ready.set_config_loaded(true)`; add setters for “listeners bound” or “shed rate ok” as you wire more signals.
* Expose gauges:

  * `READY_QUEUE_SATURATED` (already used).
  * Counters for `admission_quota_exhausted_total{scope="global"|"ip"}` (global is wired).
  * Optionally: rolling error rate sampler to inform `/readyz`.

---

### E. Policy evolution guardrails

* **Context shape drift**: current middleware builds `Context{ now_ms, method, region, tags, body_bytes }`. If the ron-policy API evolves, this is the single place to adapt.
* **Bundle hot-reload** (post-beta): watch `bundle_path` and swap `Extension(Arc<PolicyBundle>)` atomically; current design already avoids ‘static lifetime issues by constructing `Evaluator` per request.

---

### F. Observability & metrics

* Verify these metric names exist (or add them if missing):

  * `POLICY_SHORTCIRCUITS_TOTAL{code}` (incremented on 403/451/503).
  * `FAIR_Q_EVENTS_TOTAL{event="dropped"}` (already used).
  * `ADMISSION_QUOTA_EXHAUSTED_TOTAL{scope="global"}` (already used).
  * `READY_QUEUE_SATURATED` (gauge; already used).
  * Optional: `http_request_duration_seconds` histogram via your trace layer/classifier.

---

## 3) Quick Test Matrix (for today)

* **Build & lint**

  ```
  cargo fmt -p omnigate
  cargo clippy -p omnigate --no-deps -- -D warnings
  cargo build -p omnigate
  ```
* **Dev-ready smoke**

  ```
  OMNIGATE_DEV_READY=1 cargo run -p omnigate --bin omnigate
  curl -i http://127.0.0.1:9605/readyz         # 200
  curl -i http://127.0.0.1:9605/healthz        # ok
  curl -i http://127.0.0.1:5305/v1/ping        # basic liveness
  ```
* **Policy deny path** (with a tiny bundle that denies everything):

  * Put bundle at `cfg.policy.bundle_path`, set `policy.enabled=true`.
  * Expect `/v1/ping` returns **403** with JSON problem:

    ```json
    {"code":"POLICY_DENY","message":"Access denied","retryable":false,"reason":"DENY"}
    ```
* **Quota exercise**:

  * Set `admission.global_quota.qps=5`, `burst=5`.
  * Bombard with 20 RPS; expect some **429** with reason `too_many_requests`.
* **Fair-queue saturation**:

  * Set `admission.fair_queue.max_inflight=2`, `headroom=0`.
  * Open 10 concurrent slow requests (use `slow_loris` test client or `wrk` with long timeouts).
  * Expect **503** with reason `overloaded`; gauge `READY_QUEUE_SATURATED=1`.

---

## 4) Known debt / follow-ups before calling “beta”

* **Admission from config**: wire body caps + decompression (B step above).
* **IP quota**: the struct exists (`admission.ip_quota`) but guard is not implemented; either implement or explicitly ignore in `validate` (log: “ip quota disabled”).
* **Ready probe sampler**: implement a small rolling window to compute `error_rate_429_503_pct`; flip `/readyz` to 503 if above threshold for `hold_for_secs`.
* **Route DTO visibility**: ensure all public handlers return `pub` DTOs to avoid future warnings as routes expand.
* **Docs**: add short `RUNBOOK.md` showing env toggles (`OMNIGATE_DEV_READY`, `OMNIGATE_AMNESIA`) and minimal config TOML with `admission` and `policy` sections.

---

## 5) Minimal code touch points summary (for your diff tool)

* `src/admission/mod.rs` → add `attach_with_cfg(router, &cfg.admission)`.
* `src/admission/quotas.rs` → add `attach_with_cfg(router, &cfg.global_quota)`.
* `src/admission/fair_queue.rs` → add `attach_with_cfg(router, &cfg.fair_queue)`.
* `src/middleware/mod.rs` → **remove** the call to `crate::admission::attach(router)`; return the router.
* `src/lib.rs` → after `middleware::apply(...)`, call `crate::admission::attach_with_cfg(..., &cfg.admission)`.

*(Optional now, recommended next)*

* `src/middleware/body_caps.rs` + `decompress_guard.rs` → add `*_with_cfg(…)` and change `middleware::apply` to `apply_with_cfg`.

---

## 6) “Why this design holds up”

* **No ‘static traps**: we hold `Arc<PolicyBundle>` in `Extension` and build a short-lived `Evaluator` per request—so file reloads and configuration hot-swaps are easy later.
* **Config threading** is localized: only admission (and optionally body/decompress) need cfg at layering time; everything else stays generic and testable.
* **Observability** first: every shed/deny path increments a labeled counter and/or gauge so `/readyz` can become truth-based rather than time-based.

---


### END NOTE - NOVEMBER 1 2025 - 16:17 CST


### BEGIN NOTE - NOVEMBER 2 2025 - 12:38 CST

Here you go — a clean, exhaustive “carry-over” packet for **omnigate** so you can drop into a fresh instance and pick up right where we left off.

# Omnigate — Carry-Over Notes (Beta Push)

## What Omnigate Is (1-liner)

Edge API “gate” that fronts your services: policy decisions, admission controls (quotas/fairness), safe body/decompression caps, correlation/metrics, and health/ready admin plane.

---

## Current Status (quick read)

* **Boots and serves**: `/readyz`, `/healthz`, `/ops/*`, and `/v1/ping`.
* **Dev readiness**: `OMNIGATE_DEV_READY=1` forces `/readyz` → 200 for local smoke.
* **Policy**:

  * Bundle loads from `crates/omnigate/configs/policy.bundle.json`.
  * **Correct layering fixed**: Policy middleware now evaluates before handlers.
  * Deny paths verified (PUT → 403 with JSON envelope; 451 reserved for “LEGAL”).
* **Admission guards**:

  * **Body caps**: Preflight `Content-Length` check + Axum `DefaultBodyLimit`.
  * **No-body methods exemption** (GET/HEAD/DELETE/OPTIONS): no 411 on these.
  * **Decompression guard**: Allows `identity|gzip|deflate|br`, denies stacked encodings and over-budget expansions.
* **Quotas / fair-queue**: Config model present; hooks exist; wiring pattern established.
* **Metrics**: Admin plane up; per-middleware counters registered (policy/body/decompress).
* **Tests**: Policy layering + metrics pass.
* **Clippy**: Clean with `-D warnings`.

**Rough completion**: ~**82%** toward beta.

---

## What We Changed (key diffs)

1. **Config**

   * `FairQueue::hard_and_headroom()` corrected (removed ineffective `.max(0)` on unsigned math).
   * Admission defaults defined and used; env overrides validated; 1 MiB OAP frame guard remains.
2. **Policy middleware**

   * Proper **bundle injection** via `axum::Extension<Arc<ron_policy::PolicyBundle>>`.
   * Evaluator built **per request**; deny returns stable JSON; optional 451 for “LEGAL”.
   * Added **normalizer path** in loader to accept “Grok-style” bundle shapes and rewrite to strict schema (logs: strict parse fail → normalize → success).
3. **Middleware stack (order matters)**

   * `classify` → `corr_id` → `policy` → `body_caps` (preflight + default limiter) → `decompress_guard` → `slow_loris`.
   * This order ensures policy sees the admission context and runs before handlers.
4. **Body caps**

   * Preflight 413 on oversize `Content-Length`.
   * **411 Length Required** enforced **only** for methods that *should* carry a body (PUT/POST/PATCH). No false 411 for GET/HEAD.
   * Emits `BODY_REJECT_TOTAL{reason="oversize"}` for oversize.
5. **Decompression guard**

   * Denies stacked (“gzip, br”) → 415 with envelope + `DECOMPRESS_REJECT_TOTAL{reason="stacked"}`.
   * Over-budget compressed length → 413 with `DECOMPRESS_REJECT_TOTAL{reason="over_budget"}`.
6. **Error envelopes**

   * `errors/http_map.rs` standardized Problem JSON across guards:

     * `POLICY_DENY`, `LEGAL_RESTRICTION`, `PAYLOAD_TOO_LARGE`, `UNSUPPORTED_ENCODING`, `SERVICE_DEGRADED`, etc.
     * Added `to_response(Reason, msg)` helper for guard fast-paths.
7. **Tests**

   * `tests/policy_gate.rs`: proves 405 vs 403 depending on layering.
   * `tests/policy_metrics.rs`: text-scrapes Prometheus exposition to prove `POLICY_SHORTCIRCUITS_TOTAL` increments on deny.
8. **Dev logs you should expect**

   * On first run with normalized bundle:

     * `failed to parse policy bundle (strict) …`
     * `policy bundle loaded via normalized schema …`
   * With strict bundle: `policy bundle loaded and inserted …`

---

## Repro — Commands & Scripts

### 0) Build & format/lint (always start clean)

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo build -p omnigate
```

### 1) Run the service (dev-ready override on)

```bash
OMNIGATE_DEV_READY=1 cargo run -p omnigate --bin omnigate
```

**Expected boot logs** (abridged):

* `amnesia mode resolved …`
* `OMNIGATE_DEV_READY=on — /readyz will report 200 (dev override)`
* (either) `policy bundle loaded and inserted`  **or**  `strict parse fail → policy bundle loaded via normalized schema`
* `omnigate up bind=127.0.0.1:5305 admin_addr=127.0.0.1:9605`

### 2) Manual smoke (Terminal B)

```bash
# Admin plane
curl -i http://127.0.0.1:9605/readyz     # 200 (dev override)
curl -i http://127.0.0.1:9605/healthz    # 200

# Happy path (GET, no body)
curl -i http://127.0.0.1:5305/v1/ping    # 200 {"ok":true}

# Policy deny (default deny + allow only GET)
curl -i -X PUT http://127.0.0.1:5305/v1/ping \
     -H 'Content-Length: 0'              # 403 POLICY_DENY {"reason":"put blocked"}

# Stacked encodings → 415
curl -i -X PUT http://127.0.0.1:5305/v1/ping \
     -H 'Content-Encoding: gzip, br' \
     -H 'Content-Length: 0'              # 415 JSON envelope (stacked not allowed)

# Oversize preflight → 413 (uses Content-Length)
# (Example 20 MiB len without sending bytes)
curl -i -X PUT http://127.0.0.1:5305/v1/ping \
     -H "Content-Length: $((20*1024*1024))"   # 413 PAYLOAD_TOO_LARGE
```

### 3) One-shot test targets

```bash
# Layering correctness (403 vs 405)
cargo test -p omnigate --test policy_gate -- --nocapture

# Metrics counter (deny bumps total)
cargo test -p omnigate --test policy_metrics -- --nocapture
```

### 4) Smoke script (bash)

Create `crates/omnigate/scripts/smoke_policy.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

API="127.0.0.1:5305"
OPS="127.0.0.1:9605"

say() { printf "\n▶ %s\n" "$*"; }
ok()  { printf "✅ %s\n" "$*"; }
no()  { printf "❌ %s\n" "$*"; exit 1; }

say "Check /readyz"
code=$(curl -s -o /dev/null -w "%{http_code}" "http://$OPS/readyz")
[[ "$code" = "200" ]] && ok "/readyz: $code" || no "/readyz expected 200, got $code"

say "GET /v1/ping (expect 200)"
code=$(curl -s -o /dev/null -w "%{http_code}" "http://$API/v1/ping")
[[ "$code" = "200" ]] && ok "GET /v1/ping: $code" || no "GET /v1/ping expected 200, got $code"

say "PUT /v1/ping with Content-Length:0 (expect 403 policy deny)"
code=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "http://$API/v1/ping" -H 'Content-Length: 0')
[[ "$code" = "403" ]] && ok "PUT deny: $code" || no "PUT expected 403, got $code"

say "Stacked encodings (expect 415)"
code=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "http://$API/v1/ping" \
        -H 'Content-Encoding: gzip, br' -H 'Content-Length: 0')
[[ "$code" = "415" ]] && ok "stacked encodings rejected: $code" || no "expected 415, got $code"

say "Oversize preflight (expect 413)"
code=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "http://$API/v1/ping" \
        -H "Content-Length: $((20*1024*1024))")
[[ "$code" = "413" ]] && ok "oversize rejected: $code" || no "expected 413, got $code"

ok "policy smoke complete"
```

Run:

```bash
chmod +x crates/omnigate/scripts/smoke_policy.sh
crates/omnigate/scripts/smoke_policy.sh
```

---

## Files That Matter (and why)

* `crates/omnigate/src/lib.rs`

  * App wiring; admin plane startup; **inserts policy bundle** with `Extension<Arc<ron_policy::PolicyBundle>>`.
  * Dev override `OMNIGATE_DEV_READY` handled here.
* `crates/omnigate/src/config/mod.rs`

  * Config model; env/file load; **FairQueue** `hard_and_headroom()` fix.
  * Admission defaults (global/ip quotas, fair queue, body/decompress caps).
* `crates/omnigate/src/middleware/mod.rs`

  * **Canonical layering** order (see above).
* `crates/omnigate/src/middleware/policy.rs`

  * Thin wrapper around `ron-policy` Evaluator; builds context; emits deny envelopes; increments counter.
* `crates/omnigate/src/middleware/body_caps.rs`

  * Preflight Content-Length check (413 oversize), **411 on missing length only for body-methods**, then `DefaultBodyLimit::max`.
* `crates/omnigate/src/middleware/decompress_guard.rs`

  * Encoding allowlist; stacked-denies; expansion budget (cap 10× to 1 MiB).
* `crates/omnigate/src/errors/http_map.rs`

  * **Stable** Problem JSON envelopes; `GateError` mappings; `to_response()` helper.
* `crates/omnigate/src/metrics/*`

  * Exposed counters used in the middleware (names below).

---

## Metrics (names to watch)

* `POLICY_SHORTCIRCUITS_TOTAL{code="403|451|503"}` — policy middleware denied/errored requests.
* `BODY_REJECT_TOTAL{reason="oversize"}` — preflight body size rejections.
* `DECOMPRESS_REJECT_TOTAL{reason="stacked"|"unknown"|"over_budget"}` — decompression guard rejections.

Scrape helper:

```bash
curl -s http://127.0.0.1:9605/metrics | grep -E '^(POLICY_SHORTCIRCUITS_TOTAL|BODY_REJECT_TOTAL|DECOMPRESS_REJECT_TOTAL)'
```

---

## Policy Bundle (working examples)

### Strict (already passes strict parse)

`crates/omnigate/configs/policy.bundle.json`

```json
{
  "version": 1,
  "meta": { "owner": "omnigate", "description": "deny by default; allow GET" },
  "defaults": { "default_action": "deny", "max_body_bytes": 1048576 },
  "rules": [
    { "id": "allow-gets", "when": { "method": "GET" }, "action": "allow", "reason": "get ok" },
    { "id": "deny-puts", "when": { "method": "PUT" }, "action": "deny", "reason": "put blocked" }
  ]
}
```

### “Grok-style” (normalized automatically)

```json
{
  "version": 1,
  "defaults": { "default_action": "deny", "max_body_bytes": 262144 },
  "rules": [
    { "id": "allow-gets", "when": { "method": "GET" }, "action": "allow", "reason": "get ok" },
    { "id": "deny-puts", "when": { "method": "PUT" }, "action": "deny", "reason": "put blocked" }
  ]
}
```

If strict parse fails, you’ll see a warning followed by `policy bundle loaded via normalized schema`, which is fine for dev.

---

## Environment Vars

* `OMNIGATE_DEV_READY=1` → `/readyz` always 200 (dev only).
* `OMNIGATE_AMNESIA=on|1|true` → forces amnesia mode regardless of config (local smoke convenience).

---

## Known Good Outputs (sanity)

* `GET /v1/ping` → `200 {"ok": true}`
* `PUT /v1/ping` with `Content-Length: 0` → `403 POLICY_DENY {"reason":"put blocked"}`
* `PUT /v1/ping` with `Content-Encoding: gzip, br` → `415` (stacked)
* `PUT /v1/ping` with `Content-Length: 20971520` → `413` (preflight oversize)

---

## Open Items to Reach “Beta”

1. **Admission: quotas + fair queue wiring**

   * Wire `admission::quotas::attach_with_cfg()` and `admission::fair_queue::attach_with_cfg()` into the app (behind cfg toggle).
   * Ensure `FairQueue::hard_and_headroom()` values reach the actual guard.
   * Emit `429` with retry-after from quota buckets.

2. **Metrics completeness**

   * Confirm all guards increment counters with stable label sets.
   * Add histogram(s) for gate latency if not present.
   * Add `/metrics` doc snippet to README.

3. **Readiness truth**

   * Remove reliance on `OMNIGATE_DEV_READY` for real readiness.
   * Drive readiness on: listeners bound, config loaded (done), policy loaded (bool), queues headroom, error rate window, inflight < threshold.

4. **Config → middleware parity**

   * Move hardcoded sizes in guards to use `cfg.admission.body` and `cfg.admission.decompression` (allow list, deny_stacked, max ratio).
   * Ensure `reject_on_missing_length` is honored (on body-methods only).

5. **Routes for v1**

   * Keep `GET /v1/ping`.
   * Add minimal passthrough sample (if needed) to exercise downstream error mapper.

6. **Docs / runbooks**

   * Add README sections: “Policy bundle schema”, “Layering order”, “Smoke script”, “Common errors”.
   * Add RUNBOOK: on-call quick checks (cURL, metrics triage).

7. **CI gates**

   * Lint/format/test as we already use.
   * Add a small integration smoke in CI (spawn server, run `smoke_policy.sh`).

8. **Bench (optional now)**

   * Re-enable simple middleware ping bench to watch overhead.

---

## Troubleshooting Cheatsheet

* **405 instead of 403** on PUT:

  * Policy middleware layered after router handlers or bundle wasn’t inserted.
  * Fix: ensure `app_router = app_router.layer(Extension(bundle));` **before** `middleware::apply(app_router)`.

* **411 on GET**:

  * Body cap preflight isn’t exempting no-body methods.
  * Fix: check `body_caps.rs` method filter; only enforce 411 for PUT/POST/PATCH.

* **Policy bundle warnings**:

  * “strict parse failed” → “loaded via normalized schema”: OK in dev. For prod, provide strict schema shape.

* **Metrics not appearing**:

  * You might be grepping wrong name; verify full scrape:

    ```bash
    curl -s http://127.0.0.1:9605/metrics | grep -E 'POLICY|DECOMPRESS|BODY'
    ```

---

## Quick File Map (so you can jump fast)

* App & admin: `src/lib.rs`, `src/bootstrap/server.rs`
* Config: `src/config/mod.rs`, `src/config/{env,file,validate}.rs`
* Middleware:

  * `src/middleware/mod.rs` (order)
  * `src/middleware/policy.rs`
  * `src/middleware/body_caps.rs`
  * `src/middleware/decompress_guard.rs`
  * `src/middleware/{corr_id,classify,slow_loris}.rs`
* Errors: `src/errors/{http_map.rs,reasons.rs}`
* Metrics: `src/metrics/*`
* Routes: `src/routes/{ops, v1/index.rs}`
* Tests: `tests/policy_gate.rs`, `tests/policy_metrics.rs`
* Configs: `configs/policy.bundle.json`
* Scripts: `scripts/smoke_policy.sh`

---

### END NOTE - NOVEMBER 2 2025 - 12:38 CST



### BEGIN NOTE - NOVEMBER 3 2025 - 11:35 CST


Here’s a **clean, exhaustive carry-over pack** for the next instance so we can pick up Omnigate and finish beta quickly.

# Omnigate — Carry-Over Notes (Beta Push)

## 1) Current Status (what’s working)

* **Build + lint**: `cargo fmt -p omnigate && cargo clippy -p omnigate --no-deps -D warnings && cargo build -p omnigate` — clean.
* **Boot & admin plane**:

  * Starts with logs showing config, amnesia flag, policy bundle load, and listener bind at `127.0.0.1:5305` (admin/metrics on `127.0.0.1:9605`).
  * `/healthz`, `/readyz`, `/versionz` mirror under both root and `/ops/*`.
  * `OMNIGATE_AMNESIA` and `OMNIGATE_DEV_READY` env overrides recognized; amnesia wired through `ron_kernel::Metrics`.
* **v1 API**:

  * `/v1/ping`: 200 JSON `{ ok: true }`.
  * `/v1/sleep?ms=<dur>`: async sleep endpoint for creating inflight pressure (present in current tree).
  * `/v1/index/healthz` retained; a simple `/v1/index/search` echo exists for JSON plumbing.
* **Middleware & Admission**:

  * **Stack order (canonical)**: classify → corr_id → policy → body caps (preflight + DefaultBodyLimit) → decompress guard (cfg-driven) → slow-loris.
  * **Body caps**: rejects missing `Content-Length` (if configured) and oversize bodies → 413, emits `body_reject_total{reason="oversize"|"missing_length"}`.
  * **Decompression guard**: denies stacked encodings and unknown encodings → 415, emits `decompress_reject_total{reason="stacked"|"unknown"|"over_budget"}`.
  * **Policy layer**: loaded from `crates/omnigate/configs/policy.bundle.json` with tolerant normalization fallback; deny → 403 (or 451 when marked legal), short-circuits counted by `policy_middleware_shortcircuits_total{code}`.
  * **Global quota & IP quota**: token-bucket admission, returns 429 with `Retry-After` header; counts `admission_quota_exhausted_total{scope="global|ip"}`.
  * **Fair queue**: inflight cap with optional interactive headroom (via `x-omnigate-priority: interactive`), drops with 503; counts `fair_q_events_total{event="dropped"}` and toggles `ready_queue_saturated` gauge (0/1).
* **Error envelopes**:

  * Centralized in `errors/http_map.rs` with stable JSON `Problem { code, message, retryable, retry_after_ms?, reason? }`.
  * Adds **`Retry-After`** header on 429 and 503 (policy error / degraded) using seconds ceil conversion.
* **Metrics (Prometheus)**:

  * Gate counters/gauges under `metrics/gates.rs`:

    * `policy_middleware_shortcircuits_total{code}`
    * `body_reject_total{reason}`
    * `decompress_reject_total{reason}`
    * `ready_inflight_current`, `ready_error_rate_pct`, `ready_queue_saturated`
    * `ready_trips_total{reason}`, `ready_state_changes_total{to}`
  * Exposed at `/ops/metrics` (default registry).
  * **Policy bundle load counter**: `policy_bundle_loaded_total` increments when parsed successfully.
* **Smoke scripts**:

  * `crates/omnigate/scripts/smoke_policy.sh`: ✅ passes; shows correct 403/415/413 and matching metric deltas.
  * `crates/omnigate/scripts/smoke_readiness.sh`: runs, but local readiness gate **didn’t trip** yet (still 200).

## 2) Where we left off (the gap)

* We implemented a **local readiness policy** (in `lib.rs:/readyz`) that:

  * Reads two gauges: `ready_inflight_current` and `ready_error_rate_pct`.
  * Trips to **503 Degraded** when either:

    * `ready_inflight_current > cfg.readiness.max_inflight_threshold`, **or**
    * `ready_error_rate_pct >= cfg.readiness.error_rate_429_503_pct`.
  * Holds degraded state for `cfg.readiness.hold_for_secs` before re-checking.
  * Emits `ready_trips_total{reason="inflight"|"err_rate"}` and `ready_state_changes_total{to="degraded"|"ready"}`.

* We also run a **sampler task** that rolls recent 429/503/drops into `ready_error_rate_pct`.
  **Observed issue:** during `smoke_readiness.sh`, the gauges didn’t move enough to trip:

  * `ready_inflight_current` stayed very low (≈ 1).
  * `ready_error_rate_pct` stayed 0.
  * **Inference:** we are not correctly **raising/lowering `ready_inflight_current`** around requests. The fair-queue sets `ready_queue_saturated`, but **doesn’t update `ready_inflight_current`**. Hence, the readiness gate never sees inflight growth and won’t trip on inflight threshold alone.

## 3) Root-Cause hypothesis & quick fix plan

**Hypothesis A (most likely):**
`ready_inflight_current` is never incremented/decremented by admission/middleware. Our fair-queue `Gate` maintains its own `in_flight` counter but only toggles `READY_QUEUE_SATURATED`; it **does not publish** the `in_flight` value into the `ready_inflight_current` gauge.

**Fix A (simple & deterministic):**

* In `admission/fair_queue.rs`:

  * After a successful `try_enter`, **increment** the gauge with the new `in_flight` value.
  * In the RAII `Drop` guard (on leave), **decrement** and set the gauge with the updated value.
  * This makes `ready_inflight_current` reflect real concurrency at the admission layer.

**Hypothesis B (less likely but useful):**
The sampler only increments `ready_error_rate_pct` from **429/503/drops** deltas. If the readiness trip is supposed to happen from error rate, we need to **generate** some 429s/503s/drops during the smoke (e.g., hit quota limits or force drops) — otherwise that gauge stays at 0.

**Fix B (smoke script side):**

* Add a phase that:

  1. Hammers `/v1/ping` fast enough to trigger **global** or **IP** quota (429s).
  2. Or sends **stacked encodings** to elicit decompression guard rejections + fair-queue overload to log drops (503).
* This ensures `ready_error_rate_pct` becomes non-zero so the gate can trip by error-rate path even if inflight threshold isn’t reached.

## 4) What remains for **Beta** (clear checklist)

**A. Readiness correctness**

1. **Wire the inflight gauge** (see Fix A).

   * Update `admission/fair_queue.rs`:

     * On `try_enter` success:

       * `let cur = gate.in_flight.fetch_add(1, AcqRel) + 1; READY_INFLIGHT_CURRENT.set(cur as i64);`
     * On `leave` (Drop):

       * `let cur = gate.in_flight.fetch_sub(1, AcqRel) - 1; READY_INFLIGHT_CURRENT.set(cur.max(0) as i64);`
   * Re-run `smoke_readiness.sh` with `CONCURRENCY=600..1200` and `OMNIGATE_DEV_READY` **unset**.
2. **Tune thresholds** in `configs/*.toml`:

   * `readiness.max_inflight_threshold` lower (e.g., 64-256) so our sleep load trips quickly on laptops.
   * `readiness.hold_for_secs` ~10–30 to observe sticky degrade and recovery.
   * `readiness.window_secs` ~(5–10) for visible `ready_error_rate_pct`.
3. Ensure we **init** gate metrics early: call `init_gate_metrics()` (already done in `lib.rs`).
4. Confirm **recovery path**: when load drops and hold expires, `/readyz` returns 200 and bumps `ready_state_changes_total{to="ready"}`.

**B. Error-rate sampler validation**

1. Extend smoke to produce real **429** and/or **503**:

   * Temporary config for **global_quota**: set `qps=50, burst=50` and run `wrk` or a tight loop to get **429s**.
   * Or run fair-queue overload to trigger **drops** (503 from shed).
2. Verify `ready_error_rate_pct > 0` during the window and that `/readyz` returns 503 when threshold hit.

**C. Tests (minimum bar for beta)**

* **Unit**:

  * `errors/http_map.rs`: `Retry-After` formatting and Problem JSON shape.
  * `reasons.rs`: code → status mapping.
* **Integration**:

  * **Policy**: 403 on PUT `/v1/ping` with `Content-Length: 0`; 415 for stacked encodings; 413 for oversize.
  * **Admission**: 429 from global quota when qps is tiny.
  * **Readiness**: spawn N tasks hitting `/v1/sleep?ms=500`; assert `/readyz` → 503 within a few seconds; then idle and assert recovery to 200 after `hold_for_secs`.

**D. Docs & runbooks**

* `README.md` (crate-level): quick start, env flags, routes, example config.
* `RUNBOOK.md`: on-call checks (`/ops/healthz`, `/ops/readyz`, `/ops/metrics`), common failure modes, knobs (quota/fair-queue/thresholds).
* `PERFORMANCE.md`: rough baselines for local (ping latency, max QPS before degrade).
* `CONFIG.md`: full schema with comments (we already enforce / use: server, admission.*, policy.*, readiness.*).

**E. Nice-to-have (if time)**

* Add small **/ops/vars** JSON dump for the current thresholds/toggles (read-only) to help SREs.
* Optional **trace sampling** toggle in config (currently we add `http_trace_layer()` globally).

## 5) Files we touched (and their intent)

* `src/middleware/mod.rs`
  Canonical stack order; exported `decompress_guard::attach_with_cfg(..)`; corr_id/classify/policy/body_caps/decompress/slow-loris.
* `src/middleware/body_caps.rs`
  Preflight length guard + Axum’s default body limit; emits `body_reject_total`.
* `src/middleware/decompress_guard.rs`
  Guards stacked/unknown encodings; emits `decompress_reject_total`; `attach_with_cfg(..)` public.
* `src/admission/mod.rs`
  Attach order: **quotas first**, then **fair_queue**; both have `attach_with_cfg(..)` variants.
* `src/admission/quotas.rs`
  Global + IP token buckets; return 429 with `Retry-After` using `GateError`; count `admission_quota_exhausted_total{scope}`.
* **(Needs small update)** `src/admission/fair_queue.rs`
  Currently toggles `READY_QUEUE_SATURATED` and counts drops. **TODO:** also set `READY_INFLIGHT_CURRENT` on enter/leave (Fix A).
* `src/errors/http_map.rs`
  Stable JSON problem mapping; **`Retry-After` header** included for 429 and 503; helpers `to_response(..)` and `GateError` → `IntoResponse`.
* `src/metrics/gates.rs`
  Gate counters + readiness gauges + `init_gate_metrics()` to pre-create series.
* `src/routes/v1/index.rs`
  `/ping`, `/index/healthz` and `/sleep?ms=...` to simulate latency; placeholder `/index/search` echo.
* `src/lib.rs`
  App builder: admin plane, routes, middleware wiring, policy bundle normalization load, **readiness sampler**, **local readiness gate with hold**, and metrics exposition.

## 6) Known good commands & smoke

**Policy smoke (works):**

```bash
OMNIGATE_DEV_READY=1 cargo run -p omnigate --bin omnigate
crates/omnigate/scripts/smoke_policy.sh
# Expect: All checks passed + metrics deltas for policy/body/decompress
```

**Readiness smoke (needs Fix A + tuning):**

```bash
# Ensure OMNIGATE_DEV_READY is unset/0 so local readiness gate is active
unset OMNIGATE_DEV_READY
cargo run -p omnigate --bin omnigate

# Tweak thresholds low in your config (example):
# readiness:
#   max_inflight_threshold = 64
#   error_rate_429_503_pct = 1.0
#   window_secs = 10
#   hold_for_secs = 20

crates/omnigate/scripts/smoke_readiness.sh
# Expect (after Fix A): /readyz flips to 503 under inflight pressure; 
# gauges show ready_inflight_current rising; then recover to 200 after hold.
```

**Manual pokes:**

```bash
# Metrics:
curl -s http://127.0.0.1:9605/metrics | grep -E 'ready_|policy_middleware_shortcircuits_total|body_reject_total|decompress_reject_total|fair_q_events_total|admission_quota_exhausted_total'

# Force quotas (set tiny qps/burst in config first), then:
for i in {1..500}; do curl -s -o /dev/null http://127.0.0.1:5305/v1/ping; done

# Force inflight:
seq 600 | xargs -I{} -P600 curl -s "http://127.0.0.1:5305/v1/sleep?ms=800" >/dev/null
```

## 7) Open issues to resolve at start of next session

1. **Implement inflight gauge updates** in `admission/fair_queue.rs` (Fix A) and re-run readiness smoke.
2. If inflight trip still doesn’t happen:

   * Lower `max_inflight_threshold` further.
   * Confirm `/v1/sleep` is present and returns 200 while sleeping (so requests actually stay inflight).
   * Confirm `xargs -P` or `wrk` actually generates hundreds of concurrent requests on your machine.
3. **Drive error-rate path** by generating 429s/503s (Fix B) to validate `ready_error_rate_pct` tripping.
4. Add **integration tests** for readiness degrade/recover and quota.
5. Write/commit **README / RUNBOOK / CONFIG** docs.

## 8) Beta completeness estimate

* Core surfaces (routes, middleware, admission, policy, error mapping, metrics): **done**.
* Admin plane & metrics export: **done**.
* Readiness sampler & gate scaffolding: **implemented**, needs final **trip wiring** (inflight gauge publish) and test: **~80–85% done**.
* Tests/docs polish: **pending**.

**Overall beta completion: ~88–90%.**
The **main blocker** is wiring `ready_inflight_current` from the fair-queue (plus minor threshold tuning and a couple of integration tests). Once the inflight gauge is published and the smoke passes (trip + hold + recovery), we can stamp **Beta** and move on to the next crate.


### END NOTE - NOVEMBER 3 2025 - 11:35 CST