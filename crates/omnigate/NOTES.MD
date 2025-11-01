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

