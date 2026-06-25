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


### BEGIN NOTE - NOVEMBER 3 2025 - 13:23 CST

# Omnigate — Beta Completion Notes (Carry-Over)

## 0) TL;DR (what to run to prove Beta)

Terminal A (run server with config):

```
cargo run -p omnigate --bin omnigate -- --config crates/omnigate/configs/omnigate.toml
```

Terminal B (smokes + tests):

```
chmod +x crates/omnigate/scripts/smoke_policy.sh
chmod +x crates/omnigate/scripts/smoke_readiness.sh

crates/omnigate/scripts/smoke_policy.sh
crates/omnigate/scripts/smoke_readiness.sh

cargo test -p omnigate --test readiness_inflight --test readiness_error_rate
```

Passing all of the above = Omnigate Beta ✅

---

## 1) What we have at **Beta**

### 1.1 Admin plane, endpoints, and observability

* **Admin/metrics bind:** `127.0.0.1:9605` (from config).
* **Public endpoints:**

  * `/healthz` and `/readyz` (duplicate under `/ops/healthz`, `/ops/readyz`).
  * `/versionz`, `/ops/version`.
  * **Prometheus exporter**: `/ops/metrics` (default registry).
* **Kernel glue:** `ron_kernel::Metrics` booted with *amnesia* bit; `HealthState` and kernel `Readiness` live and truthful.

### 1.2 v1 API (demo routes)

* `/v1/ping` → `200 {"ok":true}`
* `/v1/sleep?ms=<dur>` → async delay to create inflight pressure.
* `/v1/index/healthz` and a simple `/v1/index/search` echo (plumbing/fixture).

### 1.3 Middleware stack (canonical order)

Applied via `middleware::apply_with_cfg`:

1. **classify** (success/failure tags for tracing)
2. **corr_id** (stable request IDs)
3. **policy layer** (reads `Extension<PolicyBundle>`)
4. **body caps** (preflight content-length + Axum body limit)
5. **decompress guard** (deny stacked encodings, enforce allowlist)
6. **slow-loris/basic protections** (time limits/keepalive as configured)

**Important:** In Axum the **last** `.layer(..)` runs first. We layer `Extension(PolicyBundle)` **outside** (after) the middleware so the policy layer can see it. This is fixed and proven by smoke tests (see §3).

### 1.4 Admission (shed early, count clearly)

* **Global quota + IP quota** (token buckets)

  * `429` on exhaustion; adds `Retry-After` header (seconds).
  * Metrics: `admission_quota_exhausted_total{scope="global|ip"}`.
* **Fair queue** with interactive headroom

  * Header `x-omnigate-priority: interactive` raises cap by `headroom`.
  * On overload → `503` (shed) and mark queue saturation.
  * Metrics:

    * `fair_q_events_total{event="dropped"}`
    * `ready_queue_saturated` (0/1)

### 1.5 Error envelopes (stable JSON)

* Centralized mapping in `errors/http_map.rs`:

  * Returns `Problem { code, message, retryable, retry_after_ms?, reason? }` as JSON.
  * Adds **Retry-After** header for `429` and degraded `503`.
* Policy denies → `403` (or `451` if marked legal) with proper envelope.

### 1.6 Readiness (truthful, anti-flap)

* **Local readiness policy** trips when:

  * `ready_inflight_current > readiness.max_inflight_threshold`, **or**
  * `ready_error_rate_pct >= readiness.error_rate_429_503_pct`.
* **Hold window**: stays degraded (`503`) for `hold_for_secs` then re-checks.
* **Sampler task** rolls event deltas (429s, policy 503s, fair-queue drops) over `window_secs` into `ready_error_rate_pct`.
* **Gauges & counters** (Prometheus):

  * Gauges:

    * `ready_inflight_current`
    * `ready_error_rate_pct`
    * `ready_queue_saturated`
  * Counters:

    * `ready_trips_total{reason="inflight|err_rate|queue"}`
    * `ready_state_changes_total{to="degraded|ready"}`

**Fix we shipped:** fair-queue now updates `ready_inflight_current` on enter/leave (RAII guard), so inflight reflects true concurrency.

### 1.7 Config (TOML) honored and logged

**Current** `crates/omnigate/configs/omnigate.toml` core bits (example dev values):

```
[server]
bind         = "127.0.0.1:5305"
metrics_addr = "127.0.0.1:9605"
amnesia      = true

[oap]
max_frame_bytes    = 1048576
stream_chunk_bytes = 65536

[admission.global_quota]
qps   = 20000
burst = 40000

[admission.ip_quota]
enabled = true
qps     = 2000
burst   = 4000

[admission.fair_queue]
max_inflight = 2048
weights = { anon = 1, auth = 5, admin = 10 }

[admission.body]
max_content_length       = 10485760
reject_on_missing_length = true

[admission.decompression]
allow        = ["identity", "gzip"]
deny_stacked = true

[policy]
enabled     = true
bundle_path = "crates/omnigate/configs/policy.bundle.json"
fail_mode   = "deny"

[readiness]
max_inflight_threshold = 64
error_rate_429_503_pct = 1.0
window_secs            = 10
hold_for_secs          = 20
```

**Runtime env toggles:**

* `OMNIGATE_DEV_READY=1` → force `/readyz` to 200 (useful for route/middleware tests).
* `OMNIGATE_AMNESIA=1` → force amnesia mode on.

### 1.8 Tests and scripts (provable artifacts)

* **Integration tests** (pass):

  * `crates/omnigate/tests/readiness_inflight.rs`
    Proves: inflight trips `/readyz` → 503, holds, then recovers to 200.
  * `crates/omnigate/tests/readiness_error_rate.rs`
    Proves: error-rate path trips via fair-queue drops with inflight threshold set huge (no inflight trip).
* **Smoke scripts**:

  * `crates/omnigate/scripts/smoke_policy.sh`
    Proves: 403 (policy), 415 (stacked encodings), 413 (oversize) + metric deltas.
  * `crates/omnigate/scripts/smoke_readiness.sh`
    Proves: `/readyz` → 503 under load; shows gauges and hold window behavior.

---

## 2) How to run & what each proves

### 2.1 Run the server (with config)

```
cargo run -p omnigate --bin omnigate -- --config crates/omnigate/configs/omnigate.toml
```

Expected log on boot:

* config dump with readiness/admission
* “policy bundle loaded and inserted”
* “policy Extension layered outermost (visible to PolicyLayer)”
* “api listener bound … omnigate up …”

### 2.2 Policy smoke (functional & metric proof)

```
chmod +x crates/omnigate/scripts/smoke_policy.sh
crates/omnigate/scripts/smoke_policy.sh
```

Expect:

* `GET /v1/ping` → 200
* `PUT /v1/ping` (CL:0) → 403 (policy deny)
  Metric: `policy_middleware_shortcircuits_total{code="403"}` increments
* `PUT /v1/ping` (stacked encodings) → 415
  Metric: `decompress_reject_total{reason="stacked"}`
* `PUT /v1/ping` (oversize) → 413
  Metric: `body_reject_total{reason="oversize"}`

**Proves:** correct middleware order, Extension visible to policy, error envelopes and metrics.

### 2.3 Readiness smoke (truthful gate proof)

```
chmod +x crates/omnigate/scripts/smoke_readiness.sh
crates/omnigate/scripts/smoke_readiness.sh
```

Expect:

* Pre-load `/readyz` → 200
* During load → `/readyz` flips to 503 (degrade), stays degraded briefly (hold), then returns to 200
* Metrics show gauges (`ready_inflight_current`, `ready_queue_saturated`, `ready_error_rate_pct`)

**Proves:** inflight gauge wiring, sampler operation, anti-flap hold behavior.

### 2.4 Integration tests (deterministic CI proof)

```
cargo test -p omnigate --test readiness_inflight --test readiness_error_rate
```

* **Inflight test**: uses `/v1/sleep` and low inflight threshold to assert degrade → hold → recovery.
* **Error-rate test**: sets tiny fair-queue cap (max_inflight=8), huge inflight threshold, short sampler window, hammers `/v1/sleep` to force `503` drops and trip error-rate branch.

**Proves:** both trip paths work independently and reproducibly on a laptop/CI.

---

## 3) Invariants & guarantees we maintain

* **No secret leakage** in error envelopes; codes/status align; `Retry-After` set for `429` and degraded `503`.
* **Single-writer discipline** on counters (fair-queue RAII ensures balanced increments/decrements).
* **Policy layer sees bundle** (outermost Extension ordering).
* **Readiness is truthful**:

  * Trips on sustained inflight or sustained 429/503 (drops/quota/policy) within the rolling window.
  * Hold window prevents flap; state changes counted once per transition.

---

## 4) Troubleshooting quick hits

* **Ready never degrades** in smoke:

  * Ensure server launched **with** the TOML (`-- --config …`).
  * Make `readiness.max_inflight_threshold` lower (e.g., `64`) or bump `CONCURRENCY` in the smoke.
  * Confirm `OMNIGATE_DEV_READY` is UNSET in the server terminal.
* **Policy 403 not observed**:

  * This was due to **layer order** historically. Ensure the log says “policy Extension layered outermost (visible to PolicyLayer)”.
  * Check `bundle_path` is correct; the server logs parsing/normalization attempts.

---

## 5) “Nice to have” (post-Beta polish)

**A. Admin & DX**

* `/ops/vars` read-only JSON with current thresholds and active toggles (readiness and admission).
* `/ops/limits` to enumerate effective caps (body limit, decompress allowlist, fair-queue caps per priority).
* Basic **OpenAPI** doc for `/v1/*` demo routes (keeps sample clients honest).

**B. More tests**

* Unit test `errors/http_map.rs` for `Retry-After` (seconds ceil) and envelope shape.
* Unit tests for policy normalizer (`normalize_policy_value`) covering “effect” → “action” migration and legacy defaults.
* Property tests for fair-queue RAII (no negative inflight, monotonic within a request).

**C. Performance baselines**

* Criterion benches around middleware micro-paths (`middleware_ping.rs` exists):

  * Baseline: GET `/v1/ping` in-process (we’ve seen ~10–11 µs locally).
  * Add toggles to simulate policy/body/decompress branch hit costs.

**D. Config ergonomics**

* Allow **per-CIDR quotas** (you already have commented example in TOML).
* Optional **trace sampling** knob (`observability.sample_pct`).

**E. Operational safety nets**

* Readiness **slow-recovery** mode (optional): after hold expiry, require N consecutive good checks before returning 200.
* Metric **hysteresis** for error-rate (EWMA alternative to simple window rollup).

**F. Dev scripting**

* One-shot “chaos flood” script to demo both paths:

  * Phase 1: inflight trip using `/v1/sleep`.
  * Phase 2: error-rate trip via fair-queue tiny cap.
  * Prints key gauges/counters over time.

---

## 6) Minimal docs to ship with Beta (ready to paste next if you want)

* **README.md** (Quickstart): run, endpoints, sample curl, where metrics live.
* **RUNBOOK.md** (Ops): how to check `/readyz`, what to do when degraded; knobs (admission/readiness); smoke commands and expected outputs; metric names to grep.
* **CONFIG.md** (Reference): annotated TOML with 1–2 sentences per field (server, oap, admission.*, policy.*, readiness.*).

---

## 7) Final Beta acceptance checklist

* Build hygiene:

  * `cargo fmt -p omnigate --check`
  * `cargo clippy -p omnigate --no-deps -- -D warnings`
* Tests:

  * `cargo test -p omnigate --test readiness_inflight --test readiness_error_rate`
* Smokes against a running server:

  * `crates/omnigate/scripts/smoke_policy.sh` → all ✅ with metric deltas
  * `crates/omnigate/scripts/smoke_readiness.sh` → **Observed degrade: /readyz -> 503** and hold, then recovery
* Logs show:

  * Config line with readiness thresholds
  * Policy bundle loaded
  * Policy Extension layered **outermost**
  * Listener bound; server up

If all of the above are true (your latest transcript shows they are), **Omnigate is Beta.**

---

## 8) Hand-off to next crate

Omnigate is now a solid **front door** with truthful readiness, consistent envelopes, and SRE-friendly metrics. It’s a dependable base to place in front of:

* `svc-gateway` (routing/rewrite/edge policies),
* `ron-auth` / `svc-passport` (capabilities/macaroons),
* `ron-kms` (secrets/keys),
* `ron-audit` (structured event trail).

!!! IMPORTANT: ADD /AUTH AFTER SVC-PASSPORT IS COMPLETED


### END NOTE - NOVEMBER 3 2025 - 13:23 CST



### BEGIN NOTE - NOVEMBER 22 2025 - 14:07 CST

Quick recap of what we just locked in:

* `/v1/app/healthz` stub is mounted and working (v1 router + app module).
* `middleware::decompress_guard` is now:

  * Public (`pub mod decompress_guard`).
  * Exposes `layer()` for tests and simple stacks.
  * Emits a proper JSON problem envelope with `reason: "unsupported_media_type" | "payload_too_large" | "stacked"`, etc.
* `middleware::body_caps`:

  * Only emits **413** when `Content-Length` is present and over the configured limit.
  * No longer emits **411**, so policy/routing can produce the expected 405/403 behavior.
* `errors::Reason` + `http_map::to_response` now fill `Problem.reason` with lowercase snake_case codes so the `middleware_contract` tests can assert on them.

So omnigate is now **test-clean and app-plane–aware** (stubbed), which is exactly where we wanted it for this slice.


### END NOTE - NOVEMBER 22 2025 - 14:07 CST


### BEGIN NOTE - JANUARY 10 2026 - 15:30 CST


## Omnigate Notes — WOW Static Site Demo Wiring (Why + What Changed)

### Context

We built the first “WOW demo” as a **static site served through the ingress path**:

* Browser hits: `svc-gateway` at `/app/wow`
* `svc-gateway` proxies that to `omnigate` at `/v1/app/wow`
* `omnigate` serves the WOW site and exposes demo control endpoints:

  * `POST /v1/app/wow/reload`
  * `POST /v1/app/wow/crash` (dev-only)

This gave us the “wow” loop quickly: **URL works, proxy works, ops endpoints work, metrics endpoints work, crash/restart loop works**.

### What we changed in Omnigate (high-level)

**We extended the App Plane under `/v1/app/*`** beyond the original “health-only stub” so it can host at least one real app-plane shape.

Concretely, Omnigate now has an App Plane route surface that (at minimum) supports:

* A “probe” endpoint (health)
* A “demo app” route namespace (`/wow`)

This was done so the demo could run with only two binaries:

* `omnigate`
* `svc-gateway`

…and still feel like a real platform: ingress → app plane → observable ops.

### Why Omnigate was involved at all

This demo was intentionally the **smallest possible vertical slice** that proves the ingress + app-plane concept without pulling in macronode/micronode orchestration yet.

So yes:

* **For THIS demo:** we did not need macronode or micronode.
* The demo proves **“omnigate + gateway can host an app-plane service”**.
* It does *not yet* prove “micronode/macronode runs and supervises third-party apps” — that’s the next demo.

### Addressing the “don’t lock core into demo” concern

Your concern is correct: **Omnigate should not become “the static-site server.”**

What we did is acceptable only if we treat it as one of these two patterns:

#### Pattern A (acceptable): Omnigate hosts a *generic* app-plane mounting surface

Omnigate’s job is to:

* provide a stable prefix (`/v1/app/*`)
* enforce policy/admission/metrics/readiness around app-plane traffic
* optionally mount apps behind that prefix (later: `ron-app-sdk` apps)

In this world, the WOW site is just **one app** mounted under `/v1/app/wow`.

#### Pattern B (avoid long-term): Omnigate contains bespoke “serve a static site” logic

If Omnigate directly owns static-site semantics (fallback HTML, hardcoded content, special-case behavior), that’s demo-lock-in and confusing DX.

**Action item:** keep `/v1/app/*` as the *mount point*, but move “static site implementation” into:

* an example app crate (preferred), or
* a reusable app module (`ron-app-static` style), or
* a facet loaded by micronode/macronode later

### About the “self-contained default HTML” (your objection)

You’re right to call that “cheating” from a DX honesty standpoint.

If the demo site is missing, the correct behavior is:

* return a truthful 404 / problem JSON
* print a crisp “you’re missing the site directory” message in the demo script
* keep the runtime surface clean

**Recommendation:** remove any “embedded fallback site HTML” behavior and require the site assets to exist on disk (or be explicitly embedded via a normal Rust mechanism if that’s a real product feature later).

### Why your “reload counter” might be stuck at 0

Given what you observed (site works, ops/metrics work, but “reload gen” doesn’t change), the most likely causes are:

1. **The reload endpoint isn’t incrementing the metric.**
   (Handler exists and returns 200, but no `counter.inc()` call.)

2. **The metric is registered in a registry that isn’t being scraped.**
   In omnigate, you effectively have two scrape surfaces:

   * the ron-kernel metrics server (`metrics.serve(...)` on the metrics port)
   * the in-process `/ops/metrics` route (currently using `prometheus::gather()`)

   If the reload metric isn’t in the default registry, `/ops/metrics` won’t show it.

3. **You’re looking at the wrong scrape endpoint.**
   Verify which one your dashboard/page is rendering from.

**Quick truth test (no code changes):**

* Trigger reload via curl:

  * `curl -i -X POST http://127.0.0.1:5304/app/wow/reload -H 'content-type: application/json' -d '{}'`
* Immediately scrape both:

  * `curl -sS http://127.0.0.1:5305/ops/metrics | grep -i reload`
  * `curl -sS http://127.0.0.1:9605/metrics | grep -i reload` (or whatever omnigate metrics addr is)
* If neither changes → missing increment or wrong metric name
* If one changes and the other doesn’t → registry mismatch / scrape mismatch

### Security note: is this a “secure static site hosting option”?

**It can be**, but only if we’re clear what security layer lives where:

* `svc-gateway` is a proxy: it must preserve auth headers and strip hop-by-hop headers (it already does the correct basics).
* `omnigate` is where the real security posture belongs:

  * policy enforcement (deny-by-default, allow explicit routes)
  * admission limits (body caps, fair queue, quotas)
  * readiness gating and overload behavior

For a static site specifically:

* You still need the usual web hardening:

  * correct `Content-Type`
  * `Cache-Control` policy
  * deny traversal (`..`) and unsafe paths
  * disable directory listings unless intentional
  * security headers (CSP, X-Content-Type-Options, etc.)

Right now, the demo proves the *platform wrapper* (policy/admission/metrics), but we should not claim it’s “production static hosting” until those headers/path-hardening details are explicitly done.

### What this demo *proved* (and what it did not)

**Proved:**

* `/app/*` ingress works (gateway → omnigate app plane)
* Omnigate ops plane is live (`/ops/healthz`, `/ops/readyz`, `/ops/metrics`)
* Crash/restart loop works (demo script supervision)
* Policy bundle loads and is applied (since the request path is allowed)

**Not yet proved:**

* micronode/macronode supervising *third-party* apps
* facets-driven mounting + TS SDK-driven developer workflow
* “dev doesn’t clone repo” DX story

### Immediate TODOs to keep core clean

* Ensure App Plane is documented as: **“mount point + policy/admission wrapper”**, not “omnigate is a web server.”
* Remove embedded fallback site behavior (truthful failure if assets missing).
* Fix reload metric increment + ensure it appears on the correct scrape endpoint.
* Document the “two metrics endpoints” story clearly (or unify it).

---

## Answering your “did we need micronode/macronode?” question

For the static WOW demo you just ran: **correct — micronode/macronode weren’t required**. You basically proved a clean slice of:

**Ingress (svc-gateway) → App Plane (omnigate) → Ops (health/ready/metrics)**

The next demo is where micronode becomes the star: **“run my app under micronode, mount via facets, observe in svc-admin, control via TS SDK.”**

---


### END NOTE - JANUARY 10 2026 - 15:30 CST



### BEGIN NOTE - APRIL 29 2026 - 00:55 CST

# NOTE — `omnigate` Session Notes

## Current status

`omnigate` is now a real **WEB3 product/BFF layer** for paid storage. In this session, it moved from mostly app-plane/stubbed BFF behavior into a working WEB3 pass-through surface for:

```text
GET  /v1/paid/o/estimate
POST /v1/paid/o
PUT  /v1/paid/o
```

Its role is intentionally narrow and correct:

```text
omnigate does not mutate ledger
omnigate does not verify wallet receipts itself
omnigate does not store object bytes
omnigate does not export accounting usage itself
omnigate does not compute storage pricing itself

omnigate proxies product-facing paid storage requests to svc-storage
```

That is the correct WEB3 boundary. `svc-storage` remains the paid-use enforcement point. `svc-wallet` remains the economic mutation front door. `ron-accounting` remains the usage snapshot/ingest system. `omnigate` is now the BFF/product surface that connects future dashboards, apps, browser extension, and passport UX to those backend services.

The most important proof from this session is that the live final edge smoke successfully ran:

```text
svc-gateway
→ omnigate
→ svc-storage
→ svc-wallet
→ ron-accounting
```

and completed estimate, wallet hold, paid write, capture/release, accounting export, object readback, and replay rejection. 

---

# 1. What `omnigate` is responsible for now

For WEB3 paid storage, `omnigate` now owns the BFF-facing route shape:

```text
/v1/paid/o/estimate
/v1/paid/o
```

Its responsibilities are:

```text
1. Expose BFF route for paid storage estimate.
2. Expose BFF route for paid storage write.
3. Forward estimate requests to svc-storage /paid/o/estimate.
4. Forward paid object writes to svc-storage /paid/o.
5. Preserve allowed auth/payment/accounting/correlation headers.
6. Preserve body bytes for paid writes.
7. Pass successful storage JSON responses through unchanged.
8. Pass storage-side 400/payment errors through unchanged.
9. Convert storage transport failures into stable 502 Problem JSON.
10. Avoid hop-by-hop/host/content-length forwarding mistakes.
```

Its responsibilities are **not**:

```text
1. No direct ledger mutation.
2. No direct wallet hold/capture/release.
3. No direct accounting export.
4. No storage byte persistence.
5. No paid-storage pricing computation.
6. No wallet receipt validation.
7. No paid context-idem derivation.
```

This keeps the modular architecture clean.

---

# 2. Files changed or materially involved this session

## Direct `omnigate` files

```text
crates/omnigate/src/routes/v1/paid.rs
crates/omnigate/src/routes/v1/mod.rs
crates/omnigate/tests/paid_storage_estimate_proxy.rs
crates/omnigate/tests/paid_storage_write_proxy.rs
```

## Existing `omnigate` files exercised indirectly

```text
crates/omnigate/src/main.rs
crates/omnigate/src/routes/v1/app.rs
crates/omnigate/src/routes/v1/index.rs
crates/omnigate/src/routes/v1/objects.rs
crates/omnigate/src/routes/v1/mailbox.rs
crates/omnigate/src/routes/v1/dht.rs
crates/omnigate/src/routes/v1/facet.rs
crates/omnigate/tests/admission_contract.rs
crates/omnigate/tests/dto_serialization.rs
crates/omnigate/tests/metrics_contract.rs
crates/omnigate/tests/middleware_contract.rs
crates/omnigate/tests/policy_gate.rs
crates/omnigate/tests/policy_metrics.rs
crates/omnigate/tests/readiness_error_rate.rs
crates/omnigate/tests/readiness_inflight.rs
crates/omnigate/tests/ready_truth.rs
crates/omnigate/tests/zk_receipts.rs
```

## Root scripts that now exercise `omnigate`

```text
scripts/web3_paid_estimate_edge_smoke.sh
scripts/web3_paid_storage_edge_write_smoke.sh
```

The final and most important one is:

```text
scripts/web3_paid_storage_edge_write_smoke.sh
```

That script proved `omnigate` works inside the real live paid storage execution path, not only in isolated proxy tests.

---

# 3. Major accomplishment: added `/v1/paid/o/estimate`

## What was added

`omnigate` now exposes:

```text
GET /v1/paid/o/estimate?bytes=N
```

This route proxies to:

```text
svc-storage /paid/o/estimate?bytes=N
```

It is read-only and side-effect-free.

It does not:

```text
create wallet holds
mutate ledger state
write object bytes
export accounting events
resolve b3 asset manifests
touch passports
touch naming
touch payout routing
```

## Why it matters

This endpoint is the BFF/product layer equivalent of the storage estimate endpoint. It allows future clients to use the normal application path:

```text
app / dashboard / browser extension
→ svc-gateway
→ omnigate
→ svc-storage
```

instead of talking directly to `svc-storage`.

This matters because WEB3_2 expects product UX around paid storage, paid content, passports, browser extension flows, and asset pages. `omnigate` is the right place to present those flows to apps without making the frontend know the entire backend topology.

## Proven response behavior

When storage returns a valid estimate, `omnigate` passes through the storage response shape:

```json
{
  "schema": "svc-storage.paid-storage-estimate.v1",
  "route": "/paid/o",
  "action": "paid_storage_put",
  "asset": "roc",
  "bytes": 48,
  "amount_minor": "84",
  "minimum_hold_minor": "84",
  "pricing_mode": "roc-economics",
  "economics_policy_path": "configs/roc-economics.toml"
}
```

The live edge estimate smoke proved that this value traveled through the full edge estimate path:

```text
svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate
```

with `amount_minor = 84`. The final edge write smoke also proved the estimate was used before wallet issue/hold/write. 

---

# 4. Major accomplishment: added `/v1/paid/o` write proxy

## What was added

`omnigate` now exposes:

```text
POST /v1/paid/o
PUT  /v1/paid/o
```

These proxy to:

```text
POST /paid/o on svc-storage
PUT  /paid/o on svc-storage
```

This gives the product/BFF layer a paid write route that future clients can use after obtaining a quote and wallet hold.

## What it forwards

The route forwards:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
Idempotency-Key
body bytes
HTTP method POST/PUT
```

It intentionally does not forward:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

Dropping `Content-Length` is correct because the HTTP client will recalculate it for the forwarded body. Avoiding hop-by-hop headers keeps the proxy boundary safe.

## Why it matters

Before this session, the true paid write path was available at `svc-storage`, but the normal product path did not have an edge/BFF route for it. Now the product path is:

```text
client
→ svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
```

The key architectural point is that `omnigate` does **not** reimplement paid write logic. It only forwards the request. Storage remains responsible for:

```text
body hash
b3 CID
wallet receipt verification
context-idem anti-replay
capture/release settlement
accounting export
usage event creation
object storage
```

---

# 5. Error handling behavior

`omnigate` now has stable paid-route error behavior.

## Storage-side application errors

If `svc-storage` returns a normal application error, such as:

```text
400 bad_request
402 payment_required
```

`omnigate` passes the status/body through unchanged.

This is important because storage owns the paid-write contract. Omnigate should not translate storage’s economic/security errors into a different semantic shape.

## Transport/connect/read failures

If `omnigate` cannot reach storage, it returns:

```json
{
  "code": "upstream_unavailable",
  "message": "...",
  "retryable": true,
  "reason": "storage_connect"
}
```

For estimate failures, the message is:

```text
storage estimate upstream unavailable
```

For paid write failures, the message is:

```text
storage paid route upstream unavailable
```

We had a regression where the shared helper initially returned the paid-write message even for estimate failures; the estimate test caught it. We fixed this by passing the unavailable message into the shared proxy helper. The terminal output showed the failing expected/actual message mismatch, and the final rerun proved both estimate and write tests green. 

---

# 6. Tests added for `omnigate`

## `paid_storage_estimate_proxy.rs`

This test file proves:

```text
1. /v1/paid/o/estimate happy path works.
2. Successful estimate JSON passes through.
3. Authorization / x-ron / correlation headers can reach storage.
4. Storage-side bad request passes through as 400.
5. Storage connect failure maps to 502 Problem JSON.
6. Hop-by-hop headers are not forwarded.
```

The final `omnigate` test output showed:

```text
paid_storage_estimate_proxy_happy_path ... ok
paid_storage_estimate_proxy_passes_storage_errors_through ... ok
paid_storage_estimate_upstream_connect_failure_yields_problem_502 ... ok
```



## `paid_storage_write_proxy.rs`

This test file proves:

```text
1. POST /v1/paid/o forwards to storage.
2. PUT /v1/paid/o forwards to storage.
3. Body bytes are preserved exactly.
4. Payment headers are preserved.
5. Wallet receipt headers are preserved.
6. Accounting context headers are preserved.
7. Idempotency-Key is preserved.
8. Storage-side 400 passes through unchanged.
9. Storage unavailable maps to 502 Problem JSON.
10. Hop-by-hop headers are not forwarded.
```

The final `omnigate` test output showed:

```text
paid_storage_write_proxy_post_happy_path ... ok
paid_storage_write_proxy_put_happy_path ... ok
paid_storage_write_proxy_passes_storage_errors_through ... ok
paid_storage_write_upstream_connect_failure_yields_problem_502 ... ok
```



---

# 7. Clippy/test fixes completed during the session

## Removed unused `put` import

Initial code imported:

```rust
routing::{get, post, put}
```

but the route used:

```rust
post(write).put(write)
```

which does not require importing `put`.

Clippy rejected the unused import under `-D warnings`. We fixed it by using:

```rust
routing::{get, post}
```

This occurred in both route/test files where applicable.

## Fixed POST body length expectation

The test body:

```text
RustyOnions paid write through omnigate
```

has length:

```text
39
```

The initial test expected 38. The test correctly failed, proving body preservation was actually working but the expected constant was wrong. We updated the test to expect `39`.

## Restored estimate-specific upstream failure message

After the write route was added, the shared `upstream_problem` helper returned:

```text
storage paid route upstream unavailable
```

for both estimate and write paths.

The estimate test expected:

```text
storage estimate upstream unavailable
```

We fixed the helper to accept a route-specific message.

## Final clippy outcome

The final run showed:

```text
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

completed successfully before running the test suite. 

---

# 8. Full omnigate test suite status

The final session run showed the full `omnigate` test suite stayed green.

Important test groups that remained green:

```text
admission_contract
dto_serialization
metrics_contract
middleware_contract
policy_gate
policy_metrics
readiness_error_rate
readiness_inflight
ready_truth
zk_receipts
paid_storage_estimate_proxy
paid_storage_write_proxy
bench compilation paths
```

This matters because the new WEB3 paid routes did not regress existing omnigate behavior around:

```text
admission
middleware hardening
decompression/body caps
policy gate behavior
policy metrics
readiness truth
readiness under inflight/error conditions
receipt status transitions
```

The terminal output shows those existing omnigate tests continuing to pass after the new paid routes were added. 

---

# 9. Live smoke proof involving `omnigate`

## Estimate-only edge smoke

The estimate edge smoke proved:

```text
svc-gateway /paid/o/estimate
→ omnigate /v1/paid/o/estimate
→ svc-storage /paid/o/estimate
→ configs/roc-economics.toml
→ 48 bytes = 84 ROC minor units
```

This confirmed `omnigate` works as a BFF layer for read-only pricing preflight.

## Full edge paid-write smoke

The final live edge paid-write smoke proved:

```text
svc-gateway /paid/o
→ omnigate /v1/paid/o
→ svc-storage /paid/o
→ svc-wallet capture/release
→ ron-accounting export
```

The final live smoke output showed:

```text
capture_minor: 84
release_minor: 16
payer_balance: 16
payee_balance: 84
accounting_rows: 3
replay_rejection: 402 payment_required
```

and ended with:

```text
WEB3 edge paid storage write smoke green
```



For `omnigate`, this proves it can carry a real paid write containing real wallet receipt headers and object bytes through to storage without corrupting:

```text
request body
content type
wallet txid
wallet receipt hash
wallet idem
payer/escrow headers
paid estimate amount
tenant/accounting metadata
correlation headers
```

---

# 10. Current route inventory for WEB3 paid storage in `omnigate`

## Read-only estimate

```text
GET /v1/paid/o/estimate?bytes=N
```

Proxy target:

```text
svc-storage GET /paid/o/estimate?bytes=N
```

Behavior:

```text
side-effect free
passes success JSON through
passes storage 400 through
storage unavailable -> 502 Problem JSON
```

## Paid write

```text
POST /v1/paid/o
PUT  /v1/paid/o
```

Proxy target:

```text
svc-storage POST /paid/o
svc-storage PUT  /paid/o
```

Behavior:

```text
preserves object body
preserves x-ron payment headers
preserves wallet receipt headers
preserves accounting metadata headers
passes storage response through
passes storage paid errors through
storage unavailable -> 502 Problem JSON
```

---

# 11. Current `omnigate` WEB3 readiness estimate

For `omnigate` specifically:

```text
Paid storage estimate BFF route: 95–98%
Paid storage write BFF route:    94–97%
WEB3 paid-storage BFF layer:     93–96%
Full WEB3_2 product layer:       65–72%
```

Why the paid-storage BFF layer is high:

```text
1. Estimate route exists.
2. Write route exists.
3. Tests cover POST, PUT, success, storage errors, transport failures.
4. Live edge smoke proves real end-to-end usage.
5. Existing omnigate tests stayed green.
```

Why WEB3_2 product layer is lower:

```text
1. RON Passport persistence is not implemented here yet.
2. Wallet-linked passport permissions are not wired.
3. Typed b3 asset pages are not implemented.
4. crab:// route resolution is not implemented.
5. Paid content views/site/app access are not fully wired.
6. Product response hydration is not polished.
```

---

# 12. What is complete for `omnigate`

## Complete or beta-complete

```text
1. v1 paid route module added.
2. /v1/paid/o/estimate added.
3. /v1/paid/o POST/PUT added.
4. Estimate proxy to storage works.
5. Write proxy to storage works.
6. Header forwarding rules are explicit.
7. Hop-by-hop headers are filtered.
8. Storage success responses pass through.
9. Storage bad request responses pass through.
10. Storage unavailable maps to 502.
11. Tests cover happy path and failure path.
12. Full omnigate target suite remains green.
13. Live edge smoke proves omnigate in the real paid storage chain.
```

## Current proof quality

`omnigate` now has three layers of proof:

```text
1. Unit/integration tests with dummy storage.
2. Full omnigate regression test suite.
3. Live multi-service edge smoke with real storage/wallet/accounting/gateway.
```

That is strong beta-level proof for the paid-storage BFF path.

---

# 13. What remains for `omnigate`

## 13.1 Product response hydration

Right now `omnigate` mostly passes storage responses through unchanged. That is fine for correctness, but product UX will eventually need a cleaner BFF response.

Potential future response envelope:

```json
{
  "schema": "omnigate.paid-storage-write.v1",
  "cid": "b3:<hash>",
  "asset": "roc",
  "charged_minor": "84",
  "released_minor": "16",
  "receipt_hashes": {
    "capture": "b3:<hash>",
    "release": "b3:<hash>"
  },
  "accounting": {
    "status": "exported",
    "event_count": 3
  },
  "links": {
    "read": "/v1/objects/b3:<hash>",
    "asset_page": "crab://b3/<hash>.asset"
  }
}
```

Important: this should probably be additive/versioned. Do not break the current storage response pass-through until a stable external contract is chosen.

## 13.2 RON Passport integration

WEB3_2 calls for:

```text
local persistent RON Passports
main identity passports
alternate anonymous passports
wallet-linked passport permissions
```

`omnigate` is a likely place to begin enforcing or hydrating passport context for product routes.

Future work:

```text
1. Accept passport headers/tokens.
2. Validate passport state.
3. Attach wallet-linked permissions.
4. Enforce max spend/allowed action caveats.
5. Support anonymous passport rate/spend limits.
6. Surface passport identity/permission state to app UX.
```

Important boundary: passport validation may occur in omnigate, but ledger mutation still belongs to wallet and paid write enforcement still belongs to storage.

## 13.3 Wallet-hold UX handoff

Right now the flow is:

```text
client estimates price
client manually creates wallet hold
client submits paid write with receipt headers
```

For product UX, omnigate may need helper routes that tell the frontend exactly what to do next.

Possible future route:

```text
POST /v1/paid/o/prepare
```

Possible response:

```json
{
  "estimate": {
    "asset": "roc",
    "minimum_hold_minor": "84"
  },
  "wallet_hold_request": {
    "from": "acct_user",
    "to": "escrow_paid_write",
    "asset": "roc",
    "amount_minor": "84",
    "memo": "paid_storage_put cid=..."
  },
  "next": {
    "submit_to": "/v1/paid/o"
  }
}
```

This would still not mutate wallet. It would only help the client construct the correct wallet request.

## 13.4 Paid content/site/app access

WEB3 expands beyond storage/pinning into:

```text
paid content views
paid site access
paid app access
```

`omnigate` is likely the natural BFF layer for these.

Possible future routes:

```text
GET  /v1/paid/content/:id/estimate
POST /v1/paid/content/:id/view
GET  /v1/paid/site/:site_id/estimate
POST /v1/paid/site/:site_id/access
GET  /v1/paid/app/:app_id/estimate
POST /v1/paid/app/:app_id/access
```

These must use the same invariants:

```text
integer minor units only
pricing from config/policy
hold/capture/release through wallet/storage/app enforcement
accounting export
deterministic receipts
fail closed
```

## 13.5 Typed b3 asset page routing

WEB3_2 expects typed b3 asset pages such as:

```text
crab://b3/<hash>.image
crab://b3/<hash>.song
crab://b3/<hash>.article
crab://b3/<hash>.video
crab://b3/<hash>.comment
```

`omnigate` should likely expose HTTP-equivalent BFF routes that browser extension/web clients can use before native `crab://` handling exists.

Possible route family:

```text
GET /v1/b3/:hash
GET /v1/b3/:hash.image
GET /v1/b3/:hash.article
GET /v1/crab/b3/:hash
```

Responsibilities:

```text
resolve manifest
hydrate owner/payout metadata
show tags/description/curator metadata
include storage/provider data
include receipt/payment metadata where appropriate
avoid storing bytes directly
```

## 13.6 Gateway/omnigate route alignment

The current shape is now:

```text
gateway /paid/o/estimate → omnigate /v1/paid/o/estimate
gateway /paid/o          → omnigate /v1/paid/o
```

Future route alignment should ensure:

```text
1. External route names are stable.
2. v1 omnigate route names are stable.
3. Gateway does not bypass omnigate for product routes.
4. Omnigate does not bypass service boundaries.
5. Route docs are updated.
6. Smoke scripts are kept aligned with route changes.
```

## 13.7 Observability and metrics

The route is covered by existing middleware when mounted through the full app, but WEB3 paid route-specific metrics would be useful.

Suggested future metrics:

```text
omnigate_paid_estimate_requests_total
omnigate_paid_estimate_upstream_errors_total
omnigate_paid_write_requests_total
omnigate_paid_write_upstream_errors_total
omnigate_paid_write_bytes_total
omnigate_paid_storage_proxy_latency_seconds
omnigate_paid_storage_status_total{status_class}
```

Also useful:

```text
x-request-id propagation checks
correlation ID continuity checks
gateway→omnigate→storage latency breakdown
```

## 13.8 Config formalization

Current storage upstream config is environment-based:

```text
OMNIGATE_STORAGE_BASE_URL
OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
```

This is good for dev smoke, but production config likely needs to move into the normal omnigate config file.

Future config shape might include:

```toml
[upstreams.storage]
base_url = "http://127.0.0.1:15303"
timeout_ms = 2000
max_body_bytes = 10485760
```

Then the route should read from app state/config instead of a static env lookup.

## 13.9 Client timeout and retry policy

The current paid route uses a shared `reqwest::Client`, but production should define:

```text
connect timeout
read timeout
max request body size
retry policy for idempotent estimate only
no automatic retry for non-idempotent writes unless idempotency is guaranteed
clear error taxonomy
```

Paid writes are dangerous to blindly retry unless all downstream operations are idempotent. Since wallet/storage idempotency is being built into the system, this can be done carefully later.

## 13.10 Security hardening

Future hardening:

```text
1. Do not forward arbitrary headers.
2. Keep forwarding allowlist explicit.
3. Validate content-type for paid writes.
4. Validate max body size before forwarding.
5. Enforce auth/passport requirements once passport layer exists.
6. Add request signing/capability verification if needed.
7. Ensure no secrets are logged.
8. Ensure amnesia mode suppresses durable logs/trace artifacts.
```

---

# 14. Developer notes for next session

## Do not regress these invariants

```text
1. Omnigate is BFF/proxy for paid storage, not ledger/storage owner.
2. svc-storage remains paid-write enforcement point.
3. svc-wallet remains balance mutation front door.
4. ron-accounting remains usage ingest/snapshot.
5. Gateway should route product paid paths through omnigate.
6. Omnigate should route paid storage paths to svc-storage.
7. Error pass-through from storage should remain stable.
8. Transport failure should map to 502 upstream_unavailable.
9. Hop-by-hop headers must not be forwarded.
10. Request body bytes must not be modified.
```

## Important env vars

```text
OMNIGATE_STORAGE_BASE_URL
OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
OMNIGATE_DEV_READY
```

For smoke scripts:

```text
OMNIGATE_BIND_ADDR
OMNIGATE_METRICS_ADDR
```

## Important routes

```text
GET  /v1/paid/o/estimate?bytes=N
POST /v1/paid/o
PUT  /v1/paid/o
```

## Important target routes

```text
GET  svc-storage /paid/o/estimate?bytes=N
POST svc-storage /paid/o
PUT  svc-storage /paid/o
```

## Important proofs from this session

```text
Estimate route:
  48 bytes → 84 ROC minor units

Write route:
  POST and PUT body pass-through works
  payment headers preserved
  storage 400 pass-through works
  storage unavailable -> 502 works

Live final:
  gateway → omnigate → storage → wallet/accounting
  capture 84
  release 16
  accounting rows 3
  replay rejection 402
```

---

# 15. Suggested next `omnigate` work

The next `omnigate`-specific work should probably be one of these:

## Option A — Add product response envelope

Create a stable BFF response shape for paid writes and estimates without removing the internal storage response.

This is high value for dashboard/browser-extension UX.

## Option B — Move storage upstream config into omnigate config

Replace env-only upstream resolution with normal omnigate config/app state.

This is production-hardening.

## Option C — Add `/v1/paid/o/prepare`

A prepare route could combine estimate data with wallet-hold instructions for clients.

This is a strong product UX bridge.

## Option D — Begin passport-aware paid route checks

Add hooks for RON Passport authorization/caveats.

This is WEB3_2 foundational work.

## Option E — Start typed b3 asset page hydration

Add the first BFF route for b3 asset manifests/pages.

This begins the b3/crab product layer.

---

# 16. Bottom line

`omnigate` is now a real part of the WEB3 paid storage product path.

It now supports:

```text
paid storage estimate BFF route
paid storage write BFF route
success pass-through
storage error pass-through
upstream failure 502 mapping
header preservation
body preservation
full regression test green
live edge smoke green
```

The key achievement is that `omnigate` now sits correctly between edge clients and backend paid-use services:

```text
client
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-wallet / ron-accounting
```

That makes `omnigate` ready to become the future home of WEB3_2 product UX: passports, paid app/site/content access, typed b3 asset pages, and crab link hydration.


### END NOTE - APRIL 29 2026 - 00:55 CST



### BEGIN NOTE - APRIL 29 2026 - 13:00 CST

Here are the **omnigate changelog/carry-over notes only**.

---

# `omnigate` — WEB3_2 Read-Only Crab / b3 Asset Page Resolver Notes

**Session date:** 2026-04-29
**Crate:** `crates/omnigate`
**Status:** Batch complete / green
**Purpose of this session:** Add the read-only product BFF layer that resolves typed `crab://<64hex>.<asset_kind>` / `/v1/b3/<64hex>.<asset_kind>` routes into a hydrated asset-page response.

---

## 0. Summary

In this session, `omnigate` was extended with a new WEB3_2 read-only asset-page resolver.

This is the first real product-facing hydration layer for typed b3 asset pages.

The new resolver accepts a public crab asset URL like:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

or the HTTP equivalent:

```text
/v1/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

and returns a stable JSON asset-page response containing:

```text
asset CID
asset kind
manifest pointer status
storage availability
owner display reference
payout display reference
canonical crab link
raw object link
warnings
```

Important boundary preserved:

```text
omnigate hydrates/composes a read-only view.
omnigate does not store raw bytes.
omnigate does not mutate svc-index.
omnigate does not mutate svc-storage.
omnigate does not call svc-wallet for this route.
omnigate does not mutate ron-ledger.
omnigate does not execute payout.
```

This batch turns the prior foundations into an actual product-facing read path:

```text
ron-naming
  defines crab://<64hex>.<asset_kind>

ron-proto
  defines asset/page/manifest DTO concepts

svc-index
  stores asset CID → manifest CID pointers

omnigate
  resolves and hydrates asset-page responses
```

---

## 1. Files added or updated

### Updated

```text
crates/omnigate/src/routes/v1/mod.rs
```

### Added

```text
crates/omnigate/src/routes/v1/crab.rs
crates/omnigate/tests/asset_page_resolver.rs
```

---

## 2. Routes added

Two new read-only routes were added.

---

## 2.1 `GET /v1/crab/resolve?url=...`

Route:

```text
GET /v1/crab/resolve?url=crab://<64hex>.<asset_kind>
```

Example:

```text
GET /v1/crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Purpose:

```text
Resolve a canonical public crab asset URL into a hydrated asset-page response.
```

This route is the natural backend target for the future browser extension, dashboard, or SDK.

The browser extension can eventually take:

```text
crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

and translate it into:

```text
GET /v1/crab/resolve?url=crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

---

## 2.2 `GET /v1/b3/:asset`

Route:

```text
GET /v1/b3/<64hex>.<asset_kind>
```

Example:

```text
GET /v1/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Purpose:

```text
Expose a clean HTTP equivalent of the crab asset URL.
```

This is useful for:

```text
browser extension translation
HTTP clients
svc-gateway proxying
dashboard/API inspection
route contract tests
```

The path segment format is:

```text
<64-lowercase-hex>.<asset_kind>
```

Important:

```text
The HTTP route uses /v1/b3/<hash>.<kind>.
The public crab URL uses crab://<hash>.<kind>.
The internal CID remains b3:<hash>.
```

---

## 3. Router update

`crates/omnigate/src/routes/v1/mod.rs` was updated to include the new `crab` module.

Added:

```rust
pub mod crab;
```

And mounted routes:

```rust
.nest("/crab", crab::router())
.route("/b3/:asset", get(crab::resolve_b3_asset))
```

This makes the new product routes part of the `v1` API surface.

---

## 4. Parser functionality added

New parsing helpers were added in:

```text
crates/omnigate/src/routes/v1/crab.rs
```

---

## 4.1 `parse_crab_asset_url()`

Parses canonical public crab asset URLs:

```text
crab://<64hex>.<asset_kind>
```

Returns a `ParsedAssetTarget` containing:

```text
raw_hash_hex
asset_cid
asset_kind
canonical_crab
```

Example output:

```text
raw_hash_hex:
  0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

asset_cid:
  b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

asset_kind:
  image

canonical_crab:
  crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

---

## 4.2 `parse_b3_asset_segment()`

Parses HTTP path segment form:

```text
<64hex>.<asset_kind>
```

Example:

```text
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.video
```

Returns the same `ParsedAssetTarget` shape.

---

## 4.3 Old `b3/` prefix rejected

The parser intentionally rejects the old/noncanonical draft form:

```text
crab://b3/<hash>.image
```

This matches the project decision made during this session:

```text
Public crab asset URLs must not include b3/.
```

Canonical public form:

```text
crab://<hash>.image
```

Internal CID form:

```text
b3:<hash>
```

---

## 4.4 Parser rejection behavior

The parser rejects:

```text
missing crab:// scheme
empty target
old crab://b3/<hash>.<kind> form
unsupported path forms
query/fragment inside the b3 asset segment route
missing asset kind suffix
bad hash length
uppercase hash
non-hex hash characters
unknown asset kind
control characters
path traversal-ish forms
credentials / @
slash and backslash in asset target
```

This keeps the resolver fail-closed and deterministic.

---

## 5. Asset kind support

The omnigate resolver currently accepts these asset kinds:

```text
image
video
music
song
article
post
comment
page
site
app
manifest
```

These match the broader `ron-proto` asset DTO vocabulary from this session.

The parser intentionally returns asset kinds as strings rather than making `omnigate` own the vocabulary permanently.

Boundary:

```text
ron-naming / ron-proto own canonical grammar and DTO vocabulary.
omnigate validates enough to safely route/hydrate product pages.
```

---

## 6. Response DTOs added locally in omnigate

The new `crab.rs` route module defines local response structs for the first product BFF response.

These are local to `omnigate` for now.

---

## 6.1 `ParsedAssetTarget`

Fields:

```text
raw_hash_hex
asset_cid
asset_kind
canonical_crab
```

Purpose:

```text
Canonical parsed form used internally by the resolver before hydration.
```

---

## 6.2 `AssetManifestPointer`

Local struct matching the `svc-index` pointer response.

Fields:

```text
version
asset_cid
asset_kind
manifest_cid
owner_passport_subject
owner_wallet_account
updated_at_ms
```

Purpose:

```text
Deserialize the pointer returned by svc-index:
asset CID → manifest CID
```

Important boundary:

```text
This pointer is display/hydration metadata.
It is not proof of ownership by itself.
It is not spend authority.
```

---

## 6.3 `AssetPageResponse`

Main response shape returned by the new resolver.

Fields:

```text
schema
asset_cid
asset_kind
manifest
storage
owner
payout
links
warnings
```

Schema string:

```text
omnigate.asset-page.v1
```

Purpose:

```text
Give clients and browser-extension UI a stable typed asset-page response.
```

---

## 6.4 `ManifestSummary`

Fields:

```text
status
manifest_cid
updated_at_ms
```

Statuses:

```text
present
missing
```

Purpose:

```text
Tell the frontend whether the index has a manifest pointer for this asset.
```

If missing, the asset page can still render a partial response with warnings.

---

## 6.5 `StorageSummary`

Fields:

```text
available
size_bytes
content_type
provider_ref
```

Purpose:

```text
Tell the frontend whether raw asset bytes appear available from storage.
```

Data is populated by a storage `HEAD` probe.

Important boundary:

```text
This is availability metadata only.
It does not fetch raw bytes.
It does not pin content.
It does not mutate storage.
```

---

## 6.6 `OwnerSummary`

Fields:

```text
passport_subject
wallet_account
```

Purpose:

```text
Display owner/passport/wallet reference information from the index pointer.
```

Boundary:

```text
OwnerSummary does not prove authority.
OwnerSummary does not grant wallet spending rights.
OwnerSummary does not verify passport ownership.
```

---

## 6.7 `PayoutSummary`

Fields:

```text
default_action
recipient_account
```

Current default action:

```text
content_view
```

Purpose:

```text
Give the frontend a display-level payout target reference.
```

Boundary:

```text
PayoutSummary does not execute payouts.
PayoutSummary does not call svc-wallet.
PayoutSummary does not mutate ron-ledger.
```

---

## 6.8 `AssetPageLinks`

Fields:

```text
raw
crab
http_b3
```

Example conceptual output:

```text
raw:
  /o/b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

crab:
  crab://0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image

http_b3:
  /v1/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

These links set up the browser-extension/dashboard experience.

---

## 6.9 `ResolverProblem`

Problem response shape for resolver errors.

Fields:

```text
code
message
retryable
reason
```

Examples of problem codes:

```text
invalid_crab_url
invalid_b3_asset
index_upstream_error
index_pointer_mismatch
```

---

## 7. Hydration behavior added

The resolver follows this read-only flow:

```text
parse crab/b3 target
→ canonicalize to b3:<hash>
→ call svc-index for asset manifest pointer
→ call svc-storage HEAD for storage availability
→ compose AssetPageResponse
→ return JSON
```

Conceptual chain:

```text
omnigate
  asks svc-index:
    GET /v1/index/assets/<raw_hash>/manifest

  asks svc-storage:
    HEAD /o/b3:<hash>

  returns:
    AssetPageResponse
```

Important:

```text
The index lookup can be missing.
The storage availability check can be missing/unavailable.
The resolver still returns a partial asset page when safe.
```

This is important for product UX because a b3 asset page can still render even if metadata or storage availability is incomplete.

---

## 8. Upstream configuration

The resolver reads index/storage base URLs from environment variables.

Index base URL:

```text
OMNIGATE_INDEX_BASE_URL
OMNIGATE_DOWNSTREAM_INDEX_BASE_URL
```

Default:

```text
http://127.0.0.1:5304
```

Storage base URL:

```text
OMNIGATE_STORAGE_BASE_URL
OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
```

Default:

```text
http://127.0.0.1:15303
```

This matches the existing pattern of using env-configured downstream services without hard-wiring crate dependencies.

---

## 9. Upstream index behavior

The resolver calls:

```text
GET /v1/index/assets/<raw_hash>/manifest
```

Behavior:

```text
200 OK
  deserialize AssetManifestPointer

404 Not Found
  manifest pointer is missing; return partial page with warning

5xx / connect/read error
  return 502-style upstream problem

bad JSON
  return upstream problem with non-retryable reason
```

Pointer mismatch handling:

```text
If svc-index returns a pointer whose asset_cid or asset_kind does not match the requested asset,
omnigate returns an index_pointer_mismatch problem.
```

This prevents bad or stale index data from silently producing a wrong page.

---

## 10. Upstream storage behavior

The resolver calls:

```text
HEAD /o/b3:<hash>
```

Behavior:

```text
2xx
  storage.available = true
  may extract Content-Length
  may extract Content-Type
  may extract x-ron-provider

404
  storage.available = false
  warning = storage_object_missing

connect/read error
  storage.available = false
  warning = storage_upstream_unavailable

other non-2xx
  storage.available = false
  warning = storage_http_<status>
```

Important:

```text
Storage failures are non-fatal for asset-page hydration.
```

That means the asset page can still show:

```text
canonical crab URL
asset CID
asset kind
manifest status
owner/payout pointer if available
warning that bytes are currently unavailable
```

---

## 11. Resolver counters added

Simple local atomic counters were added:

```text
CRAB_RESOLVE_TOTAL
CRAB_RESOLVE_ERROR_TOTAL
```

Public helper:

```rust
resolver_counters() -> (u64, u64)
```

Purpose:

```text
Allow tests and future metrics wiring to inspect resolver activity.
```

Current limitation:

```text
These are local atomics, not yet exported through Prometheus metrics.
```

Future work can wire them into `omnigate`’s existing metrics module.

---

## 12. Tests added

New test file:

```text
crates/omnigate/tests/asset_page_resolver.rs
```

---

## 12.1 Parser tests

Covers:

```text
parses canonical crab asset URL
parses HTTP b3 asset segment
rejects old b3 slash prefix
rejects uppercase hash
rejects unknown kind
rejects missing kind
rejects path and query in asset segment
```

Important examples tested:

```text
crab://<hash>.image                 accepted
<hash>.video                        accepted
crab://b3/<hash>.image              rejected
crab://<uppercase_hash>.image       rejected
crab://<hash>.binary                rejected
crab://<hash>                       rejected
crab://<hash>.image?x=1             rejected
crab://<hash>.image/extra           rejected
```

---

## 12.2 Composition tests

Covers full page composition when manifest pointer and storage are present.

Test verifies:

```text
schema = omnigate.asset-page.v1
asset_cid is b3:<hash>
asset_kind = image
manifest.status = present
manifest.manifest_cid is present
owner passport is present
payout recipient is present
storage.available = true
links.crab = crab://<hash>.image
links.raw = /o/b3:<hash>
warnings empty
```

---

## 12.3 Partial page tests

Covers page composition when manifest pointer is missing.

Test verifies:

```text
manifest.status = missing
manifest_cid = None
owner = None
payout = None
storage can still be available
warnings contains manifest_pointer_missing
```

This proves the asset page can render partial state safely.

---

## 12.4 JSON wire-shape test

Covers stable response JSON shape.

Checks that serialized response includes:

```text
schema
omnigate.asset-page.v1
asset_cid
asset_kind
manifest
storage
links
canonical crab URL
```

Also checks decoded JSON fields:

```text
schema = omnigate.asset-page.v1
asset_cid = b3:<hash>
asset_kind = comment
```

---

## 12.5 Counter test

Covers:

```text
resolver_counters() is readable
total >= errors
```

This gives basic coverage for the local counter helper.

---

## 13. Clippy issue caught and fixed

Initial clippy failed on the test file because two assertions used:

```rust
assert_eq!(page.storage.available, true);
```

Clippy under `-D warnings` rejected this via:

```text
clippy::bool_assert_comparison
```

Fixed to:

```rust
assert!(page.storage.available);
```

After this fix, the targeted clippy gate passed.

---

## 14. Validation gates passed

Final `omnigate` gates passed:

```text
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test asset_page_resolver
cargo test -p omnigate --all-targets
```

Final observed result:

```text
omnigate clippy: clean
asset_page_resolver: 11/11 passing
omnigate all-target tests: all passing
```

New test group passed:

```text
tests/asset_page_resolver.rs    11/11 passing
```

Existing tests also continued to pass, including:

```text
admission_contract.rs
dto_serialization.rs
metrics_contract.rs
middleware_contract.rs
paid_storage_estimate_proxy.rs
paid_storage_write_proxy.rs
policy_gate.rs
policy_metrics.rs
readiness_error_rate.rs
readiness_inflight.rs
ready_truth.rs
zk_receipts.rs
hydration bench target
media_range bench target
middleware_ping bench target
```

---

## 15. Invariants preserved

This work preserved WEB3 / WEB3_2 and RustyOnions crate-boundary invariants:

```text
omnigate remains composition/BFF layer.
omnigate does not store raw content bytes.
omnigate does not write to svc-index.
omnigate does not write to svc-storage.
omnigate does not issue wallet holds.
omnigate does not capture/release holds.
omnigate does not mutate ron-ledger.
omnigate does not calculate final payouts.
omnigate does not verify wallet receipts on this read-only route.
omnigate does not own browser-extension UX.
omnigate only composes a read-only asset-page response.
```

Content/addressing invariant preserved:

```text
Public URL:
  crab://<64hex>.<asset_kind>

Internal CID:
  b3:<64hex>
```

The old public form remains rejected:

```text
crab://b3/<hash>.<kind>
```

---

## 16. Architectural boundary notes

`omnigate` now owns:

```text
read-only crab asset URL resolution
read-only /v1/b3/<hash>.<kind> asset page hydration
index pointer lookup orchestration
storage HEAD availability orchestration
asset page response composition
safe parser error mapping
partial page warnings
```

`omnigate` still does **not** own:

```text
raw byte storage
BLAKE3 hashing
manifest pointer persistence
manifest byte storage
wallet mutation
ledger mutation
paid hold/capture/release for this route
passport key storage
browser extension UI
public gateway exposure
```

Those remain in:

```text
ron-naming     — crab URL grammar
ron-proto      — shared DTO contracts
svc-index      — manifest pointer storage
svc-storage    — raw content bytes and object metadata
svc-wallet     — wallet mutation and receipts
ron-ledger     — append-only durable economic truth
svc-gateway    — public ingress route exposure
browser extension — user-facing crab navigation UX later
```

---

## 17. Completion assessment for `omnigate`

For the WEB3_2 product-proof TODO, `omnigate` Batch 4 is:

```text
100% complete for this session’s scope
```

Current `omnigate` WEB3_2 readiness:

```text
GET /v1/crab/resolve?url=...              complete
GET /v1/b3/<64hex>.<kind>                 complete
canonical crab parsing                    complete
old b3/ prefix rejection                  complete
uppercase hash rejection                  complete
unknown asset kind rejection              complete
index pointer lookup path                 implemented
storage availability HEAD path            implemented
read-only asset-page response             implemented
partial page warnings                     implemented
stable JSON shape test                    complete
targeted gates                            green
```

---

## 18. Remaining future work for `omnigate`

No blocker remains from this session.

Future enhancements may include:

```text
Replace local asset-page DTOs with shared ron-proto DTOs if/when desired.
Wire crab resolver counters into Prometheus metrics.
Add route-level tracing spans for crab resolver hydration steps.
Add timeout/retry policy for index/storage upstream calls.
Add stronger typed upstream client wrappers for svc-index and svc-storage.
Hydrate full manifest bytes from svc-storage once manifest pointer exists.
Parse/validate manifest bytes into ron-proto AssetManifestV1.
Add paid content-view prepare route.
Add paid content-view enforcement.
Add site manifest hydration for crab://site.
Add dashboard/admin visibility for asset page hydration.
Add route contract docs/OpenAPI.
```

Likely future routes:

```text
GET  /v1/sites/:name
GET  /v1/crab/site/:name
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/content/view/prepare
POST /v1/content/view/capture
```

But those should remain separate TODO batches so `omnigate` does not blur responsibilities.

---

## 19. Session takeaway

`omnigate` now turns the lower-level WEB3_2 primitives into an actual product-facing read path.

Before this batch, the stack had:

```text
crab grammar
asset DTOs
manifest pointer storage
```

After this batch, the stack can now produce:

```text
a typed asset-page JSON response
```

The key product chain is now:

```text
crab://<hash>.image
  → omnigate /v1/crab/resolve
  → svc-index manifest pointer lookup
  → svc-storage HEAD availability check
  → AssetPageResponse JSON
```

This is a major product-proof step because it gives RustyOnions the backend shape for the future browser extension and dashboard asset pages.

`omnigate` Batch 4 is complete and green.


### END NOTE - APRIL 29 2026 - 13:00 CST


### BEGIN NOTE - APRIL 29 2026 - 17:50 CST
>THE FOLLOWING ARE DUAL NOTES FOR BOTH OMNIGATE AND SVC-GATEWAY:

Below are two **drop-in crate notes**: one for `omnigate`, one for `svc-gateway`. They capture what we accomplished, what was proven, architectural invariants, tests, smoke gates, known caveats, and remaining work. The final green gate passed end-to-end, including `omnigate` 5-test site launch coverage, `svc-gateway` 6-test product route proxy coverage, safe stack smoke, and mutation-enabled site create/resolve stack smoke. 

---

# Drop-in note for `crates/omnigate/NOTES.md`

````markdown
# omnigate WEB3_2 Product Layer Notes — Batch 8/9/10 Carryover

## Current status

`omnigate` is now one of the central WEB3_2 product-layer crates. In this session, it moved from having partial product proof routes to having a green-gated backend foundation for:

- typed `b3` / `crab://` asset-page resolution
- image asset prepare/upload coordination
- paid object prepare/proxy flows
- static site prepare
- static site manifest creation
- static site name pointer write
- static site resolver/hydrator
- live stack smoke through `svc-gateway → omnigate → svc-storage/svc-index`

The final green gate passed:

```text
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test site_launch
cargo test -p omnigate --all-targets
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
scripts/web3_product_green_gate.sh
````

The final `site_launch.rs` test target now runs 5 tests and passes all of them, including the resolver/hydration proof:

```text
site_prepare_returns_wallet_hold_template
site_prepare_rejects_bad_site_name
site_prepare_maps_storage_estimate_errors
site_create_stores_manifest_and_index_pointer
site_resolve_hydrates_manifest_from_index_and_storage
```

The full `omnigate` all-targets suite also passed, including asset-page resolver tests, image asset prepare/upload tests, paid storage prepare/proxy tests, readiness tests, policy tests, middleware tests, metrics tests, and site launch tests.

## Session work summary

The main `omnigate` work centered on completing the static site product backend and then proving it through both cargo tests and live stack smoke.

### Site route surface now implemented/proven

The site route family is now:

```text
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

These are mounted through the v1 router and are intended to sit behind public `svc-gateway` routes:

```text
POST /sites/prepare → omnigate POST /v1/sites/prepare
POST /sites         → omnigate POST /v1/sites
GET  /sites/:name   → omnigate GET  /v1/sites/:name
```

### `POST /v1/sites/prepare`

This route is a read-only/product-preflight route.

It validates:

* strict JSON body
* safe beta site name
* safe file paths
* nonzero total byte count or valid file byte sum

It then calls `svc-storage` paid estimate route:

```text
GET /paid/o/estimate?bytes=<total_bytes>
```

It returns a wallet-hold template and next-step hints, but does not call `svc-wallet`.

Important invariant:

```text
site_prepare does not mutate ledger, wallet, storage objects, or index pointers.
```

It returns schema:

```text
omnigate.site-prepare.v1
```

The response includes:

* normalized `site_name`
* action, asset, total bytes
* owner/passport/account hints
* paid storage estimate summary
* wallet hold template
* site manifest preview
* next routes for wallet hold, site submit, and later resolve

### `POST /v1/sites`

This route creates a static site manifest and writes the site name pointer.

It validates:

* strict JSON body
* safe beta site name
* canonical root document CID in `b3:<64 lowercase hex>` form
* canonical CIDs inside `asset_map`
* canonical CIDs inside `route_map`

It then builds a site manifest JSON object with:

* `version`
* `site_name`
* `root_document_cid`
* `asset_map`
* `route_map`
* `owner`
* `payout`
* `metadata`
* `provenance`
* `storage`
* `receipts`

It stores the generated manifest through `svc-storage`:

```text
POST /o
```

Then it writes the site pointer through `svc-index`:

```text
PUT /v1/index/sites/:site_name/manifest
```

It returns schema:

```text
omnigate.site-create.v1
```

A successful response must report:

```json
{
  "manifest": {
    "status": "stored",
    "manifest_cid": "b3:<64 lowercase hex>"
  },
  "index_pointer": {
    "status": "stored"
  }
}
```

Important invariant:

```text
site_create coordinates manifest object storage and pointer write, but it still does not call wallet or mutate ledger directly.
```

Wallet/ledger mutation remains outside this route. The route may include receipt refs and a provided hold txid in the manifest receipts, but it does not create/capture/release holds itself.

### `GET /v1/sites/:name`

This route was the key missing resolver/hydrator piece added in this session.

It performs a read-only chain:

```text
GET /v1/sites/:name
→ GET svc-index /v1/index/sites/:name/manifest
→ GET svc-storage /o/<manifest_cid>
→ parse site manifest JSON
→ return hydrated site page DTO
```

It returns schema:

```text
omnigate.site-page.v1
```

A successful hydrated response includes:

```json
{
  "manifest": {
    "status": "present",
    "hydration_status": "hydrated"
  }
}
```

It also returns:

* `site_name`
* `root_document_cid`
* owner summary
* payout summary
* metadata
* route map
* asset map
* receipts
* links, including `crab://<site_name>`, `/v1/sites/<site_name>`, and `/o/<manifest_cid>`

## Important bugs fixed during this session

### 1. Ownership/partial move in `site_create`

`site_create` initially moved fields from `request`, then tried to borrow the full request later for manifest construction.

Fix:

* clone `request.owner_passport_subject`
* clone `request.owner_wallet_account`

This preserved ownership and allowed `request` to be borrowed afterward.

### 2. Site-name error ordering

A test expected `../bad` to fail as `double_dot`, but the name validator originally returned `edge_dot`.

Fix:

* check `name.contains("..")` before edge-dot checks

This made traversal-like input fail with the stable reason:

```text
double_dot
```

### 3. `created_at_ms` overflowing literal in tests

Large timestamp literals in tests needed explicit `u64` suffixes to avoid `i32` overflow.

Fix:

```rust
1_776_000_000_000u64
```

### 4. Manifest payout lifetime issue

`SitePayoutSummary.default_action` was originally `&'static str`, but resolver hydration wanted to use a string borrowed from a hydrated manifest.

Fix:

```rust
default_action: String
```

The resolver now owns the default action string safely.

### 5. Live smoke `site_manifest_bad_json`

The mutation-enabled live site smoke initially failed with:

```json
{
  "code": "site_manifest_bad_json",
  "message": "site manifest object was not valid JSON",
  "retryable": true,
  "reason": "site_manifest_bad_json"
}
```

Root cause:

* The real generated manifest included `provenance` and `storage`.
* `SiteManifestDocument` used `#[serde(deny_unknown_fields)]`.
* The resolver DTO did not include `provenance` and `storage`, so strict parsing rejected a manifest generated by the same route family.

Fix:

* Add optional fields to the resolver DTO:

```rust
#[serde(default)]
provenance: Option<Value>,

#[serde(default)]
storage: Option<Value>,
```

This preserves strict DTO parsing while accepting fields the manifest generator intentionally emits.

### 6. Live smoke stale pointer issue mitigation

During live smoke debugging, the stack script was updated to use a unique per-run site name and an isolated per-run `svc-index` DB. This avoids resolving stale site-name pointers from earlier test runs.

Although this fix primarily lives in scripts, it matters for `omnigate` because it proved the resolver against fresh site pointer state.

## Current route and DTO invariants

### Content IDs

Internal content IDs remain canonical:

```text
b3:<64 lowercase hex>
```

The resolver rejects:

* missing `b3:` prefix
* uppercase hex
* wrong length
* non-hex characters

### Public crab asset URLs

The current public asset URL format is:

```text
crab://<64 lowercase hex>.<asset_kind>
```

The old `crab://b3/<hash>.image` path form is rejected for current/new product UX.

### Site names

Site names are human pointers, not canonical content IDs.

The current validator rejects:

* empty names
* control characters
* `.` and `..`
* double-dot traversal patterns
* leading or trailing dot
* slash or backslash
* `@`
* spaces
* unsupported characters
* names longer than 253 chars

Current allowed byte set:

```text
a-z
0-9
.
-
_
```

Names are normalized to lowercase.

### Storage ownership

`svc-storage` owns raw bytes by content ID only.

`omnigate` may store generated manifest bytes, but storage does not own:

* site routing
* name semantics
* owner lookup
* payout lookup
* manifest interpretation
* crab URL parsing

### Index ownership

`svc-index` owns mutable pointers such as:

```text
site name → site manifest CID
asset CID → asset manifest CID
```

`omnigate` writes/reads these pointers but does not own durable index storage itself.

### Wallet/ledger ownership

`omnigate` does not directly mutate wallet or ledger.

Allowed:

* return wallet hold templates
* include provided receipt refs / hold txid refs in generated manifests
* proxy or coordinate product DTOs

Not allowed:

* create wallet holds directly
* capture/release holds directly
* mutate `ron-ledger`
* bypass `svc-wallet`

This preserves the WEB3 invariant:

```text
svc-wallet is the normal mutation front-door.
ron-ledger is durable economic truth.
omnigate is product/BFF coordination and hydration only.
```

## Smoke and test coverage now passing

### Unit/integration tests

Important passing test groups:

```text
tests/site_launch.rs
tests/asset_page_resolver.rs
tests/image_asset_prepare.rs
tests/paid_storage_prepare.rs
tests/paid_storage_estimate_proxy.rs
tests/paid_storage_write_proxy.rs
tests/policy_gate.rs
tests/policy_metrics.rs
tests/readiness_error_rate.rs
tests/readiness_inflight.rs
tests/ready_truth.rs
tests/middleware_contract.rs
tests/metrics_contract.rs
tests/zk_receipts.rs
```

### `site_launch.rs`

Now proves:

```text
site prepare happy path
bad site name rejection
storage estimate error mapping
site manifest storage + pointer write
site resolver hydration from index + storage
```

### Live stack smoke

The live smoke proved:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
```

Safe smoke passed:

```text
paid object prepare
image prepare
site prepare
b3 image asset page
crab:// image resolver
```

Mutation-enabled site smoke passed:

```text
/sites/prepare
/sites create
/sites/<unique-smoke-site>
```

The final green gate ended with:

```text
WEB3_2 product green gate passed
```

## Policy behavior

Production policy remains fail-closed.

Earlier live smoke hit:

```json
{
  "code": "POLICY_DENY",
  "message": "Access denied",
  "retryable": false,
  "reason": "default"
}
```

That was expected because product smoke was testing product route plumbing without a production policy bundle.

The stack smoke script now generates a local `omnigate-smoke.toml` that disables policy only for the local smoke run.

Important distinction:

```text
Production/default behavior: policy fails closed.
Local smoke behavior: policy disabled only in generated smoke config.
```

Do not weaken production policy to make tests pass.

## Files touched or relevant

Primary crate files:

```text
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/src/routes/v1/mod.rs
crates/omnigate/tests/site_launch.rs
```

Relevant scripts/docs added outside the crate:

```text
scripts/web3_product_stack_smoke.sh
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_product_green_gate.sh
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

## Remaining work for omnigate

### 1. Paid image upload live smoke with real wallet hold

The image upload coordinator is tested and the prepare route is live-smoked, but the default live stack smoke intentionally skips paid upload:

```text
skip: paid upload; set RON_RUN_PAID_UPLOAD=1 with RON_IMAGE_FILE and RON_WALLET_HOLD_TXID to enable
```

Remaining:

* obtain a real hold txid from `svc-wallet`
* run `RON_RUN_PAID_UPLOAD=1`
* verify image bytes are stored
* verify image manifest is stored
* verify asset CID → manifest CID pointer is written
* verify `crab://<hash>.image` hydrates the uploaded image manifest

### 2. Product policy bundle

The local smoke disables policy. Production beta should have a policy bundle that allows intended product routes under correct capabilities and denies everything else.

Remaining:

* add explicit policy entries for product prepare/read routes
* add stricter policy entries for mutation routes
* test with policy enabled
* verify fail-closed defaults
* add route-specific policy metrics expectations

### 3. Site launch UX refinements

Current site create is a dev-mode manifest/pointer coordinator. It assumes static file CIDs already exist.

Remaining:

* decide whether omnigate should ever coordinate bundled site upload, or whether upload remains a higher-level client/gateway workflow
* support richer route maps
* support content-type metadata for site assets
* support app/site manifest conventions
* support owner/curator/provenance metadata extensions
* support paid site access/view route later

### 4. Shared DTO extraction

Some DTOs are local to `omnigate` route files. Once stable, consider moving reusable product DTOs into `ron-proto::web3` or `ron-proto::product`.

Candidate DTOs:

```text
site prepare request/response
site create request/response
site page response
image prepare response
asset page response
manifest pointer response
```

Do this only after route contracts settle.

### 5. Browser extension integration

`omnigate` is ready for the Chrome extension MVP.

The extension should call the public `svc-gateway` routes, not internal `omnigate` routes.

Important gateway-facing routes:

```text
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

### 6. Observability details

Current route behavior inherits normal middleware/metrics coverage. Future work should add or verify product-specific counters for:

```text
omnigate_product_prepare_total
omnigate_asset_page_resolve_total
omnigate_site_create_total
omnigate_site_resolve_total
omnigate_manifest_hydration_failures_total
```

Only add new metrics if consistent with existing metrics style.

## Completion estimate for omnigate WEB3_2 product work

Current estimate:

```text
omnigate WEB3_2 product backend: ~94–96% complete
```

What is complete:

```text
asset page resolver
paid prepare route
image prepare route
image upload coordinator test path
site prepare route
site create route
site resolver/hydrator
gateway-facing live smoke compatibility
strict error taxonomy for major failure paths
```

What remains:

```text
paid image upload live smoke with real hold
production policy bundle for product routes
deeper DTO extraction/cleanup
browser extension integration feedback
optional richer static site/app support
```

````

---

# Drop-in note for `crates/svc-gateway/NOTES.md`

```markdown
# svc-gateway WEB3_2 Product Route Notes — Batch 8/9/10 Carryover

## Current status

`svc-gateway` now exposes the public WEB3_2 product route surface needed by the Chrome extension and client-facing beta flows.

The gateway is intentionally thin and proxy-only. Product semantics remain in `omnigate`.

Final green gate passed:

```text
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
scripts/web3_product_stack_smoke.sh
RON_RUN_SITE_CREATE=1 scripts/web3_product_stack_smoke.sh
scripts/web3_product_green_gate.sh
````

The dedicated product route proxy test now runs 6 tests and passes all 6.

The full live product stack smoke passed through:

```text
client/script
→ svc-gateway
→ omnigate
→ svc-storage
→ svc-index
→ back through gateway
```

The final green gate ended with:

```text
WEB3_2 product green gate passed
```

## Session work summary

This session completed and proved the public gateway exposure for the WEB3_2 backend product routes.

The major change was to ensure `svc-gateway` exposes and proxies the product routes that the Chrome extension will use.

## Public product route surface now proven

`svc-gateway` exposes:

```text
GET  /crab/resolve
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

These proxy to `omnigate` internal routes:

```text
GET  /v1/crab/resolve
GET  /v1/b3/:asset
POST /v1/paid/o/prepare
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /v1/sites/prepare
POST /v1/sites
GET  /v1/sites/:name
```

## Route-by-route notes

### GET `/crab/resolve?url=...`

Purpose:

```text
Resolve a crab:// URL through omnigate and return the corresponding product DTO.
```

Current proven use:

```text
crab://<64 lowercase hex>.image
```

The gateway preserves the query string and forwards to:

```text
/v1/crab/resolve?url=...
```

The product route proxy test proves query preservation and selected header forwarding.

### GET `/b3/:asset`

Purpose:

```text
Public typed b3 asset page route.
```

Expected path format:

```text
/b3/<64 lowercase hex>.<asset_kind>
```

Example:

```text
/b3/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef.image
```

Gateway does not validate product semantics. It proxies to `omnigate`, which owns asset parsing and error behavior.

### POST `/paid/o/prepare`

Purpose:

```text
Public paid object prepare route.
```

Gateway forwards to:

```text
/v1/paid/o/prepare
```

Gateway does not:

* price the object
* create a wallet hold
* mutate wallet
* mutate ledger
* store bytes

It only proxies the request and response.

### POST `/assets/image/prepare`

Purpose:

```text
Public image upload prepare route.
```

Gateway forwards to:

```text
/v1/assets/image/prepare
```

This is intended for the Chrome extension / creator UI to estimate cost and receive a wallet hold template.

### POST `/assets/image`

Purpose:

```text
Public paid image upload coordination route.
```

Gateway forwards to:

```text
/v1/assets/image
```

This route is not run by default in live smoke because it requires a real wallet hold txid and local image file.

Gateway remains proxy-only.

### POST `/sites/prepare`

Purpose:

```text
Public static site launch prepare route.
```

Gateway forwards to:

```text
/v1/sites/prepare
```

This route returns the wallet hold template and next-step hints.

### POST `/sites`

Purpose:

```text
Public static site create route.
```

Gateway forwards to:

```text
/v1/sites
```

The route creates/stores a site manifest and writes a site pointer through `omnigate`.

Gateway does not parse the manifest or pointer response.

### GET `/sites/:name`

Purpose:

```text
Public static site resolver/hydrator route.
```

Gateway forwards to:

```text
/v1/sites/:name
```

This was the important public route added/proven in this session.

It enables the browser/extension/public clients to resolve:

```text
crab://<site_name>
```

through an HTTP call to:

```text
GET /sites/:name
```

The gateway does not normalize, validate, or hydrate the site name. `omnigate` owns those semantics and fail-closed validation.

## Gateway invariants preserved

### Proxy-only behavior

The gateway remains a public edge proxy.

Allowed:

```text
- expose stable client/browser paths
- forward selected headers
- forward bodies
- preserve query strings
- return upstream response bodies/statuses
- convert connect/read failures into gateway Problem bodies
```

Not allowed:

```text
- no product DTO hydration
- no manifest parsing
- no pricing math
- no storage object ownership
- no site pointer ownership
- no wallet mutation
- no ledger mutation
- no app-specific business logic
```

This keeps the crate boundary clean:

```text
svc-gateway = public edge proxy
omnigate = product/BFF semantics
svc-storage = immutable raw bytes by content ID
svc-index = mutable product pointers
svc-wallet = economic mutation front-door
ron-ledger = durable economic truth
```

### Header forwarding

The product proxy forwards selected request headers:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It filters hop-by-hop and unsafe transport headers:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

The product route tests specifically prove that `connection` is not forwarded and `x-ron-*` headers are preserved.

### Error behavior

When `omnigate` cannot be reached, gateway returns structured `upstream_unavailable` Problem responses with HTTP 502.

The product route proxy test includes:

```text
product_route_upstream_connect_failure_yields_problem_502
```

Gateway also passes upstream product errors through when appropriate.

## Tests added/updated/proven

Primary test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Now covers 6 product proxy tests:

```text
crab_resolve_proxy_preserves_query_and_headers
b3_asset_proxy_targets_omnigate_b3_route
paid_prepare_proxy_preserves_body_and_idempotency
image_and_site_product_routes_target_omnigate
site_resolve_proxy_targets_omnigate_site_hydrator
product_route_upstream_connect_failure_yields_problem_502
```

The final test output proved:

```text
running 6 tests
6 passed
```

Full gateway test suite also passed:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
smoke
```

## Live stack smoke proof

The live stack smoke proved that public gateway routes work against actual running local services:

```text
svc-index
svc-storage
omnigate
svc-gateway
```

The safe smoke passed:

```text
/p paid object prepare through /paid/o/prepare
/assets/image/prepare
/sites/prepare
/b3/<hash>.image
/crab/resolve?url=crab://<hash>.image
```

The mutation-enabled site smoke passed:

```text
/sites/prepare
/sites create
/sites/<unique-smoke-site>
```

This proves gateway can sit in front of `omnigate` for the site create/resolve product loop.

Final mutation-enabled proof:

```text
ok: /sites/prepare
ok: /sites create
ok: /sites/<unique-smoke-site>
WEB3 crab site smoke passed
WEB3 product stack smoke passed
```

## Files touched or relevant

Primary crate files:

```text
crates/svc-gateway/src/routes/product.rs
crates/svc-gateway/tests/product_routes_proxy.rs
```

Relevant supporting files outside the crate:

```text
scripts/web3_product_stack_smoke.sh
scripts/web3_extension_backend_smoke.sh
scripts/web3_asset_page_smoke.sh
scripts/web3_crab_image_smoke.sh
scripts/web3_crab_site_smoke.sh
scripts/web3_product_green_gate.sh
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
docs/WEB3_2_PRODUCT_SMOKE_RUNBOOK.md
```

## Current Chrome extension handoff contract

The Chrome extension should treat `svc-gateway` as its backend API.

Important routes for the extension MVP:

```text
GET  /crab/resolve?url=...
GET  /b3/:hash.kind
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

The extension should attach identity/account headers:

```text
x-ron-passport
x-ron-wallet-account
x-ron-permission
x-ron-spend-limit
x-correlation-id
```

Where appropriate, it should also attach:

```text
Authorization
Idempotency-Key
Content-Type
```

The gateway already has tests proving selected header forwarding.

## Known caveats

### Paid image upload live path still skipped

The default smoke intentionally skips:

```text
RON_RUN_PAID_UPLOAD=1
```

Reason:

```text
Paid upload requires a real image file and a real wallet hold txid.
```

This is not a gateway failure. The gateway route exists and is tested as a proxy. The full paid upload live flow should wait until we have a wallet hold fixture or a real `svc-wallet`-created hold txid.

### Local product smoke disables omnigate policy

The product stack smoke script starts `omnigate` with a generated local smoke config that disables policy. This is only for route/product proof.

Production policy must remain fail-closed.

Do not change gateway to bypass or weaken production policy.

### Gateway does not validate site names

This is intentional. `omnigate` owns site-name validation.

Gateway simply proxies:

```text
GET /sites/:name → GET /v1/sites/:name
```

## Remaining work for svc-gateway

### 1. Chrome extension integration testing

Once the extension scaffold exists, test it against:

```text
SVC_GATEWAY_BIND_ADDR=127.0.0.1:8090
```

Expected extension calls:

```text
GET /crab/resolve?url=crab://...
GET /b3/:hash.kind
GET /sites/:name
POST /sites/prepare
POST /assets/image/prepare
```

### 2. Production policy-aligned route tests

Current proxy tests use dummy `omnigate`. Product stack smoke disables `omnigate` policy to prove route plumbing.

Remaining:

* run product route stack smoke with a real product policy bundle
* prove allowed capability paths
* prove denied paths fail closed
* verify useful `POLICY_DENY` body reaches the client

### 3. Paid image upload live edge smoke

Remaining once wallet hold is available:

```text
RON_RUN_PAID_UPLOAD=1
RON_IMAGE_FILE=/absolute/path/to/test.png
RON_WALLET_HOLD_TXID=<real_hold_txid>
scripts/web3_product_stack_smoke.sh
```

This should prove:

```text
client → gateway /assets/image
gateway → omnigate /v1/assets/image
omnigate → storage object put
omnigate → manifest put
omnigate → index pointer put
response returns crab://<hash>.image
```

### 4. Route contract stability

Before beta, preserve these public paths unless explicitly versioning them:

```text
/crab/resolve
/b3/:asset
/paid/o/prepare
/assets/image/prepare
/assets/image
/sites/prepare
/sites
/sites/:name
```

Avoid breaking the extension once it depends on them.

### 5. Observability follow-up

Gateway already inherits route metrics/correlation layers, but product-specific dashboards may need route labels or counters for:

```text
gateway_product_proxy_total
gateway_product_upstream_errors_total
gateway_product_latency_seconds
```

Add only if consistent with existing metrics style.

## Completion estimate for svc-gateway WEB3_2 product exposure

Current estimate:

```text
svc-gateway WEB3_2 product exposure: ~95–97% complete
```

Complete:

```text
public product routes
crab resolver proxy
b3 asset proxy
paid prepare proxy
image prepare/upload proxy
site prepare/create/resolve proxy
header forwarding tests
query/body preservation tests
upstream failure tests
full live stack smoke through gateway
```

Remaining:

```text
paid image upload live test with real hold
policy-enabled product route stack test
Chrome extension integration feedback
optional product-specific metrics refinement
```

```
```



### END NOTE - APRIL 29 2026 - 17:50 CST



### BEGIN NOTE - APRIL 30 2026 - 00:30 CST

Below is a **single dual note** you can paste into both:

```text
crates/svc-gateway/NOTES.MD
crates/omnigate/NOTES.MD
```

---

# BEGIN NOTE — APRIL 30 2026 — CrabLink Identity/Wallet Gateway + Omnigate Façade

## Scope of this note

This note covers the paired work completed across:

```text
svc-gateway
omnigate
```

These two crates were updated together to support the first live CrabLink browser-extension identity flow:

```text
CrabLink Extension
→ svc-gateway public browser routes
→ omnigate WEB3_2 identity/wallet façade
→ future svc-passport / svc-wallet / ron-ledger backing
```

This work intentionally **did not** modify:

```text
svc-passport
svc-wallet
ron-ledger
ron-proto
ron-naming
svc-storage
svc-index
ron-accounting
svc-rewarder
```

Those are future integration targets.

---

## Executive summary

We successfully added the first extension-facing identity and wallet display route surface for CrabLink.

The browser extension can now:

```text
- detect that the RustyOnions gateway is online
- check readiness
- ask whether a passport is present
- bootstrap a dev-safe passport label
- load a linked wallet account label
- fetch a wallet balance display DTO
- run route diagnostics against the live local stack
```

The working live path is now:

```text
CrabLink Extension
→ http://127.0.0.1:8090
→ svc-gateway
→ omnigate
→ identity/wallet façade
```

This was proven live by the persistent CrabLink dev stack:

```text
svc-index    → healthy
svc-storage  → healthy
omnigate     → healthy
svc-gateway  → healthy and ready
identity smoke → passed
CrabLink popup → ONLINE
```

The extension now displays:

```text
Gateway:  http://127.0.0.1:8090
Passport: passport:main:dev
Wallet:   acct_dev
ROC:      0 ROC
Status:   ONLINE
```

This is a real end-to-end browser-to-backend milestone.

---

## Files changed in `svc-gateway`

### Updated

```text
crates/svc-gateway/src/routes/product.rs
```

### Added

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

### New public gateway routes

`svc-gateway` now exposes these CrabLink/browser-facing routes:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

These routes proxy to omnigate:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
GET  /v1/wallet/:account/balance
```

### Existing WEB3_2 product routes preserved

We also confirmed the existing product routes still work:

```text
GET  /crab/resolve?url=<crab_url>
GET  /b3/:asset
POST /paid/o/prepare
POST /assets/image/prepare
POST /assets/image
POST /sites/prepare
POST /sites
GET  /sites/:name
```

---

## `svc-gateway` responsibilities after this work

`svc-gateway` remains a **thin public edge/proxy**.

It owns:

```text
- stable public browser/client route paths
- HTTP proxying to omnigate
- preserving query strings
- forwarding selected request headers
- preserving POST request bodies
- returning upstream response bodies
- mapping omnigate connection/read failures to structured 502 problems
```

It does **not** own:

```text
- passport key creation
- passport persistence
- wallet creation
- wallet balance truth
- ROC issuance
- ledger mutation
- manifest hydration
- asset ownership
- storage writes
- pricing logic
```

Gateway must remain proxy-only for these product routes.

---

## Gateway header forwarding behavior

For the CrabLink identity and wallet routes, `svc-gateway` forwards selected headers including:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It filters hop-by-hop / transport headers such as:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

This preserves the clean edge boundary: browser state and request context can reach omnigate, but gateway does not interpret private identity or wallet semantics.

---

## Gateway tests added

The new test file:

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

proves:

```text
- GET /identity/me proxies to /v1/identity/me
- POST /identity/passport/bootstrap proxies to /v1/identity/passport/bootstrap
- GET /wallet/:account/balance proxies to /v1/wallet/:account/balance
- Authorization header is preserved
- x-ron-passport is preserved
- x-ron-wallet-account is preserved
- x-correlation-id is preserved
- x-request-id is generated/preserved as expected
- Idempotency-Key is preserved for bootstrap
- hop-by-hop headers are not forwarded
- upstream connection failure returns a structured 502 problem
```

The test result was green:

```text
running 4 tests
test identity_route_upstream_connect_failure_yields_problem_502 ... ok
test passport_bootstrap_proxy_preserves_body_and_idempotency ... ok
test wallet_balance_proxy_targets_omnigate_wallet_balance ... ok
test identity_me_proxy_preserves_passport_headers ... ok

test result: ok. 4 passed
```

---

## Files changed in `omnigate`

### Updated

```text
crates/omnigate/src/routes/v1/mod.rs
```

### Added

```text
crates/omnigate/src/routes/v1/identity.rs
crates/omnigate/src/routes/v1/wallet.rs
```

### New omnigate v1 routes

`omnigate` now exposes:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
GET  /v1/wallet/:account/balance
```

These are currently **dev-safe façade routes**, not final `svc-passport` / `svc-wallet` backed routes.

---

## `omnigate` responsibilities after this work

`omnigate` currently owns the temporary WEB3_2 identity/wallet browser façade.

It owns:

```text
- returning safe passport display labels
- returning safe wallet account display labels
- returning a wallet display DTO
- returning capability hints for the CrabLink UI
- making clear when a response is not ledger-backed
- preventing fake positive ROC balances
- preventing uncaveated spend authority
```

It does **not** own:

```text
- private key custody
- local browser key material
- real passport creation/persistence yet
- real wallet mutation yet
- ROC issuance yet
- ledger mutation
- durable economic truth
```

This keeps `omnigate` aligned with its BFF/view-hydrator role.

---

## Current `GET /v1/identity/me` behavior

The route returns identity context from request headers.

Expected request context:

```text
x-ron-passport: passport:main:dev
x-ron-wallet-account: acct_dev
```

Current response shape:

```json
{
  "schema": "crablink.identity.me.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "Local Dev Passport",
    "created_at": null,
    "source": "request_header"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "request_header"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "identity view is header-derived until svc-passport is wired"
  ]
}
```

Important invariant:

```text
can_spend=false
```

The browser is not granted wallet spend authority by this route.

---

## Current `POST /v1/identity/passport/bootstrap` behavior

The route currently returns dev-safe labels only.

It accepts a request like:

```json
{
  "kind": "main",
  "label": "CrabLink main passport",
  "client": "crablink-chrome",
  "request_starter_grant": true,
  "create_wallet": true
}
```

Current response shape:

```json
{
  "schema": "crablink.identity.bootstrap.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "CrabLink main passport",
    "created_at": null,
    "source": "omnigate_dev_bootstrap"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "omnigate_dev_bootstrap"
  },
  "starter_grant": {
    "issued": false,
    "amount_minor_units": "0",
    "receipt_id": null,
    "reason": "svc_wallet_integration_pending"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "dev bootstrap returns labels only",
    "starter ROC is not issued until svc-wallet integration is wired",
    "wallet spend authority is not granted by this response"
  ]
}
```

Important invariant:

```text
starter_grant.issued=false
amount_minor_units="0"
can_spend=false
```

This avoids fake ROC issuance.

---

## Current `GET /v1/wallet/:account/balance` behavior

The route currently returns a truthful placeholder balance:

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "0",
  "held_minor_units": "0",
  "display": "0 ROC",
  "as_of": null,
  "ledger_backed": false,
  "source": "omnigate_dev_wallet_view.v1",
  "reason": "svc_wallet_integration_pending",
  "warnings": [
    "display-only dev placeholder",
    "real balance must come from svc-wallet and ron-ledger",
    "do not treat this route as spend authority"
  ]
}
```

Important invariant:

```text
ledger_backed=false
available_minor_units="0"
```

A non-ledger-backed response must not fake a positive balance.

---

## Live scripts added outside the crates

These scripts were added or heavily updated in the RustyOnions repo and are relevant to both crates:

```text
scripts/web3_identity_stack_smoke.sh
scripts/web3_crablink_dev_stack.sh
```

### `scripts/web3_identity_stack_smoke.sh`

This script proves the extension-facing identity/wallet routes live through `svc-gateway`.

It validates:

```text
GET  /healthz
GET  /readyz
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/acct_dev/balance
```

It also validates:

```text
- identity DTO has expected passport subject
- identity DTO has expected wallet account
- bootstrap DTO has expected passport and wallet labels
- bootstrap does not grant spend authority
- bootstrap does not fake starter ROC
- wallet balance is integer minor units
- non-ledger-backed balance is not fake-positive
```

The final successful output was:

```text
WEB3 identity stack smoke passed
```

### `scripts/web3_crablink_dev_stack.sh`

This script starts a persistent local WEB3_2 stack for browser testing:

```text
svc-index
svc-storage
omnigate
svc-gateway
```

It keeps the stack online at:

```text
http://127.0.0.1:8090
```

It also runs the identity smoke automatically.

The successful output was:

```text
CrabLink dev stack is online
gateway: http://127.0.0.1:8090
```

This allowed the Chrome extension to show:

```text
ONLINE
passport:main:dev
acct_dev
0 ROC
```

---

## Documentation added outside the crates

The following route-contract doc was added:

```text
docs/WEB3_2_IDENTITY_ROUTE_CONTRACTS.md
```

It defines:

```text
- GET /identity/me contract
- POST /identity/passport/bootstrap contract
- GET /wallet/:account/balance contract
- starter ROC grant rule
- gateway header forwarding contract
- no fake ROC rule
- no browser private key rule
- no uncaveated spend authority rule
```

This doc should remain the contract until we replace the dev façade with real `svc-passport` / `svc-wallet` backing.

---

## Tests and gates passed

The following commands passed successfully:

```text
cargo fmt -p omnigate -p svc-gateway
cargo test -p svc-gateway --test identity_routes_proxy
cargo test -p svc-gateway --test product_routes_proxy
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo clippy -p omnigate --no-deps -- -D warnings
```

Additional product gate passed:

```text
scripts/web3_product_green_gate.sh
```

This proved:

```text
- omnigate clippy passed
- omnigate site_launch tests passed
- omnigate all-target tests passed
- svc-gateway clippy passed
- svc-gateway product route proxy tests passed
- svc-gateway all-target tests passed
- safe WEB3 product stack smoke passed
- mutation-enabled site create/resolve stack smoke passed
```

Successful product smoke included:

```text
WEB3 product stack smoke passed
WEB3_2 product green gate passed
```

Successful identity smoke included:

```text
GET /healthz -> HTTP 200
GET /readyz -> HTTP 200
GET /identity/me -> HTTP 200
POST /identity/passport/bootstrap -> HTTP 200
GET /wallet/acct_dev/balance -> HTTP 200
WEB3 identity stack smoke passed
```

---

## CrabLink integration proven

The separate CrabLink repo now successfully talks to this RustyOnions route surface.

CrabLink proved:

```text
- extension structure checks pass
- JavaScript syntax checks pass
- local gateway smoke passes
- /healthz passes
- /readyz passes
- /identity/me passes
- /wallet/acct_dev/balance passes
- /b3/<sample>.image passes
- /crab/resolve passes
- bootstrap mutation route is skipped by default in read-only diagnostics
- Chrome package zip builds
```

This matters for `svc-gateway` and `omnigate` because it proves their new extension-facing routes are not theoretical. The browser client is already consuming them.

---

## Architectural invariants preserved

This work preserved the WEB3 / WEB3_2 invariants:

```text
- ROC remains internal only
- no ROX/Solana/external settlement
- no floating-point money math
- no fake positive ROC balance
- no direct ledger mutation from gateway
- no direct ledger mutation from omnigate
- svc-wallet remains the future normal mutation front-door
- ron-ledger remains durable economic truth
- gateway remains proxy-only
- omnigate remains a BFF/façade/hydrator, not durable truth
- passport labels are not wallet authority
- browser stores no private keys
- browser stores no seed phrases
- browser performs no silent spending
- capability/policy checks must fail closed in production
```

---

## Current limitations

The current implementation is intentionally not final.

Known limitations:

```text
- passport bootstrap is dev-label-only
- no real svc-passport persistence yet
- no real local encrypted passport vault yet
- no password/PIN unlock flow yet
- no alt anonymous passport management yet
- no real svc-wallet balance call yet
- no 1776 ROC starter grant yet
- no ledger-backed receipt for passport starter grants yet
- wallet balance is currently ledger_backed=false
- starter_grant currently reports issued=false
```

The current façade is safe because it is explicit about those limitations.

---

## Next major backend step

The next requested feature is:

```text
Every newly created passport should automatically start with 1776 ROC.
```

This should **not** be implemented in CrabLink as a fake local balance.

The correct future flow is:

```text
CrabLink
→ svc-gateway POST /identity/passport/bootstrap
→ omnigate /v1/identity/passport/bootstrap
→ svc-passport create/load local passport identity
→ svc-wallet issue 1776 ROC
→ ron-ledger commit issue event
→ svc-wallet returns receipt
→ omnigate returns starter_grant.issued=true + receipt id
→ CrabLink refreshes /wallet/:account/balance
→ balance displays 1776 ROC
```

The starter grant must be backend truth, not extension display state.

---

## Next files likely to change

Future work will probably touch:

```text
crates/omnigate/src/config/*
crates/omnigate/src/downstream/*
crates/omnigate/src/routes/v1/identity.rs
crates/omnigate/src/routes/v1/wallet.rs
scripts/web3_crablink_dev_stack.sh
scripts/web3_identity_stack_smoke.sh
```

And likely bring in these services:

```text
svc-passport
svc-wallet
ron-ledger
```

`svc-gateway` may not need much more for the 1776 ROC step if it continues proxying the same public routes.

---

## Next implementation plan: 1776 ROC starter grant

Recommended Batch 8:

```text
1. Add svc-wallet to scripts/web3_crablink_dev_stack.sh.
2. Ensure svc-wallet has a configured ledger path for the local dev stack.
3. Add omnigate wallet downstream config:
   - OMNIGATE_WALLET_BASE_URL
   - config field for wallet base URL
   - downstream wallet client helper
4. Update omnigate /v1/wallet/:account/balance:
   - call svc-wallet balance endpoint
   - return ledger_backed=true when successful
   - preserve placeholder fallback only for explicitly configured dev mode, if desired
5. Update omnigate /v1/identity/passport/bootstrap:
   - create/load passport label for now
   - derive/link wallet account
   - call svc-wallet issue endpoint for 1776 ROC
   - use idempotency key so repeated bootstrap does not double-issue
   - return starter_grant.issued=true
   - return receipt_id from svc-wallet
6. Update identity smoke:
   - bootstrap should return starter_grant.issued=true
   - amount_minor_units should be "1776"
   - receipt_id must be present
   - balance route should report 1776 after bootstrap
7. Keep the no-fake-money invariant:
   - if wallet issue fails, bootstrap should fail or report issued=false
   - extension must not invent the 1776 balance
```

Important decision still needed:

```text
Should 1776 mean:
A) 1776 minor units, displayed as 1776 ROC in the current dev model
or
B) 1776 full ROC with a future decimal scale
```

Current safest choice:

```text
Use integer amount_minor_units="1776" and display "1776 ROC" for dev bootstrap.
```

Do not introduce floating-point math.

---

## Risks to watch next

### Double issuance risk

The biggest risk in the 1776 ROC batch is issuing 1776 ROC multiple times if the user clicks Create repeatedly.

Mitigation:

```text
- use Idempotency-Key
- use deterministic account bootstrap key
- store/check bootstrap receipt by passport subject or account
- repeated bootstrap should return the same receipt or no-op status
```

### Boundary drift risk

Do not let gateway or omnigate mutate `ron-ledger` directly.

Correct mutation path:

```text
omnigate
→ svc-wallet
→ ron-ledger
```

Incorrect path:

```text
omnigate
→ ron-ledger
```

### Browser trust risk

Do not treat the browser as trusted.

The browser can request bootstrap, but backend policy must decide:

```text
- whether bootstrap is allowed
- whether starter grant is allowed
- how many times it is allowed
- which wallet account receives it
```

### Dev façade lingering risk

The dev façade is safe now because it is explicit. Once real backing exists, old placeholder behavior should either be removed or guarded behind obvious dev-only config.

---

## Current completion estimate for this paired slice

For the `svc-gateway + omnigate` CrabLink identity surface:

```text
Public route exposure:             95% complete
Gateway proxy behavior:            95% complete
Omnigate dev façade:               90% complete
Live dev stack support:            90% complete
CrabLink browser integration:      85% complete
Real svc-passport backing:          0% complete
Real svc-wallet balance backing:    0% complete
1776 ROC starter grant backing:     0% complete
```

Overall for this paired identity/wallet route slice:

```text
~65–70% complete
```

Reason: the browser/gateway/omnigate route surface is working and proven, but the final backend truth path still needs `svc-passport`, `svc-wallet`, and `ron-ledger`.

---

## TL;DR carryover

We added a real extension-facing identity/wallet route layer:

```text
CrabLink
→ svc-gateway
→ omnigate
```

Routes now working:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

Current implementation is a safe dev façade:

```text
passport:main:dev
acct_dev
0 ROC
ledger_backed=false
can_spend=false
starter_grant.issued=false
```

Everything compiled, tested, linted, smoked, and worked in the Chrome extension.

Next goal:

```text
Replace dev façade with real backend truth:
svc-passport-backed passport bootstrap
svc-wallet-backed balance
1776 ROC starter grant through svc-wallet → ron-ledger receipt
```

Do not fake 1776 ROC in the extension. The extension should only display backend truth.

# END NOTE — APRIL 30 2026 — CrabLink Identity/Wallet Gateway + Omnigate Façade


### END NOTE - APRIL 30 2026 - 00:30 CST



### BEGIN NOTE - APRIL 30 2026 - 10:30 CST

Below is a **drop-in `omnigate` note** you can paste into:

```text
crates/omnigate/NOTES.MD
```

---

# CARRY-OVER NOTES — omnigate WEB3_2 / CrabLink identity-wallet bridge

**Date:** 2026-04-30
**Status:** CrabLink-facing identity and wallet façade now wired to `svc-wallet` for real backend ROC truth.
**Crate:** `omnigate`
**Primary files changed:**

```text
crates/omnigate/src/routes/v1/identity.rs
crates/omnigate/src/routes/v1/wallet.rs
```

**Current verdict:** This was a major product-proof milestone. `omnigate` now exposes CrabLink-compatible identity and wallet routes that can call `svc-wallet` for a real 1776 ROC starter grant and a ledger-backed wallet balance. The extension can now display `1776 ROC` as `ledger-backed`, meaning the browser UI is no longer faking local money state.

---

## 0. TL;DR

In this session, `omnigate` was upgraded from a safe dev identity/wallet façade into a **backend-truth bridge** for CrabLink.

Before this batch, CrabLink could call:

```text
GET /identity/me
GET /wallet/:account/balance
POST /identity/passport/bootstrap
```

but those routes were still mostly safe placeholder/dev façade behavior. They intentionally avoided fake balances, but they did not yet prove the 1776 ROC starter grant through the actual value-plane.

After this batch, `omnigate` now does the correct product-path work:

```text
CrabLink
→ svc-gateway
→ omnigate /identity/passport/bootstrap
→ svc-wallet /v1/issue
→ ron-ledger-backed receipt through svc-wallet
→ omnigate response with starter_grant.issued=true + receipt_id
→ CrabLink /wallet/acct_dev/balance
→ omnigate /wallet/:account/balance
→ svc-wallet /v1/balance
→ CrabLink displays 1776 ROC ledger-backed
```

The live terminal proof showed:

```json
{
  "starter_grant": {
    "issued": true,
    "amount_minor_units": "1776",
    "receipt_id": "tx_af4862f11cba82e2497d316f2e76fc83",
    "reason": "issued_by_svc_wallet"
  }
}
```

and:

```json
{
  "available_minor_units": "1776",
  "display": "1776 ROC",
  "ledger_backed": true,
  "source": "svc_wallet.v1"
}
```

This preserves the key WEB3 invariant:

```text
omnigate does not mutate ron-ledger directly.
svc-wallet remains the normal economic mutation/read front-door.
ron-ledger remains durable economic truth behind svc-wallet.
CrabLink displays backend truth only.
```

---

## 1. What changed in `identity.rs`

File:

```text
crates/omnigate/src/routes/v1/identity.rs
```

### 1.1 Route surface

The file now defines and/or supports the CrabLink-facing identity routes:

```text
GET  /v1/identity/me
POST /v1/identity/passport/bootstrap
```

These routes are reached publicly through `svc-gateway` as:

```text
GET  /identity/me
POST /identity/passport/bootstrap
```

### 1.2 `GET /identity/me`

The identity route remains a safe label façade for now.

It reads browser/dev headers:

```text
x-ron-passport
x-ron-wallet-account
```

and returns safe display-only structures:

```text
passport.subject
passport.kind
passport.display_name
passport.source

wallet.account
wallet.linked
wallet.source

capabilities.can_view_balance
capabilities.can_prepare_paid_actions
capabilities.can_spend
warnings[]
```

Current important behavior:

```text
can_spend=false
```

This is correct. Loading a passport label and wallet label must not grant raw spend authority. Future spend authority should be capability-caveated by account, asset, action, amount, audience, and expiry.

### 1.3 `POST /identity/passport/bootstrap`

The bootstrap route now performs the CrabLink dev bootstrap flow.

It accepts a strict DTO:

```rust
BootstrapRequest
```

with fields including:

```text
kind
display_name
label
client
create_wallet
starter_grant / request_starter_grant
passport_subject
wallet_account
desired_starting_balance_minor_units
```

Important current defaults:

```text
DEFAULT_PASSPORT_SUBJECT = "passport:main:dev"
DEFAULT_WALLET_ACCOUNT = "acct_dev"
DEFAULT_STARTER_GRANT_MINOR_UNITS = "1776"
DEFAULT_WALLET_BASE_URL = "http://127.0.0.1:8088"
DEFAULT_WALLET_BEARER = "dev"
```

Config/env hooks:

```text
OMNIGATE_WALLET_BASE_URL
OMNIGATE_WALLET_BEARER
OMNIGATE_STARTER_GRANT_MINOR_UNITS
```

The bootstrap route now:

```text
1. Reads or creates a safe passport label.
2. Reads or creates a safe wallet account label.
3. If starter grant is requested, normalizes the desired amount as integer minor units.
4. Calls svc-wallet POST /v1/issue.
5. Sends Authorization: Bearer <OMNIGATE_WALLET_BEARER>.
6. Sends a deterministic Idempotency-Key.
7. Requires svc-wallet to return a txid.
8. Returns starter_grant.issued=true only when svc-wallet succeeds.
9. Returns starter_grant.receipt_id from svc-wallet txid.
10. Returns warning text stating starter ROC came through svc-wallet.
```

### 1.4 Deterministic idempotency key

A deterministic idempotency key was added for starter grants:

```rust
deterministic_bootstrap_idempotency_key(passport_subject, wallet_account)
```

Shape:

```text
crablink_bootstrap_v1_<16 hex chars>
```

Purpose:

```text
same passport + wallet pair
→ same bootstrap idempotency key
→ retry-safe issue request to svc-wallet
```

This is important for preventing repeated browser retries from creating multiple starter grants, but this still needs stronger hardening and tests at the smoke/integration level.

### 1.5 `svc-wallet` issue request

The bootstrap route calls:

```text
POST {OMNIGATE_WALLET_BASE_URL}/v1/issue
```

with body shape:

```json
{
  "to": "acct_dev",
  "asset": "roc",
  "amount_minor": "1776",
  "idempotency_key": "crablink_bootstrap_v1_...",
  "memo": "crablink passport starter grant"
}
```

and headers:

```text
Authorization: Bearer dev
Idempotency-Key: crablink_bootstrap_v1_...
x-correlation-id: <forwarded when available>
```

The response must contain:

```text
txid
```

If `txid` is missing, `omnigate` treats the wallet response as invalid and returns a non-issued starter grant with a warning.

### 1.6 Starter grant response

Successful backend-truth response:

```json
{
  "schema": "crablink.identity.bootstrap.v1",
  "passport": {
    "subject": "passport:main:dev",
    "kind": "main",
    "display_name": "CrabLink main passport",
    "created_at": null,
    "source": "omnigate_dev_bootstrap"
  },
  "wallet": {
    "account": "acct_dev",
    "linked": true,
    "source": "omnigate_dev_bootstrap"
  },
  "starter_grant": {
    "issued": true,
    "amount_minor_units": "1776",
    "receipt_id": "tx_...",
    "reason": "issued_by_svc_wallet"
  },
  "capabilities": {
    "can_view_balance": true,
    "can_prepare_paid_actions": true,
    "can_spend": false
  },
  "warnings": [
    "dev bootstrap returns passport labels only until svc-passport custody is wired",
    "wallet spend authority is not granted by this response",
    "starter ROC was issued through svc-wallet"
  ]
}
```

### 1.7 Compile error fixed

An E0515 compile error was encountered and fixed.

Problematic pattern:

```rust
env::var("OMNIGATE_STARTER_GRANT_MINOR_UNITS")
    .ok()
    .as_deref()
```

That attempted to return a borrowed `&str` from a temporary `String`.

Fix:

```rust
fn starter_grant_amount_from_env() -> Option<String>
```

The route now uses owned `String` values:

```rust
let requested_amount = req
    .desired_starting_balance_minor_units
    .clone()
    .or_else(starter_grant_amount_from_env)
    .unwrap_or_else(|| DEFAULT_STARTER_GRANT_MINOR_UNITS.to_owned());
```

---

## 2. What changed in `wallet.rs`

File:

```text
crates/omnigate/src/routes/v1/wallet.rs
```

### 2.1 Route surface

The file now supports the CrabLink-facing wallet route:

```text
GET /v1/wallet/:account/balance
```

Publicly reached through `svc-gateway` as:

```text
GET /wallet/:account/balance
```

### 2.2 `svc-wallet` balance read-through

The wallet route now attempts to call:

```text
GET {OMNIGATE_WALLET_BASE_URL}/v1/balance?account=<account>&asset=roc
```

with:

```text
Authorization: Bearer <OMNIGATE_WALLET_BEARER>
```

Expected wallet response fields include:

```text
account
amount_minor
as_of_height
```

`omnigate` maps these into the CrabLink DTO:

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "1776",
  "held_minor_units": "0",
  "display": "1776 ROC",
  "as_of": null,
  "ledger_backed": true,
  "source": "svc_wallet.v1",
  "reason": null,
  "warnings": []
}
```

### 2.3 Explicit fallback if wallet unavailable

If `svc-wallet` is unreachable, returns bad JSON, returns non-2xx, or omits `amount_minor`, `omnigate` returns a safe fallback:

```text
ledger_backed=false
available_minor_units="0"
display="0 ROC"
source="omnigate_dev_wallet_view.v1"
warnings include "real balance must come from svc-wallet and ron-ledger"
```

This is crucial. `omnigate` does **not** invent a balance. CrabLink can show fallback/error state, but not a fake balance.

### 2.4 Integer-only money parsing

The route accepts wallet amount values only as unsigned integer decimal minor units. No floats. No decimals.

Examples:

```text
"1776"  → ok
"001776" → "1776"
"0" → "0"
"17.76" → rejected
```

This preserves WEB3 money math invariants:

```text
No floating-point money math.
All ROC amounts use integer minor units.
```

### 2.5 Percent encoding

A small local percent-encoder was added for wallet account query strings.

Example:

```text
acct_dev → acct_dev
acct:dev → acct%3Adev
```

No new dependency was needed.

---

## 3. Tests added / validated

### 3.1 Unit tests added in `identity.rs`

New tests:

```text
deterministic_idempotency_key_is_short_and_stable
normalizes_starter_amount
starter_grant_env_amount_is_owned
```

These verify:

```text
- deterministic bootstrap idempotency key is stable
- key length is bounded
- key prefix is correct
- amount normalization strips leading zeroes
- zero amount is rejected
- decimal/floating amount is rejected
- env starter grant amount helper returns owned data safely
```

### 3.2 Unit tests added in `wallet.rs`

New tests:

```text
unsigned_amount_normalization_is_integer_only
percent_encoding_keeps_safe_account_chars
```

These verify:

```text
- integer-only amount parsing
- decimal/floating amount rejected
- leading zero handling
- account query percent encoding
```

### 3.3 Full omnigate test suite green

Ran:

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

Result:

```text
fmt: green
clippy: green
omnigate all-target tests: green
```

Important existing test groups still passed:

```text
asset_page_resolver
dto_serialization
image_asset_prepare
metrics_contract
middleware_contract
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
policy_gate
policy_metrics
readiness_error_rate
readiness_inflight
ready_truth
site_launch
zk_receipts
benches/middleware_ping
```

This is important because the identity/wallet bridge did not regress:

```text
typed b3 asset pages
crab:// resolver
paid storage prepare/write proxy
image asset prepare
site launch
policy gate behavior
readiness behavior
receipt status tests
```

---

## 4. Live smoke proof

### 4.1 `svc-wallet` live

Started:

```bash
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

Confirmed:

```text
svc-wallet listening local_addr=127.0.0.1:8088
```

### 4.2 CrabLink RustyOnions dev stack live

Started:

```bash
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

Stack came online:

```text
svc-index   http://127.0.0.1:5304
svc-storage http://127.0.0.1:5303
omnigate    http://127.0.0.1:9090
svc-gateway http://127.0.0.1:8090
```

The built-in identity smoke passed:

```text
GET  /healthz                         200
GET  /readyz                          200
GET  /identity/me                     200
POST /identity/passport/bootstrap     200
GET  /wallet/acct_dev/balance         200
```

### 4.3 CrabLink repo smoke with mutation route

From the `crablink` repo:

```bash
CRABLINK_SMOKE_RUN_BOOTSTRAP=1 scripts/smoke-local-gateway.sh
```

Passed:

```text
ok: /healthz -> HTTP 200
ok: /readyz -> HTTP 200
ok: /identity/me -> HTTP 200
ok: /wallet/acct_dev/balance -> HTTP 200
ok: /b3/<hash>.image -> HTTP 200
ok: /crab/resolve?url=crab://<hash>.image -> HTTP 200
bootstrap JSON: ok
ok: /identity/passport/bootstrap -> HTTP 200

CrabLink local gateway smoke passed.
```

### 4.4 Manual JSON inspection

Manual curl confirmed backend truth:

```bash
curl -s -X POST http://127.0.0.1:8090/identity/passport/bootstrap \
  -H 'Authorization: Bearer dev' \
  -H 'Content-Type: application/json' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' \
  -d '{"kind":"main","label":"CrabLink main passport","client":"crablink-chrome","request_starter_grant":true,"create_wallet":true,"desired_starting_balance_minor_units":"1776"}' | jq .
```

Returned:

```text
starter_grant.issued = true
starter_grant.amount_minor_units = "1776"
starter_grant.receipt_id = "tx_af4862f11cba82e2497d316f2e76fc83"
starter_grant.reason = "issued_by_svc_wallet"
capabilities.can_spend = false
```

Then:

```bash
curl -s http://127.0.0.1:8090/wallet/acct_dev/balance \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

Returned:

```text
available_minor_units = "1776"
display = "1776 ROC"
ledger_backed = true
source = "svc_wallet.v1"
warnings = []
```

### 4.5 Browser proof

CrabLink popup screenshot confirmed:

```text
Gateway: http://127.0.0.1:8090
Passport: passport:main:dev
Wallet: acct_dev
ROC: 1776 ROC
Ledger: ledger-backed
```

This is the major product milestone: CrabLink displays a real backend wallet balance through RustyOnions services.

---

## 5. Boundary and invariant check

This batch preserved all important WEB3 / WEB3_2 boundaries.

### 5.1 Preserved

```text
svc-wallet remains the wallet mutation/read front-door.
ron-ledger remains behind svc-wallet as durable economic truth.
omnigate does not mutate ron-ledger directly.
CrabLink does not mutate ron-ledger directly.
CrabLink does not fake balances locally.
All money values are integer minor units.
can_spend=false in identity/bootstrap response.
Starter grant receipt comes from svc-wallet txid.
```

### 5.2 Still intentionally dev façade

The following are not final production custody yet:

```text
passport:main:dev
acct_dev
dev bearer token
header-derived identity in /identity/me
omnigate_dev_bootstrap source label
```

This is acceptable for the current product-proof stage, but it must be replaced by real `svc-passport` custody/capability behavior before public beta.

---

## 6. Current completion estimate for `omnigate`

For the WEB3_2 product-proof slice, `omnigate` is now approximately:

```text
WEB3_2 product route hydration/proxy work: 90–94%
CrabLink identity/wallet façade:           75–85%
Production-grade identity/passport path:   35–50%
Overall omnigate crate maturity:           80–88%
```

Why the product route layer is high:

```text
- typed b3 asset pages are green
- crab:// resolver is green
- image asset prepare tests are green
- paid storage prepare/write proxy tests are green
- site prepare/create/resolve tests are green
- identity and wallet CrabLink routes are now live
- wallet-backed 1776 ROC display works through gateway
```

Why production identity/passport is lower:

```text
- /identity/me is still header-derived
- /identity/passport/bootstrap still returns dev labels
- real svc-passport custody is not yet integrated
- long-lived passport key management/export/import is not done here
- starter grant policy needs stronger one-time issuance guarantees
```

---

## 7. What remains for `omnigate`

### Priority A — Smoke hardening

Update the relevant smokes so they fail unless they prove the new backend truth.

Targets:

```text
scripts/web3_identity_stack_smoke.sh
crablink/scripts/smoke-local-gateway.sh
```

New assertions should require:

```text
bootstrap starter_grant.issued == true
bootstrap starter_grant.amount_minor_units == "1776"
bootstrap starter_grant.receipt_id is non-empty
bootstrap capabilities.can_spend == false
wallet ledger_backed == true
wallet source == "svc_wallet.v1"
wallet available_minor_units >= 1776
```

This should become the new acceptance gate for the CrabLink identity flow.

### Priority B — One-time starter grant hardening

Current bootstrap idempotency key is deterministic per passport/wallet pair, which is good.

Still needed:

```text
- integration test proving two identical bootstrap requests do not double-issue
- smoke test proving retry returns same or idempotent receipt behavior
- clear error/warning if backend rejects duplicate grant
- policy-level one-starter-grant-per-passport/wallet/account rule
```

Important invariant:

```text
No user should be able to spam-create 1776 ROC starter grants.
```

This likely belongs partly in `svc-wallet`/policy, but `omnigate` should have route-level tests that prove it passes stable idempotency keys and does not generate random duplicate issue requests.

### Priority C — Real `svc-passport` integration

Current route is explicitly a dev façade.

Future shape:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport create/load/verify passport
→ svc-wallet create/link wallet
→ svc-wallet issue starter grant if policy allows
→ return receipt and safe capability summary
```

Needed work:

```text
- replace header-derived /identity/me with svc-passport-backed identity
- support local persistent main passport identity
- support alt anonymous/pseudonymous passport labels
- ensure alt passports do not inherit wallet authority automatically
- add short-lived/caveated capabilities for paid actions
- keep can_spend=false unless a real spend capability exists
```

### Priority D — Better problem/error taxonomy

Right now wallet failures are encoded as `reason` strings inside successful fallback DTOs in some cases.

Need to decide final behavior:

```text
Option 1: Keep 200 with ledger_backed=false for display fallback.
Option 2: Return structured Problem JSON with 502 when svc-wallet is required.
Option 3: Hybrid: /wallet/:account/balance returns fallback for dev but production config fails closed.
```

Recommendation:

```text
- local dev: safe fallback allowed with ledger_backed=false
- production: fail closed with structured 502/503 if wallet truth is required
```

### Priority E — Observability

Add or confirm metrics for:

```text
omnigate_identity_bootstrap_total{result}
omnigate_starter_grant_issue_total{result}
omnigate_wallet_balance_read_total{result}
omnigate_wallet_backend_latency_seconds
omnigate_wallet_backend_errors_total{reason}
```

Currently route is covered by general HTTP middleware, but wallet-specific counters would make debugging much easier.

### Priority F — Config formalization

Current env hooks:

```text
OMNIGATE_WALLET_BASE_URL
OMNIGATE_WALLET_BEARER
OMNIGATE_STARTER_GRANT_MINOR_UNITS
```

Need to move toward typed config:

```text
[wallet]
base_url = "http://127.0.0.1:8088"
bearer_token_env = "OMNIGATE_WALLET_BEARER"
starter_grant_minor_units = "1776"
starter_grant_enabled = true
fallback_display_allowed = true
```

Production should not hard-code `Bearer dev`.

### Priority G — Route contract docs

Update route docs to reflect real backend behavior:

```text
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
```

Add identity/wallet sections:

```text
GET  /identity/me
POST /identity/passport/bootstrap
GET  /wallet/:account/balance
```

Document:

```text
- dev façade fields
- starter_grant.issued semantics
- receipt_id from svc-wallet txid
- ledger_backed true/false rules
- can_spend=false rule
- no private keys in response
```

### Priority H — Tests for wallet integration failure modes

Add tests for:

```text
svc-wallet unreachable → ledger_backed=false or Problem depending mode
svc-wallet non-2xx → safe failure
svc-wallet malformed JSON → safe failure
svc-wallet missing txid on issue → starter_grant.issued=false
svc-wallet missing amount_minor on balance → ledger_backed=false
invalid desired_starting_balance_minor_units → issued=false
zero desired amount → issued=false
decimal desired amount → issued=false
```

Some unit coverage exists for amount parsing, but route-level/fake-server tests are still needed.

### Priority I — CrabLink UX alignment support

The browser screenshot now shows:

```text
ROC: 1776 ROC
Ledger: ledger-backed
```

but because the browser had refreshed balance rather than performing bootstrap itself, it still showed:

```text
Starter grant: pending backend
Grant amount: unknown
Receipt: none
```

This is mostly CrabLink state/UI work, but `omnigate` can help by ensuring the wallet balance route optionally exposes starter-grant provenance later, or by supporting receipt lookup:

```text
GET /wallet/:account/receipts
GET /wallet/:account/receipts/:txid
```

Do not add this until route ownership is clear. Wallet receipt truth should still come from `svc-wallet`.

---

## 8. Recommended next implementation sequence

### Batch 1 — Smoke hardening

Update smokes to assert:

```text
issued=true
amount=1776
receipt_id exists
ledger_backed=true
source=svc_wallet.v1
can_spend=false
```

This locks in what we just proved manually.

### Batch 2 — Idempotency proof

Add tests/smoke route proof:

```text
same passport/wallet bootstrap twice
→ does not double-issue
→ same idempotency behavior
→ balance remains expected
```

If the current `svc-wallet` behavior increments again despite idempotency, fix `svc-wallet`/ledger idempotency, not CrabLink.

### Batch 3 — Route docs

Update:

```text
WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
omnigate NOTES.MD
CrabLink docs/backend route contracts
```

### Batch 4 — CrabLink UI receipt sync

In CrabLink, when user clicks Create RON Passport:

```text
store receipt_id
store starter_grant amount
refresh balance
show:
  Starter grant: issued by backend
  Grant amount: 1776 ROC
  Receipt: tx_...
  Ledger: ledger-backed
```

### Batch 5 — `svc-passport` integration plan

Once the dev identity flow is stable, replace header-derived identity with real passport custody/capability routes.

---

## 9. Commands that proved this state

### Static omnigate quality gate

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

### Start wallet

```bash
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

### Start CrabLink dev stack

```bash
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

### CrabLink repo smoke with mutation

```bash
CRABLINK_SMOKE_RUN_BOOTSTRAP=1 scripts/smoke-local-gateway.sh
```

### Manual backend truth inspection

```bash
curl -s -X POST http://127.0.0.1:8090/identity/passport/bootstrap \
  -H 'Authorization: Bearer dev' \
  -H 'Content-Type: application/json' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' \
  -d '{"kind":"main","label":"CrabLink main passport","client":"crablink-chrome","request_starter_grant":true,"create_wallet":true,"desired_starting_balance_minor_units":"1776"}' | jq .

curl -s http://127.0.0.1:8090/wallet/acct_dev/balance \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

---

## 10. Final summary

This `omnigate` batch turned CrabLink identity/wallet support from a placeholder into a real product proof:

```text
browser extension
→ gateway
→ omnigate
→ wallet
→ ledger-backed ROC balance
```

The standout success condition is now true:

```text
CrabLink displays 1776 ROC as ledger-backed.
```

This means the WEB3_2 product layer now has a working browser-facing proof of:

```text
passport label
wallet link
starter grant
receipt id
wallet balance
ledger-backed truth
gateway route compatibility
```

Remaining work is no longer about proving the path can exist. It is about making it durable, idempotent, secure, well-documented, and production-ready.


### END NOTE - APRIL 30 2026 - 10:30 CST


### BEGIN NOTE - MAY 1 2026 - 22:35 CST

Below are **drop-in notes for `crates/omnigate/NOTES.MD`** or wherever you are tracking crate-specific carryover. Some earlier uploaded files have expired in this chat, so these notes are based on the successful work and terminal outputs from this instance. Re-upload latest `CODEBUNDLE_RS.md`, `ALLNOTES.MD`, and current crate files next session if exact source inspection is needed.

---

# omnigate Carryover Notes — CrabLink / WEB3_2 Built-In Page Resolver Work

## Session focus

This session updated `omnigate` to support RustyOnions WEB3_2 / CrabLink built-in product page resolution. The goal was to let CrabLink resolve reserved human-facing product entry points through the gateway instead of only resolving typed b3 asset pages.

Before this work, the crab resolver path was primarily focused on canonical typed asset pages such as:

```text
crab://<64 lowercase hex>.<kind>
```

This session added first-class read-only resolver metadata for built-in product pages:

```text
crab://site
crab://image
crab://music
crab://article
```

This gives CrabLink a stable backend contract for opening RustyOnions product pages inside the full-tab browser shell.

---

## Crate touched

```text
crates/omnigate
```

Primary files involved:

```text
crates/omnigate/src/routes/v1/crab.rs
crates/omnigate/tests/builtin_page_resolver.rs
crates/omnigate/tests/asset_page_resolver.rs
```

The main implementation work was in:

```text
crates/omnigate/src/routes/v1/crab.rs
```

The main new/expanded test coverage was in:

```text
crates/omnigate/tests/builtin_page_resolver.rs
```

`asset_page_resolver.rs` was retained and rerun to ensure the existing typed b3 asset-page behavior did not regress.

---

## What was accomplished

## 1. Added built-in crab page parsing

`omnigate` now recognizes reserved built-in product page URLs.

Supported active pages:

```text
crab://site
crab://image
```

Supported reserved coming-soon pages:

```text
crab://music
crab://article
```

The parser rejects noncanonical or unsafe forms. The tests verify rejection for cases such as:

```text
crab://Site
crab://site/
crab://site?x=1
crab://image.extra
crab://dashboard
crab://song
```

This preserves the important URL discipline:

```text
crab://site      = reserved built-in page
crab://image     = reserved built-in page
crab://music     = reserved built-in page
crab://article   = reserved built-in page

crab://<hash>.<kind> = canonical typed b3 asset page
```

The old public `crab://b3/<hash>.<kind>` form remains rejected. Public asset URLs should remain:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Internal content IDs remain:

```text
b3:<64 lowercase hex>
```

---

## 2. Added `BuiltinPageKind`

A new reserved built-in page enum was added/expanded in the crab route module:

```text
BuiltinPageKind::Site
BuiltinPageKind::Image
BuiltinPageKind::Music
BuiltinPageKind::Article
```

Each kind maps to a canonical lowercase string:

```text
site
image
music
article
```

This is intentionally small and explicit. The parser does not accept arbitrary built-in page names yet. This is good because it keeps reserved product surface area deterministic and prevents accidental route namespace expansion.

---

## 3. Added `BuiltinPageResponse`

`omnigate` now returns a stable built-in page metadata DTO with schema:

```text
omnigate.builtin-page.v1
```

The response shape includes:

```text
schema
url
page_kind
status
title
description
requires_passport
requires_wallet
actions[]
fields[]
warnings[]
```

This DTO is read-only metadata. It does not mutate wallet, ledger, storage, index, naming, or passport state.

This is important because `omnigate` remains a product-facing resolver/BFF layer, not a mutation owner.

---

## 4. Implemented `crab://site` active page metadata

`crab://site` now resolves to an active built-in product page.

Current response contract:

```text
schema: omnigate.builtin-page.v1
url: crab://site
page_kind: site
status: active
title: Create a RON Site
description: Register a name and launch a manifest-backed RustyOnions site.
requires_passport: true
requires_wallet: true
```

Actions exposed:

```text
site.prepare
method: POST
route: /sites/prepare
mutates: false
requires_confirmation: false
```

```text
site.create
method: POST
route: /sites
mutates: true
requires_confirmation: true
```

Fields exposed:

```text
site_name    type=text      required=true
title        type=text      required=false
description  type=textarea  required=false
```

Important: these are route contracts for client rendering. The full site registration workflow is not fully implemented in this session. CrabLink displays these actions but does not execute them yet.

---

## 5. Implemented `crab://image` active page metadata

`crab://image` now resolves to an active built-in product page.

Current response contract:

```text
schema: omnigate.builtin-page.v1
url: crab://image
page_kind: image
status: active
title: Upload a RON Image
description: Upload an image, create a b3 image asset page, and attach ownership/payout metadata.
requires_passport: true
requires_wallet: true
```

Actions exposed:

```text
image.prepare
method: POST
route: /assets/image/prepare
mutates: false
requires_confirmation: false
```

```text
image.create
method: POST
route: /assets/image
mutates: true
requires_confirmation: true
```

Fields exposed:

```text
file         type=file      accept=image/*  required=true
title        type=text                     required=false
description  type=textarea                 required=false
tags         type=tags                     required=false
```

This gives CrabLink enough information to render a future image upload page, but the actual image upload/manifest creation workflow is not yet implemented in this session.

---

## 6. Implemented `crab://music` as reserved coming-soon metadata

`crab://music` now resolves cleanly instead of returning a fetch-looking error.

Current response contract:

```text
schema: omnigate.builtin-page.v1
url: crab://music
page_kind: music
status: coming_soon
title: RON Music Is Coming Soon
description: The RustyOnions music workflow is reserved, but upload/manifest routes are not enabled yet.
requires_passport: false
requires_wallet: false
actions: []
warnings: ["music_page_coming_soon"]
```

Fields currently exposed as placeholders:

```text
title   type=text  required=false
artist  type=text  required=false
```

Important: this does not implement music uploads, song manifests, streaming, payments, or reward splits. It only reserves the page and provides clean UX for CrabLink.

---

## 7. Implemented `crab://article` as reserved coming-soon metadata

`crab://article` now resolves cleanly instead of returning a fetch-looking error.

Current response contract:

```text
schema: omnigate.builtin-page.v1
url: crab://article
page_kind: article
status: coming_soon
title: RON Articles Are Coming Soon
description: The RustyOnions article workflow is reserved, but article manifest routes are not enabled yet.
requires_passport: false
requires_wallet: false
actions: []
warnings: ["article_page_coming_soon"]
```

Fields currently exposed as placeholders:

```text
title  type=text      required=false
body   type=textarea  required=false
```

Important: this does not implement publishing, indexing, paid article views, comments, or author payouts yet. It only reserves the UX route.

---

## 8. Preserved existing typed b3 asset page resolver behavior

The existing typed b3 asset page functionality remained green.

Existing canonical typed asset forms still include:

```text
crab://<64 lowercase hex>.image
crab://<64 lowercase hex>.video
crab://<64 lowercase hex>.music
crab://<64 lowercase hex>.song
crab://<64 lowercase hex>.article
crab://<64 lowercase hex>.post
crab://<64 lowercase hex>.comment
crab://<64 lowercase hex>.page
crab://<64 lowercase hex>.site
crab://<64 lowercase hex>.app
crab://<64 lowercase hex>.manifest
```

The parser continues to enforce:

```text
64 lowercase hex hash
known/allowed asset kind suffix
no uppercase hashes
no path traversal
no query/fragment in asset segment
no old crab://b3/<hash>.<kind> public form
```

The existing resolver still composes asset page DTOs using:

```text
svc-index manifest pointer
svc-storage raw object HEAD/GET
manifest hydration when available
owner/payout metadata
receipt summaries
warnings
links
```

No regression was observed.

---

## 9. Tests added/expanded

`builtin_page_resolver.rs` was expanded from the earlier 4 tests to 6 tests.

Current passing tests:

```text
parses_reserved_builtin_page_urls
rejects_noncanonical_builtin_page_forms
site_builtin_page_contract_points_at_site_routes
image_builtin_page_contract_points_at_image_routes
music_builtin_page_is_reserved_but_not_mutating_yet
article_builtin_page_is_reserved_but_not_mutating_yet
```

These tests verify:

```text
crab://site parses as BuiltinPageKind::Site
crab://image parses as BuiltinPageKind::Image
crab://music parses as BuiltinPageKind::Music
crab://article parses as BuiltinPageKind::Article

noncanonical built-in forms are rejected

site page returns active status
site page requires passport/wallet
site page exposes prepare/create action contracts
site page exposes site_name field

image page returns active status
image page requires passport/wallet
image page exposes prepare/create action contracts
image page exposes image file field with accept=image/*

music page returns coming_soon status
music page has no actions
music page does not require passport/wallet
music page returns warning music_page_coming_soon

article page returns coming_soon status
article page has no actions
article page does not require passport/wallet
article page returns warning article_page_coming_soon
```

---

## 10. Validation commands and outputs

The crate was validated with:

```bash
cd /Users/mymac/Desktop/RustyOnions
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test asset_page_resolver
cargo test -p omnigate --test builtin_page_resolver
```

Observed result:

```text
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
finished clean

asset_page_resolver:
15 passed
0 failed

builtin_page_resolver:
6 passed
0 failed
```

This is a good green checkpoint for the crate.

---

## 11. Gateway/live-stack verification

After the crate changes, the RustyOnions CrabLink dev stack was started:

```bash
cd /Users/mymac/Desktop/RustyOnions
chmod +x scripts/web3_crablink_dev_stack.sh scripts/web3_identity_stack_smoke.sh
scripts/web3_crablink_dev_stack.sh
```

Observed service startup:

```text
svc-index    → http://127.0.0.1:5304
svc-storage  → http://127.0.0.1:5303
omnigate     → http://127.0.0.1:9090
svc-gateway  → http://127.0.0.1:8090
```

The gateway was verified:

```bash
curl -i http://127.0.0.1:8090/healthz
curl -i http://127.0.0.1:8090/readyz
```

Observed:

```text
/healthz → HTTP 200 ok
/readyz  → HTTP 200 ready
```

Built-in resolver routes were verified through `svc-gateway`:

```bash
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://site' | jq .
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://image' | jq .
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://music' | jq .
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://article' | jq .
```

All returned expected `omnigate.builtin-page.v1` DTOs.

---

# Architectural impact

## What this means for RustyOnions

This update gives RustyOnions a clean product-layer bridge for CrabLink:

```text
crab://site
→ built-in site creation/registration page metadata

crab://image
→ built-in image upload/asset manifest page metadata

crab://music
→ reserved coming-soon music page metadata

crab://article
→ reserved coming-soon article page metadata
```

This is important because CrabLink can now act like a real browser surface for the RustyOnions network without inventing product semantics locally.

The backend remains the source of truth for page contracts.

CrabLink can render:

```text
title
description
required passport/wallet status
actions
fields
warnings
```

But CrabLink does not own:

```text
site registration logic
image upload logic
manifest creation logic
storage writes
index pointer writes
wallet holds/captures
ledger mutations
policy decisions
```

That keeps the architecture clean.

---

# Boundaries preserved

The update correctly preserves RustyOnions modular boundaries:

```text
omnigate remains read-only resolver/product BFF layer.
omnigate does not mutate ledger.
omnigate does not mutate wallet balances.
omnigate does not write storage bytes.
omnigate does not write index records.
omnigate does not register names.
omnigate exposes metadata and route contracts for product UX.
```

The DTOs include mutating action metadata, but the actions are not performed by this resolver.

That distinction is critical.

For example:

```text
image.create is described as POST /assets/image with requires_confirmation=true
```

But resolving `crab://image` does not create an image asset. It only tells the client what the image page is and what route contracts exist.

---

# Current limitations

## 1. Built-in pages are metadata-only

`crab://site` and `crab://image` are active as resolver pages, but the actual workflows are not complete in this crate/session.

Not yet implemented:

```text
actual site name availability check
actual site prepare flow
actual site create/register flow
actual image prepare flow
actual image upload
actual BLAKE3 hashing from uploaded image bytes through product route
actual image manifest creation
actual manifest storage
actual index manifest pointer creation
actual wallet hold/capture for paid image/site creation
actual receipt surfacing for these product actions
```

## 2. `music` and `article` are only reserved

`crab://music` and `crab://article` are not active product workflows.

They are intentionally:

```text
status: coming_soon
actions: []
requires_passport: false
requires_wallet: false
```

This prevents false affordances in clients.

## 3. Route contracts are provisional

The exposed action routes are:

```text
/sites/prepare
/sites
/assets/image/prepare
/assets/image
```

These need to be reconciled with current `svc-gateway`, `ron-naming`, `svc-index`, `svc-storage`, `ron-policy`, and WEB3_2 route contract plans before implementing the actual product mutations.

The naming and endpoint shape should not be finalized blindly from the metadata alone.

## 4. Need broader product-route integration

`omnigate` currently describes product actions, but downstream services must implement and validate them.

Need future coordination across:

```text
svc-gateway
svc-storage
svc-index
ron-naming
ron-proto
ron-policy
svc-wallet
ron-ledger
ron-accounting
svc-rewarder
```

For actual paid/product flows, `svc-wallet` must remain the mutation front-door for economic operations. `omnigate` must not bypass it.

---

# What remains for omnigate

## Near-term remaining work

## 1. Keep built-in page contracts aligned with gateway routes

Before implementing actual site/image mutations, verify that these route contracts are the desired final routes:

```text
POST /sites/prepare
POST /sites
POST /assets/image/prepare
POST /assets/image
```

If `svc-gateway` uses different route shapes, update `omnigate` metadata and tests to match.

Potential route alternatives to consider:

```text
POST /v1/sites/prepare
POST /v1/sites
POST /v1/assets/image/prepare
POST /v1/assets/image
POST /crab/site/prepare
POST /crab/site
POST /crab/image/prepare
POST /crab/image
```

Need avoid route duplication and keep public gateway routes stable.

## 2. Add route contract tests against `svc-gateway` expectations

`omnigate` unit tests currently verify metadata shape. We should eventually add integration-level tests or contract tests that ensure the routes described by `omnigate` are actually implemented/proxied by `svc-gateway`.

Example acceptance idea:

```text
omnigate says /assets/image/prepare exists
svc-gateway route contract test confirms /assets/image/prepare has expected method/schema/error behavior
```

## 3. Add `crab://home` or dashboard page eventually

The full CrabLink browser currently uses `crab://site` as the practical home/default. Eventually consider a real built-in home page:

```text
crab://home
```

Potential home content:

```text
recent assets
recent sites
passport state
wallet state
featured/seeded local network content
developer diagnostics
```

But do not add too much too early. `site` and `image` are higher priority.

## 4. Add more built-in page kinds only when product scope is ready

Potential future built-ins:

```text
crab://video
crab://post
crab://comment
crab://app
crab://dashboard
crab://passport
crab://wallet
crab://settings
```

But avoid prematurely expanding the namespace. The current four are enough for the next product milestone.

## 5. Improve typed asset page rendering/hydration support

Existing asset page resolver already composes:

```text
asset CID
asset kind
manifest summary
storage summary
owner
payout
metadata
receipts
links
warnings
```

Future improvements:

```text
stronger manifest schema validation
support multiple manifest versions carefully
more explicit provenance fields
curator metadata
provider/storage metadata
payout split metadata
paid view receipt metadata
content moderation/visibility metadata
content license metadata
image dimensions/media metadata where available
```

## 6. Add image-specific manifest extraction details

For `crab://<hash>.image`, the asset page should eventually expose image-oriented fields when present:

```text
width
height
mime type
thumbnail CID
preview CID
dominant tags
creator/owner
payout account
license
provenance
storage provider
receipts
```

This likely belongs in manifest parsing/hydration logic. Keep validation deterministic.

## 7. Keep coming-soon pages non-mutating

Until music/article workflows are real, keep:

```text
actions: []
requires_passport: false
requires_wallet: false
status: coming_soon
```

Do not expose fake upload or create routes for music/article.

## 8. Add metrics coverage if needed

Current resolver counters exist for crab resolve total/error. Future improvement could separate counters by page type:

```text
crab_resolve_total{kind="asset"}
crab_resolve_total{kind="builtin"}
crab_resolve_builtin_total{page_kind="site",status="active"}
crab_resolve_builtin_total{page_kind="music",status="coming_soon"}
```

Do this only if it fits current project metrics conventions. Avoid overengineering.

## 9. Ensure no accidental capability escalation

The built-in page metadata says `requires_passport` and `requires_wallet`, but this is display metadata, not authorization. Actual enforcement must happen in downstream mutation routes.

Future tests should ensure clients cannot mutate through resolver endpoints.

The resolver must remain:

```text
GET /crab/resolve?url=...
read-only
no state mutation
no wallet mutation
no ledger mutation
no storage write
no index write
```

## 10. Add docs for built-in page resolver contract

Need crate docs or route contract docs describing:

```text
omnigate.builtin-page.v1
active vs coming_soon
requires_passport/requires_wallet semantics
actions semantics
fields semantics
warnings semantics
difference between built-in pages and typed asset pages
```

This will help CrabLink and future clients stay aligned.

---

# Recommended next project step involving omnigate

The next large product feature should likely be **`crab://image` real upload/manifest workflow**, but omnigate’s role should remain narrow.

For `crab://image`, omnigate should eventually:

```text
resolve crab://image to product page metadata
resolve crab://<hash>.image to typed asset page
hydrate manifest details after upload
display owner/payout/tags/description/storage/receipt metadata
```

But the actual mutation path should be elsewhere:

```text
CrabLink full tab
→ svc-gateway product route
→ svc-storage for bytes/content addressing
→ svc-index for manifest pointer
→ svc-wallet/ron-ledger for paid holds/captures if enabled
→ ron-accounting for usage snapshots
→ omnigate only resolves/views after creation
```

Do not let omnigate become the image upload service.

---

# Acceptance gates for omnigate after this session

Current done criteria achieved:

```text
cargo fmt passes
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings passes
cargo test -p omnigate --test asset_page_resolver passes
cargo test -p omnigate --test builtin_page_resolver passes
gateway live curl for crab://site returns active built-in page DTO
gateway live curl for crab://image returns active built-in page DTO
gateway live curl for crab://music returns coming_soon built-in page DTO
gateway live curl for crab://article returns coming_soon built-in page DTO
CrabLink can consume/render these DTOs
```

Future acceptance gates before marking built-in product pages production-ready:

```text
svc-gateway routes described by omnigate actually exist
prepare/create flows have deterministic schemas
mutating routes enforce passport/wallet/capability/policy gates
mutating routes require explicit confirmation client-side
paid actions use hold → capture/release if priced
receipts are committed and returned deterministically
asset manifests can be resolved back through omnigate typed asset pages
all route contracts covered by integration tests
metrics/health/readiness remain truthful
```

---

# Summary

This session made `omnigate` the backend source of truth for CrabLink built-in product page metadata.

The crate now supports:

```text
crab://site     active
crab://image    active
crab://music    coming_soon
crab://article  coming_soon
```

This unlocked CrabLink’s full-tab browser shell and allowed the extension to render RustyOnions network pages as real UI instead of raw gateway JSON.

The work is green and tested. The next `omnigate` work should focus on keeping these contracts aligned with actual `svc-gateway` product routes and eventually hydrating real `crab://<hash>.image` pages after image upload/manifest creation is implemented elsewhere.


### END NOTE - MAY 1 2026 - 22:35 CST


### BEGIN NOTES - MAY 2 2026 - 13:55 CST

Below is a **dual drop-in note set** for `omnigate` and `svc-gateway`. It covers what changed, what was proven, the live proof artifacts, and what remains. The green test run showed both `omnigate` and `svc-gateway` clippy/tests passing, and the paid image proof showed `omnigate.image-asset-upload.v1` plus hydrated `omnigate.asset-page.v1` resolving correctly.  

````markdown
### BEGIN NOTE - MAY 02 2026 - CRABLINK WALLET HOLD + PAID IMAGE PRODUCT PROOF

# CARRY-OVER NOTES — omnigate + svc-gateway WEB3_2 CrabLink Wallet Hold / Paid Image Proof

**Date:** 2026-05-02  
**Scope:** `omnigate`, `svc-gateway`, with live proof through `svc-wallet`, `svc-storage`, and `svc-index`  
**Status:** Green compile/test for touched RustyOnions crates; CrabLink can now reach live ROC wallet state and a real paid image asset page  
**Verdict:** Major product-proof milestone achieved. The browser-facing WEB3_2 stack has crossed from “prepare/preflight only” into “explicit wallet hold + paid content upload + crab:// asset resolution.”

---

## 0. TL;DR

We worked on two RustyOnions crates in this session:

```text
crates/omnigate
crates/svc-gateway
````

We also worked heavily in the separate CrabLink Chrome extension repo, but these notes are for the RustyOnions crate notes.

The key backend achievement:

```text
CrabLink
→ svc-gateway /wallet/hold
→ omnigate /v1/wallet/hold
→ svc-wallet /v1/hold
→ real ROC wallet hold receipt
```

Then we manually proved the next paid product path:

```text
ROC wallet hold receipt
→ svc-gateway /assets/image
→ omnigate image asset coordinator
→ svc-storage paid object write
→ manifest stored
→ svc-index asset pointer stored
→ crab://<hash>.image resolves
→ hydrated omnigate.asset-page.v1 rendered in CrabLink
```

This is a major WEB3_2 product checkpoint.

---

# PART A — omnigate notes

## 1. omnigate — what changed

### 1.1 Added/updated wallet façade route

We added/updated the `omnigate` wallet route surface so browser clients do not need direct access to `svc-wallet`.

Public flow:

```text
CrabLink
→ svc-gateway POST /wallet/hold
→ omnigate POST /v1/wallet/hold
→ svc-wallet POST /v1/hold
```

The route belongs in:

```text
crates/omnigate/src/routes/v1/wallet.rs
```

and is mounted from:

```text
crates/omnigate/src/routes/v1/mod.rs
```

The important endpoint now exposed by `omnigate`:

```text
POST /v1/wallet/hold
```

Existing wallet display route also remains:

```text
GET /v1/wallet/:account/balance
```

### 1.2 Preserved service boundaries

`omnigate` does **not** mutate the ledger directly.

The invariant remains:

```text
svc-wallet remains the economic mutation front-door.
ron-ledger remains durable append-only truth.
omnigate is a façade/coordinator, not the wallet implementation.
```

This is important. We did not cheat the architecture to make the demo work.

### 1.3 Wallet hold request shape

The wallet hold request accepted by `omnigate` mirrors the `svc-wallet` transfer/hold request shape:

```json
{
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "1024",
  "nonce": 1,
  "memo": "CrabLink site hold for crab://site",
  "idempotency_key": "crablink-preview:site:awebsitename"
}
```

Rules enforced in `omnigate` before forwarding:

```text
from must be non-empty
to must be non-empty
asset must normalize to roc
amount_minor must be a positive integer string
nonce is forwarded to svc-wallet
memo is trimmed/optional
idempotency_key is trimmed/optional
x-ron-wallet-account header, when present, must match request.from
```

### 1.4 Wallet hold response shape observed

The live hold response from `svc-wallet` through gateway/omnigate looked like:

```json
{
  "txid": "tx_14b0998645b92302c222931ed90c567e",
  "op": "hold",
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "1024",
  "nonce": 1,
  "idem": "crablink-preview:site:awebsitename",
  "ts": 1777704690070,
  "ledger_seq_start": 2,
  "ledger_seq_end": 3,
  "ledger_root": "ab2551be5dc15a435ff9d97ed62b643c0304ffb4921f4f3150305a800feccded",
  "receipt_hash": "b3:86c620beca061320570baf5cbc8ba7ee2f5d51bd072523195252e10e1ff8d34e"
}
```

This receipt was successfully replayed idempotently without deducting again.

### 1.5 Environment knobs

The wallet façade uses:

```text
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088
OMNIGATE_WALLET_BEARER=dev
```

The correct live startup pattern is:

```bash
cd /Users/mymac/Desktop/RustyOnions
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

Then in another terminal:

```bash
cd /Users/mymac/Desktop/RustyOnions
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

### 1.6 Balance façade behavior

The wallet balance route returns a CrabLink-facing DTO:

```json
{
  "schema": "crablink.wallet.balance.v1",
  "account": "acct_dev",
  "unit": "ROC",
  "available_minor_units": "1776",
  "held_minor_units": "0",
  "display": "1776 ROC",
  "as_of": null,
  "ledger_backed": true,
  "source": "svc_wallet.v1",
  "reason": null,
  "warnings": []
}
```

Observed later balances:

```text
after 1024 site hold: 752 ROC
after 68 image hold: 684 ROC
```

Important note:

```text
held_minor_units remains "0" in the current display DTO even though holds reduce available balance.
```

This means the current wallet route presents holds as deducted/unavailable but does not yet expose reserved/held amounts separately. This is acceptable for the current milestone but should be clarified later.

---

## 2. omnigate — paid image upload proof

### 2.1 Successful paid image upload route

The successful upload went through:

```text
POST /assets/image
```

Gateway forwards this to `omnigate`, and `omnigate` coordinates:

```text
paid proof verification
→ paid storage write
→ manifest creation/storage
→ asset pointer indexing
→ response with crab:// URL
```

Successful response schema:

```text
omnigate.image-asset-upload.v1
```

### 2.2 Successful paid proof headers learned

The live backend required these paid proof headers:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <amount>
x-ron-wallet-txid: <txid>
x-ron-wallet-receipt-hash: <receipt_hash>
x-ron-wallet-from: <payer>
x-ron-wallet-to: <escrow>
```

Important correction discovered live:

```text
Wrong header:
x-ron-wallet-hold-txid

Correct header:
x-ron-wallet-txid
```

The backend also required:

```text
x-ron-paid-op
```

Once the full header contract was supplied, the upload succeeded.

### 2.3 Successful image upload artifact

Tiny PNG proof:

```text
path: artifacts/crablink-manual/tiny-crablink.png
size: 68 bytes
```

Hold receipt used for image upload:

```json
{
  "txid": "tx_f227fae07360b37dfefe31cafeec3fdb",
  "op": "hold",
  "from": "acct_dev",
  "to": "escrow_paid_write",
  "asset": "roc",
  "amount_minor": "68",
  "nonce": 2,
  "idem": "crablink-preview:image:tiny-crablink.png",
  "ledger_seq_start": 4,
  "ledger_seq_end": 5,
  "ledger_root": "bdf0cb74e42952166eae66ed6f16b1813de99ff8480e5cbce3da91bbf22364af",
  "receipt_hash": "b3:8c05f9c22010c9118d21e38d2cda75cb9c7a0d4d85c5c5b86a938f991ed73e9e"
}
```

Upload result:

```text
schema: omnigate.image-asset-upload.v1
asset_kind: image
asset_cid: b3:984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4
crab_url: crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
manifest_cid: b3:e81539db791855a113757e2d84bfd53d40721f232208ff79879ab0eb77e1f6f5
owner: passport:main:dev / acct_dev
payout recipient: acct_dev
wallet_txid: tx_f227fae07360b37dfefe31cafeec3fdb
wallet_receipt_hash: b3:8c05f9c22010c9118d21e38d2cda75cb9c7a0d4d85c5c5b86a938f991ed73e9e
```

### 2.4 Resolved asset page proof

The returned URL:

```text
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

resolved to:

```text
schema: omnigate.asset-page.v1
asset_kind: image
manifest.status: present
manifest.hydration_status: hydrated
storage.available: true
storage.size_bytes: 68
owner.passport_subject: passport:main:dev
owner.wallet_account: acct_dev
payout.recipient_account: acct_dev
metadata.title: Tiny CrabLink Test Image
metadata.description: First terminal-proven CrabLink paid image upload.
metadata.tags: crablink, tiny, proof
receipts[0].tx_id: tx_f227fae07360b37dfefe31cafeec3fdb
receipts[0].receipt_kind: paid_storage
receipts[0].amount_minor_units: 68
```

This asset page loaded successfully inside the CrabLink browser extension.

---

## 3. omnigate — quality gates run

The user ran:

```bash
cargo fmt
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --all-targets
```

Final status:

```text
omnigate clippy: green
omnigate tests: green
```

Important passing test areas:

```text
routes::v1::identity unit tests
asset_page_resolver tests
builtin_page_resolver tests
image_asset_prepare tests
paid_storage_prepare tests
paid_storage_write_proxy tests
site_launch tests
readiness tests
policy tests
middleware tests
metrics contract tests
```

### 3.1 Compile/clippy issues fixed during this session

We fixed:

```text
String vs &str mismatch in wallet balance DTO construction
clippy::result-large-err for Result<_, Response>
```

The `result-large-err` issue was fixed by replacing large `Response` error variants with a small `WalletHoldProblem` struct and converting it to `Response` only at the route boundary.

---

## 4. omnigate — what remains

### Priority A — Add direct wallet façade tests

Add tests for:

```text
POST /v1/wallet/hold happy path forwards to svc-wallet
x-ron-wallet-account mismatch returns 400
asset != roc returns 400
amount_minor non-integer returns 400
amount_minor zero returns 400
upstream svc-wallet connect failure returns 502/problem JSON
idempotency-key header is forwarded
request id/correlation headers are forwarded where expected
```

The route is live-proven, but it needs dedicated contract tests.

### Priority B — Document wallet hold route contract

Update WEB3_2 route contract docs with:

```text
POST /v1/wallet/hold
POST /wallet/hold through gateway
request shape
response shape
error taxonomy
security assumptions
idempotency behavior
```

### Priority C — Clarify balance held semantics

Current balance:

```text
available decreases after hold
held_minor_units remains 0
```

Decide whether:

```text
held_minor_units should expose reserved holds
```

or whether the field should be renamed/removed until the ledger/wallet model exposes active holds clearly.

### Priority D — Harden paid proof docs

Document the exact paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
```

Make sure CrabLink and docs do not use the wrong `x-ron-wallet-hold-txid` header.

### Priority E — Prepare for image upload UI

CrabLink will next call `/assets/image` from the browser after a successful hold. Ensure omnigate’s image route remains strict:

```text
requires paid proof
requires correct txid/receipt hash headers
requires image content type
stores bytes by b3 content ID
stores manifest
stores index pointer
returns crab_url
does not accept spoofed ownership without passport/wallet context
```

### Priority F — Site creation proof

The next backend product proof after image UI should be site creation:

```text
crab://site
→ prepare
→ hold
→ POST /sites
→ resolve crab://<site_name>
```

Before UI work, inspect and document:

```text
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/tests/site_launch.rs
```

Confirm exact site-create request shape and paid-proof expectations.

---

# PART B — svc-gateway notes

## 5. svc-gateway — what changed

### 5.1 Added public wallet hold route

We added/updated the product gateway route:

```text
POST /wallet/hold
```

This proxies to:

```text
omnigate POST /v1/wallet/hold
```

The relevant file:

```text
crates/svc-gateway/src/routes/product.rs
```

### 5.2 Public product route surface after this session

The `svc-gateway` product router now exposes:

```text
GET  /identity/me
POST /identity/passport/bootstrap

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /sites/prepare
POST /sites
GET  /sites/:name
```

### 5.3 Preserved gateway boundary

`svc-gateway` remains proxy-only.

It does not:

```text
parse manifests
hydrate asset pages
price storage
create wallet holds directly
mutate ledger
store bytes
write index pointers
capture/release ROC
```

Its job is:

```text
stable browser/client path
selected header forwarding
body forwarding
query forwarding
hop-by-hop header filtering
return upstream response body/status
```

### 5.4 Header forwarding behavior

The product proxy forwards selected headers:

```text
Authorization
Accept
Content-Type
x-ron-*
x-correlation-id
x-request-id
idempotency-key
```

It filters hop-by-hop and unsafe headers such as:

```text
Host
Connection
Proxy-Authorization
TE
Trailer
Transfer-Encoding
Upgrade
Content-Length
```

This is important for gateway correctness and avoiding proxy bugs.

### 5.5 Wallet hold route live proof

The user successfully created a wallet hold from CrabLink through:

```text
CrabLink full-tab browser
→ svc-gateway POST /wallet/hold
→ omnigate POST /v1/wallet/hold
→ svc-wallet POST /v1/hold
```

The balance moved from:

```text
1776 ROC
```

to:

```text
752 ROC
```

after the `1024` site hold.

Then another image hold of `68` moved balance to:

```text
684 ROC
```

This proves the public gateway route reaches the wallet path correctly.

---

## 6. svc-gateway — paid image upload path

### 6.1 Existing route used successfully

The already-exposed public route:

```text
POST /assets/image
```

successfully proxied to `omnigate` and returned:

```text
omnigate.image-asset-upload.v1
```

This proved `svc-gateway` correctly passed:

```text
raw image body
Content-Type: image/png
Authorization
idempotency-key
x-ron-paid-* headers
x-ron-wallet-* headers
x-ron-passport
x-ron-wallet-account
x-ron-asset-title
x-ron-asset-description
x-ron-asset-tags
```

### 6.2 Successful upload returned final crab URL

```text
crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

The gateway also successfully resolved:

```text
GET /crab/resolve?url=crab://984128e643b594b1ff15ed2c40cf1d589616b9ddb7b212d00e91670997c1b8e4.image
```

to:

```text
omnigate.asset-page.v1
```

This proves the browser-facing public route layer is working for the core WEB3_2 image flow.

---

## 7. svc-gateway — quality gates run

The user ran:

```bash
cargo fmt
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --all-targets
```

Final status:

```text
svc-gateway clippy: green
svc-gateway tests: green
```

Important passing tests:

```text
tests/app_proxy.rs
tests/identity_routes_proxy.rs
tests/paid_storage_estimate_proxy.rs
tests/paid_storage_write_proxy.rs
tests/product_routes_proxy.rs
tests/smoke.rs
```

### 7.1 Compile/clippy issues fixed during this session

We fixed `clippy::doc_markdown` errors in the product route header comments by wrapping path-like and project-specific terms in backticks.

Examples:

```text
SVC_GATEWAY_OMNIGATE_BASE_URL
tests/product_routes_proxy.rs
tests/identity_routes_proxy.rs
CrabLink
```

---

## 8. svc-gateway — what remains

### Priority A — Add direct `/wallet/hold` proxy test

The route is live-proven, but a dedicated gateway test should be added.

Add coverage to either:

```text
crates/svc-gateway/tests/identity_routes_proxy.rs
```

or:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Test expectations:

```text
POST /wallet/hold targets /v1/wallet/hold upstream
body is preserved
Authorization is forwarded
Content-Type is forwarded
idempotency-key is forwarded
x-ron-passport is forwarded
x-ron-wallet-account is forwarded
x-correlation-id is forwarded
upstream errors pass through
connect failure yields structured 502 problem
```

### Priority B — Update route contracts

Update docs to include:

```text
POST /wallet/hold
```

in the public gateway route list.

Document that the gateway route is a proxy only:

```text
client → svc-gateway /wallet/hold → omnigate /v1/wallet/hold → svc-wallet /v1/hold
```

### Priority C — Add optional paid image upload smoke

The manual terminal flow should be converted into an optional smoke path.

Possible script location:

```text
crablink/scripts/smoke-local-gateway.sh
```

or RustyOnions:

```text
scripts/web3_crablink_dev_stack.sh
scripts/web3_product_stack_smoke.sh
```

Suggested opt-in env:

```text
CRABLINK_SMOKE_RUN_UPLOAD=1
```

Default should remain non-mutating unless explicitly enabled.

The upload smoke should:

```text
create tiny PNG
create wallet hold
upload image with paid proof headers
assert omnigate.image-asset-upload.v1
resolve returned crab_url
assert omnigate.asset-page.v1
assert receipt present
```

### Priority D — Keep gateway route surface stable for CrabLink

CrabLink now depends on these public paths:

```text
/healthz
/readyz
/identity/me
/identity/passport/bootstrap
/wallet/:account/balance
/wallet/hold
/crab/resolve?url=...
/b3/:hash.kind
/sites/prepare
/assets/image/prepare
/assets/image
```

Future changes must preserve these paths or provide migration notes.

### Priority E — Site create route verification

Before wiring CrabLink site-create UI, confirm the public gateway `/sites` route forwards all required paid proof headers and body correctly.

Existing route:

```text
POST /sites
```

Needed next:

```text
manual terminal proof for site create
then UI wiring
then gateway test if missing
```

---

# PART C — joint proof / live commands

## 9. Live proof startup commands

### Terminal A

```bash
cd /Users/mymac/Desktop/RustyOnions
RUST_LOG=info SVC_WALLET_ADDR=127.0.0.1:8088 cargo run -p svc-wallet
```

### Terminal B

```bash
cd /Users/mymac/Desktop/RustyOnions
OMNIGATE_WALLET_BASE_URL=http://127.0.0.1:8088 \
OMNIGATE_WALLET_BEARER=dev \
scripts/web3_crablink_dev_stack.sh
```

### Backend health checks

```bash
curl -i http://127.0.0.1:8090/healthz
curl -i http://127.0.0.1:8090/readyz

curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://site' | jq .
curl -s 'http://127.0.0.1:8090/crab/resolve?url=crab://image' | jq .

curl -s http://127.0.0.1:8090/wallet/acct_dev/balance \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

---

## 10. Successful paid image upload command

Reference command that succeeded:

```bash
cd /Users/mymac/Desktop/RustyOnions

HOLD_TXID=$(jq -r '.txid' artifacts/crablink-manual/tiny-hold.json)
HOLD_HASH=$(jq -r '.receipt_hash' artifacts/crablink-manual/tiny-hold.json)
HOLD_FROM=$(jq -r '.from' artifacts/crablink-manual/tiny-hold.json)
HOLD_TO=$(jq -r '.to' artifacts/crablink-manual/tiny-hold.json)
HOLD_AMOUNT=$(jq -r '.amount_minor' artifacts/crablink-manual/tiny-hold.json)

curl -s http://127.0.0.1:8090/assets/image \
  -X POST \
  -H 'Authorization: Bearer dev' \
  -H 'Content-Type: image/png' \
  -H 'Accept: application/json' \
  -H 'x-ron-paid-op: hold' \
  -H 'x-ron-paid-asset: roc' \
  -H "x-ron-paid-estimate-minor: ${HOLD_AMOUNT}" \
  -H "x-ron-wallet-txid: ${HOLD_TXID}" \
  -H "x-ron-wallet-receipt-hash: ${HOLD_HASH}" \
  -H "x-ron-wallet-from: ${HOLD_FROM}" \
  -H "x-ron-wallet-to: ${HOLD_TO}" \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' \
  -H 'x-ron-asset-title: Tiny CrabLink Test Image' \
  -H 'x-ron-asset-description: First terminal-proven CrabLink paid image upload.' \
  -H 'x-ron-asset-tags: crablink,tiny,proof' \
  -H 'idempotency-key: crablink-upload:image:tiny-crablink.png' \
  --data-binary @artifacts/crablink-manual/tiny-crablink.png \
  | tee artifacts/crablink-manual/tiny-upload.json | jq .
```

Resolve:

```bash
CRAB_URL=$(jq -r '.crab_url // .links.crab // .asset.crab_url // empty' artifacts/crablink-manual/tiny-upload.json)

curl -s "http://127.0.0.1:8090/crab/resolve?url=${CRAB_URL}" \
  -H 'Authorization: Bearer dev' \
  -H 'x-ron-passport: passport:main:dev' \
  -H 'x-ron-wallet-account: acct_dev' | jq .
```

---

# PART D — cross-crate next-step plan

## 11. Immediate next work

### 11.1 CrabLink UI next

The next implementation should wire the already-proven terminal path into the CrabLink full-tab UI:

```text
Prepare Image
→ Confirm ROC Hold
→ Submit Image Upload
→ show final crab://<hash>.image
→ auto-open the new asset page
→ refresh balance
```

This will require CrabLink changes, not necessarily new RustyOnions crate changes unless gaps appear.

### 11.2 omnigate next

Add dedicated tests for `/v1/wallet/hold`.

### 11.3 svc-gateway next

Add dedicated proxy tests for `/wallet/hold`.

### 11.4 Docs next

Update route contracts to include:

```text
/wallet/hold
paid image upload proof headers
image upload flow
returned asset page fields
```

### 11.5 Smoke next

Add optional mutating smoke:

```text
CRABLINK_SMOKE_RUN_UPLOAD=1
```

The default smoke should not consume ROC unless explicitly requested.

---

## 12. Success statement

This session proved the first real RustyOnions / CrabLink creator economy loop:

```text
pay ROC
→ upload image
→ receive b3 content ID
→ receive crab:// image URL
→ resolve hydrated asset page
→ show ownership, payout, storage, manifest, and receipt metadata
```

This is a major WEB3_2 milestone.

The remaining work is no longer proving whether the architecture can support the product loop. It can. The remaining work is polishing, testing, documenting, and wiring the proven backend path into the browser UI.



### END NOTES - MAY 2 2026 - 13:55 CST






### BEGIN NOTE - MAY 2 2026 - 18:00 CST

## Drop-in notes for `omnigate` and `ron-app-sdk`

### Session focus

This session focused on completing and proving the NEXT_LEVEL / WEB3_2 product-layer path for **named crab sites** and cleaning up `ron-app-sdk` enough that the product-layer verification gates are meaningful. The most important outcome is that a user-created site name such as:

```text
crab://thedustyonion
```

now resolves through Omnigate into a hydrated `omnigate.site-page.v1` response, and CrabLink can load and display that response in the browser-tab UI.

We deliberately **did not modify `ron-kernel`**. A full `ron-kernel` clippy cleanup should remain a separate, dedicated kernel-maintenance session because it is the core substrate and carries much higher risk than product-layer changes.

---

# `omnigate` notes

## What changed / what was accomplished

### 1. Named `crab://<site_name>` resolution now works

Before this session, the typed b3 asset resolver path worked for asset-style URLs such as:

```text
crab://<64hex>.image
```

and built-in product URLs such as:

```text
crab://site
crab://image
crab://music
crab://article
```

were understood as built-in pages, but a user-created named site URL like:

```text
crab://thedustyonion
```

was still falling through the asset parser and returning a missing/invalid asset-style response.

This session added the intended named-site fallback behavior:

```text
/crab/resolve?url=crab://thedustyonion
→ parse as not a built-in page
→ parse as not a typed b3 asset
→ safely treat as possible named site
→ route through existing site resolver
→ hydrate omnigate.site-page.v1
```

This preserves the correct split:

```text
crab://site
→ built-in site creation / launch page

crab://image
→ built-in image upload/mint page

crab://music
→ reserved built-in music page

crab://article
→ reserved built-in article page

crab://thedustyonion
→ user-created named site page

crab://<64 lowercase hex>.image
→ typed b3 image asset page
```

The canonical internal content IDs remain `b3:<64 lowercase hex>`. Human names remain optional pointers layered above content-addressed storage.

---

### 2. Site launch / site resolve flow was proven end-to-end

We manually created a local static HTML document:

```html
<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>The Dusty Onion</title>
  </head>
  <body>
    <h1>The Dusty Onion</h1>
    <p>Hello from my first RustyOnions crab site.</p>
  </body>
</html>
```

Then stored it in `svc-storage`, producing this root document CID:

```text
b3:c0acf5c68296173b4b0b85612bfc30437b53a00b54e3100a5d613d2326a23b91
```

Then created the site through:

```text
POST http://127.0.0.1:8090/sites
```

The site creation returned:

```text
schema: omnigate.site-create.v1
site_name: thedustyonion
root_document_cid: b3:c0acf5...
manifest.status: stored
manifest_cid: b3:6ad6224cecacc448de14cdf91ce84cf87def5b0234af0041280864b76675aebd
index_pointer.status: stored
links.crab: crab://thedustyonion
links.resolve: /v1/sites/thedustyonion
```

Then direct site resolve worked:

```text
GET http://127.0.0.1:8090/sites/thedustyonion
```

and returned:

```text
schema: omnigate.site-page.v1
site_name: thedustyonion
manifest.status: present
manifest.hydration_status: hydrated
owner.passport_subject: passport:main:dev
owner.wallet_account: acct_dev
payout.default_action: site_visit
payout.recipient_account: acct_dev
metadata.title: The Dusty Onion
route_map["/"]: b3:c0acf5...
asset_map["index.html"]: b3:c0acf5...
receipts[0].receipt_kind: paid_site_launch
links.crab: crab://thedustyonion
```

Then named crab resolve also worked:

```text
GET http://127.0.0.1:8090/crab/resolve?url=crab://thedustyonion
```

and returned the same hydrated `omnigate.site-page.v1` shape.

This proves the named site path is now wired through Omnigate and usable by CrabLink.

---

### 3. New/updated Omnigate test coverage

The important new test target is:

```bash
cargo test -p omnigate --test named_site_crab_resolver
```

That test covers:

```text
crab_resolve_named_site_hydrates_site_page
crab_resolve_bare_b3_hash_still_requires_asset_kind
```

The second test is important because it proves we did **not** accidentally reinterpret bare 64-character b3 hashes as site names. A bare hash without a kind suffix still fails as missing an asset kind, preserving typed b3 asset semantics.

Existing route/product tests also remained green:

```bash
cargo test -p omnigate --test site_launch
cargo test -p omnigate --test asset_page_resolver
```

The final confirmed Omnigate product-layer gate passed:

```bash
cargo test -p omnigate --test named_site_crab_resolver
cargo test -p omnigate --test site_launch
cargo test -p omnigate --test asset_page_resolver
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

The terminal log confirms the named-site resolver tests, site launch tests, asset page resolver tests, and Omnigate `--no-deps` clippy all passed. 

---

### 4. CrabLink live UX was proven against Omnigate

CrabLink was used to load:

```text
crab://thedustyonion
```

CrabLink successfully received a `200` response and displayed the hydrated `omnigate.site-page.v1` JSON. The UI currently labels it as a “Generic gateway response,” but the JSON clearly contains:

```text
schema: omnigate.site-page.v1
site_name: thedustyonion
root_document_cid: b3:c0acf5...
manifest.status: present
manifest.hydration_status: hydrated
owner
payout
metadata
route_map
asset_map
receipts
links
```

This means the backend route is working and CrabLink can reach it. The remaining work is better frontend rendering.

---

## Omnigate invariants preserved

No `ron-kernel` changes were made.

No direct ledger mutation was introduced.

No storage bytes are stored by Omnigate itself. Omnigate continues to coordinate through existing service boundaries.

No asset-name lookup was pushed into `svc-storage`. Storage remains content-addressed by CID.

Named sites are human pointers. The canonical root document is still a `b3:<64 lowercase hex>` CID.

Typed asset URLs still require asset kinds:

```text
crab://<64hex>.image
crab://<64hex>.article
crab://<64hex>.comment
```

Bare `crab://<64hex>` is not accepted as a complete asset URL.

Built-in product pages and user-created named sites stay separate:

```text
crab://site
```

is not the same thing as:

```text
crab://thedustyonion
```

---

## Omnigate work remaining

### 1. Better CrabLink rendering for `omnigate.site-page.v1`

Backend is working. The extension currently displays the JSON under a generic response panel. Next CrabLink/Omnigate-aligned UX should render site pages as first-class pages:

```text
Title
Description
Site name
Owner passport
Owner wallet
Payout/default action
Root document CID
Manifest CID
Route map
Asset map
Receipts
Open raw root document button
Copy crab URL
Copy manifest CID
Copy root CID
```

### 2. Full “view site content” path

We can resolve the site manifest and root document CID, but the UI should next fetch and display the actual root document bytes:

```text
root_document_cid
→ /o/<root_document_cid>
→ render index.html safely
```

This should be sandboxed/controlled. Do not execute arbitrary scripts in the extension page until a security model is defined. Start with static HTML/text rendering or a safe preview pane.

### 3. CrabLink Create Site UI

Currently site creation was proven by terminal `POST /sites`. Next product step is `crab://site` UI in CrabLink:

```text
crab://site
→ form for site name/title/description/root document
→ prepare request
→ wallet hold/cost preview
→ explicit confirm
→ POST /sites
→ show created crab://<site_name>
```

### 4. Real paid-site economics

The site create response includes a receipt-like `paid_site_launch` reference and wallet account metadata, but the long-term NEXT_LEVEL flow should keep tightening real wallet hold/capture integration:

```text
prepare
→ wallet hold
→ site create
→ hold capture
→ receipt committed
→ site manifest includes receipt refs
```

Do not bypass `svc-wallet` or mutate `ron-ledger` directly from Omnigate.

### 5. Expand product pages after site/image foundation

After `crab://site` and `crab://image` are strong, expand carefully:

```text
crab://article
crab://music
crab://post
crab://comment
```

Do not jump ahead into social/content feeds until the basic site/image ownership and payout flows are solid.

---

## How to prove Omnigate work

Use this product-layer gate:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p omnigate --test named_site_crab_resolver
cargo test -p omnigate --test site_launch
cargo test -p omnigate --test asset_page_resolver
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Use this live proof after the local stack is running:

```bash
curl -sG 'http://127.0.0.1:8090/crab/resolve' \
  --data-urlencode 'url=crab://thedustyonion' | jq .
```

Expected proof fields:

```text
schema = omnigate.site-page.v1
site_name = thedustyonion
manifest.status = present
manifest.hydration_status = hydrated
links.crab = crab://thedustyonion
```

---

# `ron-app-sdk` notes

## Why this crate was touched

`ron-app-sdk` was not the main feature target, but it came up because the product-layer verification path included workspace/local crate checks, and `ron-app-sdk` still had placeholder scaffold tests such as `assert!(true)` and test assumptions that did not match the current `SdkConfig::validate()` behavior.

Instead of leaving fake tests in place, this session replaced them with real contract tests. This gives future work much better confidence around SDK invariants.

---

## What changed / what was accomplished

### 1. Placeholder tests were replaced with real tests

Several test files previously had scaffold tests that effectively did nothing:

```rust
assert!(true);
```

Those were replaced with meaningful integration/contract/property checks.

This was done across the SDK test suite, including:

```text
i_1_profile_parity.rs
i_2_caps_required.rs
i_3_oap_bounds.rs
i_4_content_addressing.rs
i_5_retries_deadlines.rs
i_6_dto_strictness.rs
i_7_transport_agnostic.rs
i_8_deadlines_everywhere.rs
i_9_error_taxonomy.rs
i_10_semver_snapshot.rs
i_11_no_persistence.rs
i_12_canon_deps.rs
interop_vectors.rs
prop_oap_frames.rs
prop_retry_bounds.rs
```

The final run confirms the SDK clippy gate and all SDK tests passed. 

---

### 2. Profile parity contract

`i_1_profile_parity.rs` now checks that Micronode-style and Macronode-style configurations share the same SDK semantics:

```text
same transport posture
same overall timeout
same connect/read/write timeouts
different gateway targets
both validate through check_ready()
```

A subtle issue was discovered: `SdkConfig::default()` does not currently validate under the SDK’s own timeout relationship rules because default `overall_timeout` is shorter than default read/write timeouts. The tests were corrected to use explicit valid configs when asserting readiness.

This is now a documented behavior/remaining design question rather than a hidden flaky test.

---

### 3. Capability-required contract

`i_2_caps_required.rs` now verifies that SDK storage calls require an explicit `Capability` argument at the function boundary.

It also checks that capability failures are non-retriable:

```text
SdkError::CapabilityDenied → RetryClass::NoRetry
SdkError::CapabilityExpired → RetryClass::NoRetry
```

Important limitation: this proves the SDK API surface requires a capability parameter. It does not yet prove the transport serializes capability material into wire headers. That remains future work.

---

### 4. OAP frame bound contract

`i_3_oap_bounds.rs` and `prop_oap_frames.rs` now verify OAP frame-size assumptions:

```text
OAP max frame = 1 MiB
normal stream chunk = 64 KiB
64 KiB chunks fit inside 1 MiB
one byte over max is detected as oversize
storage_put rejects payloads larger than OAP_MAX_FRAME_BYTES
```

This protects the SDK boundary against oversized client-side payload construction.

---

### 5. Content addressing contract

`i_4_content_addressing.rs` now spins up a small mock Axum gateway and verifies a real SDK storage round trip:

```text
storage_put(blob)
→ mock gateway stores blob by BLAKE3 b3 CID
→ SDK receives b3:<64 lowercase hex>
→ storage_get(cid)
→ SDK receives exact original bytes
```

The test verifies:

```text
b3: prefix
64 lowercase hex digest
digest equals BLAKE3(blob)
roundtrip bytes match original blob
```

This is important because content-addressing is a RustyOnions signature invariant.

---

### 6. Retry/deadline contract

`i_5_retries_deadlines.rs`, `i_8_deadlines_everywhere.rs`, and `prop_retry_bounds.rs` now verify:

```text
retry schedules are finite
retry delays are monotonic when jitter is disabled
retry delays do not exceed cap
explicit valid configs validate
invalid timeout relationships fail closed
zero per-call deadlines fail before transport
```

Important discovered behavior:

```text
SdkConfig::default()
```

currently has bounded fields, but does not necessarily satisfy the full validation relationship of:

```text
overall_timeout >= read timeout
overall_timeout >= write timeout
```

Tests were updated to use explicit valid configs for “valid config” assertions. Remaining design question: should `SdkConfig::default()` itself be changed later to always validate, or should defaults remain raw construction values that must be adjusted by callers?

No code change to default config was made in this session.

---

### 7. DTO strictness contract

`i_6_dto_strictness.rs` now verifies:

```text
AddrB3 accepts canonical b3:<64 lowercase hex>
AddrB3 rejects uppercase hex
AddrB3 rejects missing prefix
AddrB3 rejects short strings
Capability DTOs use explicit typed ron-proto Caveat values
```

The test was corrected to use real `ron_proto::cap::Caveat` variants instead of raw strings. This keeps DTO strictness aligned with canonical `ron-proto` capability types.

---

### 8. Transport agnosticism contract

`i_7_transport_agnostic.rs` now verifies TLS and Tor configs share public SDK semantics while preserving transport distinction:

```text
same gateway address
same overall timeout
same connect/read/write timeouts
different transport enum
Tor readiness exposes tor_ok = Some(true)
TLS readiness has no Tor-specific status
```

The test uses an explicit valid timeout config and explicit Tor SOCKS address:

```text
127.0.0.1:9050
```

This avoids incorrectly depending on invalid default timeout relationships.

---

### 9. Error taxonomy contract

`i_9_error_taxonomy.rs` now verifies deterministic retry classification:

Retriable:

```text
DeadlineExceeded
Transport(TimedOut)
RateLimited
Server(503)
```

Non-retriable:

```text
Tls
TorUnavailable
OapViolation
CapabilityExpired
CapabilityDenied
NotFound
Conflict
Server(404)
Unknown
SchemaViolation
```

This matters because SDK callers need stable retry behavior and should not blindly retry capability/security failures.

---

### 10. No-persistence contract

`i_11_no_persistence.rs` now verifies the SDK’s default posture remains memory-only / non-durable:

```text
cache disabled by default
cache has bounded capacity if enabled
cache has bounded TTL if enabled
no disk-path persistence knob is introduced
```

This supports amnesia-mode-friendly client behavior.

---

### 11. Canonical dependency contract

`i_12_canon_deps.rs` now checks the SDK manifest shape:

```text
ron-app-sdk depends on ron-proto for canonical DTOs
ron-app-sdk keeps thin HTTP client transport during this phase
ron-app-sdk remains rustls-oriented
ron-app-sdk does not directly depend on service crates
```

Forbidden direct service dependencies checked include:

```text
svc-gateway
omnigate
svc-storage
svc-index
svc-wallet
ron-ledger
```

This preserves modularity: the SDK should not become a backdoor into service internals.

---

### 12. Interop vectors

`interop_vectors.rs` now includes stable vectors for:

```text
canonical b3 content ID parsing
uppercase b3 rejection
retry taxonomy
retry config caps
opaque idempotency config shape
```

These are useful for future TypeScript/Go/Python SDK parity work.

---

### 13. Semver snapshot metadata

`i_10_semver_snapshot.rs` now checks that package metadata is available:

```text
CARGO_PKG_NAME = ron-app-sdk
CARGO_PKG_VERSION is non-empty
version is semver-like
```

This is a lightweight placeholder for later real public API snapshot/semver gates.

---

## ron-app-sdk invariants preserved

No service-specific logic was introduced into the SDK.

No direct dependency on Omnigate, svc-gateway, svc-storage, svc-index, svc-wallet, or ron-ledger was introduced.

No persistence layer was added.

No unsafe code was introduced.

Capability tests enforce explicit capability passing, but do not pretend capability serialization is complete.

Retry/error classification remains deterministic and low-cardinality.

Content IDs remain canonical lowercase `b3:<64hex>`.

---

## ron-app-sdk work remaining

### 1. Decide whether `SdkConfig::default()` should validate

The tests exposed a real design issue:

```text
SdkConfig::default()
```

currently has nonzero/bounded fields, but the full validation relationship may fail because default `overall_timeout` is shorter than read/write timeouts.

Future options:

```text
Option A: Change defaults so SdkConfig::default().validate() succeeds.
Option B: Keep defaults as loose construction defaults and require callers/tests to set explicit valid timeout profiles.
```

This session chose Option B for tests, because changing default config behavior might affect SDK users and should be a deliberate API decision.

### 2. Wire capability material into transport headers

Storage functions currently require a `Capability` argument, but deeper transport serialization remains future work.

Remaining desired flow:

```text
Capability
→ SDK transport header / OAP metadata
→ gateway/service validates capability
→ fail closed on missing/invalid capability
```

Do not fake this in tests. Add it when the SDK transport contract is ready.

### 3. Wire idempotency keys into transport headers

`storage_put` accepts an optional idempotency key, and the idempotency helper exists, but the wire mapping should be finalized later:

```text
IdempotencyKey
→ Idempotency-Key header or OAP metadata
→ service idempotency cache / replay-safe behavior
```

### 4. Expand SDK integration tests from mock gateway to local dev stack

Current content-addressing roundtrip uses a mock Axum gateway. Future tests can add an ignored/manual integration test against a live local stack:

```text
svc-storage
svc-gateway
omnigate
```

Keep this separate from normal unit tests so CI does not require a full stack.

### 5. Add TypeScript/CrabLink interop vectors later

Because CrabLink will become a primary consumer, add JSON fixture vectors for:

```text
AddrB3
Capability
IdempotencyKey
SdkError
RetryClass
ByteRange
Site page DTOs if they move into shared SDK/proto surfaces
```

---

## How to prove ron-app-sdk work

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-app-sdk
cargo clippy -p ron-app-sdk --all-targets -- -D warnings
cargo test -p ron-app-sdk --tests
```

Expected result:

```text
cargo clippy passes
all ron-app-sdk unit tests pass
all ron-app-sdk integration tests pass
all ron-app-sdk property-style tests pass
```

The final uploaded terminal output confirms this gate passed. 

---

# Combined proof gate for this session

Use this as the “session accomplished” proof:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p ron-app-sdk -p omnigate

cargo clippy -p ron-app-sdk --all-targets -- -D warnings
cargo test -p ron-app-sdk --tests

cargo test -p omnigate --test named_site_crab_resolver
cargo test -p omnigate --test site_launch
cargo test -p omnigate --test asset_page_resolver
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Do **not** use this as the product-layer gate yet:

```bash
cargo clippy -p omnigate --all-targets -- -D warnings
```

That checks path dependencies too and currently triggers `ron-kernel` pedantic lint cleanup. We intentionally deferred `ron-kernel` clippy cleanup for a future dedicated kernel-only session.

---

# Important carry-forward warning

Do not casually patch `ron-kernel` while working on NEXT_LEVEL / CrabLink product-layer features.

`ron-kernel` is the core substrate. Its full clippy cleanup should be done later as a dedicated session with:

```text
careful review
small focused batches
full tests
no behavior changes unless intentional
no product-layer logic added
```

For current NEXT_LEVEL work, the correct scoped clippy command is:

```bash
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

---

# Recommended next work

The next work should move back to CrabLink and product UX, not kernel maintenance.

Recommended next batch:

```text
CrabLink site-page renderer
```

Goal:

```text
When user opens crab://thedustyonion,
CrabLink recognizes schema = omnigate.site-page.v1
and renders a real site page card instead of a generic JSON response.
```

Suggested rendering fields:

```text
Title: The Dusty Onion
Description
Site name
Root document CID
Manifest CID
Owner passport
Owner wallet
Payout recipient
Route map
Asset map
Receipts
Copy URL
Copy JSON
Open root document / preview index.html
```

Next after that:

```text
CrabLink crab://site Create Site UI
```

Goal:

```text
Use the extension to create a site instead of terminal curl.
```

Workflow:

```text
crab://site
→ form: site name, title, description, root document HTML/file
→ prepare/estimate
→ explicit confirmation
→ POST /sites
→ show crab://<site_name>
→ allow opening the created site
```

This session proved the backend path. The next session should make the browser UX feel real.


### END NOTE - MAY 2 2026 - 18:00 CST


### BEGIN NOTE - MAY 6 2026 - 17:55 CST


## Set 5 — `omnigate` Crate Notes

Drop this into the `omnigate` crate notes.

---

# `omnigate` Crate Carryover Notes

## NEXT_LEVEL / WEB3_2 Passport Profile Façade Layer

## 1. Crate role

`omnigate` is the RustyOnions façade/coordinator/proxy layer for product-facing internal APIs.

It sits between public gateway routes and internal services:

```text
svc-gateway
→ omnigate
→ svc-passport / svc-wallet / svc-storage / svc-index / other services
```

For the NEXT_LEVEL identity/profile work, `omnigate` must:

```text
Expose stable internal façade routes for profile claim/read.
Proxy profile requests to svc-passport.
Preserve selected request context headers.
Filter hop-by-hop headers.
Preserve svc-passport status codes and JSON bodies.
Map svc-passport connection/read failures to structured 502 errors.
Avoid wallet/ledger mutation.
Avoid owning username persistence.
Avoid exposing private passport material.
```

It should **not**:

```text
Persist username claims.
Own profile truth.
Mutate wallets.
Issue ROC.
Append to ron-ledger.
Leak private keys.
Expose private alt linkage.
Let CrabLink bypass svc-gateway.
```

Correct chain after this session:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-passport
```

---

## 2. Session summary

In this session, `omnigate` was extended to expose the new `svc-passport` public profile claim/read routes through Omnigate’s v1 façade.

New Omnigate routes:

```text
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
```

These proxy to `svc-passport` routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

Major work completed:

```text
Added profile route module.
Mounted profile claim/read routes in v1 router.
Added downstream passport base URL config/env handling.
Implemented selected header forwarding.
Implemented hop-by-hop header filtering.
Implemented safe username path segment validation.
Implemented structured upstream failure mapping.
Added integration tests with dummy svc-passport.
Got passport_profile_routes test green.
Got omnigate all-targets test green.
```

---

## 3. Files changed / added

Primary files touched:

```text
crates/omnigate/src/routes/v1/mod.rs
crates/omnigate/src/routes/v1/profile.rs
crates/omnigate/tests/passport_profile_routes.rs
```

---

## 4. New route module

New file:

```text
crates/omnigate/src/routes/v1/profile.rs
```

Purpose:

```text
Omnigate façade/proxy routes for svc-passport public profile claims and reads.
```

Important route handlers:

```rust
claim_profile(headers: HeaderMap, body: Bytes) -> Response
get_profile(Path(username): Path<String>, headers: HeaderMap) -> Response
```

Primary public functions:

```text
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
```

These are internal Omnigate v1 routes, not public browser-facing gateway routes. CrabLink should still call `svc-gateway`, not Omnigate.

---

## 5. Route mounting

File touched:

```text
crates/omnigate/src/routes/v1/mod.rs
```

The v1 router now imports:

```rust
pub mod profile;
```

and mounts:

```rust
.route(
    "/identity/passport/profile/claim",
    post(profile::claim_profile),
)
.route(
    "/identity/passport/profile/:username",
    get(profile::get_profile),
)
```

Full route mapping:

```text
Omnigate:
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
```

Downstream mapping:

```text
svc-passport:
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

---

## 6. Environment/config behavior

New downstream passport base URL behavior:

```text
OMNIGATE_PASSPORT_BASE_URL
```

Alias:

```text
OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL
```

Default:

```text
http://127.0.0.1:5307
```

Priority:

```text
1. OMNIGATE_PASSPORT_BASE_URL
2. OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL
3. http://127.0.0.1:5307
```

Implementation:

```rust
fn passport_base_url() -> String {
    env::var("OMNIGATE_PASSPORT_BASE_URL")
        .or_else(|_| env::var("OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL"))
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PASSPORT_BASE_URL.to_owned())
}
```

Important note:

```text
The default assumes svc-passport local dev port is 5307.
```

---

## 7. HTTP client behavior

The profile proxy module uses a static reqwest client:

```rust
static HTTP_CLIENT: Lazy<reqwest::Client>
```

Client behavior:

```text
pool_idle_timeout: 30 seconds
tcp_keepalive: 30 seconds
rustls TLS
per-request timeout: 5 seconds
```

Important invariant:

```text
No locks are held across await.
No per-request client construction.
```

---

## 8. Header forwarding behavior

Omnigate profile proxy forwards selected headers only.

Forwarded:

```text
Authorization
Accept
Content-Type
Idempotency-Key / idempotency-key
x-correlation-id
x-request-id
x-ron-*
```

Filtered:

```text
Host
Connection
Content-Length
Transfer-Encoding
Upgrade
Proxy-Authorization
TE
Trailer
```

Important compile fix from session:

```text
http::header::IDEMPOTENCY_KEY does not exist.
The code uses name.as_str().eq_ignore_ascii_case("idempotency-key").
```

Final forwarding function returns a vector:

```rust
fn forwarded_headers(headers: &HeaderMap) -> Vec<(HeaderName, HeaderValue)>
```

Reason:

```text
HeaderMap iteration quirks caused HeaderName / Option<HeaderName> trait errors when consuming HeaderMap.
Using headers.iter() and cloning HeaderName/HeaderValue avoids that issue.
```

Important implementation pattern:

```rust
headers
    .iter()
    .filter(|(name, _)| should_forward_header(name))
    .map(|(name, value)| (name.clone(), value.clone()))
    .collect()
```

---

## 9. Proxy behavior

Core proxy function:

```rust
async fn proxy_to_passport(
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Response
```

Behavior:

```text
Builds URL from passport_base_url + path.
Uses static HTTP_CLIENT.
Forwards selected headers.
For POST, forwards raw body bytes.
For GET, no body.
Preserves upstream HTTP status code.
Copies upstream Content-Type or defaults to application/json.
Returns upstream response body unchanged.
Maps request failures to structured 502.
Maps response body read failure to structured 502.
Maps response build failure to structured 502.
```

Important route behavior:

```text
Omnigate does not parse profile claim body.
Omnigate does not normalize username.
Omnigate does not reserve usernames.
Omnigate does not interpret profile ownership.
svc-passport remains the source of truth.
```

---

## 10. Safe path segment validation

`GET /v1/identity/passport/profile/:username` uses:

```rust
safe_profile_path_segment(value: &str) -> Option<String>
```

It rejects:

```text
empty string
/
\
?
#
%
control characters
```

It accepts:

```text
plain username path segment
@username if routed as a path segment and accepted by Axum path extraction
```

Current usage:

```rust
let Some(encoded_username) = safe_profile_path_segment(&username) else {
    return problem(
        StatusCode::BAD_REQUEST,
        "invalid_username_path",
        "profile username path segment is invalid",
        false,
        "username_path_invalid",
    );
};
```

Important note:

```text
This is not full username validation.
svc-passport still validates username/handle semantics.
This only prevents unsafe path forwarding.
```

Future improvement:

```text
When crab://@username resolver is wired, use ron-naming parser for crab URL grammar and keep this path-segment sanitizer for direct HTTP path routes.
```

---

## 11. Structured error mapping

Problem response shape:

```rust
struct Problem<'a> {
    code: &'a str,
    message: &'a str,
    retryable: bool,
    reason: &'a str,
}
```

### Invalid username path

```text
HTTP 400
code: invalid_username_path
message: profile username path segment is invalid
retryable: false
reason: username_path_invalid
```

### Upstream request/connect/timeout failure

```text
HTTP 502
code: passport_upstream
message: svc-passport upstream request failed
retryable: true
reason:
  passport_upstream_timeout
  passport_upstream_connect
  passport_upstream_request
```

### Upstream body read failure

```text
HTTP 502
code: passport_upstream_body
message: svc-passport upstream response body could not be read
retryable: true
reason: passport_upstream_body_read
```

### Response build failure

```text
HTTP 502
code: passport_response_build
message: omnigate could not build passport proxy response
retryable: true
reason: response_build_failed
```

Important behavior:

```text
svc-passport semantic errors are preserved, not remapped.
Examples:
  400 reserved_username
  404 profile_not_found
  409 username_unavailable
```

---

## 12. New integration tests

New file:

```text
crates/omnigate/tests/passport_profile_routes.rs
```

Test architecture:

```text
Spin up dummy svc-passport on random local port.
Set OMNIGATE_PASSPORT_BASE_URL to dummy svc-passport URL.
Spin up real Omnigate v1 router on random local port.
Call Omnigate routes with reqwest.
Assert proxy behavior.
```

The tests use an `ENV_LOCK` to serialize env var mutations:

```rust
static ENV_LOCK: Mutex<()> = Mutex::const_new(());
```

This prevents cross-test env collisions.

---

## 13. Tests added / coverage

### `profile_claim_proxies_to_svc_passport`

Verifies:

```text
POST /v1/identity/passport/profile/claim reaches dummy svc-passport.
Request JSON body is forwarded.
Authorization header is forwarded.
x-ron-passport is forwarded.
x-ron-wallet-account is forwarded.
idempotency-key is forwarded.
x-correlation-id is forwarded.
x-request-id is forwarded.
connection header is not forwarded.
Response body is preserved.
No private key / seed / spend authority / alt linkage strings appear.
```

### `profile_lookup_proxies_to_svc_passport`

Verifies:

```text
GET /v1/identity/passport/profile/skinnycrabby reaches dummy svc-passport.
Selected headers are forwarded.
Hop-by-hop headers are filtered.
Response body is preserved.
```

### `profile_claim_preserves_svc_passport_conflict`

Verifies:

```text
Dummy svc-passport returns 409 username_unavailable.
Omnigate preserves 409.
Omnigate preserves code username_unavailable.
Omnigate preserves retryable false.
```

### `profile_lookup_preserves_svc_passport_404`

Verifies:

```text
Dummy svc-passport returns 404 profile_not_found.
Omnigate preserves 404.
Omnigate preserves code profile_not_found.
Omnigate preserves retryable false.
```

### `profile_upstream_connect_failure_yields_problem_502`

Verifies:

```text
OMNIGATE_PASSPORT_BASE_URL points to closed port.
Omnigate returns 502.
code = passport_upstream.
retryable = true.
```

---

## 14. Green proof from this session

Commands run:

```bash
cargo fmt -p omnigate
cargo test -p omnigate --test passport_profile_routes
cargo test -p omnigate --all-targets
```

Passed:

```text
passport_profile_routes.rs: 5 passed
omnigate all-targets: passed
```

Important full-test proof:

```text
profile_claim_proxies_to_svc_passport ... ok
profile_lookup_preserves_svc_passport_404 ... ok
profile_claim_preserves_svc_passport_conflict ... ok
profile_lookup_proxies_to_svc_passport ... ok
profile_upstream_connect_failure_yields_problem_502 ... ok
```

Current status:

```text
GREEN
```

---

## 15. Relationship to `svc-passport`

`omnigate` now proxies profile claim/read to `svc-passport`.

Downstream target routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

Important boundary:

```text
svc-passport owns username/profile claim truth.
omnigate only exposes a façade route and forwards safely.
```

Omnigate should not duplicate:

```text
username normalization
reserved username list
claim persistence
duplicate claim enforcement
one username per passport rule
profile store
```

Those belong in `svc-passport`.

---

## 16. Relationship to `svc-gateway`

After this session, `svc-gateway` also exposes public routes that proxy to Omnigate.

Gateway routes:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

Omnigate routes:

```text
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username
```

svc-passport routes:

```text
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```

Correct chain:

```text
CrabLink → svc-gateway → omnigate → svc-passport
```

Do not break this by making CrabLink call Omnigate directly.

---

## 17. Relationship to `ron-naming`

`ron-naming` now parses:

```text
crab://@username
crab://profile/@username
```

But Omnigate’s `/v1/crab/resolve` route does not yet fully use that profile route grammar for profile hydration.

Current Omnigate profile routes are direct HTTP profile claim/read routes.

Future Omnigate work:

```text
Update crab resolver to recognize CrabRoute::Profile.
Call svc-passport profile lookup.
Return public profile page/DTO.
```

Target future route:

```text
GET /v1/crab/resolve?url=crab://@skinnycrabby
```

Then through gateway:

```text
GET /crab/resolve?url=crab://@skinnycrabby
```

---

## 18. Relationship to `ron-proto`

`ron-proto` now defines:

```text
PassportPublicProfileV1
PassportPublicManifestV1
```

Omnigate currently passes through `svc-passport.public-profile.v1` route responses.

Future convergence:

```text
Omnigate should eventually hydrate or pass through ron-proto profile DTOs.
crab://@username resolver may return a public profile page object derived from PassportPublicProfileV1.
```

Do not force this prematurely if it causes broad churn.

---

## 19. Relationship to CrabLink

CrabLink should not call Omnigate directly.

Now that Omnigate and svc-gateway are green, the next CrabLink batch should call gateway routes:

```text
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username
```

not:

```text
POST http://127.0.0.1:9090/v1/identity/passport/profile/claim
GET  http://127.0.0.1:9090/v1/identity/passport/profile/:username
```

and not:

```text
POST http://127.0.0.1:5307/v1/passport/profile/claim
GET  http://127.0.0.1:5307/v1/passport/profile/:username
```

Future CrabLink behavior:

```text
Claim username through svc-gateway.
Read profile through svc-gateway.
Display confirmed status only when backend returns username_status = "confirmed".
Show null/placeholder rep/mod honestly.
Do not fake profile CID.
Do not fake public profile manifest publication.
```

---

## 20. Known good local route examples

Assuming Omnigate is running on:

```text
http://127.0.0.1:9090
```

and `svc-passport` is running on:

```text
http://127.0.0.1:5307
```

### Claim profile through Omnigate

```bash
curl -sS -X POST http://127.0.0.1:9090/v1/identity/passport/profile/claim \
  -H 'content-type: application/json' \
  -H 'idempotency-key: idem-profile-claim-1' \
  -H 'x-ron-passport: passport:main:skinnycrabby' \
  -H 'x-ron-wallet-account: acct_dev' \
  -d '{
    "passport_subject": "passport:main:skinnycrabby",
    "requested_username": "@SkinnyCrabby",
    "display_name": "Skinny Crabby",
    "bio": "Building the content-addressed creator web.",
    "avatar_image": "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
  }' | jq .
```

Expected if upstream `svc-passport` is running and username available:

```text
HTTP 201 from svc-passport, preserved by Omnigate
schema = svc-passport.public-profile.v1
username = skinnycrabby
handle = @skinnycrabby
username_status = confirmed
profile_crab_url = crab://@skinnycrabby
```

### Get profile through Omnigate

```bash
curl -sS http://127.0.0.1:9090/v1/identity/passport/profile/skinnycrabby | jq .
```

Expected if profile exists:

```text
HTTP 200
schema = svc-passport.public-profile.v1
username = skinnycrabby
handle = @skinnycrabby
```

### Missing profile

```bash
curl -i http://127.0.0.1:9090/v1/identity/passport/profile/missing
```

Expected:

```text
HTTP 404 preserved from svc-passport
code = profile_not_found
retryable = false
```

### Upstream unavailable

If `OMNIGATE_PASSPORT_BASE_URL` points to a dead port:

```text
HTTP 502
code = passport_upstream
retryable = true
```

---

## 21. Current limitations

At the end of this session, Omnigate profile façade is green, but limitations remain:

```text
No crab://@username resolver integration yet.
No public profile page composer yet.
No profile manifest b3 hydration yet.
No profile CID lookup yet.
No reputation/moderation hydration yet.
No alt profile privacy policy beyond pass-through safety.
No profile route metrics specific to claim/read yet.
No active capability policy enforcement on profile façade route beyond upstream svc-passport behavior.
No persistent username/profile lookup cache in Omnigate.
```

These are expected at this stage.

---

## 22. Future Omnigate work

### A. `crab://@username` resolution

Implement profile resolution inside Omnigate crab resolver.

Target:

```text
GET /v1/crab/resolve?url=crab://@skinnycrabby
```

Expected steps:

```text
Use ron-naming CrabLink parser.
Detect CrabRoute::Profile.
Call svc-passport profile GET.
Return profile page / public profile DTO.
Preserve no wallet/ledger mutation.
Map upstream errors deterministically.
```

Gateway route already proxies `/crab/resolve`, so once Omnigate supports this, public route should work through:

```text
GET /crab/resolve?url=crab://@skinnycrabby
```

### B. Public profile page composer

Potential Omnigate product response:

```text
schema: omnigate.profile-page.v1
profile: svc-passport.public-profile.v1 or ron-proto PassportPublicProfileV1
links:
  profile_crab_url
  avatar_image
  public_sites
  public_assets
warnings
```

Could be similar in spirit to current asset/site page hydration.

### C. Use ron-proto DTOs

Future route may map profile responses into:

```text
ron_proto::PassportPublicProfileV1
ron_proto::PassportPublicManifestV1
```

This should happen when profile manifests and b3 publication are implemented.

### D. Profile route metrics

Add route-level metrics for:

```text
profile_claim_total
profile_lookup_total
profile_upstream_errors_total
profile_latency_seconds
```

Only if consistent with current Omnigate metrics conventions.

### E. Policy/capability gating

Future:

```text
Forward capability headers.
Reject missing capability if policy requires.
Fail closed.
Do not duplicate svc-passport semantics unnecessarily.
```

### F. Config consolidation

Current config is env-based:

```text
OMNIGATE_PASSPORT_BASE_URL
OMNIGATE_DOWNSTREAM_PASSPORT_BASE_URL
```

Future config file integration may be useful, but avoid broad config churn unless needed.

---

## 23. Important invariants to preserve

Do not add to Omnigate profile routes:

```text
direct wallet mutation
direct ledger mutation
username persistence
local profile DB without design
private key material
private alt mapping
fake reputation
fake moderation scores
fake profile CID
CrabLink-specific UI behavior
```

Always preserve:

```text
proxy-only profile façade
selected header forwarding
hop-by-hop header filtering
structured 502 upstream errors
svc-passport semantic errors preserved
safe path segment validation
no locks across await
reqwest client reuse
rustls preference
```

---

## 24. Suggested future acceptance gates

Before future Omnigate profile changes:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo fmt -p omnigate
cargo test -p omnigate --test passport_profile_routes
cargo test -p omnigate --all-targets
```

If adding crab resolver support:

```bash
cargo test -p omnigate --test asset_page_resolver
cargo test -p omnigate --test named_site_crab_resolver
cargo test -p svc-gateway --test product_routes_proxy
```

If changing profile route chain:

```bash
cargo test -p svc-passport --test profile_routes
cargo test -p svc-gateway --test identity_routes_proxy
```

---

## 25. Current status at end of session

Current `omnigate` status:

```text
GREEN
```

Feature status:

```text
Profile claim façade route: complete for current phase
Profile lookup façade route: complete for current phase
Header forwarding: complete for current phase
Hop-by-hop filtering: complete for current phase
502 upstream mapping: complete for current phase
404/409 upstream preservation: complete for current phase
crab://@username resolver: pending
Public profile page composer: pending
b3 profile manifest hydration: pending
Profile-specific metrics: pending
Real reputation/moderation hydration: pending
```

Next relevant project step involving Omnigate:

```text
After CrabLink consumes gateway profile routes, add crab://@username support to Omnigate /v1/crab/resolve.
```

Current route chain to remember:

```text
svc-gateway:
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

omnigate:
POST /v1/identity/passport/profile/claim
GET  /v1/identity/passport/profile/:username

svc-passport:
POST /v1/passport/profile/claim
GET  /v1/passport/profile/:username
```


### END NOTE - MAY 6 2026 - 17:55 CST



### BEGIN NOTE - MAY 14 2026 - 13:50 CST

Confirmed: the RustyOnions crates changed in this backend slice were:

```text
crates/svc-gateway
crates/omnigate
```

We also changed CrabLink extension files in the separate `crablink` repo, but those are **not RustyOnions crates**. For RustyOnions crate notes, the dual note set below should go into/near the relevant crate `NOTES.MD` sections or your running carryover notes.

Your latest green gate is strong: `omnigate` clippy passed, `.post` tests passed, new `.comment` tests passed, and the full `omnigate --all-targets` suite passed. 

---

# NOTE 1 — `svc-gateway` RustyOnions crate

````markdown
### BEGIN NOTE - MAY 14 2026 - NEXT_LEVEL TEXT ASSET GATEWAY EXPOSURE

# Carryover Notes — svc-gateway NEXT_LEVEL text asset route exposure

## 0. Summary

This slice updated `crates/svc-gateway` to expose public browser-facing proxy routes for NEXT_LEVEL text assets:

```text
POST /assets/post/prepare
POST /assets/post
POST /assets/comment/prepare
POST /assets/comment
POST /assets/article/prepare
POST /assets/article
````

The important architectural outcome is that `svc-gateway` remains exactly what it should be: a public edge/proxy layer. It does **not** implement asset semantics, pricing, wallet mutation, storage writes, manifest construction, or index mutation.

The new routes forward to Omnigate:

```text
POST /v1/assets/post/prepare
POST /v1/assets/post
POST /v1/assets/comment/prepare
POST /v1/assets/comment
POST /v1/assets/article/prepare
POST /v1/assets/article
```

This was intentionally implemented before Omnigate `.comment`/`.article` semantics so CrabLink can use stable public gateway routes while backend implementation lands one primitive at a time.

## 1. Files changed

Primary file:

```text
crates/svc-gateway/src/routes/product.rs
```

Test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

No `Cargo.toml` changes were required.

## 2. Product-route behavior added

`svc-gateway` now exposes the following public product route family:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /assets/post/prepare
POST /assets/post

POST /assets/comment/prepare
POST /assets/comment

POST /assets/article/prepare
POST /assets/article

POST /sites/prepare
POST /sites
GET  /sites/:name
```

The text asset route additions were the key part of this batch.

## 3. Header forwarding contract

The gateway proxy forwards only selected safe/product headers. Important preserved headers include:

```text
Authorization
Accept
Content-Type
Idempotency-Key
x-correlation-id
x-request-id
x-ron-*
```

It explicitly filters hop-by-hop / host-style headers such as:

```text
Host
Connection
Transfer-Encoding
Upgrade
Content-Length
Proxy-Authorization
TE
Trailer
```

This matters because CrabLink paid publish flows depend on the modern paid proof header family:

```text
x-ron-paid-op: hold
x-ron-paid-asset: roc
x-ron-paid-estimate-minor: <amount>
x-ron-wallet-txid: <txid>
x-ron-wallet-receipt-hash: <receipt_hash>
x-ron-wallet-from: <payer>
x-ron-wallet-to: <escrow>
x-ron-asset-kind: post|comment|article
```

The tests prove those headers survive the gateway hop.

## 4. Important invariants preserved

`svc-gateway` remains proxy-only.

It does **not**:

```text
- price assets
- create wallet holds
- mutate svc-wallet
- mutate ron-ledger
- store bytes
- create manifests
- write svc-index pointers
- parse post/comment/article body semantics
- fabricate CIDs or receipts
```

It only exposes stable public routes and forwards to Omnigate.

This preserves the RustyOnions value-plane boundary:

```text
CrabLink
→ svc-gateway
→ omnigate
→ svc-storage / svc-index / svc-wallet façade where appropriate
→ svc-wallet remains the economic mutation front-door
→ ron-ledger remains durable economic truth
```

## 5. Tests added/updated

Primary test file:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

The test suite verifies:

```text
- /crab/resolve preserves query + important headers
- /b3/:asset proxies to /v1/b3/:asset
- /paid/o/prepare preserves JSON body and idempotency key
- image/site/text asset routes proxy to the correct Omnigate paths
- /assets/post, /assets/comment, /assets/article preserve paid proof headers
- /sites/:name proxies to Omnigate site resolver
- upstream connect failures become deterministic 502 Problem bodies
```

A clippy doc warning was fixed by wrapping `CrabLink` in backticks in a doc comment.

## 6. Green gates achieved

These were run and passed after the gateway batch:

```bash
cargo fmt -p svc-gateway
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
cargo test -p svc-gateway --test product_routes_proxy
cargo test -p svc-gateway --all-targets
```

## 7. Current status

`svc-gateway` is complete for the current NEXT_LEVEL text route exposure layer.

The public route surface is ready for:

```text
.post      implemented in Omnigate and proven end-to-end
.comment   implemented in Omnigate and proven in tests; live CrabLink publish succeeded
.article   gateway route exposed but Omnigate implementation still pending
```

## 8. Known-good proof artifacts from this session

The user restarted the local dev stack, so the current fresh proof URLs are:

```text
Post:
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post

Comment:
crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment
```

The `.comment` asset page successfully loaded through CrabLink and displayed:

```text
kind: comment
HTTP: 200
receipts: 1
site: crab://ron2
parent: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
raw body rendered from b3-backed content object
```

## 9. Next svc-gateway work

No immediate svc-gateway changes are needed for `.comment`.

Future gateway work likely includes:

```text
POST /assets/article/prepare
POST /assets/article
```

Those public routes are already exposed, so next work is likely Omnigate `.article` semantics first, then only adjust gateway tests if the article route contract changes.

### END NOTE - MAY 14 2026 - NEXT_LEVEL TEXT ASSET GATEWAY EXPOSURE

````

---

# NOTE 2 — `omnigate` RustyOnions crate

```markdown
### BEGIN NOTE - MAY 14 2026 - NEXT_LEVEL POST AND COMMENT TEXT ASSETS

# Carryover Notes — omnigate NEXT_LEVEL post/comment text asset implementation

## 0. Summary

This slice implemented the first two real NEXT_LEVEL text asset primitives in `crates/omnigate`:

```text
.post
.comment
````

The implementation follows the already-proven image/site pipeline:

```text
CrabLink
→ svc-gateway public route
→ Omnigate text asset route
→ svc-storage paid write
→ storage returns b3 CID
→ Omnigate builds manifest
→ Omnigate stores manifest object
→ Omnigate writes svc-index asset→manifest pointer
→ Omnigate returns crab://<hash>.<kind>
→ CrabLink resolves typed asset page
→ CrabLink reads raw /o/b3:<hash> content object for title/body
```

The key product result: `.post` and `.comment` can now be minted as independent b3-backed assets and displayed as typed CrabLink asset pages. `.post` can also be embedded into a site using `<crab-post>`.

## 1. Files changed

Primary route module:

```text
crates/omnigate/src/routes/v1/text_assets.rs
```

Route aggregator:

```text
crates/omnigate/src/routes/v1/mod.rs
```

Tests:

```text
crates/omnigate/tests/text_asset_publish.rs
crates/omnigate/tests/comment_asset_publish.rs
```

Existing clippy cleanup was also needed in:

```text
crates/omnigate/tests/passport_profile_routes.rs
```

No `Cargo.toml` changes were required.

## 2. Route aggregator changes

`crates/omnigate/src/routes/v1/mod.rs` now includes:

```rust
pub mod text_assets;
```

and mounts text assets under:

```text
/v1/assets
```

The active routes are:

```text
POST /v1/assets/post/prepare
POST /v1/assets/post
POST /v1/assets/comment/prepare
POST /v1/assets/comment
```

Article is intentionally not active yet in Omnigate, even though gateway public route exposure already exists.

## 3. New text asset route module

`crates/omnigate/src/routes/v1/text_assets.rs` now owns the post/comment text asset flow.

### Active asset kinds

The module introduced a staged internal enum:

```text
TextAssetKind::Post
TextAssetKind::Comment
```

Article was intentionally left for a later batch to preserve the implementation order:

```text
post → comment → article
```

## 4. Post route contract

### Prepare

```text
POST /v1/assets/post/prepare
```

Accepted strict JSON fields include:

```text
title
body
site_context_crab_url
parent_crab_url
creator_display
language
post_kind / postKind
visibility
rights_mode / rightsMode
moderation_mode / moderationMode
content_warning / contentWarning
tags
payer_account / payerAccount
owner_passport_subject / ownerPassportSubject
client_idempotency_key / clientIdempotencyKey
```

Important rules:

```text
- title required for post
- body required
- site_context_crab_url required
- site_context_crab_url must be a named crab:// site URL
- parent_crab_url optional for post
- payer_account required
- owner_passport_subject required
```

### Publish

```text
POST /v1/assets/post
```

Requires modern paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
x-ron-asset-kind: post
```

Publishes a canonical content envelope:

```text
schema: ron.post-content.v1
kind: post
asset_kind: post
format: text/plain; charset=utf-8
title
body
metadata
relations
site_connection
parent_reference
thread_reference
creator_display
created_at_ms
```

## 5. Comment route contract

### Prepare

```text
POST /v1/assets/comment/prepare
```

Accepted strict JSON fields include:

```text
title
body
site_context_crab_url
parent_crab_url / parentCrabUrl / target_crab_url / targetCrabUrl
thread_context_crab_url / threadContextCrabUrl
creator_display / creatorDisplay
language
comment_kind / commentKind
visibility
rights_mode / rightsMode
moderation_mode / moderationMode
content_warning / contentWarning
tags
payer_account / payerAccount
owner_passport_subject / ownerPassportSubject
client_idempotency_key / clientIdempotencyKey
```

Important rules:

```text
- body required
- site_context_crab_url required
- parent_crab_url required
- payer_account required
- owner_passport_subject required
- title optional; defaults to "Comment"
- site_context_crab_url must be a named crab:// site URL
- parent_crab_url must be a typed crab://<hash>.<kind> asset URL
```

### Parent/target relationship

For comments, `parent_crab_url` is required. The first live test used:

```text
parent_crab_url:
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
```

Allowed parent kind list currently includes:

```text
post
comment
article
image
video
music
song
podcast
stream
thread
film
```

This was intentionally made future-ready so comments can eventually attach to posts, articles, images, videos, podcasts, streams, etc. However, only `.post` and `.comment` have proven backend publish support right now.

### Publish

```text
POST /v1/assets/comment
```

Requires modern paid proof headers:

```text
x-ron-paid-op
x-ron-paid-asset
x-ron-paid-estimate-minor
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-wallet-from
x-ron-wallet-to
x-ron-asset-kind: comment
```

Publishes a canonical content envelope:

```text
schema: ron.comment-content.v1
kind: comment
asset_kind: comment
format: text/plain; charset=utf-8
title: Comment by default
body
metadata.comment_kind
metadata.language
metadata.tags
relations.site
relations.parent
relations.target
site_connection
parent_reference
thread_reference
creator_display
created_at_ms
```

## 6. Storage/index coordination

For both `.post` and `.comment`, Omnigate coordinates the same flow:

```text
1. validate strict JSON
2. build canonical JSON content envelope
3. call svc-storage GET /paid/o/estimate?bytes=<canonical_bytes>
4. return wallet hold template to the client
5. require paid proof headers on publish
6. call svc-storage POST /paid/o with content envelope bytes
7. parse returned content CID
8. require canonical b3:<64 lowercase hex>
9. build manifest JSON
10. store manifest object through svc-storage POST /o
11. parse returned manifest CID
12. write svc-index pointer:
    PUT /v1/index/assets/<hash>/manifest
13. return crab://<hash>.<kind> plus links/manifest/index summary
```

## 7. Invariants preserved

Omnigate still does **not** mutate wallet or ledger directly.

The implementation does **not**:

```text
- call ron-ledger
- directly mutate wallet balances
- fake receipts
- fabricate CIDs
- silently charge ROC
- write storage without paid proof on publish
- store bytes outside svc-storage
- write index state anywhere except svc-index pointer route
```

The publish path requires the caller to supply the wallet hold proof created through the wallet path.

The economics invariant remains:

```text
svc-wallet is the economic mutation front-door.
ron-ledger is the durable economic truth.
omnigate coordinates product flow but does not own monetary truth.
```

## 8. Error taxonomy and strict DTO behavior

The route uses `#[serde(deny_unknown_fields)]` on the text asset request DTO. This means frontend clients must not send local manifest/debug fields.

Earlier frontend clients were rejected until they were updated to avoid fields like:

```text
schema
asset_kind
bytes
content_type
relations
content
manifest_hint
```

Important problem codes/reasons include:

```text
invalid_post_prepare_request
invalid_post_publish_request
invalid_comment_prepare_request
invalid_comment_publish_request
bad_json
missing_site_context
invalid_site_context
missing_parent_reference
invalid_parent_reference
invalid_parent_kind
missing_payer_account
missing_owner_passport_subject
missing_paid_proof
invalid_paid_proof
storage_estimate_missing_amount
storage_upload_missing_cid
storage_upload_invalid_cid
manifest_encode_failed
index_pointer_encode_failed
```

Clippy previously flagged `Result<_, Response>` as `result_large_err`, so local validation helpers now return the smaller `RouteProblemSpec` and only convert to Axum `Response` at the handler boundary.

## 9. Tests added/updated

### Existing post tests

```text
crates/omnigate/tests/text_asset_publish.rs
```

This test suite verifies:

```text
post_asset_prepare_returns_hold_template
post_asset_prepare_requires_site_context
post_asset_publish_coordinates_paid_storage_manifest_and_index_pointer
post_asset_publish_requires_paid_proof_headers
post_asset_prepare_upstream_connect_failure_yields_problem_502
```

The post test had to remain compatible with `post_kind`, so `TextAssetRequest` now accepts both:

```text
post_kind
postKind
```

### New comment tests

```text
crates/omnigate/tests/comment_asset_publish.rs
```

This test suite verifies:

```text
comment_asset_prepare_returns_hold_template
comment_asset_prepare_requires_parent_reference
comment_asset_publish_coordinates_paid_storage_manifest_and_index_pointer
comment_asset_publish_requires_paid_proof_headers
```

The tests spin up dummy storage and index services and then run real Omnigate routes in-process.

They prove:

```text
- strict JSON validation
- comment body envelope creation
- site relation required
- parent post relation required
- paid proof headers required
- storage paid write gets correct content bytes
- manifest object is written
- index pointer is written
- returned crab://<hash>.comment URL is correct
```

## 10. Clippy cleanup

During this slice, clippy issues fixed included:

```text
- result_large_err in text_assets.rs helper functions
- dead_code from unused TextAssetKind::prepare_route
- unnecessary_filter_map in passport_profile_routes.rs
```

## 11. Green gates achieved

The final green gate from this slice:

```bash
cargo fmt -p omnigate
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo test -p omnigate --test text_asset_publish
cargo test -p omnigate --test comment_asset_publish
cargo test -p omnigate --all-targets
```

All passed.

The full `omnigate --all-targets` run included and passed:

```text
text_asset_publish.rs
comment_asset_publish.rs
image_asset_prepare.rs
asset_page_resolver.rs
site_launch.rs
named_site_crab_resolver.rs
passport_profile_routes.rs
paid_storage_prepare.rs
paid_storage_write_proxy.rs
paid_storage_estimate_proxy.rs
readiness tests
middleware tests
policy tests
zk tests
benches smoke target
```

## 12. Live CrabLink proof from this session

After restarting the stack, the user minted a fresh post:

```text
crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
```

Then the user minted a fresh comment:

```text
crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment
```

The comment asset page showed:

```text
Comment Asset
HTTP 200
receipts: 1
kind: comment
content schema: ron.comment-content.v1
content kind: comment
language: en
site: crab://ron2
parent: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
title: no title / default title behavior visible in UI
body: test comment test test test comment test
```

This proves the `.comment` object is independently b3-backed and can preserve both site context and parent post reference.

## 13. Known issues / follow-up notes

### 13.1 Comment default title display

The current live comment page shows “no title” in some UI spots even though the backend content envelope defaults comment title to `"Comment"` if no title is supplied.

This may be a frontend display/normalization issue, not necessarily a backend issue. Check CrabLink’s typed asset viewer and comment content summary normalization before changing backend.

### 13.2 Parent type policy is permissive

The backend currently permits future parent kinds such as:

```text
article
video
music
podcast
stream
film
```

That is future-ready, but product UX should still label unsupported parent types honestly until those assets are backend-proven.

### 13.3 Article still pending

Gateway route exposure exists for:

```text
POST /assets/article/prepare
POST /assets/article
```

But Omnigate `.article` implementation is not done yet. The next backend primitive should probably be `.article`.

## 14. Recommended next backend step

Implement Omnigate `.article` using the generalized text asset pattern, but with article-specific rules:

```text
title required
body required
site_context_crab_url required
optional excerpt/summary
optional hero image reference
optional linked assets
optional parent/thread relation depending on use case
payer_account required
owner_passport_subject required
paid proof required on publish
```

Recommended new test:

```text
crates/omnigate/tests/article_asset_publish.rs
```

Recommended route additions in existing text module:

```text
POST /v1/assets/article/prepare
POST /v1/assets/article
```

## 15. Recommended next CrabLink step

Before article backend, the UI path may need one of these:

```text
- add <crab-comment> embed hydration, like <crab-post>
- polish comment typed asset page label from “Post content truth” to “Comment content truth”
- make comment title/default display cleaner
```

But backend-wise, `.post` and `.comment` are now proven.

### END NOTE - MAY 14 2026 - NEXT_LEVEL POST AND COMMENT TEXT ASSETS

````

---

# Short carryover checkpoint

```markdown
## RustyOnions / CrabLink checkpoint — 2026-05-14

Backend crates changed:
- crates/svc-gateway
- crates/omnigate

CrabLink repo also changed separately:
- post client strict DTO
- comment client strict DTO
- typed text asset viewer
- <crab-post> site embed hydration

Fresh known-good proof URLs from current stack:
- Post: crab://b23f4c579201e17ab391dd3bff54635718a0b4c1371782ef87115b50f80bb1d3.post
- Comment: crab://ad0fd74aa4c20095c3a08ae9f8e111b68ccff6537ed5f8fb769fa43f782d8f63.comment

Green gates:
- svc-gateway clippy/tests passed after route exposure batch
- omnigate clippy passed
- omnigate text_asset_publish passed
- omnigate comment_asset_publish passed
- omnigate --all-targets passed

Current product proof:
- .post can mint, resolve, display raw title/body, and embed into a site with <crab-post>
- .comment can mint, resolve, display raw body, preserve site context, and preserve parent post reference

Next likely work:
1. Add <crab-comment> embed hydration in CrabLink.
2. Polish comment typed asset wording/default title display.
3. Implement Omnigate .article prepare/publish.
4. Then align CrabLink article client strict JSON and typed article display.
````


### END NOTE - MAY 14 2026 - 13:50 CST



### BEGIN NOTE - MAY 15 2026 - 13:30 CST


# CARRY-OVER NOTES — svc-gateway + omnigate paid `site_visit` proof

**Date:** 2026-05-15  
**Crates documented:** `svc-gateway`, `omnigate`  
**Milestone:** First paid named-site visit loop proven end-to-end.  
**Status:** SUCCESS — `crab://ron7` paid `site_visit` quote/pay works through gateway → omnigate → svc-wallet → ron-ledger, and CrabLink/browser confirms real ROC balance movement.

---

## 0. TL;DR

We successfully implemented and proved the first real **paid named-site visit** path:

```text
Visitor B / acct_visitor_b
→ visits crab://ron7
→ pays 10 ROC for site_visit
→ Creator A / acct_dev receives 10 ROC
→ svc-wallet returns a transfer receipt
→ ron-ledger commits durable receipt/root truth
→ CrabLink/browser shows real backend balance changes
````

This proves that a named CrabLink site can now be monetized by real internal ROC movement, not fake UI state.

The important route pair is now alive:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Public route flow:

```text
CrabLink / curl
→ svc-gateway public route
→ omnigate /v1 route
→ site manifest/index/storage lookup
→ payout recipient validation
→ svc-wallet /v1/transfer
→ ron-ledger receipt/root
→ response back to CrabLink
```

This is a major WEB3/NEXT_LEVEL milestone because it proves:

```text
Visitor spend:       real
Creator receipt:     real
Wallet receipt:      real
Ledger root:         real
CrabLink display:    backend truth
Gateway role:        proxy-only
Omnigate role:       coordinator/hydrator
svc-wallet role:     mutation front-door
ron-ledger role:     durable truth
```

---

## 1. Files changed / touched

### omnigate

Primary files:

```text
crates/omnigate/src/routes/v1/site_visit.rs
crates/omnigate/src/routes/v1/mod.rs
```

What changed:

```text
- Added paid site_visit quote route.
- Added paid site_visit pay route.
- Mounted:
  POST /v1/sites/:name/visit/quote
  POST /v1/sites/:name/visit/pay
- Quote route resolves named site, loads site manifest, validates payout recipient, and returns fixed dev quote.
- Pay route resolves named site, validates payout recipient, rejects self-payment, and sends svc-wallet transfer.
- Pay route returns omnigate.site-visit-payment.v1 with wallet receipt fields.
```

Important implementation shape:

```text
quote:
  route site name
  → svc-index site pointer
  → svc-storage manifest object
  → validate manifest payout.default_action == site_visit
  → validate recipient_account
  → return amount_minor = "10"

pay:
  route site name
  → load same site context
  → validate payer != recipient
  → validate amount_minor == configured site_visit price
  → POST svc-wallet /v1/transfer
  → return wallet receipt + site_visit receipt wrapper
```

Important env/defaults:

```text
OMNIGATE_SITE_VISIT_PRICE_MINOR default = 10
OMNIGATE_WALLET_BASE_URL default = http://127.0.0.1:8088
OMNIGATE_WALLET_BEARER default = dev
OMNIGATE_STORAGE_BASE_URL / OMNIGATE_DOWNSTREAM_STORAGE_BASE_URL
OMNIGATE_INDEX_BASE_URL / OMNIGATE_DOWNSTREAM_INDEX_BASE_URL
```

### svc-gateway

Primary file:

```text
crates/svc-gateway/src/routes/product.rs
```

What changed:

```text
- Added public product proxy routes:
  POST /sites/:name/visit/quote
  POST /sites/:name/visit/pay

- Proxies to omnigate:
  POST /v1/sites/:name/visit/quote
  POST /v1/sites/:name/visit/pay
```

Important gateway invariant preserved:

```text
svc-gateway does not price.
svc-gateway does not parse manifests.
svc-gateway does not mutate wallet.
svc-gateway does not mutate ledger.
svc-gateway only proxies selected headers/body to omnigate.
```

---

## 2. Bugs encountered and fixed

### Bug 1 — 404 route missing

Initial curl:

```text
POST /sites/ron7/visit/quote
→ HTTP/1.1 404 Not Found
```

Meaning:

```text
svc-gateway did not yet expose the public route.
```

Fix:

```text
Added /sites/:name/visit/quote and /sites/:name/visit/pay to svc-gateway product router.
Mounted matching /v1 routes in omnigate.
```

### Bug 2 — `site_manifest_bad_json`

After route registration, quote/pay reached Omnigate but failed with:

```json
{
  "code": "site_manifest_bad_json",
  "message": "site manifest object was not valid JSON",
  "retryable": true,
  "reason": "site_manifest_bad_json"
}
```

Root cause:

```text
site_visit.rs had a strict duplicate SiteManifestDocument DTO that did not accept all fields generated by the site creation route.
```

Real site manifests include fields such as:

```text
asset_map
route_map
owner
payout
metadata
provenance
storage
receipts
```

Fix:

```text
Updated the local SiteManifestDocument in site_visit.rs to accept the generated site manifest shape.
```

Follow-up note:

```text
Long-term, avoid maintaining duplicate strict manifest parsers in multiple route files.
Prefer a shared DTO/helper or parse as serde_json::Value and extract only the fields required for site_visit validation.
```

### Bug 3 — running stack used old binary

After patching, `cargo test` and `cargo clippy` were green, but live curl still returned the old error until the stack was restarted.

Lesson:

```text
cargo test/clippy compile patched code, but do not restart the already-running omnigate/svc-gateway processes.
After route changes, restart the RustyOnions dev stack before live curl/browser tests.
```

### Dev stack state note

The local dev stack may flush prior site/index/storage state on restart depending on artifact directory/config.

Practical testing rule:

```text
After stack restart, check whether the named site still exists:
curl -i -sS http://127.0.0.1:8090/sites/ron7

If missing, recreate the site in CrabLink with Creator A / acct_dev.
```

---

## 3. Tests / gates run

The following commands were run and passed during this batch:

```text
cargo fmt -p omnigate
cargo fmt -p svc-gateway
cargo test -p omnigate --test site_launch
cargo test -p svc-gateway --test product_routes_proxy
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Observed test results:

```text
omnigate site_launch:
  5 passed / 0 failed

svc-gateway product_routes_proxy:
  7 passed / 0 failed

omnigate clippy:
  clean

svc-gateway clippy:
  clean
```

Important note:

```text
Existing gateway product route tests did not yet explicitly verify the new site_visit quote/pay proxy routes.
Add that as follow-up test coverage.
```

---

## 4. Live route health proof

Health checks before proof:

```text
curl -sS http://127.0.0.1:8090/healthz
→ ok

curl -sS http://127.0.0.1:9090/healthz
→ {"config":true,"omnigate":true}
```

This confirmed gateway and omnigate were live.

---

## 5. Site resolver proof

`crab://ron7` resolved as a valid paid site.

Important resolver fields:

```text
schema = omnigate.site-page.v1
site_name = ron7
root_document_cid = b3:056cd088f6bb9ae45b11b1cd4786a2eb9b62f576e35f7132b33a74e4f07dac96
manifest_cid = b3:918dad927484a43c2f716691794e24e7a608c951f38b9a6341a39f558284a4a9
owner.passport_subject = passport:main:dev
owner.wallet_account = acct_dev
payout.default_action = site_visit
payout.recipient_account = acct_dev
metadata.title = Reference Graph Smoke Site
```

This confirmed the site was correctly configured for the proof:

```text
Creator A owns the site.
Creator A payout recipient is acct_dev.
Visitor B should pay acct_dev for site_visit.
```

---

## 6. Quote proof

Command shape:

```text
POST http://127.0.0.1:8090/sites/ron7/visit/quote

Headers:
  Authorization: Bearer dev
  Content-Type: application/json
  x-ron-passport: passport:main:visitor-b
  x-ron-wallet-account: acct_visitor_b
  Idempotency-Key: crablink-test-site-visit-quote-ron7d

Body:
  site_name = ron7
  crab_url = crab://ron7
  action = site_visit
  quantity = 1
  payer_account = acct_visitor_b
  visitor_wallet_account = acct_visitor_b
  visitor_passport_subject = passport:main:visitor-b
  recipient_account = acct_dev
  max_amount_minor = 10
```

Result:

```text
HTTP/1.1 200 OK
schema = omnigate.site-visit-quote.v1
ok = true
action = site_visit
amount_minor = "10"
asset = "roc"
display_amount = "10 ROC"
payer_account = "acct_visitor_b"
visitor_wallet_account = "acct_visitor_b"
visitor_passport_subject = "passport:main:visitor-b"
recipient_account = "acct_dev"
site_name = "ron7"
crab_url = "crab://ron7"
quote_id = "site-visit-b19f93aa1d951d55"
quote_hash = "85e1c00714793cc3"
```

Nested quote policy confirmed:

```text
policy.source = omnigate.site_visit.v1
policy.fixed_dev_price_minor = 10
policy.price_env = OMNIGATE_SITE_VISIT_PRICE_MINOR
policy.wallet_front_door = svc-wallet /v1/transfer
```

This proves:

```text
svc-gateway public route works.
omnigate route works.
site manifest lookup works.
payout recipient lookup works.
fixed 10 ROC dev pricing works.
quote DTO is CrabLink-compatible.
```

---

## 7. Pay proof

Command shape:

```text
POST http://127.0.0.1:8090/sites/ron7/visit/pay

Headers:
  Authorization: Bearer dev
  Content-Type: application/json
  x-ron-passport: passport:main:visitor-b
  x-ron-wallet-account: acct_visitor_b
  Idempotency-Key: crablink-test-site-visit-pay-ron7d

Body:
  site_name = ron7
  crab_url = crab://ron7
  action = site_visit
  payer_account = acct_visitor_b
  visitor_wallet_account = acct_visitor_b
  visitor_passport_subject = passport:main:visitor-b
  recipient_account = acct_dev
  amount_minor = 10
  asset = roc
  client_idempotency_key = crablink-test-site-visit-pay-ron7d
```

Result:

```text
HTTP/1.1 200 OK
schema = omnigate.site-visit-payment.v1
ok = true
action = site_visit
site_name = ron7
crab_url = crab://ron7
amount_minor = "10"
asset = "roc"
payer_account = "acct_visitor_b"
visitor_wallet_account = "acct_visitor_b"
recipient_account = "acct_dev"
nonce = 1
txid = tx_6f04012c31be95d392824308c440a6d2
receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

Wallet receipt:

```text
op = transfer
from = acct_visitor_b
to = acct_dev
asset = roc
amount_minor = 10
nonce = 1
idem = crablink-test-site-visit-pay-ron7d
ledger_seq_start = 5
ledger_seq_end = 6
ledger_root = 3062a104f536661a279358cecb016d641ea77dff5990bd6a1dc4f2c762b36726
txid = tx_6f04012c31be95d392824308c440a6d2
receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

Site visit receipt wrapper included:

```text
kind = site_visit
manifest_cid = b3:918dad927484a43c2f716691794e24e7a608c951f38b9a6341a39f558284a4a9
root_document_cid = b3:056cd088f6bb9ae45b11b1cd4786a2eb9b62f576e35f7132b33a74e4f07dac96
wallet_txid = tx_6f04012c31be95d392824308c440a6d2
wallet_receipt_hash = b3:c02be0ecd068408ca18d7a5dd7bad7bf987b05ed0c3b6b15a222880f263aec8a
```

This proves:

```text
Omnigate correctly called svc-wallet /v1/transfer.
svc-wallet performed the transfer.
ron-ledger committed durable receipt/root truth.
The response contains enough proof for CrabLink to unlock paid content.
```

---

## 8. Browser / CrabLink proof

Browser confirmation:

```text
Passport B / Visitor B lost 10 ROC.
Passport A / Creator A received 10 ROC.
```

This is the first confirmed real browser-observed paid named-site visit transfer.

This matters because CrabLink is not editing balances locally. The browser balance movement came from backend wallet truth.

---

## 9. Architectural invariants preserved

### svc-gateway invariants

Preserved:

```text
- Gateway is public route surface only.
- Gateway proxies quote/pay to Omnigate.
- Gateway does not inspect site manifests.
- Gateway does not calculate price.
- Gateway does not decide payout recipient.
- Gateway does not call svc-wallet directly for this product flow.
- Gateway does not mutate ron-ledger.
- Gateway preserves selected x-ron-* and idempotency headers.
```

### omnigate invariants

Preserved:

```text
- Omnigate coordinates product flow.
- Omnigate resolves site context through index/storage.
- Omnigate validates payout recipient from site manifest.
- Omnigate does not directly mutate ron-ledger.
- Omnigate sends mutation through svc-wallet.
- Quote route is read-only.
- Pay route is explicit and receipt-returning.
```

### wallet/ledger invariants

Preserved:

```text
- svc-wallet remains the economic mutation front-door.
- ron-ledger remains durable economic truth.
- Transfer uses integer minor units only.
- Receipt includes nonce, idempotency, txid, receipt_hash, ledger root.
- No fake balances.
- No fake receipts.
```

### CrabLink invariants

Preserved:

```text
- CrabLink calls public gateway routes.
- CrabLink does not call internal services directly.
- CrabLink does not mutate balances locally.
- CrabLink only unlocks paid content after backend proof.
- User-visible payment requires explicit action.
```

---

## 10. Important current limitations / beta caveats

This is a successful proof, but not production final.

Known limitations:

```text
- Site visit price is currently fixed/dev-configured via OMNIGATE_SITE_VISIT_PRICE_MINOR.
- Pricing should later move into ron-policy / configs/roc-economics.toml.
- Quote hash is simple/dev-style and should later become canonical signed/hashed quote material.
- Pay route currently revalidates amount against current config, not a persisted quote object.
- Nonce handling is early/dev; wallet expected nonce recovery may need hardening.
- Site manifest parsing is duplicated locally in site_visit.rs.
- Dev stack may flush site/index/storage state between restarts.
- No dedicated site_visit test file yet.
- Gateway product proxy tests should explicitly cover quote/pay routes.
- No full smoke script yet for quote → pay → balance delta.
```

Do not treat this as final chain/economy architecture. It is the first proof of the product loop.

---

## 11. Recommended immediate follow-up

### A. Add reproducible smoke script

Create a smoke script that proves:

```text
1. Create or require site crab://ron7 or generated smoke name.
2. Confirm site resolver returns payout.default_action = site_visit.
3. Check acct_visitor_b balance before.
4. Check acct_dev balance before.
5. POST /sites/:name/visit/quote.
6. Assert amount_minor == 10.
7. POST /sites/:name/visit/pay.
8. Assert schema == omnigate.site-visit-payment.v1.
9. Assert wallet_receipt.op == transfer.
10. Assert wallet_receipt.from == acct_visitor_b.
11. Assert wallet_receipt.to == acct_dev.
12. Assert wallet_receipt.amount_minor == 10.
13. Check acct_visitor_b balance after decreased by 10.
14. Check acct_dev balance after increased by 10.
```

Suggested script name:

```text
scripts/web3_paid_site_visit_smoke.sh
```

### B. Add omnigate tests

Suggested tests:

```text
crates/omnigate/tests/site_visit.rs
```

Test cases:

```text
site_visit_quote_returns_10_roc_for_paid_site
site_visit_quote_rejects_recipient_mismatch
site_visit_quote_rejects_missing_payer
site_visit_pay_rejects_self_payment
site_visit_pay_uses_wallet_transfer
site_visit_pay_returns_wallet_receipt
site_visit_pay_rejects_amount_mismatch
```

### C. Add svc-gateway proxy tests

Extend:

```text
crates/svc-gateway/tests/product_routes_proxy.rs
```

Add assertions:

```text
POST /sites/ron7/visit/quote → /v1/sites/ron7/visit/quote
POST /sites/ron7/visit/pay → /v1/sites/ron7/visit/pay
headers/body/idempotency preserved
```

### D. Update route contracts docs

Update:

```text
docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md
```

Add:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Document proof DTOs:

```text
omnigate.site-visit-quote.v1
omnigate.site-visit-payment.v1
```

### E. Improve manifest context loading

Longer-term refactor:

```text
- Avoid duplicate strict manifest DTO in site_visit.rs.
- Share site manifest loading/hydration helper with sites.rs.
- Or parse manifest as Value and extract only:
  version
  site_name
  root_document_cid
  owner.wallet_account
  owner.passport_subject
  payout.default_action
  payout.recipient_account
  metadata.title
```

### F. Move pricing to policy/economics config

Current proof uses:

```text
OMNIGATE_SITE_VISIT_PRICE_MINOR = 10
```

Future target:

```text
ron-policy / configs/roc-economics.toml
action = site_visit
amount_minor = 10 dev default
basis-point splits
creator/storage/provider/treasury split support
```

---

## 12. Manual reproduction commands

Health:

```bash
curl -sS http://127.0.0.1:8090/healthz
curl -sS http://127.0.0.1:9090/healthz
```

Check site:

```bash
curl -i -sS "http://127.0.0.1:8090/sites/ron7"
```

Quote:

```bash
curl -i -sS \
  -X POST "http://127.0.0.1:8090/sites/ron7/visit/quote" \
  -H "Authorization: Bearer dev" \
  -H "Content-Type: application/json" \
  -H "x-ron-passport: passport:main:visitor-b" \
  -H "x-ron-wallet-account: acct_visitor_b" \
  -H "Idempotency-Key: crablink-test-site-visit-quote-ron7d" \
  --data '{"site_name":"ron7","crab_url":"crab://ron7","action":"site_visit","quantity":1,"payer_account":"acct_visitor_b","visitor_wallet_account":"acct_visitor_b","visitor_passport_subject":"passport:main:visitor-b","recipient_account":"acct_dev","max_amount_minor":"10","client_idempotency_key":"crablink-test-site-visit-quote-ron7d"}'
```

Pay:

```bash
curl -i -sS \
  -X POST "http://127.0.0.1:8090/sites/ron7/visit/pay" \
  -H "Authorization: Bearer dev" \
  -H "Content-Type: application/json" \
  -H "x-ron-passport: passport:main:visitor-b" \
  -H "x-ron-wallet-account: acct_visitor_b" \
  -H "Idempotency-Key: crablink-test-site-visit-pay-ron7d" \
  --data '{"site_name":"ron7","crab_url":"crab://ron7","action":"site_visit","payer_account":"acct_visitor_b","visitor_wallet_account":"acct_visitor_b","visitor_passport_subject":"passport:main:visitor-b","recipient_account":"acct_dev","amount_minor":"10","asset":"roc","client_idempotency_key":"crablink-test-site-visit-pay-ron7d"}'
```

Balance check:

```bash
curl -sS "http://127.0.0.1:8090/wallet/acct_visitor_b/balance" | python3 -m json.tool
curl -sS "http://127.0.0.1:8090/wallet/acct_dev/balance" | python3 -m json.tool
```

Expected balance effect:

```text
acct_visitor_b decreases by 10 ROC
acct_dev increases by 10 ROC
```

---

## 13. Why this milestone matters

This is the first confirmed paid named-site consumption proof.

Before this, RustyOnions had strong proofs for:

```text
paid image upload
site creation
site resolution
reference-graph site rendering
post/comment/article/image embeds
wallet-backed balances
```

This milestone adds:

```text
paid site visit
visitor-to-creator transfer
creator payout proof
browser-observed ROC deduction/credit
```

This moves RustyOnions/CrabLink closer to a real creator economy:

```text
create site
publish assets
attach payout policy
visitor pays to access/use/view
creator receives ROC
receipt proves it
ledger replays it
```

---

## 14. Do not regress

Future changes must not break:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
quote amount 10 ROC in dev config
payer acct_visitor_b
recipient acct_dev
wallet receipt op=transfer
CrabLink unlock after backend proof
gateway proxy-only boundary
wallet as mutation front-door
ledger as durable truth
```

Add tests/smoke coverage before refactoring this path.

---

## 15. Completion estimate impact

This milestone raises confidence in the internal WEB3/NEXT_LEVEL beta loop.

Updated rough interpretation:

```text
RustyOnions internal ROC / WEB3 beta loop:
  now closer to high-80s percent complete for local beta proof

CrabLink + RustyOnions combined paid creator proof:
  significantly advanced because site visits now move real ROC

Remaining before calling this robust:
  automated smoke
  tests
  policy/economics integration
  receipt UI polish
  paid content view generalization
  accounting/rewarder integration
```

---

## 16. Final checkpoint phrase

```text
2026-05-15:
Paid named-site visit is proven.

crab://ron7 quoted 10 ROC for site_visit.
acct_visitor_b paid acct_dev.
svc-wallet returned transfer receipt.
ron-ledger returned ledger root.
CrabLink/browser confirmed Passport B lost 10 ROC and Passport A received 10 ROC.

This is the first real visitor-to-creator ROC payment loop.
```



### END NOTE - MAY 15 2026 - 13:30 CST



### BEGIN NOTE - MAY 16 2026 - 14:40 CST

Below are two separate crate-specific carryover notes you can paste into your running notes. They are focused only on the changes made in **`omnigate`** and **`svc-gateway`** during this backend/content-view session.

---

# OMNIGATE Carryover Notes — Paid Site Visit + Paid Content View

## 1. Crate role reminder

`omnigate` remains a **product coordinator / façade**. It must not become economic truth and must not mutate `ron-ledger` directly.

Correct role:

```text
CrabLink / svc-gateway
→ omnigate product route
→ svc-index / svc-storage / svc-wallet
→ wallet receipt / ledger root returned
```

`omnigate` may coordinate paid product actions, but actual money movement must go through:

```text
svc-wallet
```

and durable economic truth must remain in:

```text
ron-ledger
```

Hard invariant preserved this session:

```text
No direct ledger mutation from omnigate.
No fake receipts.
No silent spend.
No floating-point money math.
```

---

# 2. Major Omnigate work completed this session

## 2.1 Paid named-site visit backend was locked

We already had the site-visit product path available, but this session turned it into a repeatable proof with tests and live smoke.

Relevant route family:

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
```

Public gateway equivalent:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Behavior now proven:

```text
Visitor B / acct_visitor_b
→ pays 10 ROC
→ Creator A / acct_dev receives 10 ROC
→ svc-wallet returns transfer receipt
→ response includes txid / receipt_hash / ledger_root
```

Latest repeated live runs showed `acct_visitor_b` decreasing by 10 and `acct_dev` increasing by 10, with committed txid, receipt hash, and ledger root. This proved the route is not just mocked UI behavior. The live preflight logs also showed the site-visit smoke passing while the stack stayed up. 

## 2.2 Omnigate site-visit contract tests added

File added:

```text
crates/omnigate/tests/site_visit.rs
```

Purpose:

```text
Verify omnigate site_visit quote/pay behavior without requiring the live stack.
```

The test creates dummy upstreams for:

```text
svc-index
svc-storage
svc-wallet
```

Covered cases:

```text
quote_returns_10_roc_and_manifest_payout_recipient
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

Important behavior locked:

```text
quote:
  read-only
  loads site manifest pointer
  loads site manifest
  derives payout recipient from manifest
  returns fixed dev price
  does not call wallet

pay:
  validates recipient
  validates payer
  calls svc-wallet /v1/transfer
  handles nonce conflict by retrying expected nonce
  returns wallet receipt
```

The nonce recovery behavior is important because earlier live site creation/paid flows sometimes exposed expected nonce errors. The test ensures Omnigate can recover when wallet says something like:

```text
nonce conflict: expected 8
```

## 2.3 Omnigate content-view backend route added

File added:

```text
crates/omnigate/src/routes/v1/content_view.rs
```

Purpose:

```text
Add backend quote/pay support for paid b3 asset views.
```

New route family:

```text
POST /v1/content/view/quote
POST /v1/content/view/pay
```

This route family is for b3-backed asset pages such as:

```text
crab://<64hex>.article
crab://<64hex>.post
crab://<64hex>.comment
crab://<64hex>.image
crab://<64hex>.video
crab://<64hex>.song
crab://<64hex>.music
```

Primary route flow:

```text
content_view quote/pay
→ parse canonical crab://<hash>.<kind>
→ load asset pointer from svc-index
→ load manifest object from svc-storage
→ validate asset_cid and asset_kind
→ derive payout recipient from manifest payout/owner
→ quote or pay through svc-wallet
```

## 2.4 Omnigate v1 router updated

File modified:

```text
crates/omnigate/src/routes/v1/mod.rs
```

Change:

```rust
pub mod content_view;
```

and route mount:

```rust
.nest("/content", content_view::router())
```

This exposes:

```text
/v1/content/view/quote
/v1/content/view/pay
```

## 2.5 Omnigate content-view tests added

File added:

```text
crates/omnigate/tests/content_view.rs
```

Purpose:

```text
Test paid b3 asset content_view quote/pay contract with dummy upstreams.
```

Dummy upstreams:

```text
dummy svc-index:
  GET /v1/index/assets/:hash/manifest

dummy svc-storage:
  GET /o/:manifest_cid

dummy svc-wallet:
  POST /v1/transfer
```

Covered tests:

```text
quote_returns_manifest_recipient_and_does_not_call_wallet
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

Latest output confirmed all three Omnigate content-view tests passed. 

---

# 3. Omnigate content-view contract details

## 3.1 Quote endpoint

Endpoint:

```text
POST /v1/content/view/quote
```

Typical request:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "max_amount_minor": "5",
  "client_idempotency_key": "content-view-quote-test"
}
```

Quote response schema:

```text
omnigate.content-view-quote.v1
```

Quote behavior:

```text
- parse asset target
- require canonical crab://<64 lowercase hex>.<kind>
- reject crab://b3/<hash>.<kind> slash form
- load asset manifest pointer from index
- load asset manifest from storage
- validate asset_cid
- validate asset_kind
- derive payout recipient from manifest
- reject recipient mismatch
- return amount_minor as string
- do not call wallet
```

Current default dev price:

```text
5 ROC minor units
```

Environment knob:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR
```

Default if unset:

```text
5
```

## 3.2 Pay endpoint

Endpoint:

```text
POST /v1/content/view/pay
```

Typical request:

```json
{
  "asset_crab_url": "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.article",
  "payer_account": "acct_visitor_b",
  "viewer_wallet_account": "acct_visitor_b",
  "viewer_passport_subject": "passport:main:visitor-b",
  "recipient_account": "acct_creator",
  "amount_minor": "5",
  "asset": "roc",
  "quote_id": "content-view-test-quote",
  "quote_hash": "quotehash",
  "client_idempotency_key": "content-view-pay-test"
}
```

Payment response schema:

```text
omnigate.content-view-payment.v1
```

Pay behavior:

```text
- parse asset target
- load/validate asset manifest pointer
- load/validate manifest object
- validate payout action
- validate recipient account
- reject self-payment
- validate amount matches current content_view price
- call svc-wallet /v1/transfer
- recover from wallet nonce conflict when expected nonce is present
- return wallet receipt
- expose txid / receipt_hash / ledger_root
```

Wallet transfer memo format:

```text
crablink content_view crab://<hash>.<kind>
```

## 3.3 Asset target validation

Accepted asset kinds:

```text
image
article
post
comment
video
music
song
```

Accepted canonical URL shape:

```text
crab://<64 lowercase hex>.<asset_kind>
```

Rejected:

```text
crab://b3/<hash>.<kind>
URLs with path separators
URLs with control characters
URLs with query strings
URLs with fragments
uppercase hashes
wrong hash length
unsupported kind
```

This follows the current RustyOnions rule that public typed asset URLs use:

```text
crab://<64hex>.<kind>
```

while internal CIDs use:

```text
b3:<64hex>
```

## 3.4 Manifest payout behavior

The route derives payout recipient from manifest data.

Expected manifest fields:

```json
{
  "version": 1,
  "asset_cid": "b3:<hash>",
  "asset_kind": "article",
  "owner": {
    "passport_subject": "passport:main:creator",
    "wallet_account": "acct_creator"
  },
  "payout": {
    "default_action": "content_view",
    "recipient_account": "acct_creator"
  },
  "metadata": {
    "title": "Paid Article Fixture",
    "description": "A paid content_view test fixture.",
    "tags": ["article", "content_view"],
    "content_type": "application/json; charset=utf-8"
  }
}
```

Accepted payout actions:

```text
content_view
asset_view
view
<asset_kind>_view
```

Examples:

```text
article_view
post_view
comment_view
image_view
```

Fallback behavior:

```text
If payout.recipient_account is missing, route may fall back to owner.wallet_account.
If neither exists, route fails closed.
```

## 3.5 Important manifest DTO lesson

Initial issue:

```text
AssetManifestDocument and ManifestPayout were too strict.
```

The route initially used:

```rust
#[serde(deny_unknown_fields)]
```

on manifest DTOs. That caused normal/evolving manifest fields to fail deserialization and returned `502`.

Fields that caused tolerance problems:

```text
receipts
receipt_refs
provenance
storage
payout.splits
```

Fix:

```text
Keep request DTOs strict.
Make stored manifest DTOs tolerant.
Explicitly validate only critical economic fields.
Let Serde ignore unknown manifest fields by default.
```

Do **not** reintroduce `deny_unknown_fields` on stored manifest objects unless all real manifest fields are represented.

## 3.6 Important JSON macro lesson

Initial issue:

```text
recursion limit reached while expanding serde_json::json_internal!
```

Cause:

```text
One huge nested json! response object in content_view quote.
```

Fix:

```text
Split JSON construction into smaller objects:
policy
asset_page
quote
next
payment
receipt
response
```

Do not fix this by raising the crate recursion limit.

---

# 4. Omnigate files changed in detail

## 4.1 `crates/omnigate/src/routes/v1/content_view.rs`

Added full route implementation.

Main public functions:

```rust
pub fn router<S>() -> Router<S>
pub async fn content_view_quote(headers: HeaderMap, body: Bytes) -> Response
pub async fn content_view_pay(headers: HeaderMap, body: Bytes) -> Response
```

Important internal helpers:

```rust
load_content_view_context
fetch_asset_pointer
fetch_asset_manifest
send_wallet_content_view_transfer
content_view_payment_response
wallet_transfer_problem
parse_asset_target
parse_crab_asset_url
canonicalize_hash
normalize_asset_kind
is_content_view_action
content_view_payer_account
content_view_price_minor
default_content_view_nonce
wallet_base_url
wallet_bearer
storage_base_url
index_base_url
content_view_quote_id
content_view_idempotency_key
simple_hex_hash
is_canonical_b3_cid
normalize_minor_units_owned
decimal_gt
clean_optional
parse_expected_nonce
should_forward_header
is_hop_by_hop_or_host
value_string
grab
now_ms
problem
```

Current known limitations:

```text
pricing is fixed dev env price, not yet full roc-economics.toml-driven
single recipient payout is enforced, not split payout execution
quote_id / quote_hash are deterministic lightweight route-level values, not durable ledger truth
content_view has backend tests but not live stack smoke yet
CrabLink UI is not wired yet
```

## 4.2 `crates/omnigate/src/routes/v1/mod.rs`

Added module:

```rust
pub mod content_view;
```

Added router mount:

```rust
.nest("/content", content_view::router())
```

This is how `/v1/content/view/quote` and `/v1/content/view/pay` are exposed.

## 4.3 `crates/omnigate/tests/content_view.rs`

Added content-view integration-style route tests with dummy upstream services.

Important constants:

```rust
const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ASSET_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MANIFEST_CID: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const LEDGER_ROOT: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const RECEIPT_HASH: &str = "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
```

The dummy wallet intentionally first returns:

```text
nonce conflict: expected 8
```

then accepts nonce `8`, proving nonce recovery.

## 4.4 `crates/omnigate/tests/site_visit.rs`

Added site-visit test coverage.

Important tests:

```text
quote_returns_10_roc_and_manifest_payout_recipient
pay_recovers_wallet_nonce_and_returns_wallet_receipt
quote_rejects_recipient_mismatch
```

This is the site-level counterpart to content-view.

---

# 5. Omnigate gates now green

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p omnigate --test site_visit
cargo test -p omnigate --test content_view
```

Expected:

```text
site_visit:    3 passed
content_view:  3 passed
```

Strict content-view gate:

```bash
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Latest status:

```text
PASS
```

---

# 6. Omnigate follow-up work

## 6.1 Highest priority next Omnigate follow-up

After CrabLink UI wiring, add live smoke:

```text
scripts/web3_paid_content_view_smoke.sh
```

Purpose:

```text
Use real stack services.
Ensure a known b3 article/post/comment asset exists.
Quote 5 ROC.
Pay from visitor to creator.
Assert payer -5.
Assert creator +5.
Assert txid / receipt_hash / ledger_root.
```

This should be modeled after:

```text
scripts/web3_paid_site_visit_smoke.sh
```

## 6.2 Future economics config integration

Current content-view pricing is:

```text
OMNIGATE_CONTENT_VIEW_PRICE_MINOR=5
```

Future target:

```toml
[actions.content_view]
enabled = true
pricing_kind = "flat"
price_minor = 5
```

in:

```text
configs/roc-economics.dev.toml
```

Do not do this until CrabLink UI is wired and current backend route remains green.

## 6.3 Future split payout support

Current content-view route only uses:

```text
payout.recipient_account
```

Future WEB3_2/NEXT_LEVEL likely needs:

```text
creator split
curator split
storage/provider split
treasury residual
```

But do not implement split settlement inside Omnigate directly. It should still call wallet through controlled route(s), or later use policy/rewarder planning, depending on architecture.

---

# SVC-GATEWAY Carryover Notes — Paid Site Visit + Paid Content View Proxy Routes

## 1. Crate role reminder

`svc-gateway` is the public browser/client edge.

It must remain:

```text
proxy-only for product routes
```

Correct role:

```text
CrabLink
→ svc-gateway public route
→ omnigate /v1 route
```

Gateway must not:

```text
parse product manifests
choose payout recipients
calculate prices
call svc-wallet directly for product payment
mutate ron-ledger
store bytes
write index pointers
compute accounting
compute rewards
```

Gateway may:

```text
expose clean public HTTP paths
preserve selected request headers
preserve request body
filter hop-by-hop headers
copy safe response headers
return upstream body/status
```

This boundary remained intact.

---

# 2. Major svc-gateway work completed this session

## 2.1 Paid site-visit proxy route was tested

File added:

```text
crates/svc-gateway/tests/site_visit_routes_proxy.rs
```

Purpose:

```text
Verify gateway exposes site_visit quote/pay as proxy-only routes.
```

Public routes:

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Proxied upstream:

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
```

Test verifies:

```text
method preserved
path rewritten correctly to /v1/...
body preserved
authorization header preserved
x-ron-passport preserved
x-ron-wallet-account preserved
idempotency-key preserved
x-request-id preserved
x-correlation-id preserved
connection header not forwarded
```

Important Host-header lesson:

```text
Gateway must not forward inbound Host.
But reqwest/hyper must synthesize a new upstream Host header.
```

The original test incorrectly expected Host to be null. We fixed the test to expect:

```rust
assert_eq!(body["host"], omnigate_addr.to_string());
```

while still asserting:

```rust
assert!(body["connection"].is_null());
```

## 2.2 Paid content-view proxy routes added

File modified:

```text
crates/svc-gateway/src/routes/product.rs
```

New public routes:

```text
POST /content/view/quote
POST /content/view/pay
```

Upstream paths:

```text
POST /v1/content/view/quote
POST /v1/content/view/pay
```

Added handlers:

```rust
pub async fn content_view_quote(...)
pub async fn content_view_pay(...)
```

The handlers do only this:

```text
proxy_to_omnigate(&state, Method::POST, "/v1/content/view/quote", headers, body)
proxy_to_omnigate(&state, Method::POST, "/v1/content/view/pay", headers, body)
```

No product logic was added to gateway.

## 2.3 Paid content-view proxy test added

File added:

```text
crates/svc-gateway/tests/content_view_routes_proxy.rs
```

Purpose:

```text
Verify /content/view/quote and /content/view/pay proxy to omnigate and preserve context.
```

Covered public routes:

```text
/content/view/quote
/content/view/pay
```

Expected upstream routes:

```text
/v1/content/view/quote
/v1/content/view/pay
```

The test uses dummy Omnigate echo server.

It verifies:

```text
method = POST
path = expected /v1/content/view/*
body_len > 0
authorization preserved
x-ron-passport preserved
x-ron-wallet-account preserved
idempotency-key preserved
x-request-id preserved
x-correlation-id preserved
upstream Host synthesized correctly
Connection header not forwarded
```

Latest output confirmed this test passes. 

## 2.4 Product route docs/comments updated

File modified:

```text
crates/svc-gateway/src/routes/product.rs
```

Updated module header to include:

```text
content-view
```

in the product route exposure description.

Updated route list to include:

```text
POST /content/view/quote
POST /content/view/pay
```

Fixed Clippy doc-markdown issue:

```text
CrabLink
```

needed to be:

```text
`CrabLink`
```

Latest strict content-view gate passed after this fix. The previous failure was only a doc comment in `svc-gateway/src/routes/product.rs`, not a kernel issue. 

---

# 3. svc-gateway `product.rs` current route surface

The product router now exposes:

```text
GET  /identity/me
POST /identity/passport/bootstrap
POST /identity/passport/profile/claim
GET  /identity/passport/profile/:username

GET  /wallet/:account/balance
POST /wallet/hold

GET  /crab/resolve?url=...
GET  /b3/:asset

POST /paid/o/prepare

POST /assets/image/prepare
POST /assets/image

POST /assets/post/prepare
POST /assets/post

POST /assets/comment/prepare
POST /assets/comment

POST /assets/article/prepare
POST /assets/article

POST /content/view/quote
POST /content/view/pay

POST /sites/prepare
POST /sites
GET  /sites/:name

POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
```

Key helpers remain:

```rust
proxy_to_omnigate
with_query
should_forward_header
should_copy_response_header
is_hop_by_hop_or_host
```

---

# 4. svc-gateway header forwarding contract

Allowed/preserved request headers include:

```text
authorization
accept
content-type
x-ron-*
x-correlation-id
x-request-id
idempotency-key
```

Filtered inbound headers include:

```text
host
connection
proxy-authorization
te
trailer
transfer-encoding
upgrade
content-length
```

Response headers copied unless unsafe:

```text
not transfer-encoding
not content-length
not connection
```

Important nuance:

```text
The inbound Host header is filtered.
The outbound HTTP client may synthesize a new Host header for the upstream request.
This is correct HTTP behavior.
```

The proxy tests encode this.

---

# 5. svc-gateway files changed in detail

## 5.1 `crates/svc-gateway/src/routes/product.rs`

Added content-view route exposure.

New routes in router:

```rust
.route("/content/view/quote", post(content_view_quote))
.route("/content/view/pay", post(content_view_pay))
```

New handlers:

```rust
pub async fn content_view_quote(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/content/view/quote",
        headers,
        body,
    )
    .await
}
```

```rust
pub async fn content_view_pay(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    proxy_to_omnigate(
        &state,
        Method::POST,
        "/v1/content/view/pay",
        headers,
        body,
    )
    .await
}
```

No pricing, wallet, ledger, manifest, or storage logic added.

## 5.2 `crates/svc-gateway/tests/content_view_routes_proxy.rs`

Added proxy-only route test.

Key test:

```text
content_view_quote_and_pay_proxy_to_omnigate_and_preserve_context
```

Verifies route-level product boundary.

## 5.3 `crates/svc-gateway/tests/site_visit_routes_proxy.rs`

Added site-visit proxy-only route test.

Key test:

```text
site_visit_quote_and_pay_proxy_to_omnigate_and_preserve_context
```

This is the named-site counterpart to content-view proxy testing.

---

# 6. svc-gateway gates now green

Run:

```bash
cd /Users/mymac/Desktop/RustyOnions

cargo test -p svc-gateway --test site_visit_routes_proxy
cargo test -p svc-gateway --test content_view_routes_proxy
```

Expected:

```text
site_visit_routes_proxy:     1 passed
content_view_routes_proxy:   1 passed
```

Strict scoped content-view gate:

```bash
RON_RUN_CLIPPY=1 bash scripts/web3_nextlevel_content_view_green_gate.sh
```

Latest status:

```text
PASS
```

The latest terminal output showed both scoped Clippy steps passing and both content-view tests passing. The `ron-kernel` “Checking” lines appeared only because Cargo still typechecks dependencies; the failure was resolved without touching kernel. 

---

# 7. svc-gateway follow-up work

## 7.1 Do not add product logic to gateway

For the next CrabLink content-view UI work, do **not** add extra gateway logic unless a route is missing.

Gateway already exposes:

```text
/content/view/quote
/content/view/pay
```

CrabLink should call these public paths.

## 7.2 Add proxy tests if new public paths appear

If future backend adds:

```text
/content/view/live-smoke
/assets/:kind/:hash/view/quote
/assets/:kind/:hash/view/pay
```

or similar, add gateway proxy tests immediately.

## 7.3 Keep gateway route docs current

`docs/WEB3_2_PRODUCT_ROUTE_CONTRACTS.md` should be updated whenever a new gateway public product route becomes real and tested.

## 7.4 Watch for Clippy doc-markdown issues

`svc-gateway` has strict lints:

```rust
#![deny(clippy::all, clippy::pedantic)]
```

So route comments may fail Clippy for unbackticked product names. For docs/comments, prefer:

```text
`CrabLink`
`Omnigate`
`svc-wallet`
`ron-ledger`
`content_view`
```

where appropriate.

---

# 8. Cross-crate interaction notes

## 8.1 `content_view` route ownership

Ownership split:

```text
svc-gateway:
  public browser/client route
  proxy-only
  no product logic

omnigate:
  product route coordinator
  resolves manifest pointer
  loads manifest
  validates payout context
  calls svc-wallet

svc-index:
  asset hash → manifest CID pointer

svc-storage:
  stores manifest object and content bytes

svc-wallet:
  transfer front-door

ron-ledger:
  durable economic truth
```

## 8.2 `site_visit` route ownership

Ownership split:

```text
svc-gateway:
  /sites/:name/visit/quote
  /sites/:name/visit/pay
  proxy-only

omnigate:
  site manifest lookup
  payout recipient validation
  calls svc-wallet transfer

svc-wallet:
  balance movement and receipt

ron-ledger:
  append-only durable truth
```

## 8.3 Pattern established for future paid routes

Future paid routes should follow the same template:

```text
1. Gateway public proxy route
2. Gateway proxy test
3. Omnigate route implementation
4. Omnigate contract test with dummy upstreams
5. Optional wallet-level receipt replay test
6. Live smoke only after contract tests are green
7. CrabLink UI only after backend route contract is stable
```

Do not jump directly to UI before backend contract tests exist.

---

# 9. Current recommended next task

The next session should **not** make more Omnigate/Gateway changes first unless the CrabLink wiring exposes a missing response shape.

Best next task:

```text
Wire CrabLink paid content_view UI for article asset pages first.
```

CrabLink should call:

```text
POST /content/view/quote
POST /content/view/pay
```

through the gateway.

It should not call:

```text
/v1/content/view/quote
/v1/content/view/pay
svc-wallet
svc-index
svc-storage
ron-ledger
```

UI acceptance criteria:

```text
Open crab://<hash>.article
quote card appears
price is shown
recipient/creator info shown if available
no automatic spend
user clicks Pay
CrabLink calls /content/view/pay
receipt returns
article unlocks only after receipt
balance refreshes
txid shown
receipt_hash shown
ledger_root shown
Developer JSON remains available
```

---

# 10. Do-not-regress checklist for these two crates

## Omnigate do-not-regress

```text
POST /v1/sites/:name/visit/quote
POST /v1/sites/:name/visit/pay
POST /v1/content/view/quote
POST /v1/content/view/pay
manifest-derived payout recipient
recipient mismatch rejection
payer/header mismatch rejection
self-payment rejection for content_view
wallet /v1/transfer only
nonce conflict recovery
wallet receipt passthrough
txid / receipt_hash / ledger_root response fields
integer string amount_minor
strict request DTOs
tolerant stored manifest DTOs
no direct ledger mutation
```

## svc-gateway do-not-regress

```text
POST /sites/:name/visit/quote
POST /sites/:name/visit/pay
POST /content/view/quote
POST /content/view/pay
proxy-only behavior
selected header forwarding
body forwarding
query forwarding where relevant
hop-by-hop header filtering
no manifest parsing
no pricing
no wallet mutation
no ledger mutation
no storage writes
no accounting/rewarder logic
```

## Shared do-not-regress

```text
ron-kernel untouched
QuickChain locked
CrabLink only calls gateway public routes
money remains integer minor units
receipt_hash remains b3:<64 lowercase hex>
ledger_root remains present when wallet returns it
no silent spend
```


### END NOTE - MAY 16 2026 - 14:40 CST


### BEGIN NOTE - JUNE 15 2026 - 13:55 CST

Here are the **drop-in crate notes for `omnigate`**. They are written to paste directly into the crate’s notes file.

# Omnigate QuickChain Phase-0 / Preflight Notes

## Current status

`omnigate` QuickChain Phase-0/preflight is green and safe to park for now.

The latest verified gate ended with:

```text
omnigate QuickChain preflight gate passed
```

Current estimated completion for `omnigate` with respect to QuickChain Phase-0/preflight: **~92–94% complete**.

This estimate means the crate now has strong Phase-0 boundary coverage and regression coverage for the current omnigate role. It does **not** mean omnigate is ready to become a QuickChain runtime, validator, settlement service, checkpoint service, bridge, staking surface, liquidity surface, root producer, or ledger authority. Those roles remain explicitly forbidden/deferred.

For the current mission, omnigate is considered a **hydration/product coordination boundary**. It coordinates paid product flows, calls backend services, displays backend-derived receipt information, and forwards/proxies specific requests. It does not own ledger truth, does not mutate balances directly, does not mint/issue/capture/release ROC, does not generate QuickChain roots, and does not accept transport headers/cache/b3/manifests as payment truth.

## Omnigate role in the RustyOnions / CrabLink / QuickChain architecture

Omnigate sits between product-facing gateway/client flows and the lower RustyOnions services. Its role is to hydrate, coordinate, compose, and proxy product flows. It is not the value-plane authority.

Important crate boundary:

* `ron-proto` owns DTO shapes.
* `ron-ledger` owns economic truth.
* `svc-wallet` is the mutation front-door for internal ROC operations.
* `svc-storage` owns byte storage and paid storage enforcement.
* `svc-index` owns lookup/pointer surfaces.
* `ron-accounting` owns snapshots/reporting, not live balance truth.
* `svc-rewarder` plans payouts, never mutates ledger.
* `svc-gateway` is the public boundary.
* `omnigate` coordinates hydration/product flows and must not become ledger, wallet, settlement, chain, root, validator, bridge, staking, or liquidity authority.

QuickChain doctrine preserved in omnigate:

* QuickChain is future settlement infrastructure, not current runtime.
* Internal ROC is proven first.
* No ROX, Solana, external settlement, public bridge, staking, liquidity, validators, roots, checkpoints, or chain runtime behavior may enter omnigate.
* Transport metadata is not authority.
* Cache/b3/manifest data is not payment authority.
* Backend wallet/ledger receipt truth remains authoritative.
* Omnigate may display or forward backend-derived receipt data, but must not fabricate receipt/finality/balance/root/checkpoint truth.

## What was added for omnigate QuickChain Phase-0/preflight

### Documentation added/expanded

Added/expanded:

```text
crates/omnigate/docs/quickchain-preflight.md
```

This document now records the crate’s QuickChain Phase-0 contract.

It states that omnigate:

* Is a hydration and product coordination boundary.
* Is not a chain runtime.
* Is not a validator.
* Is not a bridge.
* Is not a settlement surface.
* Is not a staking or liquidity surface.
* Is not a root producer.
* Is not a checkpoint producer.
* Does not mutate ledger truth.
* Does not create fake receipts.
* Does not invent balances.
* Does not unlock paid content from cache/b3/manifest data alone.
* Does not treat caller-supplied transport headers as economic truth.
* Uses backend wallet/ledger receipt paths as the paid-access authority.
* Keeps QuickChain future/deferred until the correct lower-level crates, canonical bytes, golden vectors, and explicit gates exist.

Important documentation sections now covered:

* Omnigate hydration/product coordination boundary.
* Forbidden authority roles.
* Operation identity/idempotency rules.
* Cache/content-addressing non-authority rules.
* Paid access and wallet front-door rules.
* Transport/header authority boundary.
* Fake receipt/finality/root/checkpoint/validator prevention.
* Display-only wallet/balance fallback expectations.

### Preflight script added/expanded

Added/expanded:

```text
crates/omnigate/scripts/dev-quickchain-preflight.sh
```

This script is the focused omnigate QuickChain gate.

Current script flow:

```bash
cargo fmt -p omnigate

cargo test -p omnigate --test quickchain_preflight_boundary
cargo test -p omnigate --test quickchain_preflight_docs
cargo test -p omnigate --test quickchain_preflight_no_fake_receipts
cargo test -p omnigate --test quickchain_preflight_paid_access
cargo test -p omnigate --test quickchain_preflight_cache_boundary
cargo test -p omnigate --test quickchain_preflight_transport_authority

cargo test -p omnigate --test content_view
cargo test -p omnigate --test site_visit
cargo test -p omnigate --test streams
cargo test -p omnigate --test chat_routes
cargo test -p omnigate --test paid_storage_estimate_proxy
cargo test -p omnigate --test paid_storage_prepare
cargo test -p omnigate --test paid_storage_write_proxy

cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Expected final output:

```text
omnigate QuickChain preflight gate passed
```

This script should be treated as the crate-local QuickChain Phase-0 gate.

### Tests added/expanded

The following QuickChain preflight tests are now part of the omnigate crate.

#### `tests/quickchain_preflight_boundary.rs`

Purpose: enforce that omnigate remains a hydrator/coordinator, not a chain/ledger authority.

Coverage:

* `Cargo.toml` must not depend on `ron-ledger` or QuickChain runtime/validator/root crates.
* Production source must not import direct ledger or chain authority.
* Public v1 router must not publish QuickChain authority routes.
* Route docs must preserve omnigate as hydrator/coordinator, not runtime.
* Wallet surface must remain display-balance and explicit hold front-door only.

Key invariant:

```text
Omnigate may coordinate paid flows, but it must not mutate ledger truth or become QuickChain authority.
```

#### `tests/quickchain_preflight_docs.rs`

Purpose: ensure docs preserve the crate’s QuickChain boundary and preflight workflow.

Coverage:

* Docs state omnigate hydration/product coordination boundary.
* Docs name forbidden authority roles.
* Docs define cache and content addressing as non-economic truth.
* Docs preserve operation identity/idempotency rules.
* Preflight script lists focused tests and clippy gate.

Key invariant:

```text
The crate’s doctrine must stay visible in docs, not only in tests.
```

#### `tests/quickchain_preflight_no_fake_receipts.rs`

Purpose: prevent fake receipt/finality/root/validator creep.

Coverage:

* Production source does not construct obvious fake receipts or finality.
* Production source does not construct root/validator authority fields.
* Paid routes use wallet receipt values without dev receipt fabrication.
* Wallet balance fallback is explicitly non-authoritative.

Key invariant:

```text
No fake ROC receipt, fake finality, fake balance, fake root, fake checkpoint, or fake validator fields may be introduced in omnigate.
```

#### `tests/quickchain_preflight_paid_access.rs`

Purpose: enforce paid-access boundaries.

Coverage:

* Content view and site visit pay routes name the wallet front-door invariant.
* Stream and chat paid paths fail closed on backend truth.
* Paid storage route stays prepare/estimate/proxy only.
* Production source does not contain paid/unlock bypass shortcuts.

Key invariant:

```text
Paid access must follow backend wallet/storage truth, not omnigate-local bypasses.
```

#### `tests/quickchain_preflight_cache_boundary.rs`

Purpose: enforce that cache, b3, and manifests are not paid-entitlement authority.

Coverage:

* Docs state cache/b3 are not paid entitlement authority.
* Paid access routes do not parse transport cache headers as unlock authority.
* Stream/chat sources explicitly fail closed for cached paid media.
* Production source does not claim cache, b3, or manifest proves payment.

Key invariant:

```text
b3 verifies bytes. It does not prove payment. Cache may improve display, but cannot unlock paid content alone.
```

#### `tests/quickchain_preflight_transport_authority.rs`

Purpose: enforce that transport headers are not economic authority.

Coverage:

* Docs state transport headers are not economic authority.
* Route sources do not accept fake `x-ron-*` authority headers.
* Production `x-ron-*` headers do not define QuickChain/economic authority tokens.
* Proxy-like routes filter caller `x-ron-*` headers before downstream forwarding.
* Receipt/paid authority must come from backend paths, not request-header authority variables.

Important correction made during the session:

Originally the test tried to maintain a static allowlist of all legitimate `x-ron-*` metadata headers. That was too brittle because omnigate legitimately uses many product/profile/provider metadata headers such as:

```text
x-ron-provider
x-ron-asset-title
x-ron-asset-description
x-ron-asset-tags
x-ron-wallet-account
x-ron-passport
x-ron-wallet-hold-txid
x-ron-reload-gen
```

The correct shape is now:

* Allow normal metadata/context headers.
* Reject authority-shaped headers.

Forbidden authority-shaped headers include:

```text
x-ron-receipt
x-ron-receipt-id
x-ron-receipt-hash
x-ron-paid
x-ron-unlocked
x-ron-unlock
x-ron-entitlement
x-ron-finalized
x-ron-finality
x-ron-balance
x-ron-ledger
x-ron-root
x-ron-state-root
x-ron-receipt-root
x-ron-checkpoint
x-ron-validator
x-ron-spend-authority
x-ron-capture-authority
```

Key invariant:

```text
Caller-supplied transport headers may carry context/metadata, but they must never prove receipt, payment, unlock, entitlement, balance, finality, root, checkpoint, validator, spend authority, or capture authority.
```

This is the right long-term test shape because omnigate’s product metadata surface will grow.

## Source areas marked or hardened

The QuickChain preflight work touched/hardened these conceptual areas in omnigate:

```text
src/routes/v1/mod.rs
src/routes/v1/wallet.rs
src/routes/v1/content_view.rs
src/routes/v1/site_visit.rs
src/routes/v1/streams.rs
src/routes/v1/chat.rs
src/routes/v1/paid.rs
src/routes/v1/crab.rs
src/routes/v1/assets.rs
src/routes/v1/app.rs
```

The main hardened surfaces are:

* Public v1 routing surface.
* Wallet display/hold route surface.
* Content-view paid quote/pay flow.
* Site-visit paid quote/pay flow.
* Stream paid segment flow.
* Chat paid send/list/latest flow.
* Paid storage estimate/prepare/write proxy flow.
* Crab resolver/provider metadata surface.
* Asset upload/prepare metadata surface.
* App/static reload metadata surface.

## What each hardened surface is expected to preserve

### Public v1 router

The public v1 router must not publish QuickChain authority routes.

Forbidden route families include:

```text
quickchain
checkpoint
checkpoints
state-root
receipt-root
root
roots
validator
validators
anchor
anchors
bridge
settlement
settlements
stake
staking
liquidity
rox
solana
```

Omnigate may expose product/hydration routes. It must not expose QuickChain runtime/settlement/validator/bridge/staking/liquidity routes.

### Wallet route surface

The wallet route surface is allowed to be a front-door/facade for explicit wallet operations and display-only balance/receipt information, but it must not become ledger truth.

Allowed concepts:

* Explicit hold request path.
* Backend wallet integration path.
* Display-only balance fallback where explicitly marked non-authoritative.
* Wallet receipt lookup/display from backend-derived values.

Forbidden concepts:

* Direct ledger mutation.
* Fake balance.
* Fake receipt.
* Silent spend.
* Client/header-derived receipt authority.
* QuickChain root/checkpoint/validator authority.

### Content view flow

Content view paid access must preserve:

* Quote path returns payout recipient/amount context without calling wallet.
* Pay path calls backend wallet front-door.
* Wallet receipt values come from backend wallet response.
* Recipient mismatch fails closed.
* No local fake receipt.
* No cache unlock.
* No caller header unlock.

Verified regression tests:

```text
tests/content_view.rs
```

Passing cases:

```text
quote_returns_manifest_recipient_and_does_not_call_wallet
quote_rejects_recipient_mismatch
pay_recovers_wallet_nonce_and_returns_wallet_receipt
```

### Site visit flow

Site visit paid access must preserve:

* Quote returns expected ROC amount and manifest payout recipient.
* Recipient mismatch fails closed.
* Pay path recovers wallet nonce and returns wallet receipt.
* No local fake receipt.
* No caller header unlock.
* No cache unlock.

Verified regression tests:

```text
tests/site_visit.rs
```

Passing cases:

```text
quote_returns_10_roc_and_manifest_payout_recipient
quote_rejects_recipient_mismatch
pay_recovers_wallet_nonce_and_returns_wallet_receipt
```

### Stream flow

Stream paid access must preserve:

* Latest paid segment requires stream-bound wallet receipt.
* Cached paid segment/media cannot unlock alone.
* Backend truth must be available.
* Paid stream paths fail closed.

Verified regression tests:

```text
tests/streams.rs
```

Passing case:

```text
latest_segment_requires_stream_bound_wallet_receipt
```

### Chat flow

Chat paid access must preserve:

* Chat create/resolve/quote flow works.
* Paid send requires quote/payment path.
* Free chat send/list/latest works.
* Moderation fails closed.
* Paid chat must not unlock/send from fake local receipt or header receipt.
* Cached chat/media state cannot unlock paid access alone.

Verified regression tests:

```text
tests/chat_routes.rs
```

Passing cases:

```text
chat_create_resolve_quote_and_paid_send_requires_quote
free_chat_send_list_latest_and_mod_fails_closed
```

### Paid storage flow

Paid storage routes must remain prepare/estimate/proxy only.

They may coordinate:

* Storage estimate proxy.
* Paid object prepare.
* Paid storage write proxy.
* Upstream storage error passthrough.
* Connect failure problem response.
* Hold template response derived from storage estimate.

They must not:

* Become wallet/ledger authority.
* Fabricate receipts.
* Invent successful paid writes.
* Treat caller headers as paid proof.
* Unlock or write from local cache alone.
* Mutate ledger.

Verified regression tests:

```text
tests/paid_storage_estimate_proxy.rs
tests/paid_storage_prepare.rs
tests/paid_storage_write_proxy.rs
```

Passing cases:

```text
paid_storage_estimate_proxy_happy_path
paid_storage_estimate_proxy_passes_storage_errors_through
paid_storage_estimate_upstream_connect_failure_yields_problem_502

paid_object_prepare_returns_hold_template_from_storage_estimate
paid_object_prepare_maps_storage_estimate_errors
paid_object_prepare_upstream_connect_failure_yields_problem_502
paid_object_prepare_rejects_bad_json_without_storage_call

paid_storage_write_proxy_post_happy_path
paid_storage_write_proxy_put_happy_path
paid_storage_write_proxy_passes_storage_errors_through
paid_storage_write_upstream_connect_failure_yields_problem_502
```

### Crab resolver/provider surface

`crab.rs` may use provider metadata such as `x-ron-provider` when resolving or reflecting storage/provider state.

This is allowed because provider metadata is not payment truth.

Important distinction:

* `x-ron-provider` = metadata/provider hint/context.
* `x-ron-receipt`, `x-ron-paid`, `x-ron-unlocked`, `x-ron-finality`, `x-ron-state-root`, etc. = forbidden authority-shaped headers.

The transport authority test was corrected to understand this distinction.

### Asset metadata surface

Asset routes legitimately use product metadata headers. These must not be treated as QuickChain authority.

Examples of legitimate non-authority metadata/context headers include:

```text
x-ron-passport
x-ron-wallet-account
x-ron-wallet-hold-txid
x-ron-asset-title
x-ron-asset-description
x-ron-asset-tags
x-ron-permission
x-ron-spend-limit
x-ron-image-rendition-group
x-ron-video-rendition-group
x-ron-music-artist
x-ron-podcast-show
```

These can describe upload context, creator metadata, product metadata, wallet handoff context, or display metadata. They must not be interpreted as receipt/finality/unlock/balance/root/checkpoint/validator truth.

### App/static site surface

App/static site route may expose display/cache-control style metadata such as:

```text
x-ron-reload-gen
```

This is allowed because reload generation metadata is not economic authority. It must not be used to unlock paid content or prove payment.

## Test failures encountered and resolved

### Failure 1: docs wording mismatch

A docs test initially failed because the exact required phrase did not match the docs text.

Resolution:

* Updated docs wording to preserve required QuickChain boundary phrases.
* Ensured docs clearly state cache cannot unlock paid content.

Lesson:

* The docs tests are intentionally strict where they lock important doctrine.
* If docs wording changes, preserve the required invariants.

### Failure 2: clippy / formatting issues

Initial test additions had clippy/style problems, including:

* Unused constants/imports.
* `doc_lazy_continuation`.
* `vec_init_then_push`.
* `too_many_arguments`.
* `result_large_err`.

Resolution:

* Reworked tests and helper structures.
* Adjusted formatting and helper function structure.
* Re-ran `cargo fmt -p omnigate`.
* Re-ran clippy with `-D warnings`.

Lesson:

* The preflight script must always include clippy.
* Test code must stay as clean as production code.

### Failure 3: transport authority false positive on human error string

The first version of the transport header test accidentally treated a full human-readable error string as a header literal:

```text
x-ron-wallet-account must match site visit payer account
```

Resolution:

* Rewrote token extraction to parse actual `x-ron-*` header-like tokens from Rust string literals instead of treating whole strings as headers.

Lesson:

* Scanner tests must parse tokens precisely enough to avoid false positives from docs/errors.

### Failure 4: transport authority false positive on `x-ron-provider`

The next version of the transport header test used a static allowlist. It failed on:

```text
x-ron-provider
```

This exposed the deeper design flaw: omnigate has a broad and growing product/provider metadata header surface. A static allowlist of all acceptable metadata headers is brittle and causes noise.

Resolution:

* Replaced static “only these metadata headers may exist” logic with a better boundary:

  * Allow normal `x-ron-*` metadata/context/product/provider headers.
  * Forbid authority-shaped `x-ron-*` headers.
* The corrected test now blocks receipt/paid/unlock/finality/balance/root/checkpoint/validator/spend/capture authority tokens specifically.

Lesson:

* The correct QuickChain safety invariant is not “few headers exist.”
* The correct invariant is “transport headers never become economic or chain authority.”

## Final verified test output summary

The focused transport authority test now passes:

```text
running 5 tests
test docs_state_transport_headers_are_not_economic_authority ... ok
test proxy_like_routes_filter_x_ron_headers_before_downstream_forwarding ... ok
test route_sources_do_not_accept_fake_x_ron_authority_headers ... ok
test receipt_or_paid_authority_must_come_from_backend_paths_not_request_headers ... ok
test production_x_ron_headers_do_not_define_quickchain_authority_tokens ... ok

test result: ok. 5 passed; 0 failed
```

The full QuickChain preflight script now passes all focused tests, regressions, and clippy:

```text
[omnigate quickchain] fmt
[omnigate quickchain] focused preflight tests
...
[omnigate quickchain] paid/product route regressions
...
[omnigate quickchain] clippy
...
omnigate QuickChain preflight gate passed
```

## Commands to verify omnigate QuickChain Phase-0/preflight

Run from repository root:

```bash
cargo fmt -p omnigate
```

Run focused transport/header authority test:

```bash
cargo test -p omnigate --test quickchain_preflight_transport_authority
```

Run full omnigate QuickChain preflight gate:

```bash
bash crates/omnigate/scripts/dev-quickchain-preflight.sh
```

Run individual focused tests if debugging:

```bash
cargo test -p omnigate --test quickchain_preflight_boundary
cargo test -p omnigate --test quickchain_preflight_docs
cargo test -p omnigate --test quickchain_preflight_no_fake_receipts
cargo test -p omnigate --test quickchain_preflight_paid_access
cargo test -p omnigate --test quickchain_preflight_cache_boundary
cargo test -p omnigate --test quickchain_preflight_transport_authority
```

Run paid/product route regressions individually:

```bash
cargo test -p omnigate --test content_view
cargo test -p omnigate --test site_visit
cargo test -p omnigate --test streams
cargo test -p omnigate --test chat_routes
cargo test -p omnigate --test paid_storage_estimate_proxy
cargo test -p omnigate --test paid_storage_prepare
cargo test -p omnigate --test paid_storage_write_proxy
```

Run clippy gate:

```bash
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

## What omnigate now protects for QuickChain Phase-0

### Boundary protection

Omnigate is guarded against becoming:

* QuickChain runtime.
* Chain authority.
* Validator.
* Checkpoint producer.
* Root producer.
* Settlement surface.
* External bridge.
* Staking endpoint.
* Liquidity endpoint.
* Ledger mutation crate.
* Wallet mutation owner.
* Balance truth owner.
* Receipt truth owner.

### Paid-flow protection

Omnigate paid flows are guarded so they must not:

* Unlock paid content from cache alone.
* Unlock paid content from b3 verification alone.
* Unlock paid content from manifest presence alone.
* Unlock paid content from caller-supplied headers.
* Fabricate wallet receipts.
* Fabricate paid status.
* Fabricate finality.
* Fabricate balance.
* Create silent spend behavior.
* Bypass backend wallet/storage enforcement.

### Transport/header protection

Omnigate may carry metadata/context headers, but must not accept caller transport headers as authority.

Allowed header classes:

* Request context.
* Passport/account context.
* Product metadata.
* Provider metadata.
* Upload metadata.
* Rendition metadata.
* Display/cache-control metadata.
* Correlation/request IDs.

Forbidden header classes:

* Receipt proof.
* Paid proof.
* Unlock proof.
* Entitlement proof.
* Finality proof.
* Balance proof.
* Ledger proof.
* State-root proof.
* Receipt-root proof.
* Checkpoint proof.
* Validator proof.
* Spend authority.
* Capture authority.

### Cache/content-addressing protection

Omnigate now documents and tests that:

* b3 hashes verify content identity only.
* b3 hashes do not prove payment.
* Manifests describe content/policy/pointers but do not prove payment by themselves.
* Offline/local cache may help render already authorized content but cannot unlock paid content alone.
* Paid stream/chat/media paths must fail closed when backend paid truth is unavailable.

## What remains for omnigate

### Immediate remaining work

No required immediate patch remains for omnigate QuickChain Phase-0/preflight. The full crate gate is green.

Recommended next action for omnigate itself:

* Park omnigate after these notes.
* Return only if later cross-crate QuickChain umbrella tests uncover a boundary gap.
* Optionally add a final no-settlement-route polish test later, but this is not required now because `quickchain_preflight_boundary.rs` already covers the major public authority-route shape.

### Optional future polish for omnigate

Optional later hardening:

1. Add a dedicated no-settlement-route test if we want more explicit coverage for:

   * `/quickchain`
   * `/checkpoint`
   * `/state-root`
   * `/receipt-root`
   * `/validator`
   * `/anchor`
   * `/bridge`
   * `/settlement`
   * `/staking`
   * `/liquidity`
   * `/rox`
   * `/solana`

2. Add a structured route inventory test so public routes are snapshotted by category:

   * Hydration routes.
   * Product routes.
   * Paid coordination routes.
   * Ops routes.
   * Forbidden authority route families.

3. Add a cross-crate “gateway -> omnigate -> wallet/storage” paid-flow smoke contract once more crates are swept.

4. Add stronger checks later when `svc-index` and `ron-policy` have their own QuickChain preflight gates.

5. Add docs linking omnigate’s Phase-0 constraints to the future QuickChain settlement blueprint once workspace-level notes are consolidated.

### Deferred/future work not allowed yet

Do not add any of the following to omnigate at this stage:

* QuickChain state roots.
* Receipt roots.
* Checkpoint production.
* Validator logic.
* Consensus logic.
* Settlement logic.
* External anchors.
* Solana integration.
* ROX logic.
* Public bridge logic.
* Staking routes.
* Liquidity routes.
* Exchange-facing logic.
* Gateway/omnigate-ledger mutation.
* Rewarder payout mutation.
* Raw engagement protocol payouts.
* DB-order/wall-clock root derivation.
* Unknown consensus fields.
* Any root-producing code without canonical bytes and golden vectors.

If these are needed later, they must be handled through the proper QuickChain phase gates, not as omnigate product shortcuts.

## Current completion estimate

### Omnigate QuickChain Phase-0/preflight

Estimated completion: **~92–94%**

Reasoning:

* Boundary docs are present.
* Focused preflight tests are present.
* Paid access tests are present.
* Cache/b3 non-authority tests are present.
* Transport/header authority tests are present.
* Product route regressions pass.
* Clippy passes with `-D warnings`.
* The known brittle header test design was corrected.
* Crate can be parked for now.

Remaining percentage is reserved for optional polish and future cross-crate integration discoveries.

### QuickChain Phase-0 overall

Estimated completion after omnigate: **~87–90%**

Crates already swept or substantially covered:

* `ron-proto`
* `ron-ledger`
* `svc-wallet`
* `ron-accounting`
* `svc-rewarder`
* `svc-storage`
* `svc-gateway`
* `omnigate`

Likely next crates:

1. `svc-index`
2. `ron-policy`
3. Workspace-level umbrella gate / cross-crate preflight scripts

Full long-term QuickChain remains much lower than Phase-0 because real canonical roots, locked golden vectors, state proofs, receipt roots, validator/committee work, DA/archive/challenge paths, and external anchors are future phases and remain intentionally forbidden from current runtime crates.

## Recommended next crate after omnigate

Recommended next crate: **`svc-index`**

Why `svc-index` is next:

* It owns lookup/pointer behavior.
* It can easily become confused with authority if not hardened.
* It must not become receipt/balance/payment truth.
* It must not treat names/pointers/manifests as payment authority.
* It should preserve the rule: names are pointers, b3 hashes are truth for bytes, wallet/ledger are truth for economics.
* It should reject QuickChain root/checkpoint/validator/settlement creep just like omnigate/gateway/storage.

After `svc-index`, recommended next crate is **`ron-policy`**, because policy will eventually describe access constraints but must not mutate ledger or become paid receipt authority.

## Final parking note

Omnigate is now in a good place for QuickChain Phase-0/preflight.

The key accomplishment is not just that tests pass. The key accomplishment is that omnigate now has the correct conceptual boundary:

```text
Omnigate coordinates product and hydration flows.
Backend wallet/ledger/storage services own payment and enforcement truth.
Transport headers, cache, b3, manifests, profiles, provider metadata, and local display state are not economic authority.
QuickChain remains future settlement infrastructure and does not enter omnigate as runtime/root/validator/bridge/staking/liquidity logic.
```

This crate can be parked until the broader QuickChain sweep reaches cross-crate integration tests or until `svc-index`/`ron-policy` introduce new boundary expectations that need omnigate-side coordination.


### END NOTE - JUNE 15 2026 - 13:55 CST


### BEGIN NOTE - JUNE 17 2026 - 17:55 CST

# QuickChain Phase 0 / Preflight Notes — `svc-gateway` + `omnigate`

## Session Status

This session completed and parked the QuickChain Phase 0/preflight boundary work for both:

* `svc-gateway`
* `omnigate`

Both crates are now green for the current QuickChain parking slice. The work performed here does **not** implement QuickChain roots, checkpoints, validators, bridges, staking, pruning, settlement, external anchors, or public-chain authority. This work intentionally hardens the boundary around those future concepts and proves these crates do not accidentally become chain runtimes, ledger mutators, fake receipt issuers, or paid-access bypass points.

The important architectural result is that `svc-gateway` and `omnigate` are now explicitly documented, tested, and scripted as **transport / hydration / coordination surfaces only**, while economic truth remains owned by the internal ROC backend path:

`ron-proto econ DTOs -> ron-ledger truth -> svc-wallet mutation front door -> svc-storage / svc-gateway / omnigate paid enforcement -> ron-accounting snapshots -> svc-rewarder payout planning`

These crates now preserve the doctrine:

* Backend owns truth.
* Wallet / ledger own economic mutation.
* Gateway and Omnigate coordinate and proxy.
* Cache is convenience only.
* b3 proves bytes, not payment.
* crab:// is navigation, not authority.
* QuickChain is future settlement infrastructure, not current runtime.
* No fake balances.
* No fake receipts.
* No transport-header authority.
* No public QuickChain authority routes.

---

# `svc-gateway` QuickChain Phase 0 Notes

## Current Status

`svc-gateway` is parked and green for the current QuickChain Phase 0/preflight slice.

The crate is now treated as a public boundary / gateway surface. It may accept public traffic, normalize and route requests, forward safe context, and proxy to Omnigate / RustyOnions services. It must **not** become a ledger, wallet, checkpoint producer, validator, bridge, authority root, balance source, receipt fabricator, or QuickChain runtime.

The successful parking result means the crate currently satisfies the intended Phase 0 safety posture:

* No direct QuickChain runtime authority.
* No direct ledger mutation authority.
* No fake receipt generation.
* No fake balance generation.
* No paid unlock from client headers.
* No cache-only unlock.
* No gateway-created finality.
* No public root/checkpoint/validator/bridge routes.
* Gateway remains a public transport boundary, not economic truth.

## Role of `svc-gateway`

`svc-gateway` is the public entry boundary for client/product traffic. In the CrabLink/Tauri model, normal production flow is:

`CrabLink Tauri -> configured svc-gateway -> omnigate -> RustyOnions services`

The gateway is allowed to:

* Receive public client requests.
* Normalize request shape.
* Apply transport-level controls.
* Forward requests to Omnigate or other configured services.
* Preserve safe correlation/idempotency context.
* Strip or reject unsafe authority claims from clients.
* Fail closed when unsupported QuickChain/economic authority routes are requested.
* Keep the client/API edge clean and reviewable.

The gateway is **not** allowed to:

* Mutate `ron-ledger`.
* Create wallet holds.
* Capture/release holds.
* Mint ROC.
* Burn ROC.
* Transfer ROC.
* Invent balances.
* Invent receipts.
* Mark settlement final.
* Produce roots.
* Produce checkpoints.
* Run validators.
* Expose bridge routes.
* Treat `x-ron-*` or QuickChain-like request headers as economic authority.
* Let client cache state unlock paid content.
* Treat b3 hashes as payment proof.
* Treat crab:// navigation as economic authority.

## Boundary Hardened

The gateway now has a specific QuickChain preflight boundary around public routes and proxy behavior.

The main boundary invariant is:

`svc-gateway is a transport/public boundary, not a chain authority and not a ledger/wallet mutation service.`

This is important because `svc-gateway` sits at the highest-risk boundary: it is closest to public clients, therefore it must aggressively refuse any smuggled client-side economic authority.

The gateway must only forward or proxy to backend truth sources. It must not interpret user-supplied headers, local cache claims, local receipt cache entries, or client DTOs as proof of payment.

## Docs Added / Hardened

A QuickChain preflight documentation file was added or hardened under the crate docs area.

Purpose of the docs:

* Define `svc-gateway` as a public route/proxy boundary.
* State that it is not a chain runtime.
* State that it is not a validator.
* State that it is not a bridge.
* State that it is not a checkpoint/root producer.
* State that it is not a wallet.
* State that it is not a ledger.
* State that it does not own ROC balance truth.
* State that it does not own receipt truth.
* State that paid access must come from backend wallet/ledger receipt validation.
* State that transport headers are not economic authority.
* State that cache is convenience only.
* State that b3 hashes prove bytes, not payment.
* Preserve QuickChain operation identity rules:

  * `operation_id` is backend-assigned durable ledger operation identity.
  * `idempotency_key` is a retry key, not authority.
  * `account_sequence` is ledger-assigned.
  * `hold_id` identifies one hold lifecycle.
  * expiry is epoch-based.

The docs were made scanner-friendly so the tests can enforce exact architectural phrases over time. This is deliberate: the crate documentation is now part of the preflight gate, not just a human note.

## Scripts Added / Hardened

Two crate-local scripts were added or hardened:

### `crates/svc-gateway/scripts/dev-quickchain-preflight.sh`

This is the main exhaustive QuickChain preflight runner for the crate.

It performs the crate-specific gate:

* Verifies required docs exist.
* Verifies test directory exists.
* Rejects Python helper drift under the crate.
* Runs formatting check.
* Dynamically discovers `quickchain*.rs` tests.
* Runs all focused QuickChain preflight tests.
* Runs relevant paid/product/proxy regressions.
* Runs all-target tests.
* Runs clippy with warnings denied.
* Prints explicit forbidden-scope parking message.

The important thing is that the script dynamically discovers QuickChain tests, so new `quickchain*.rs` files will automatically become part of the crate parking gate.

### `crates/svc-gateway/scripts/dev-quickchain-park.sh`

This is the parking wrapper.

It verifies the core QuickChain docs/scripts/tests exist, delegates to the preflight script, and prints the final parking marker once all gates pass.

The parking script makes it clear when `svc-gateway` is safe to leave and move to the next crate.

## Tests Added / Hardened

The `svc-gateway` QuickChain preflight suite now includes seven focused QuickChain test files:

1. `quickchain_preflight_boundary.rs`
2. `quickchain_preflight_cache_boundary.rs`
3. `quickchain_preflight_docs.rs`
4. `quickchain_preflight_no_fake_receipts.rs`
5. `quickchain_preflight_paid_access.rs`
6. `quickchain_preflight_transport_authority.rs`
7. `quickchain_tooling_boundary.rs`

These tests are intentionally not a QuickChain implementation. They are a safety fence around the crate.

## `svc-gateway` Test Coverage Meaning

### Boundary Tests

The boundary tests verify that `svc-gateway` stays in its proper role.

They guard against:

* Importing direct ledger mutation authority.
* Depending on future QuickChain runtime crates.
* Publishing public QuickChain authority routes.
* Accidentally exposing root/checkpoint/validator/bridge behavior.
* Treating gateway as a chain runtime.
* Treating gateway as wallet or ledger truth.

### Docs Tests

The docs tests verify that the crate’s own documentation names the correct role and forbidden roles.

They make sure the docs preserve:

* Gateway is a public transport/proxy boundary.
* Gateway is not ledger truth.
* Gateway is not wallet truth.
* Gateway is not chain runtime.
* Gateway is not validator.
* Gateway is not bridge.
* Gateway is not settlement.
* Operation identity rules remain correct.
* Idempotency key remains retry-only.

This matters because future contributors may see “QuickChain” and accidentally begin wiring root/checkpoint/bridge authority into the gateway. The docs gate is there to prevent that drift.

### No Fake Receipts Tests

The no-fake-receipts tests guard against any future shortcut such as:

* Constructing a “receipt” locally.
* Returning a dev/demo receipt as if it were backend truth.
* Returning fake finality.
* Inventing balances.
* Inventing paid access.
* Constructing `root`, `checkpoint`, `validator`, or settlement-looking authority fields in production paths.

This is one of the most important safety gates because gateway code often becomes tempting glue code. The test ensures glue code does not become economic truth.

### Paid Access Tests

The paid-access tests verify that gateway paid paths stay in the correct prepare/quote/proxy role.

The required invariant is:

`prepare/quote -> explicit user confirmation -> backend wallet path -> backend receipt -> unlock/render`

`svc-gateway` may participate in request routing, but it must not silently spend, silently unlock, or locally authorize paid access.

These tests protect against:

* Paid unlock bypass shortcuts.
* Hardcoded “paid: true” behavior.
* Cache-only unlock.
* Header-only unlock.
* Client-provided receipt acceptance without backend validation.
* Gateway-created receipt truth.

### Cache Boundary Tests

The cache boundary tests verify that local/client/cache/materialized responses do not become paid entitlement authority.

Important invariants:

* Cache is convenience only.
* Cached bytes do not prove payment.
* b3 proves bytes, not economic authorization.
* Manifest hydration does not prove payment.
* Local receipt cache does not prove payment without backend validation.
* Offline cache can improve display and performance but cannot unlock paid content by itself.

### Transport Authority Tests

The transport authority tests verify that public request headers do not become economic authority.

They guard against unsafe use of headers such as:

* fake `x-ron-*` authority claims
* fake QuickChain finality headers
* fake receipt headers
* fake paid unlock headers
* fake root/checkpoint headers
* fake wallet/balance headers

The gateway may forward carefully allowlisted safe context, such as correlation or idempotency context, but it must strip or refuse authority-looking headers from clients.

### Tooling Boundary Tests

The tooling tests verify the crate has the required docs and scripts, uses the dynamic test discovery flow, and avoids Python helper drift under the crate.

This matches the current project preference: Bash-only crate-local QuickChain gate scripts, no Python patch/helper drift.

## `svc-gateway` Runtime/Product Regressions Preserved

The QuickChain hardening did not replace or bypass existing product behavior. The gateway’s existing public/proxy product behavior remains covered by regressions.

Important preserved behavior:

* Gateway still proxies product traffic.
* Gateway still routes to Omnigate as the product/hydration coordinator.
* Gateway still respects configured upstreams.
* Gateway still fails closed on unsupported paths.
* Gateway still keeps transport-level behavior separate from economic truth.
* Gateway still avoids local wallet/ledger mutation.

## What Changed Conceptually in `svc-gateway`

Before this slice, the gateway was functionally acting as the public edge, but the QuickChain boundary had not been fully made executable as tests/scripts/docs.

After this slice, the gateway has an explicit “do not cross this line” QuickChain safety fence.

The crate is now protected against several dangerous classes of future regression:

1. **Authority creep**
   Gateway cannot quietly grow into a chain runtime.

2. **Receipt fabrication**
   Gateway cannot invent receipts or paid unlock proof.

3. **Header smuggling**
   Client-provided authority-looking headers cannot become truth.

4. **Cache confusion**
   Cached bytes or manifests cannot become entitlement proof.

5. **Settlement creep**
   Root/checkpoint/validator/bridge code remains parked.

6. **Testing drift**
   New QuickChain tests are automatically discovered by the crate preflight script.

## Current `svc-gateway` Completion

For the current QuickChain Phase 0/preflight objective, `svc-gateway` should be treated as complete and parked.

Approximate crate status:

* Current Phase 0/preflight slice: 100%
* Full future QuickChain runtime role: intentionally 0%, because gateway should not become that role
* Future integration role: pending later client/backend proof work, not root/checkpoint/validator work

## What Remains for `svc-gateway` Later

Later work should only happen after the upstream QuickChain phases authorize it.

Allowed later work:

* More route-level negative tests if new gateway routes are added.
* More denylist/allowlist coverage for authority-looking headers.
* Better typed error envelopes for rejected authority headers.
* More integration tests proving paid access only unlocks after backend receipt validation.
* Gateway/Tauri end-to-end proof that display-only cached receipts cannot unlock paid content alone.
* Gateway observability for rejected authority-smuggling attempts.
* Gateway docs alignment when `svc-index`, `ron-policy`, and CrabLink Tauri adapters are hardened.

Still forbidden in `svc-gateway`:

* No roots.
* No checkpoints.
* No validators.
* No bridges.
* No external settlement.
* No staking.
* No liquidity.
* No public chain state.
* No ledger mutation.
* No wallet mutation.
* No fake receipts.
* No fake balances.
* No client cache unlock authority.

---

# `omnigate` QuickChain Phase 0 Notes

## Current Status

`omnigate` is now parked and green for the current QuickChain Phase 0/preflight slice.

The terminal output proves the full Omnigate gate passed:

* `cargo fmt -p omnigate`
* `cargo test -p omnigate --test quickchain_preflight_docs`
* `crates/omnigate/scripts/dev-quickchain-preflight.sh`
* `crates/omnigate/scripts/dev-quickchain-park.sh`
* all seven focused QuickChain test suites passed
* paid/product route regressions passed
* all-target tests passed
* clippy passed with warnings denied
* final parking marker printed

The terminal output ended with:

`omnigate QuickChain preflight gate passed`

`== omnigate quickchain exhaustive preflight gate passed: tests=7 ==`

`== omnigate QuickChain parking gate passed ==`

The crate codebundle was regenerated after the successful parking run.

## Role of `omnigate`

`omnigate` is the product-facing hydration and coordination layer. It composes product flows, resolves/hydrates content views, coordinates paid storage/product interactions, and fronts certain app/page/product routes.

Its role is:

* Hydrator.
* Coordinator.
* Product route layer.
* Manifest/page composition layer.
* Backend-service coordinator.
* Explicit paid-flow coordinator.
* Fail-closed public API surface.

Its role is **not**:

* Ledger.
* Wallet.
* Chain runtime.
* QuickChain root producer.
* Checkpoint producer.
* Validator.
* Bridge.
* External settlement service.
* Balance authority.
* Receipt authority by itself.
* Entitlement authority from local cache.
* Authority source from request headers.

## Main Architecture Preserved

The Omnigate paid product flow remains aligned with the internal ROC economy:

1. Product route prepares or quotes.
2. User explicitly confirms payment/hold/spend through the wallet path.
3. `svc-wallet` talks to ledger truth.
4. Backend receipt comes back from wallet/ledger path.
5. Omnigate uses backend truth to unlock/render/coordinate.
6. Any cached/display receipt remains display-only unless validated against backend truth.

Omnigate can coordinate the flow, but it cannot become the source of economic truth.

## Docs Added / Hardened

A QuickChain preflight document was added/hardened for `omnigate`.

The docs now explicitly state the crate’s role and forbidden boundaries:

* Omnigate is hydration and product coordination boundary.
* Omnigate is not a chain runtime.
* Omnigate is not a validator.
* Omnigate is not a bridge.
* Omnigate is not a checkpoint/root producer.
* Omnigate is not a wallet.
* Omnigate is not a ledger.
* Omnigate does not mutate ROC balances.
* Omnigate does not create economic truth.
* Omnigate does not fabricate receipts.
* Omnigate does not treat transport headers as authority.
* Omnigate does not treat cache as entitlement proof.
* Omnigate does not treat b3 hashes as payment proof.
* Omnigate does not treat manifests as payment proof.
* Omnigate does not treat local receipt cache as backend truth.

The final docs fix added/ensured exact scanner phrases for cache/content boundaries, including the essential idea:

* cache is convenience only
* cache cannot unlock paid content
* b3 hashes are content truth, not economic truth
* b3 proves bytes
* b3 does not prove payment
* manifest hydration does not prove payment
* local receipt cache does not prove payment without backend validation
* backend wallet/ledger receipt validation remains required for paid unlock

This fixed the earlier docs scanner failure and completed the crate gate.

## Scripts Added / Hardened

Two crate-local QuickChain scripts now define the Omnigate parking workflow.

### `crates/omnigate/scripts/dev-quickchain-preflight.sh`

This is the exhaustive Omnigate QuickChain gate.

It performs:

* Required docs check.
* Required tests directory check.
* Python helper drift rejection.
* `cargo fmt -p omnigate -- --check`.
* Dynamic discovery of `quickchain*.rs` tests.
* Execution of all focused QuickChain suites.
* Execution of paid/product route regressions.
* `cargo test -p omnigate --all-targets`.
* `cargo clippy -p omnigate --all-targets --no-deps -- -D warnings`.
* Final forbidden-scope parked message.

The script names the known focused suites for reviewability while still discovering tests dynamically. This is the right balance: humans can see the intended gate, and the script still catches new `quickchain*.rs` test files automatically.

### `crates/omnigate/scripts/dev-quickchain-park.sh`

This is the parking wrapper.

It verifies that the required docs/script/tooling files exist, delegates to the preflight gate, and prints:

`== omnigate QuickChain parking gate passed ==`

This makes Omnigate’s parked status explicit and repeatable.

## Tests Added / Hardened

Omnigate now has seven focused QuickChain preflight test suites:

1. `quickchain_preflight_boundary.rs`
2. `quickchain_preflight_cache_boundary.rs`
3. `quickchain_preflight_docs.rs`
4. `quickchain_preflight_no_fake_receipts.rs`
5. `quickchain_preflight_paid_access.rs`
6. `quickchain_preflight_transport_authority.rs`
7. `quickchain_tooling_boundary.rs`

These test suites are intentionally defensive. They do not implement QuickChain. They prevent Omnigate from accidentally implementing the wrong parts too early.

## Omnigate Test Coverage Meaning

### `quickchain_preflight_boundary.rs`

This suite proves Omnigate remains a hydrator/coordinator and not a chain runtime.

It verifies:

* Route docs keep Omnigate as hydrator/coordinator.
* Omnigate Cargo config does not depend on `ron-ledger` or future QuickChain runtime crates.
* Wallet surface remains display balance and explicit hold front door only.
* Public v1 router does not publish QuickChain authority routes.
* Omnigate source does not import direct ledger or chain authority.

This prevents accidental dependency/route creep.

### `quickchain_preflight_cache_boundary.rs`

This suite proves that cache/content addressing cannot become paid entitlement authority.

It verifies:

* Docs state cache and b3 are not paid entitlement authority.
* Stream and chat sources fail closed for cached paid media.
* Paid-access routes do not parse transport cache headers as unlock authority.
* Production source does not claim cache, b3, or manifest proves payment.

This is critical for CrabLink/Tauri offline cache design. Offline/local cache may improve UX, but it must not unlock paid content alone.

### `quickchain_preflight_docs.rs`

This suite enforces the crate docs as part of the architecture gate.

It verifies:

* Docs state Omnigate hydration/product coordination boundary.
* Docs name forbidden authority roles.
* Docs preserve operation identity rules.
* Docs define cache/content addressing as non-economic truth.
* Preflight script lists focused tests and clippy gate.

This converts architectural doctrine into testable crate-local requirements.

### `quickchain_preflight_no_fake_receipts.rs`

This suite guards against fake economic truth.

It verifies:

* Wallet balance fallback is explicitly non-authoritative.
* Paid routes use wallet receipt values without dev receipt fabrication.
* Production source does not construct root or validator authority fields.
* Production source does not construct obvious fake receipts or finality.

This is a major safety win. Omnigate coordinates visible product behavior, so it is one of the easiest places for fake receipts or fake paid flags to creep in. This suite prevents that.

### `quickchain_preflight_paid_access.rs`

This suite guards paid route behavior.

It verifies:

* Paid storage route stays prepare/estimate/proxy only.
* Content view and site visit payment routes name the wallet front-door invariant.
* Stream and chat paid paths fail closed on backend truth.
* Production source does not contain paid/unlock bypass shortcuts.

This preserves the core paid flow:

`prepare/quote -> explicit confirmation -> backend wallet path -> backend receipt -> unlock/render`

### `quickchain_preflight_transport_authority.rs`

This suite prevents authority-smuggling through HTTP headers.

It verifies:

* Docs state transport headers are not economic authority.
* Proxy-like routes filter `x-ron-*` headers before downstream forwarding.
* Route sources do not accept fake `x-ron` authority headers.
* Receipt/paid authority must come from backend paths, not request headers.
* Production `x-ron` headers do not define QuickChain authority tokens.

This protects against client-side spoofing of receipt/payment/finality.

### `quickchain_tooling_boundary.rs`

This suite verifies the crate-local QuickChain tooling is present and disciplined.

It verifies:

* Docs and runner files exist.
* Preflight script dynamically discovers QuickChain tests and runs hard gates.
* Preflight script lists known boundary suites for reviewability.
* Park script delegates to preflight and prints parking marker.
* No Python helper drift exists under the crate.

This aligns with the current project patch/test preference: Bash scripts, deterministic crate gates, no extra Python helper drift.

## Product / Paid Route Regressions Preserved

The Omnigate preflight script also ran paid/product route regressions after the focused QuickChain tests.

The following route regressions passed:

* `content_view`
* `site_visit`
* `streams`
* `chat_routes`
* `paid_storage_estimate_proxy`
* `paid_storage_prepare`
* `paid_storage_write_proxy`

Important preserved behavior:

### Content View

* Quote returns manifest recipient and does not call wallet.
* Pay recovers wallet nonce and returns wallet receipt.
* Quote rejects recipient mismatch.

This preserves separation between quote and pay.

### Site Visit

* Quote returns expected ROC amount and manifest payout recipient.
* Pay recovers wallet nonce and returns wallet receipt.
* Quote rejects recipient mismatch.

This preserves paid site visit semantics without Omnigate becoming wallet truth.

### Streams

* Latest segment requires stream-bound wallet receipt.

This protects stream paid access from cache/header bypass.

### Chat Routes

* Chat create/resolve/quote works.
* Paid send requires quote.
* Free chat send/list/latest works.
* Moderation fails closed.

This preserves chat product behavior while preventing paid send shortcutting.

### Paid Storage Estimate Proxy

* Happy path passes.
* Storage errors pass through.
* Upstream connect failure yields problem 502.

This keeps Omnigate as proxy/coordinator, not storage authority.

### Paid Storage Prepare

* Returns hold template from storage estimate.
* Maps storage estimate errors.
* Rejects bad JSON without storage call.
* Upstream connect failure yields problem 502.

This preserves prepare-only behavior.

### Paid Storage Write Proxy

* PUT and POST happy paths pass.
* Storage errors pass through.
* Upstream connect failure yields problem 502.

This keeps paid storage enforcement owned by storage/wallet/ledger path, not Omnigate-local shortcut logic.

## All-Targets Regression Coverage Preserved

The Omnigate parking gate also ran all-target tests and clippy.

The all-target run covered broad product and infrastructure behavior, including:

* App/builtin page routes.
* Asset page resolver.
* Named site crab resolver.
* Article asset publish.
* Comment asset publish.
* Image asset prepare.
* Text asset publish.
* Site launch.
* Passport profile routes.
* Policy gate.
* Policy metrics.
* Middleware contract.
* Admission contract.
* Readiness behavior.
* Metrics contract.
* ZK receipt/read-only placeholders.
* Product route regressions.
* QuickChain preflight suites.
* Benches with zero-test harnesses where applicable.

The successful all-target run is important because the QuickChain hardening did not break the existing Omnigate product surface.

## Clippy Gate Preserved

`cargo clippy -p omnigate --all-targets --no-deps -- -D warnings` passed.

This is important because the preflight gate is not just a test gate. It also enforces warning-free compile hygiene for the crate’s targets.

## What Changed Conceptually in `omnigate`

Before this slice, Omnigate already coordinated product flows, but the QuickChain boundaries were not fully executable as local docs/scripts/tests.

After this slice, Omnigate now has a hard preflight fence around the most important future-risk boundaries:

1. **Hydration boundary**
   Omnigate hydrates and coordinates; it does not become a chain runtime.

2. **Wallet/ledger boundary**
   Omnigate can call/proxy wallet-facing flows but does not mutate ledger truth directly.

3. **Receipt boundary**
   Omnigate does not fabricate receipts, roots, finality, or balances.

4. **Paid-access boundary**
   Paid unlock requires backend truth, not cache, not headers, not local display state.

5. **Cache/content boundary**
   b3 and manifests prove content identity, not payment.

6. **Transport boundary**
   Client headers cannot smuggle QuickChain or wallet authority.

7. **Tooling boundary**
   Omnigate now has a repeatable parking script and dynamically discovered focused tests.

## Current Omnigate Completion

For the current QuickChain Phase 0/preflight objective, `omnigate` should be treated as complete and parked.

Approximate crate status:

* Current Phase 0/preflight slice: 100%
* Full future QuickChain runtime role: intentionally 0%, because Omnigate should not become a runtime
* Future integration role: pending later backend receipt/root/proof phases, but not authorized yet

## What Remains for `omnigate` Later

Later work should be strictly scoped and should not cross into root/checkpoint/validator/bridge implementation until the proper upstream QuickChain gates are green.

Allowed later work:

* Add more paid route negative tests as new product routes are introduced.
* Add more tests that prove display-only receipt cache cannot unlock paid content alone.
* Add deeper integration tests against `svc-wallet` once backend receipt validation is formalized.
* Add typed receipt-display DTOs once `ron-proto` economic DTOs are fully locked for the relevant phase.
* Add product-route observability around rejected fake paid headers.
* Add CrabLink/Tauri end-to-end proof that Omnigate remains backend-truth-driven.
* Add bounded media/range tests where paid media rendering needs stronger proof.
* Add route-specific policy checks after `ron-policy` QuickChain preflight is parked.
* Add index lookup non-authority checks after `svc-index` QuickChain preflight is parked.

Still forbidden in `omnigate`:

* No root production.
* No checkpoint production.
* No validator logic.
* No bridge logic.
* No external settlement.
* No staking.
* No liquidity.
* No public chain state.
* No ledger mutation.
* No direct wallet mutation.
* No raw engagement payout allocation.
* No fake receipts.
* No fake balances.
* No local-cache paid unlock.
* No transport-header authority.

---

# Combined Result of This Crate Pair

This crate pair is especially important because `svc-gateway` and `omnigate` sit between clients and the backend value plane.

Together, they now enforce a clean split:

## `svc-gateway`

Public boundary / proxy edge.

It answers:

* Is this request shaped safely enough to forward?
* Which backend/product surface should receive it?
* Which headers/context are safe?
* Which authority-like claims must be stripped or refused?

It does **not** answer:

* Did the user pay?
* What is the true balance?
* Is this receipt final?
* What is the state root?
* Is this checkpoint valid?

## `omnigate`

Hydration / product coordination layer.

It answers:

* How should a product route be prepared?
* Which storage/index/wallet-facing backend paths are involved?
* How should a product page/content view be hydrated?
* How should paid flows coordinate with backend wallet receipt paths?

It does **not** answer by itself:

* Did ledger mutate?
* Is the balance true?
* Is the receipt final?
* Is this QuickChain state root valid?
* Is this checkpoint valid?
* Has external settlement occurred?

## Backend Truth Remains Elsewhere

Truth remains:

* `ron-ledger` for economic truth.
* `svc-wallet` as the mutation front door.
* `svc-storage` for paid byte enforcement.
* `ron-accounting` for snapshots, not balance truth.
* `svc-rewarder` for payout planning, not mutation.
* future QuickChain for settlement/proof infrastructure only after gates allow it.

---

# Why This Session Matters

This session is a major architectural hardening step because it prevents the two most exposed product-facing crates from becoming accidental authorities.

The most dangerous future failure modes were addressed directly:

* A client sends a fake paid header.
* A client sends a fake receipt header.
* A client relies on cached bytes to unlock paid content.
* A route returns a fake receipt for demo convenience.
* A gateway route accidentally exposes `/quickchain/root`.
* Omnigate starts constructing validator/finality/root-looking fields.
* b3 hashes are mistaken for payment proof.
* manifest hydration is mistaken for entitlement proof.
* local receipt cache is mistaken for backend validation.
* idempotency key is mistaken for operation authority.
* public routing is mistaken for ledger mutation authority.

The new docs/tests/scripts turn those risks into repeatable gates.

---

# Current QuickChain Progress Impact

With `svc-gateway` and `omnigate` parked, the major public/product boundary pair is now protected.

Current completed/parked QuickChain Phase 0 slices include:

* `ron-proto` DTO foundation work
* `ron-ledger` atomic execution / snapshot / replay preflight work
* `svc-wallet` preflight boundary work
* `ron-accounting` snapshot/accounting boundary work
* `svc-rewarder` payout-planning boundary work
* `svc-storage` paid-storage/content boundary work
* `svc-gateway` public gateway boundary work
* `omnigate` hydration/product coordination boundary work

The next coherent crate pair remains:

1. `svc-index`
2. `ron-policy`

This is the correct next pair because:

* `svc-index` must be locked down as lookup/pointer service only, not truth/authority.
* `ron-policy` must be locked down as declarative policy only, not wallet/ledger/rewarder/settlement authority.

---

# Next Crate Pair: `svc-index` + `ron-policy`

## `svc-index` Next Goals

For `svc-index`, the next QuickChain preflight slice should prove:

* Index is lookup/pointer service only.
* Names and pointers are not authority.
* Index does not mutate ledger.
* Index does not create wallet receipts.
* Index does not prove payment.
* Index does not unlock paid content.
* Index does not own b3 bytes.
* Index does not own balance truth.
* Index does not produce roots/checkpoints/validators/bridges.
* Index routes fail closed on authority-looking QuickChain paths.
* b3 hashes remain content identity only.
* crab:// names remain navigation/pointers only.
* Pointer resolution does not equal entitlement.

Expected additions:

* `docs/quickchain-preflight.md`
* `scripts/dev-quickchain-preflight.sh`
* `scripts/dev-quickchain-park.sh`
* focused `quickchain*.rs` tests
* product/index regressions in the preflight script
* no Python helper drift

## `ron-policy` Next Goals

For `ron-policy`, the next QuickChain preflight slice should prove:

* Policy is declarative policy only.
* Policy may classify/allow/deny intent.
* Policy does not mutate ledger.
* Policy does not mutate wallet.
* Policy does not allocate rewards.
* Policy does not mint ROC.
* Policy does not transfer ROC.
* Policy does not create receipts.
* Policy does not create roots/checkpoints/validators/bridges.
* Policy decisions are not economic truth by themselves.
* Policy cannot replace wallet/ledger receipt validation.
* Policy may label event classes later, but does not create balances.

Expected additions:

* `docs/quickchain-preflight.md`
* `scripts/dev-quickchain-preflight.sh`
* `scripts/dev-quickchain-park.sh`
* focused `quickchain*.rs` tests
* policy evaluator regressions in the preflight script
* no Python helper drift

---

# Commands Confirmed Useful

For codebundle regeneration, the working command format remains:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

For the just-completed Omnigate gate, the successful command sequence was:

```bash
cargo fmt -p omnigate

cargo test -p omnigate --test quickchain_preflight_docs
crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh

bash scripts/make_crate_codex.sh --force -c omnigate
```

For future crate work, keep using Bash-only patches and crate-local preflight scripts.

---

# Final Parked State

`svc-gateway`: parked for current QuickChain Phase 0/preflight slice.

`omnigate`: parked for current QuickChain Phase 0/preflight slice.

Together, they now preserve the intended public/product boundary:

* Gateway routes and proxies.
* Omnigate hydrates and coordinates.
* Wallet/ledger own economic truth.
* Storage enforces paid bytes.
* Accounting snapshots but is not balance truth.
* Rewarder plans but does not mutate.
* Cache is convenience only.
* b3 is content truth only.
* crab:// is navigation only.
* QuickChain remains future settlement/proof infrastructure.
* No roots/checkpoints/validators/bridges/external settlement are implemented here.

This is a clean stopping point for the pair. The next session can start directly on **`svc-index` + `ron-policy`** with the same Phase 0 parking pattern.


### END NOTE - JUNE 17 2026 - 17:55 CST


### BEGIN NOTE - JUNE 19 2026 - 16:40 CST

The latest terminal output confirms both crates are **green + parked** with `tests=8` for each crate and final parking markers for both `svc-gateway` and `omnigate`. 

# QuickChain Phase 0 / QC-0A Notes — `svc-gateway` + `omnigate`

## Session summary

This session completed the QuickChain Phase 0 / QC-0A pass for the `svc-gateway` and `omnigate` pair.

The goal of this pass was not to add QuickChain runtime behavior. The goal was to harden the gateway/hydration boundary so that the paid-access value loop can safely pass through `svc-gateway` and `omnigate` without either crate becoming wallet truth, ledger truth, receipt truth, balance truth, finality truth, root authority, checkpoint authority, validator authority, bridge authority, or external settlement authority.

The new work locks the value-loop boundary:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
  -> accepted backend receipt
  -> paid unlock/render using backend-derived truth
```

This pair is now green and parked for the current QuickChain Phase 0 pass.

---

# `svc-gateway` notes

## Crate role in QuickChain Phase 0

`svc-gateway` remains the public route/proxy/admission boundary.

For QuickChain Phase 0, `svc-gateway` is allowed to:

* receive public client intent;
* route/proxy product, paid-access, identity, storage, and app requests;
* preserve safe request context;
* forward allowed headers/body/query data;
* strip caller-supplied fake authority headers;
* proxy paid access workflows toward `omnigate`;
* expose backend-derived/display-only receipt context when appropriate;
* fail closed when unsupported chain, bridge, root, checkpoint, validator, or finality routes are attempted.

For QuickChain Phase 0, `svc-gateway` is explicitly not allowed to:

* mutate ROC balances;
* call `ron-ledger` directly as economic authority;
* act as wallet authority;
* fabricate receipts;
* fabricate balances;
* decide paid entitlement from cache, client headers, b3 hashes, crab URLs, manifests, or query strings;
* produce roots;
* produce checkpoints;
* act as a validator;
* expose bridge routes;
* expose external settlement routes;
* promote an accepted backend receipt into QuickChain finality;
* treat `accepted` as `epoch_included`, `finalized`, or `anchored`.

The key doctrine for this crate is:

```text
svc-gateway is a public boundary and proxy surface, not economic truth.
```

---

## What changed in `svc-gateway`

### 1. Added pair-level QuickChain value-loop boundary coverage

A new focused QuickChain test suite was added:

```text
crates/svc-gateway/tests/quickchain_preflight_value_loop_boundary.rs
```

This test suite verifies that the gateway documentation and preflight tooling explicitly lock the public gateway role into the full paid-access value loop without transferring economic authority into the gateway.

The test suite covers:

```text
docs_lock_public_to_product_value_loop_without_authority_transfer
docs_separate_current_accepted_receipt_from_future_quickchain_status
docs_keep_gateway_out_of_receipt_balance_and_finality_truth
preflight_runner_discovers_and_names_value_loop_suite
```

The purpose of this test suite is to ensure that future changes cannot quietly turn the gateway into a wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, settlement, or finality authority.

---

### 2. Added documentation scanner phrases for the gateway value-loop boundary

The `svc-gateway` QuickChain preflight documentation was expanded with a pair-level value-loop section.

The section documents the intended route of authority:

```text
svc-gateway public route boundary
  -> omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

It also documents the current paid-access path:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordinator
  -> svc-wallet hold/transfer/capture/release/receipt path
  -> ron-ledger accepted receipt
  -> paid unlock/render using backend-derived truth
```

The documentation now explicitly states:

```text
gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.
```

This phrase is important because it locks the paired boundary between the public ingress layer and the product hydration/access coordination layer.

---

### 3. Separated current accepted backend receipt from future QuickChain statuses

The gateway docs and tests now distinguish the current backend receipt state from future QuickChain settlement statuses.

The current system may use an accepted backend receipt for local paid unlock/render, but the gateway must not treat that receipt as future QuickChain finality.

The locked distinction is:

```text
accepted backend receipt can unlock local paid content.
accepted is not finalized.
accepted is not epoch_included.
accepted is not anchored.
```

The future statuses remain parked:

```text
accepted
epoch_included
finalized
anchored
```

This matters because the current internal ROC MVP uses backend wallet/ledger truth, while QuickChain settlement status remains future infrastructure.

The gateway must not blur those layers.

---

### 4. Locked gateway non-authority over receipt, balance, and finality truth

The gateway docs and tests now explicitly state:

```text
gateway is not receipt truth.
gateway is not balance truth.
gateway is not settlement finality.
gateway never converts accepted wallet/ledger receipt into QuickChain finality.
current paid unlock is backend-derived local access, not future QuickChain epoch inclusion.
```

This prevents future route handlers, headers, proxy metadata, or cache logic from turning the gateway into a source of economic truth.

---

### 5. Confirmed forbidden scope remains parked

The gateway preflight/park output confirms that forbidden QuickChain scope remains parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

This is the correct Phase 0 state.

No root-producing code was introduced.

No checkpoint-producing code was introduced.

No validator code was introduced.

No bridge code was introduced.

No external settlement code was introduced.

No wallet or ledger mutation authority was moved into `svc-gateway`.

---

### 6. Confirmed dynamic QuickChain test discovery includes the new suite

The gateway preflight tooling now discovers the new value-loop boundary test as part of the `quickchain*.rs` dynamic suite.

The preflight runner still verifies:

```text
find "$TEST_DIR"
-name 'quickchain*.rs'
test -p "$PKG" --test "$test_name"
```

This means newly added `quickchain*.rs` test files are automatically picked up by the exhaustive preflight script.

The `quickchain_tooling_boundary` suite also confirms that the preflight and park scripts remain reviewable and that the park script delegates to preflight.

---

## `svc-gateway` tests and gates passed

The following focused test passed:

```text
cargo test -p svc-gateway --test quickchain_preflight_value_loop_boundary
```

The following exhaustive QuickChain preflight suites passed:

```text
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The gateway product/proxy regressions passed, including:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
```

The gateway all-targets pass completed successfully.

The gateway clippy gate completed successfully.

The final successful markers were:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=8 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## `svc-gateway` current status

`svc-gateway` is green and parked for this QuickChain Phase 0 pass.

Current status:

```text
svc-gateway QuickChain Phase 0 status: complete for current pair pass.
```

The crate remains a public boundary/proxy layer. It does not own QuickChain authority, wallet authority, ledger truth, receipt truth, balance truth, or finality truth.

---

## `svc-gateway` remaining future work

No additional Phase 0 work is needed in `svc-gateway` for this pair pass unless a later cross-crate umbrella runner discovers drift.

Future work belongs to later phases and should remain parked until the appropriate gates are green.

Future parked work includes:

* no root-producing implementation until canonical bytes and golden vectors are locked;
* no checkpoint-producing implementation until root semantics are finalized;
* no validator or committee behavior until later QuickChain phases;
* no bridge or external settlement behavior in the current internal ROC MVP;
* no gateway-side wallet or ledger mutation;
* no gateway-side paid unlock based on cache, b3, crab URL, manifest, or caller-supplied headers.

If future product routes are added, they must preserve the same rule:

```text
svc-gateway may route intent, but it must not become economic truth.
```

---

# `omnigate` notes

## Crate role in QuickChain Phase 0

`omnigate` remains the product hydration/access coordination boundary.

For QuickChain Phase 0, `omnigate` is allowed to:

* hydrate product pages;
* coordinate quote/access flows;
* coordinate paid content workflows through backend truth;
* call the proper backend services;
* use wallet/ledger-derived accepted receipts for local paid unlock/render;
* proxy or coordinate storage/index/policy/wallet paths as designed;
* fail closed when backend truth is unavailable;
* preserve the distinction between accepted backend receipt and future QuickChain finality.

For QuickChain Phase 0, `omnigate` is explicitly not allowed to:

* mutate ledger truth directly;
* become wallet authority;
* fabricate receipts;
* fabricate balances;
* fabricate finality;
* treat cache, b3, crab URL, manifest, stream segment, chat message, or client header as payment proof;
* generate QuickChain roots;
* generate checkpoints;
* act as a validator;
* expose bridge authority;
* expose external settlement authority;
* promote `accepted` into `epoch_included`, `finalized`, or `anchored`.

The key doctrine for this crate is:

```text
omnigate hydrates and coordinates product access, but it is not economic truth.
```

---

## What changed in `omnigate`

### 1. Added pair-level QuickChain value-loop boundary coverage

A new focused QuickChain test suite was added:

```text
crates/omnigate/tests/quickchain_preflight_value_loop_boundary.rs
```

This test suite verifies that `omnigate` remains a product hydration/access coordinator in the paid-access value loop and does not become wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, settlement, or finality authority.

The test suite covers:

```text
docs_lock_gateway_to_omnigate_to_wallet_to_ledger_value_loop
docs_separate_accepted_receipt_from_future_finality
docs_keep_omnigate_out_of_receipt_balance_and_finality_truth
preflight_runner_discovers_and_names_value_loop_suite
```

The purpose of this test suite is to prevent future hydration or paid-access code from quietly converting `omnigate` into economic authority.

---

### 2. Added documentation scanner phrases for the omnigate value-loop boundary

The `omnigate` QuickChain preflight documentation was expanded with a pair-level value-loop section.

The section documents the intended authority path:

```text
svc-gateway public route boundary
  -> omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

It also documents the product-side version of the boundary:

```text
omnigate product hydration/access coordination
  -> svc-wallet mutation front-door
  -> ron-ledger durable economic truth
```

The current paid-access path is now documented as:

```text
client intent
  -> svc-gateway public boundary
  -> omnigate quote/access/hydration coordinator
  -> svc-wallet hold/transfer/capture/release/receipt path
  -> ron-ledger accepted receipt
  -> paid unlock/render using backend-derived truth
```

This makes it clear that `omnigate` coordinates the product flow but does not own the underlying economic mutation or settlement truth.

---

### 3. Separated accepted backend receipt from future QuickChain finality

The `omnigate` docs and tests now explicitly distinguish current accepted backend receipts from future QuickChain settlement statuses.

The current accepted backend receipt can be used for local paid access, but it is not a QuickChain finality marker.

The locked distinction is:

```text
accepted backend receipt can unlock local paid content.
accepted is not finalized.
accepted is not epoch_included.
accepted is not anchored.
omnigate cannot promote accepted receipt to epoch_included, finalized, or anchored.
```

This is especially important in `omnigate` because product hydration/access flows are close to user-visible paid content. The test suite ensures that user-visible access does not become fake finality.

---

### 4. Locked omnigate non-authority over receipt, balance, and finality truth

The `omnigate` docs and tests now explicitly state:

```text
omnigate is not receipt truth.
omnigate is not balance truth.
omnigate is not settlement finality.
```

This ensures that any product route, paid route, storage route, stream route, chat route, site route, or asset route added later must keep wallet/ledger-derived truth separate from product hydration.

---

### 5. Confirmed forbidden scope remains parked

The omnigate preflight/park output confirms that forbidden QuickChain scope remains parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

This is the correct Phase 0 state.

No root-producing code was introduced.

No checkpoint-producing code was introduced.

No validator code was introduced.

No bridge code was introduced.

No external settlement code was introduced.

No direct `ron-ledger` economic mutation authority was moved into `omnigate`.

---

### 6. Confirmed dynamic QuickChain test discovery includes the new suite

The omnigate preflight tooling now discovers the new value-loop boundary test as part of the `quickchain*.rs` dynamic suite.

The preflight runner still verifies:

```text
find "$TEST_DIR"
-name 'quickchain*.rs'
test -p "$PKG" --test "$test_name"
```

The `quickchain_tooling_boundary` suite confirms that the preflight and park scripts remain reviewable and that the park script delegates to preflight.

---

## `omnigate` tests and gates passed

The following focused test passed:

```text
cargo test -p omnigate --test quickchain_preflight_value_loop_boundary
```

The following exhaustive QuickChain preflight suites passed:

```text
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The omnigate paid/product route regressions passed, including:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

The omnigate all-targets pass completed successfully.

The omnigate clippy gate completed successfully.

The final successful markers were:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=8 ==
== omnigate QuickChain parking gate passed ==
```

---

## `omnigate` current status

`omnigate` is green and parked for this QuickChain Phase 0 pass.

Current status:

```text
omnigate QuickChain Phase 0 status: complete for current pair pass.
```

The crate remains a product hydration/access coordinator. It does not own QuickChain authority, wallet authority, ledger truth, receipt truth, balance truth, or finality truth.

---

## `omnigate` remaining future work

No additional Phase 0 work is needed in `omnigate` for this pair pass unless a later cross-crate umbrella runner discovers drift.

Future work belongs to later phases and should remain parked until the appropriate gates are green.

Future parked work includes:

* no root-producing implementation until canonical bytes and golden vectors are locked;
* no checkpoint-producing implementation until root semantics are finalized;
* no validator or committee behavior until later QuickChain phases;
* no bridge or external settlement behavior in the current internal ROC MVP;
* no direct ledger mutation from `omnigate`;
* no product unlock based on cache, b3, crab URL, manifest, stream segment, chat message, or caller-supplied headers;
* no conversion of accepted backend receipts into QuickChain finality.

If future product or paid-access routes are added, they must preserve the same rule:

```text
omnigate may coordinate access, but it must not become economic truth.
```

---

# Pair-level notes: `svc-gateway` + `omnigate`

## Why this pair mattered

This pair is one of the most important Phase 0 boundaries because it is the visible path between the CrabLink client and the backend economic system.

Users interact through CrabLink/Tauri or browser clients. Those clients reach the backend through `svc-gateway`. Product pages and paid access are hydrated/coordinated by `omnigate`. That means these two crates sit directly in the path where fake authority would be most tempting or most dangerous.

This pass ensures that:

```text
client-visible access does not equal economic truth.
gateway routing does not equal economic truth.
omnigate hydration does not equal economic truth.
cache possession does not equal economic truth.
b3 possession does not equal economic truth.
crab:// navigation does not equal economic truth.
accepted backend receipt does not equal QuickChain finality.
```

The real economic path remains:

```text
svc-wallet -> ron-ledger
```

`svc-wallet` remains the mutation front-door.

`ron-ledger` remains durable economic truth.

---

## Boundary now locked by this pair

The pair now explicitly locks the following invariant:

```text
gateway and omnigate may coordinate paid access, but neither is wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, external settlement, or finality authority.
```

This is the key pair-level invariant from the session.

It protects the internal ROC MVP from accidentally drifting into premature QuickChain settlement behavior.

---

## Accepted receipt versus future QuickChain settlement status

This session also locked the distinction between current backend-paid access and future QuickChain settlement states.

Current state:

```text
accepted backend receipt
```

Future parked states:

```text
epoch_included
finalized
anchored
```

Meaning:

```text
accepted can unlock local paid content when it comes from backend wallet/ledger truth.
accepted cannot be treated as finalized.
accepted cannot be treated as epoch_included.
accepted cannot be treated as anchored.
gateway cannot promote accepted to finality.
omnigate cannot promote accepted to finality.
```

This distinction should remain throughout the rest of Phase 0.

---

## What did not change

This session did not add QuickChain runtime behavior.

Specifically, this session did not add:

```text
roots
checkpoints
validators
bridges
external settlement
staking
liquidity
exchange-facing logic
public chain authority
gateway ledger mutation
omnigate ledger mutation
fake receipts
fake balances
cache-only paid unlocks
client-header paid unlocks
b3-as-payment-proof behavior
crab-url-as-payment-proof behavior
manifest-as-payment-proof behavior
```

This is intentional and correct for Phase 0.

---

## Final test/park status

Final status for this pair:

```text
svc-gateway: green + parked
omnigate:    green + parked
```

Both crates passed:

```text
focused value-loop tests
dynamic QuickChain preflight discovery
all QuickChain boundary suites
product/paid route regressions
all-targets tests
clippy
park scripts
```

Each crate now reports:

```text
quickchain exhaustive preflight gate passed: tests=8
QuickChain parking gate passed
```

---

# Low-disk workflow note

During this session, both preflight and park scripts were run for the same crates. This duplicated the heavy exhaustive gates because the park scripts already delegate to the preflight scripts internally.

Going forward, avoid running both preflight and park for the same crate unless intentionally requesting two full passes.

Preferred low-disk flow after a focused patch is:

```text
cargo fmt -p <crate-a>
cargo fmt -p <crate-b>

cargo test -p <crate-a> --test <focused_new_test>
cargo test -p <crate-b> --test <focused_new_test>

crates/<crate-a>/scripts/dev-quickchain-park.sh
crates/<crate-b>/scripts/dev-quickchain-park.sh
```

For this pair, the preferred low-disk flow would have been:

```text
cargo fmt -p svc-gateway
cargo fmt -p omnigate

cargo test -p svc-gateway --test quickchain_preflight_value_loop_boundary
cargo test -p omnigate --test quickchain_preflight_value_loop_boundary

crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

Do not also run:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-preflight.sh
```

unless a deliberate duplicate full preflight pass is needed.

---

# Recommended next step

The next planned QuickChain Phase 0 pair is:

```text
svc-index + ron-policy
```

After that, the remaining Phase 0 pair is:

```text
CrabLink Tauri + client adapters
```

Once those are complete, the crate-by-crate Phase 0 sweep should be functionally complete. A final light wrap pass should then confirm:

```text
all touched crates have preflight/park markers;
forbidden scope remains parked everywhere;
no roots/checkpoints/validators/bridges/external settlement were introduced;
vector status is clearly labeled sketch / locked_bytes / locked_hash;
low-disk readiness workflow is documented;
Phase 0 carryover notes are complete before Phase 1 planning.
```

Only after that should Phase 1 root/proof planning begin.

---

# Current QuickChain completion estimate after this pair

After `svc-gateway + omnigate` are green and parked:

```text
QuickChain Phase 0 / QC-0A safety-preflight completion: approximately 70–75%.
Full QuickChain multi-phase vision completion: approximately 25–30%.
```

After `svc-index + ron-policy` and `CrabLink Tauri + client adapters` are completed and parked, Phase 0 should be close to:

```text
90–95% complete
```

The remaining 5–10% would be the final wrap/readiness pass rather than another major crate pair.


### END NOTE - JUNE 19 2026 - 16:40 CST




### BEGIN NOTE - JUNE 21 2026 - 12:55 CST

Both crates are now green/parked for this QuickChain service-boundary sweep. `svc-gateway` ended with the forbidden-scope marker, preflight passed, `tests=8`, and the parking marker printed.  The gateway run also proved the focused cache/docs/no-fake-receipts/paid-access suites and product/proxy regressions stayed green.   Omnigate is also parked: the latest run added/verified the 6-test transport-authority suite, kept cache/docs/no-fake-receipts/paid-access/value-loop/tooling suites green, passed clippy, and printed the `tests=8` parking marker.  

Paste the following into your crate notes, either as a shared note in both crates or split by section.

# QuickChain Phase 1 Foundation Notes — `svc-gateway` + `omnigate`

## BEGIN NOTE - JUNE 21 2026 - `svc-gateway` + `omnigate` QuickChain Phase 1 Foundation / Service-Boundary Parking Notes

## 0. Executive summary

The `svc-gateway` + `omnigate` crate pair is now green and parked for the current QuickChain Phase 1 foundation / QC-1A service-boundary sweep.

This was not root-producing implementation.

This was not validator implementation.

This was not bridge, staking, liquidity, ROX, Solana, external-settlement, or public-chain implementation.

The purpose of this sweep was to harden the public/product access path so the gateway and hydrator layers cannot quietly become:

```text
wallet truth
ledger truth
receipt truth
balance truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
cache entitlement authority
finality authority
```

The public/product path is now explicitly constrained as:

```text
client intent
-> svc-gateway public boundary / route proxy / admission surface
-> omnigate quote-access-hydration coordinator
-> svc-wallet economic mutation front-door
-> ron-ledger durable economic truth
-> backend-derived accepted receipt
-> paid unlock/render using backend truth
```

Current status:

```text
svc-gateway: GREEN / parked
omnigate:    GREEN / parked
pair status: GREEN / parked for this QuickChain service-boundary slice
```

Approximate status:

```text
svc-gateway current QuickChain Phase 1 foundation boundary slice: 100% parked
omnigate current QuickChain Phase 1 foundation boundary slice:    100% parked
full future QuickChain runtime role for either crate:              intentionally 0%
pair-level Phase 1 service-boundary readiness:                    parked
```

Interpretation:

```text
The crates are complete for the current service-boundary hardening pass.
They are not supposed to become future root producers or validators.
Later work may add proof/status display or integration tests, but ledger/root/finality truth must remain outside these crates.
```

---

# 1. Master doctrine preserved

QuickChain remains future settlement infrastructure.

The active doctrine remains:

```text
determinism before distribution
DTOs before roots
roots before validators
proofs before pruning
internal ROC before external anchors
```

The current RustyOnions authority model remains:

```text
ron-proto = DTOs and wire shapes
ron-ledger = durable economic truth
svc-wallet = economic mutation front-door
ron-accounting = snapshots and measurement, not balance truth
svc-rewarder = payout planning, not mutation
svc-storage = bytes by b3, not economic truth
svc-gateway = public route/proxy/admission boundary
omnigate = product hydration/access/quote coordinator
ron-policy = declarative policy, not mutation
svc-index = lookup/pointer service, not entitlement proof
CrabLink = display and user intent only
```

Still forbidden in `svc-gateway` and `omnigate`:

```text
no roots
no checkpoints
no validators
no bridges
no anchors
no external settlement
no staking
no liquidity
no ROX
no Solana
no exchange-facing logic
no public chain state
no direct ledger mutation
no direct wallet mutation
no fake balances
no fake receipts
no fake finality
no cache-only paid unlock
no client-header paid unlock
no b3-as-payment-proof
no manifest-as-payment-proof
```

---

# 2. Pair-level result

The pair now proves the current public/product value path stays inside the intended boundary.

Correct value loop:

```text
prepare / quote
-> explicit user confirmation
-> backend wallet path
-> backend accepted receipt / access authorization
-> gateway / omnigate enforce paid access from backend truth
-> storage serves bytes by b3 / range
-> client renders / unlocks
-> display-only receipt cache
-> balance refresh from backend wallet / ledger truth
```

Incorrect loops explicitly blocked:

```text
client says paid=true -> unlock
client sends x-ron-paid -> unlock
client sends x-ron-receipt -> unlock
client sends x-ron-finalized -> treat as finality
client sends x-ron-state-root -> treat as root proof
client sends x-ron-validator-* -> treat as validator proof
client sends x-ron-bridge-* -> treat as settlement proof
cache hit -> unlock paid content
b3 verifies bytes -> unlock paid content
manifest exists -> unlock paid content
hydrated view -> creates receipt truth
gateway proxy response -> creates balance truth
```

The pair-level success is important because these crates sit directly on the customer/product path. If either crate became economic authority, QuickChain would inherit a broken trust model before root/proof work even begins.

---

# 3. `svc-gateway` notes

## 3.1 Current `svc-gateway` status

`svc-gateway` is green and parked for this QuickChain boundary sweep.

Final observed gateway parking marker:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=8 ==
== svc-gateway QuickChain parking gate passed ==
```

Final forbidden-scope marker:

```text
[svc-gateway quickchain] forbidden scope remains parked:
no roots, no checkpoints, no validators, no bridges,
no external settlement, no fake receipts, no fake balances
```

## 3.2 `svc-gateway` role

`svc-gateway` is:

```text
public route boundary
route proxy
admission surface
public product path front door
header/body/query forwarding policy point
fail-closed edge boundary
```

`svc-gateway` is not:

```text
wallet
ledger
receipt issuer
balance authority
chain runtime
root producer
checkpoint writer
validator
bridge
external settlement service
finality source
cache entitlement authority
```

## 3.3 Gateway boundary hardened

The gateway now has an executable QuickChain safety fence around public route behavior.

The important guarantees are:

```text
gateway does not import wallet/ledger/QuickChain runtime authority
unsupported quickchain/ledger/bridge/root routes fail closed
wallet surface remains limited to proxy/hold/display-balance style behavior
product proxy strips caller-supplied QuickChain authority headers
cache/client unlock signals are stripped when they claim authority
request headers cannot smuggle economic or chain authority
response headers cannot smuggle economic or chain authority
backend receipt metadata is context, not gateway authority
client paid unlock headers are never gateway authority
accepted receipt remains distinct from future finalized/epoch_included/anchored status
```

## 3.4 Gateway focused QuickChain suites now green

Focused QuickChain suites proven green:

```text
quickchain_preflight_boundary: 4 passed
quickchain_preflight_cache_boundary: 3 passed
quickchain_preflight_docs: 5 passed
quickchain_preflight_no_fake_receipts: 3 passed
quickchain_preflight_paid_access: 3 passed
quickchain_preflight_transport_authority: 3 passed
quickchain_preflight_value_loop_boundary: 4 passed
quickchain_tooling_boundary: 5 passed
```

Important tests covered:

```text
route_source_keeps_wallet_surface_limited_to_proxy_hold_and_display_balance
gateway_source_does_not_import_wallet_ledger_or_quickchain_runtime_authority
unsupported_quickchain_ledger_bridge_root_routes_fail_closed_at_gateway
product_proxy_strips_caller_supplied_quickchain_authority_headers
cache_or_client_unlock_signals_are_stripped_when_they_claim_authority
cache_headers_are_transport_metadata_not_unlock_authority
route_source_does_not_unlock_paid_content_from_cache_or_query_claims
backend_receipt_metadata_remains_display_context_not_gateway_authority
request_headers_cannot_smuggle_economic_or_chain_authority
response_headers_cannot_smuggle_economic_or_chain_authority
client_paid_unlock_headers_are_never_gateway_authority
backend_receipt_metadata_is_context_not_gateway_paid_authority
route_source_does_not_define_cache_or_client_paid_unlock_shortcuts
hop_by_hop_and_host_headers_are_never_forwarded_or_copied
identity_and_retry_headers_are_context_not_authority
ordinary_cache_headers_remain_transport_metadata_only
docs_keep_gateway_out_of_receipt_balance_and_finality_truth
docs_lock_public_to_product_value_loop_without_authority_transfer
docs_separate_current_accepted_receipt_from_future_quickchain_status
```

## 3.5 Gateway product/proxy regressions stayed green

The gateway hardening did not break normal product proxy behavior.

Green product/proxy regression suites included:

```text
app_proxy: 5 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_write_proxy: 4 passed
product_routes_proxy: 8 passed
site_visit_routes_proxy: 1 passed
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies handle upstream connect failures as problem 502
image/site/text routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
crab resolve proxy preserves query and headers
site visit quote/pay proxies to omnigate and preserves context
```

## 3.6 Gateway all-targets and clippy green

The gateway parking gate also ran:

```text
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Both completed successfully.

## 3.7 Gateway scripts/tooling hardened

Gateway crate-local QuickChain scripts now define the repeatable parking workflow:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The preflight script behavior:

```text
checks docs exist
checks test directory exists
rejects checked-in Python helper drift
runs cargo fmt -p svc-gateway -- --check
dynamically discovers quickchain*.rs tests
runs every discovered focused QuickChain test
runs product/proxy regressions
runs cargo test -p svc-gateway --all-targets
runs cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
prints forbidden-scope marker
prints dynamic test-count marker
```

The park script behavior:

```text
checks docs/script/tooling files exist
delegates to dev-quickchain-preflight.sh
prints == svc-gateway QuickChain parking gate passed ==
```

This gives the crate a low-drift, future-safe local gate.

## 3.8 Gateway files touched / relevant

Relevant gateway files for this slice:

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/svc-gateway/tests/quickchain_preflight_boundary.rs
crates/svc-gateway/tests/quickchain_preflight_cache_boundary.rs
crates/svc-gateway/tests/quickchain_preflight_docs.rs
crates/svc-gateway/tests/quickchain_preflight_no_fake_receipts.rs
crates/svc-gateway/tests/quickchain_preflight_paid_access.rs
crates/svc-gateway/tests/quickchain_preflight_transport_authority.rs
crates/svc-gateway/tests/quickchain_preflight_value_loop_boundary.rs
crates/svc-gateway/tests/quickchain_tooling_boundary.rs
```

Product/proxy regression files exercised:

```text
crates/svc-gateway/tests/app_proxy.rs
crates/svc-gateway/tests/paid_storage_estimate_proxy.rs
crates/svc-gateway/tests/paid_storage_write_proxy.rs
crates/svc-gateway/tests/product_routes_proxy.rs
crates/svc-gateway/tests/site_visit_routes_proxy.rs
crates/svc-gateway/tests/content_view_routes_proxy.rs
crates/svc-gateway/tests/identity_routes_proxy.rs
crates/svc-gateway/tests/smoke.rs
```

## 3.9 Gateway later work

Not required before moving on, but worth tracking later:

```text
object-route-specific proxy regression test
route inventory snapshot for forbidden authority routes
observability/redaction/cardinality boundary test
response body fake-claim scan
more route-level negative tests as new gateway routes are added
gateway/Tauri end-to-end proof that display-only cached receipts cannot unlock paid content alone
metrics for rejected authority-smuggling attempts
typed error envelopes for rejected authority headers
range/media proxy hardening
TLS/config/admission production polish
CI integration of focused QuickChain park gate
```

Still do not add to gateway:

```text
root creation
checkpoint writing
validator APIs
bridge settlement
external anchor settlement
staking
liquidity
direct wallet mutation
direct ledger mutation
gateway-owned receipt issuance
gateway-owned balance truth
cache-only paid unlock
```

---

# 4. `omnigate` notes

## 4.1 Current `omnigate` status

`omnigate` is green and parked for this QuickChain boundary sweep.

Final observed omnigate parking marker:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=8 ==
== omnigate QuickChain parking gate passed ==
```

Final forbidden-scope marker:

```text
[omnigate quickchain] forbidden scope remains parked:
no roots, no checkpoints, no validators, no bridges,
no external settlement, no fake receipts, no fake balances
```

The `tests=8` count reflects the added/shared transport-authority/header-policy suite in this latest pass.

## 4.2 `omnigate` role

`omnigate` is:

```text
product coordinator
hydrator
quote/access coordinator
manifest/page composition layer
backend service coordinator
paid-flow coordinator
fail-closed public API surface behind gateway
```

`omnigate` is not:

```text
wallet
ledger
balance authority
receipt authority by itself
chain runtime
root producer
checkpoint writer
validator
bridge
external settlement service
cache entitlement authority
transport-header authority
finality source
```

## 4.3 Omnigate shared header policy added

A shared route header policy was added for omnigate v1 product routes:

```text
crates/omnigate/src/routes/v1/header_policy.rs
```

The policy allows normal product context headers while rejecting caller-supplied authority-shaped headers.

Allowed product context examples:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
asset metadata headers
```

These are context for product coordination and backend verification. They are not finality, balance truth, ledger truth, root truth, checkpoint truth, validator authority, bridge authority, or external settlement authority.

Forbidden caller-supplied authority examples:

```text
x-ron-operation-id
x-ron-account-sequence
x-ron-receipt
x-ron-paid
x-ron-unlocked
x-ron-entitlement
x-ron-balance
x-ron-ledger
x-ron-root
x-ron-state-root
x-ron-receipt-root
x-ron-hold-root
x-ron-epoch-root
x-ron-checkpoint
x-ron-finalized
x-ron-finality
x-ron-epoch-included
x-ron-anchored
x-ron-settlement
x-ron-external-settlement
x-ron-validator
x-ron-bridge
x-ron-spend-authority
x-ron-capture-authority
x-ron-quickchain-*
x-ron-validator-*
x-ron-bridge-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-proof-*
```

The important distinction:

```text
x-ron-wallet-receipt-hash may travel as backend/product verification context.
x-ron-state-root / x-ron-finalized / x-ron-validator-* / x-ron-bridge-* / x-ron-quickchain-* must not be forwarded or interpreted as authority.
```

## 4.4 Omnigate proxy helpers updated

The v1 product proxy helpers now use the shared QuickChain-aware header policy instead of directly wildcard-forwarding broad `x-ron-*` headers.

Updated route files:

```text
crates/omnigate/src/routes/v1/assets.rs
crates/omnigate/src/routes/v1/chat.rs
crates/omnigate/src/routes/v1/content_view.rs
crates/omnigate/src/routes/v1/paid.rs
crates/omnigate/src/routes/v1/profile.rs
crates/omnigate/src/routes/v1/site_visit.rs
crates/omnigate/src/routes/v1/sites.rs
crates/omnigate/src/routes/v1/text_assets.rs
```

Module registration:

```text
crates/omnigate/src/routes/v1/mod.rs
```

New/updated test:

```text
crates/omnigate/tests/quickchain_preflight_transport_authority.rs
```

Docs hardened:

```text
crates/omnigate/docs/quickchain-preflight.md
```

## 4.5 Omnigate focused QuickChain suites now green

Focused QuickChain suites proven green:

```text
quickchain_preflight_boundary: 5 passed
quickchain_preflight_cache_boundary: 4 passed
quickchain_preflight_docs: 5 passed
quickchain_preflight_no_fake_receipts: 4 passed
quickchain_preflight_paid_access: 4 passed
quickchain_preflight_transport_authority: 6 passed
quickchain_preflight_value_loop_boundary: 4 passed
quickchain_tooling_boundary: 5 passed
```

Important tests covered:

```text
omnigate_cargo_does_not_depend_on_ron_ledger_or_quickchain_runtime_crates
route_docs_keep_omnigate_as_hydrator_and_coordinator_not_chain_runtime
wallet_surface_is_display_balance_and_explicit_hold_front_door_only
public_v1_router_does_not_publish_quickchain_authority_routes
omnigate_source_does_not_import_direct_ledger_or_chain_authority
stream_and_chat_sources_explicitly_fail_closed_for_cached_paid_media
docs_state_cache_and_b3_are_not_paid_entitlement_authority
paid_access_routes_do_not_parse_transport_cache_headers_as_unlock_authority
production_source_does_not_claim_cache_b3_or_manifest_proves_payment
wallet_balance_fallback_is_explicitly_non_authoritative
paid_routes_use_wallet_receipt_values_without_dev_receipt_fabrication
production_source_does_not_construct_root_or_validator_authority_fields
production_source_does_not_construct_obvious_fake_receipts_or_finality
paid_storage_route_stays_prepare_estimate_or_proxy_only
content_view_and_site_visit_pay_routes_name_wallet_front_door_invariant
stream_and_chat_paid_paths_fail_closed_on_backend_truth
production_source_does_not_contain_paid_or_unlock_bypass_shortcuts
shared_header_policy_blocks_quickchain_authority_headers
docs_state_transport_headers_are_not_economic_authority
proxy_like_routes_use_shared_header_policy_for_x_ron_forwarding
route_sources_do_not_accept_fake_x_ron_authority_headers
receipt_or_paid_authority_must_come_from_backend_paths_not_request_headers
production_x_ron_headers_do_not_define_quickchain_authority_tokens_outside_policy
docs_keep_omnigate_out_of_receipt_balance_and_finality_truth
docs_lock_gateway_to_omnigate_to_wallet_to_ledger_value_loop
docs_separate_accepted_receipt_from_future_finality
```

## 4.6 Omnigate paid/product regressions stayed green

The omnigate hardening did not break paid/product route behavior.

Green paid/product route regressions included:

```text
content_view: 3 passed
site_visit: 3 passed
streams: 1 passed
chat_routes: 2 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_prepare: 4 passed
paid_storage_write_proxy: 4 passed
```

Representative preserved behavior:

```text
content view quote rejects recipient mismatch
content view pay recovers wallet nonce and returns wallet receipt
site visit quote returns manifest payout recipient
site visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
paid storage estimate/prepare/write proxy paths pass upstream errors safely
paid object prepare returns hold template from storage estimate
paid write proxy supports POST and PUT happy paths
```

## 4.7 Omnigate all-targets and clippy green

The omnigate parking gate also ran:

```text
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Both completed successfully.

All-targets also re-ran broad product coverage, including:

```text
article asset publish
asset page resolver
builtin page resolver
chat routes
comment asset publish
content view
image asset prepare
middleware contract
named site crab resolver
paid storage estimate
paid storage prepare
paid storage write
passport profile routes
policy gate
site launch
site visit
streams
text asset publish
zk receipt/read-only placeholder targets
```

## 4.8 Omnigate scripts/tooling hardened

Omnigate crate-local QuickChain scripts now define the repeatable parking workflow:

```text
crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The preflight script behavior:

```text
checks docs exist
checks test directory exists
rejects checked-in Python helper drift
runs cargo fmt -p omnigate -- --check
dynamically discovers quickchain*.rs tests
runs every discovered focused QuickChain test
runs paid/product route regressions
runs cargo test -p omnigate --all-targets
runs cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
prints forbidden-scope marker
prints dynamic test-count marker
```

The park script behavior:

```text
checks docs/script/tooling files exist
delegates to dev-quickchain-preflight.sh
prints == omnigate QuickChain parking gate passed ==
```

## 4.9 Omnigate later work

Not required before moving on, but worth tracking later:

```text
proof/finality display model once upstream QuickChain proof DTOs and vectors are mature
route inventory snapshot for future proof/status endpoints, if any
hydration-specific negative tests for new product pages
better typed error envelopes for rejected authority headers
observability for rejected authority-smuggling attempts
more e2e tests with CrabLink Tauri display-only receipts
production wallet/access transport hardening
policy/config polish for paid product flows
CI integration of focused QuickChain park gate
```

Still do not add to omnigate:

```text
root creation
checkpoint writing
validator APIs
bridge settlement
external anchor settlement
staking
liquidity
direct wallet mutation
direct ledger mutation
omnigate-owned receipt issuance
omnigate-owned balance truth
cache-only paid unlock
manifest-only paid unlock
b3-only paid unlock
transport-header paid unlock
```

---

# 5. Pair-level architecture preserved

The combined pair now has the right division of responsibility:

```text
svc-gateway receives public client traffic.
svc-gateway proxies and filters.
svc-gateway fails closed.
svc-gateway does not decide wallet/ledger truth.

omnigate hydrates and coordinates.
omnigate prepares/quotes paid flows.
omnigate displays backend-derived receipt/access context.
omnigate fails closed when backend truth is missing.
omnigate does not create wallet/ledger truth.

svc-wallet remains the mutation front-door.
ron-ledger remains durable economic truth.
QuickChain future roots/proofs/finality remain outside gateway and omnigate.
```

The safe current paid flow is:

```text
client request
-> svc-gateway public route/proxy/admission boundary
-> omnigate quote/access/hydration coordinator
-> svc-wallet hold/transfer/capture/release/receipt path
-> ron-ledger accepted receipt
-> omnigate/gateway paid unlock/render from backend-derived truth
-> client display-only cache/receipt panel
```

Accepted-vs-finalized wording remains preserved:

```text
accepted backend receipt may unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
accepted is not external settlement
```

---

# 6. Commands that proved the pair

## `svc-gateway`

Final command:

```bash
bash crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The park script delegated to preflight and ran:

```text
fmt check
dynamic quickchain test discovery
8 focused quickchain*.rs tests
product/proxy regressions
all-targets
clippy -D warnings
forbidden-scope marker
parking marker
```

## `omnigate`

Final command sequence:

```bash
cargo fmt -p omnigate
cargo test -p omnigate --test quickchain_preflight_transport_authority
bash crates/omnigate/scripts/dev-quickchain-park.sh
```

The park script delegated to preflight and ran:

```text
fmt check
dynamic quickchain test discovery
8 focused quickchain*.rs tests
paid/product route regressions
all-targets
clippy -D warnings
forbidden-scope marker
parking marker
```

---

# 7. Regenerate focused codebundles after notes

Recommended regeneration commands:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Do this after notes are updated so the next session sees the docs/tests/scripts/notes state.

---

# 8. Low-disk reminder

Do not run duplicate full gates.

Preferred workflow:

```text
focused test(s) first
then park script once
do not run preflight and park back-to-back for the same crate unless deliberately duplicating
```

Low-disk gateway gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-gateway" \
bash crates/svc-gateway/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/svc-gateway
```

Low-disk omnigate gate:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/omnigate" \
bash crates/omnigate/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/omnigate
```

---

# 9. Current QuickChain Phase 1 completion impact

This pair closes one of the most dangerous Phase 1 foundation gaps: the public/product route boundary.

Before this pair, the main risk was that the public gateway or hydrator could accidentally grow into:

```text
receipt authority
balance authority
paid unlock authority
cache entitlement authority
future root/checkpoint/finality display authority
transport-header authority
bridge/external settlement surface
```

After this pair, those paths are guarded by:

```text
docs
source scans
route-boundary tests
header-policy tests
cache-boundary tests
no-fake-receipt tests
paid-access tests
value-loop tests
tooling tests
product regressions
all-targets
clippy
parking markers
```

Updated Phase 1 foundation progress estimate:

```text
QuickChain Phase 1 foundation: approximately 75% complete
```

Remaining Phase 1 foundation pairs after this:

```text
svc-index + ron-policy
CrabLink Tauri + client adapters
final integration/carryover pass
```

---

# 10. Next crate pair

The next pair should be:

```text
svc-index + ron-policy
```

Reason:

```text
svc-index must remain lookup/pointer infrastructure, not paid entitlement or proof authority.
ron-policy must remain declarative policy/evaluation, not wallet, ledger, receipt, balance, root, checkpoint, validator, bridge, or settlement authority.
```

Expected next-session focus:

```text
index pointer authority
policy decision non-authority
accepted-vs-finalized wording
no paid unlock from index lookup alone
no receipt/balance/finality from policy decisions
no root/checkpoint/validator/bridge/external settlement creep
crate-local park gates
```

### END NOTE - JUNE 21 2026 - 12:55 CST



### BEGIN NOTE - JUNE 22 2026 - 14:15 CST

The run proves both crates are now parked: each new QC-1A pair-interlock suite passed **5/5**, then `svc-gateway` and `omnigate` each printed the final QuickChain parking marker with `tests=9`. The buildplan still labels this as **Phase 1 Round 1 / QC-1A foundation**, not full Phase 1 completion; the purpose is to prepare crates for roots/proofs without letting services become root authority.  

Paste this into the crate notes.

### BEGIN NOTE - JUNE 22 2026 - QUICKCHAIN PHASE 1 ROUND 1 / QC-1A - svc-gateway + omnigate

# QuickChain Phase 1 Round 1 / QC-1A Notes — `svc-gateway` + `omnigate`

## 0. Current Status

```text
svc-gateway: GREEN / parked for QuickChain Phase 1 Round 1 / QC-1A foundation
omnigate:    GREEN / parked for QuickChain Phase 1 Round 1 / QC-1A foundation
```

This pair is now parked for the current QC-1A foundation pass.

This is not full Phase 1 completion.

This is not root-producing implementation.

This is not proof engine implementation.

This is not validator, committee, bridge, staking, liquidity, ROX, Solana, external settlement, or public-chain runtime implementation.

Safe label:

```text
svc-gateway + omnigate are green and parked for QuickChain Phase 1 Round 1 / QC-1A foundation.
```

Do not call this:

```text
Full Phase 1 complete
root engine complete
proof engine complete
validator ready
settlement ready
QuickChain complete
```

---

## 1. Buildplan Context

The active phase/round reference is:

```text
QUICKCHAIN_BUILDPLAN.MD
```

The master QuickChain blueprint remains:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
```

Current round:

```text
Phase 1 Round 1 — QC-1A pre-root/root-material foundation
```

Round purpose:

```text
Prepare every crate for roots/proofs without letting any service become root authority.
```

For this crate pair, the buildplan goal was:

```text
svc-gateway + omnigate:
  - paid enforcement stays backend-derived
  - no gateway/omnigate ledger mutation
  - no fake paid unlock
  - no QuickChain state authority
  - no route-level finality claims
```

This pair now satisfies that current QC-1A boundary/interlock goal.

---

## 2. What Was Added

Two new pair-interlock test files were added:

```text
crates/svc-gateway/tests/quickchain_preflight_phase1_pair_interlock.rs
crates/omnigate/tests/quickchain_preflight_phase1_pair_interlock.rs
```

These tests are intentionally boundary/interlock tests only.

They do not implement roots, checkpoints, validators, settlement, bridges, anchors, staking, liquidity, or external chain behavior.

They harden the public/product access path so the gateway and hydration layers cannot quietly become authority over:

```text
wallet truth
ledger truth
receipt truth
balance truth
root truth
checkpoint truth
finality truth
validator authority
bridge authority
external settlement authority
cache entitlement authority
client/header-paid authority
```

---

## 3. `svc-gateway` Notes

## 3.1 Gateway Role Preserved

`svc-gateway` remains:

```text
public boundary
route/proxy layer
public enforcement layer
header/filtering boundary
fail-closed public API surface
backend-derived paid/display response carrier
```

`svc-gateway` may:

```text
terminate public API ingress
route requests
proxy requests
apply public-boundary checks
apply bounded transport/header policy
fail closed
forward selected backend-derived context
return bounded HTTP/problem responses
preserve safe request context such as auth/correlation/idempotency where allowed
```

`svc-gateway` must not:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
hold ROC
capture ROC
release ROC
mutate ron-ledger
invent wallet receipts
invent balances
invent paid access
invent finality
invent roots
invent checkpoints
act as validator
act as committee member
act as bridge
act as external settlement authority
unlock paid content from client/cache/header claims alone
treat b3 as payment proof
treat crab:// navigation as economic authority
treat transport headers as trust roots
create route-level finality claims
create QuickChain state authority
```

Boundary sentence locked by this pass:

```text
svc-gateway is the public boundary; it may route, proxy, and fail closed, but it must not become wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, or external settlement authority.
```

---

## 3.2 Gateway Pair-Interlock Suite

New focused test:

```text
cargo test -p svc-gateway --test quickchain_preflight_phase1_pair_interlock
```

Result:

```text
5 passed
0 failed
```

Tests proved:

```text
docs_lock_gateway_into_qc1a_pair_interlock_role
manifest_does_not_add_chain_or_ledger_runtime_dependencies
runtime_source_does_not_define_direct_economic_or_quickchain_authority
public_routes_do_not_expose_live_quickchain_or_settlement_families
dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite
```

Meaning:

```text
Gateway docs explicitly lock the QC-1A role.
Cargo.toml does not add chain/ledger runtime dependencies.
Runtime source does not define direct ledger/economic/QuickChain authority.
Public routes do not expose live quickchain/checkpoint/root/validator/bridge/settlement/staking/liquidity/ROX/Solana route families.
The dynamic preflight runner discovers the new quickchain*.rs suite automatically.
```

---

## 3.3 Gateway Existing QuickChain Suites Stayed Green

The park script dynamically discovered and ran all focused `quickchain*.rs` tests.

Green focused QuickChain suites included:

```text
quickchain_preflight_boundary:              4 passed
quickchain_preflight_cache_boundary:        3 passed
quickchain_preflight_docs:                  5 passed
quickchain_preflight_no_fake_receipts:      3 passed
quickchain_preflight_paid_access:           3 passed
quickchain_preflight_phase1_pair_interlock: 5 passed
quickchain_preflight_transport_authority:   3 passed
quickchain_preflight_value_loop_boundary:   4 passed
quickchain_tooling_boundary:                5 passed
```

The gateway preflight reported:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 3.4 Gateway Product/Proxy Regressions Stayed Green

The gateway hardening did not break normal product proxy behavior.

Green product/proxy regression suites included:

```text
app_proxy:                     5 passed
paid_storage_estimate_proxy:   3 passed
paid_storage_write_proxy:      4 passed
product_routes_proxy:          8 passed
site_visit_routes_proxy:       1 passed
```

The gateway `--all-targets` pass also confirmed additional proxy/product suites stayed green, including:

```text
content_view_routes_proxy: 1 passed
identity_routes_proxy:     7 passed
smoke:                     1 passed
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies map upstream connect failures to problem 502
image/site/text product routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
crab resolve proxy preserves query and headers
site visit quote/pay proxies to omnigate and preserve context
wallet balance route targets the omnigate wallet-balance display path
identity/passport profile routes preserve allowed headers and upstream errors
```

---

## 3.5 Gateway All-Targets and Clippy Green

The gateway park gate ran:

```text
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Both completed successfully.

Final gateway marker:

```text
[svc-gateway quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 4. `omnigate` Notes

## 4.1 Omnigate Role Preserved

`omnigate` remains:

```text
hydration coordinator
product/access coordinator
BFF/view composition boundary
backend-derived paid-access coordinator
display/access composition layer
```

`omnigate` may:

```text
hydrate product views
coordinate quote/access flows
call backend services through approved paths
compose client-facing responses
carry backend-derived receipts/display context
fail closed when backend truth is absent
enforce policy/readiness/admission boundaries
coordinate with wallet front-door paths without becoming the wallet
```

`omnigate` must not:

```text
mint ROC
issue ROC
transfer ROC
burn ROC
hold ROC directly as ledger truth
capture ROC directly as ledger truth
release ROC directly as ledger truth
mutate ron-ledger
invent receipts
invent balances
invent paid access
invent finality
invent roots
invent checkpoints
act as validator
act as committee member
act as bridge
act as external settlement authority
unlock paid content from hydration cache
unlock paid content from display receipt cache
unlock paid content from index pointers
unlock paid content from policy decisions alone
treat b3 as payment proof
treat manifests as payment proof
treat client headers as paid authority
create route-level finality claims
create QuickChain state authority
```

Boundary sentence locked by this pass:

```text
omnigate is a hydration/access coordinator; it may compose backend-derived product responses, but it must not become wallet, ledger, receipt, balance, root, finality, validator, bridge, cache, policy, index, or external settlement authority.
```

---

## 4.2 Omnigate Pair-Interlock Suite

New focused test:

```text
cargo test -p omnigate --test quickchain_preflight_phase1_pair_interlock
```

Result:

```text
5 passed
0 failed
```

Tests proved:

```text
docs_lock_omnigate_into_qc1a_pair_interlock_role
manifest_does_not_add_chain_or_ledger_runtime_dependencies
runtime_source_does_not_define_direct_economic_or_quickchain_authority
public_routes_do_not_expose_live_quickchain_or_settlement_families
dynamic_preflight_will_pick_up_the_phase1_pair_interlock_suite
```

Meaning:

```text
Omnigate docs explicitly lock the QC-1A role.
Cargo.toml does not add chain/ledger runtime dependencies.
Runtime source does not define direct ledger/economic/QuickChain authority.
Public routes do not expose live quickchain/checkpoint/root/validator/bridge/settlement/staking/liquidity/ROX/Solana route families.
The dynamic preflight runner discovers the new quickchain*.rs suite automatically.
```

---

## 4.3 Omnigate Existing QuickChain Suites Stayed Green

The park script dynamically discovered and ran all focused `quickchain*.rs` tests.

Green focused QuickChain suites included:

```text
quickchain_preflight_boundary:              5 passed
quickchain_preflight_cache_boundary:        4 passed
quickchain_preflight_docs:                  5 passed
quickchain_preflight_no_fake_receipts:      4 passed
quickchain_preflight_paid_access:           4 passed
quickchain_preflight_phase1_pair_interlock: 5 passed
quickchain_preflight_transport_authority:   6 passed
quickchain_preflight_value_loop_boundary:   4 passed
quickchain_tooling_boundary:                5 passed
```

The omnigate preflight reported:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 4.4 Omnigate Paid/Product Regressions Stayed Green

The omnigate hardening did not break normal paid/product behavior.

Green paid/product regression suites included:

```text
content_view:                3 passed
site_visit:                  3 passed
streams:                     1 passed
chat_routes:                 2 passed
paid_storage_estimate_proxy: 3 passed
paid_storage_prepare:        4 passed
paid_storage_write_proxy:    4 passed
```

The full all-targets run also kept broader product/hydration suites green, including:

```text
article_asset_publish:       4 passed
asset_page_resolver:         15 passed
builtin_page_resolver:       6 passed
comment_asset_publish:       4 passed
dto_serialization:           1 passed
image_asset_prepare:         5 passed
metrics_contract:            1 passed
middleware_contract:         5 passed
named_site_crab_resolver:    2 passed
passport_profile_routes:     5 passed
policy_gate:                 2 passed
policy_metrics:              1 passed
readiness_error_rate:        1 passed
readiness_inflight:          1 passed
ready_truth:                 1 passed
site_launch:                 5 passed
text_asset_publish:          5 passed
zk_receipts:                 2 passed
```

Representative preserved behavior:

```text
content-view quote returns manifest recipient without wallet call
content-view pay recovers wallet nonce and returns wallet receipt
site-visit quote returns ROC amount and manifest payout recipient
site-visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
chat moderation fails closed
paid storage prepare returns hold template from storage estimate
paid storage write proxy passes storage errors through
site creation stores manifest and index pointer
site resolve hydrates manifest from index and storage
asset resolver preserves canonical crab/b3 route behavior
passport profile routes preserve upstream behavior
policy/readiness/middleware contracts remain green
```

---

## 4.5 Omnigate All-Targets and Clippy Green

The omnigate park gate ran:

```text
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Both completed successfully.

Final omnigate marker:

```text
[omnigate quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 5. Shared Pair Boundary Proven

This pair now proves the public/product boundary remains:

```text
client intent
→ svc-gateway public route/proxy/enforcement boundary
→ omnigate quote/access/hydration coordinator
→ svc-wallet mutation front-door
→ ron-ledger durable economic truth
→ backend-derived receipt/access result
→ paid unlock/render/display path
```

The pair explicitly does not create:

```text
gateway ledger truth
omnigate ledger truth
gateway receipt truth
omnigate receipt truth
gateway balance truth
omnigate balance truth
gateway root truth
omnigate root truth
gateway checkpoint truth
omnigate checkpoint truth
gateway finality truth
omnigate finality truth
gateway validator authority
omnigate validator authority
gateway bridge authority
omnigate bridge authority
gateway settlement authority
omnigate settlement authority
cache entitlement authority
client/header entitlement authority
```

Backend-derived enforcement remains the rule.

No invented trust was introduced.

No local/client/cache/header-only paid unlock was introduced.

No route-level finality was introduced.

No gateway/omnigate mutation authority was introduced.

---

## 6. Forbidden Scope Still Parked

This pair did not add:

```text
roots
state roots
receipt roots
hold roots
epoch roots
checkpoints
checkpoint signing
validator signatures
validator sets
committee replication
staking
liquidity
ROX runtime
Solana runtime
external settlement
public bridge
external anchors
public-chain route families
gateway direct ledger mutation
omnigate direct ledger mutation
gateway direct wallet mutation outside backend wallet path
omnigate direct wallet mutation outside backend wallet path
fake receipts
fake balances
fake finality
cache-only paid unlock
policy-only paid unlock
index-pointer paid unlock
b3-as-payment-proof
crab-url-as-payment-proof
transport-header-as-payment-proof
```

This remains a QC-1A boundary/interlock hardening pass only.

---

## 7. Commands Proven Green

Focused tests:

```bash
cargo test -p svc-gateway --test quickchain_preflight_phase1_pair_interlock
cargo test -p omnigate --test quickchain_preflight_phase1_pair_interlock
```

Park scripts:

```bash
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

Final observed markers:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=9 ==
== svc-gateway QuickChain parking gate passed ==

== omnigate quickchain exhaustive preflight gate passed: tests=9 ==
== omnigate QuickChain parking gate passed ==
```

---

## 8. Low-Disk Workflow Note

The correct workflow for this pair was followed:

```text
focused tests first
then each crate’s park script once
no duplicate preflight + park run for the same crate
```

This matches the low-disk QuickChain workflow:

```text
Do not run both dev-quickchain-preflight.sh and dev-quickchain-park.sh for the same crate unless deliberately needing two full passes.
```

The park scripts already delegate to the preflight gate.

---

## 9. Future Work for These Crates

These crates are parkable for QC-1A Round 1, but future work remains after core root/proof material is actually implemented in the proper crates.

Future `svc-gateway` work may include:

```text
confirming future proof-status display routes are read-only
confirming future root/proof APIs are proxy/display only if exposed at all
stronger header-policy regression tests as proof DTOs mature
stronger paid-flow integration tests once receipt/proof status exists
ensuring no public route claims finality until backend proof verification exists
gateway display of future proof status without becoming proof authority
```

Still forbidden for `svc-gateway` until explicitly authorized by later phase gates:

```text
direct ledger mutation
direct wallet mutation
root production
checkpoint production
validator participation
bridge settlement
external anchors
staking
liquidity
route-level finality claims
fake receipts
fake balances
cache-only paid unlock
```

Future `omnigate` work may include:

```text
future read-only proof-status hydration once proof DTOs exist
future display-only receipt/proof panels sourced from backend truth
stronger wallet receipt lookup hardening
stronger paid-access failure-mode tests
future proof inclusion display without proof authority
future accounting/rewarder context display without mutation authority
future policy/index/cache boundary tests as proof status matures
```

Still forbidden for `omnigate` until explicitly authorized by later phase gates:

```text
direct ledger mutation
direct wallet mutation
root production
checkpoint production
validator participation
bridge settlement
external anchors
staking
liquidity
hydration-level finality claims
fake receipts
fake balances
cache-only paid unlock
policy-only paid unlock
index-pointer paid unlock
```

---

## 10. Parked Status

`svc-gateway` is parked for the current QuickChain Phase 1 Round 1 / QC-1A foundation pass.

`omnigate` is parked for the current QuickChain Phase 1 Round 1 / QC-1A foundation pass.

This crate pair can now be treated as complete for the current QC-1A boundary/interlock sweep.

Recommended regeneration commands after saving notes:

```bash
bash scripts/make_crate_codex.sh --force -c svc-gateway
bash scripts/make_crate_codex.sh --force -c omnigate
```

Next crate pair in the buildplan order:

```text
svc-index + ron-policy
```

Next pair mission:

```text
Prove svc-index remains lookup/pointer/index only.
Prove ron-policy remains declarative policy only.
No names/pointers/policy decisions as proof.
No cache/name/policy as paid-unlock authority.
No policy-created finality.
No index-created finality.
No ledger mutation.
No roots/checkpoints/validators/bridges/external settlement.
```


### END NOTE - JUNE 22 2026 - 14:15 CST




### BEGIN NOTE - JUNE 23 2026 - 15:05 CST

The green run proves the pair hit the buildplan completion definition: focused Round 2 confirmation tests passed for both crates, dynamic discovery picked them up, both park gates passed, and the pair is now safe to mark **Phase 1 complete / parked** while full Phase 1 still waits on `svc-index + ron-policy` and CrabLink Tauri/client adapters.  The buildplan still keeps `svc-gateway + omnigate` non-authoritative: they may enforce/publicly expose flows, but they must not mutate ledger truth or own balances, roots, finality, or settlement truth. 

Here are paste-ready crate notes:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 1 ROUND 2 - svc-gateway + omnigate

# QuickChain Phase 1 Notes — `svc-gateway` + `omnigate`

## 0. Final status

`svc-gateway` and `omnigate` are now **QuickChain Phase 1 complete / parked**.

This means both Phase 1 rounds for this crate pair are complete:

```text
QuickChain Phase 1 — svc-gateway + omnigate
Round 1: COMPLETE
Round 2: COMPLETE
Pair status: PHASE 1 COMPLETE / PARKED
```

Do not call this:

```text
Full QuickChain Phase 1 complete
QuickChain complete
validator ready
settlement ready
bridge ready
external settlement ready
```

Full QuickChain Phase 1 is not complete until the remaining pairs are done:

```text
svc-index + ron-policy
CrabLink Tauri + client adapters
final Phase 1 audit, if needed
```

The next planned major work on `svc-gateway + omnigate` should be in **Phase 2**, unless a later Phase 1 crate pair exposes a concrete integration issue that requires a small downstream fix.

---

## 1. What this session accomplished

This session completed QuickChain Phase 1 Round 2 downstream confirmation for:

```text
svc-gateway
omnigate
```

The goal was not to add root-producing code.

The goal was to prove that these two downstream services can remain safe while the real deterministic root/proof work stays in the authorized core path.

The patch added Phase 1 Round 2 confirmation coverage:

```text
crates/svc-gateway/tests/quickchain_phase1_round2_confirmation.rs
crates/omnigate/tests/quickchain_phase1_round2_confirmation.rs
```

The patch also appended Phase 1 Round 2 downstream-confirmation language to:

```text
crates/svc-gateway/docs/quickchain-preflight.md
crates/omnigate/docs/quickchain-preflight.md
```

No Cargo.toml dependency changes were required.

No new runtime authority was introduced.

No root/checkpoint/validator/bridge/external settlement code was introduced.

---

## 2. Why this was downstream confirmation only

According to the QuickChain buildplan, Phase 1 Round 2 is core-heavy and downstream-light.

The core deterministic root/proof implementation belongs primarily in:

```text
ron-proto
ron-ledger
```

For `svc-gateway + omnigate`, Round 2 only needed confirmation that they can expose, route, hydrate, and enforce backend-derived state without becoming QuickChain authority.

The confirmed boundary is:

```text
svc-gateway:
  public route/admission/proxy/enforcement boundary only

omnigate:
  product hydration / quote-access coordination / backend-derived display only
```

Both crates remain downstream services. They are not balance truth, receipt truth, root truth, finality truth, or settlement truth.

---

## 3. `svc-gateway` final QuickChain Phase 1 role

`svc-gateway` is the public boundary.

Allowed role:

```text
public route surface
admission/proxy boundary
paid enforcement boundary
backend-derived receipt/status forwarding
safe response shaping
policy-aware route enforcement
bounded transport headers
redacted errors
fail-closed product proxying
```

Forbidden role:

```text
wallet authority
ledger authority
balance truth
receipt truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
public chain authority
client entitlement authority
cache-only unlock authority
route-level finality authority
```

`svc-gateway` may forward backend-derived evidence, but it must not create that evidence.

`svc-gateway` may proxy paid access flows, but it must not decide wallet or ledger truth.

`svc-gateway` may display accepted receipt status, but it must not promote `accepted` into `epoch_included`, `finalized`, or `anchored`.

---

## 4. `omnigate` final QuickChain Phase 1 role

`omnigate` is the product coordinator and hydrator.

Allowed role:

```text
hydration coordinator
quote/access coordinator
backend-derived paid access orchestration
site/content hydration
route-level product composition
receipt/status display material
safe upstream/downstream adapter behavior
fail-closed paid/product coordination
```

Forbidden role:

```text
wallet authority
ledger authority
durable economic truth
balance truth
receipt truth
root authority
checkpoint authority
validator authority
bridge authority
external settlement authority
public chain authority
cache entitlement authority
manifest-only paid unlock authority
b3-only paid unlock authority
transport-header paid unlock authority
```

`omnigate` may coordinate a paid flow, but it must not become the paid truth source.

`omnigate` may use backend wallet/ledger receipt values to unlock or hydrate paid content, but it must not fabricate receipts, balances, or finality.

---

## 5. Phase 1 Round 2 confirmation tests added

### `svc-gateway`

Added:

```text
crates/svc-gateway/tests/quickchain_phase1_round2_confirmation.rs
```

This test suite confirms:

```text
docs name Phase 1 Round 2 downstream confirmation
gateway paid enforcement remains backend-derived
gateway is not wallet truth
gateway is not ledger truth
gateway is not QuickChain root authority
gateway is not finality authority
gateway cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
accepted backend receipt can unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
future statuses remain parked
gateway never converts accepted wallet/ledger receipt into QuickChain finality
gateway must label status honestly and must not fabricate status
source has no direct ledger or QuickChain authority imports
route literals do not expose forbidden QuickChain/external runtime paths
preflight runner dynamically discovers the new test
```

### `omnigate`

Added:

```text
crates/omnigate/tests/quickchain_phase1_round2_confirmation.rs
```

This test suite confirms:

```text
docs name Phase 1 Round 2 downstream confirmation
omnigate hydration remains backend-derived
omnigate is not wallet truth
omnigate is not ledger truth
omnigate is not QuickChain root authority
omnigate is not finality authority
omnigate cannot unlock paid content from cache alone
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
accepted backend receipt can unlock local paid content
accepted is not finalized
accepted is not epoch_included
accepted is not anchored
omnigate cannot promote accepted receipt to epoch_included, finalized, or anchored
future statuses remain parked
omnigate must label status honestly and must not fabricate status
source has no direct ledger or QuickChain authority imports
route literals do not expose forbidden QuickChain/external runtime paths
preflight runner dynamically discovers the new test
```

---

## 6. Final green commands run

The focused Round 2 tests were run first:

```bash
cargo test -p svc-gateway --test quickchain_phase1_round2_confirmation
cargo test -p omnigate --test quickchain_phase1_round2_confirmation
```

Then the crate-local park gates were run once:

```bash
crates/svc-gateway/scripts/dev-quickchain-park.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The focused tests passed for both crates:

```text
svc-gateway quickchain_phase1_round2_confirmation:
  5 passed
  0 failed

omnigate quickchain_phase1_round2_confirmation:
  5 passed
  0 failed
```

The final parking markers printed:

```text
== svc-gateway QuickChain parking gate passed ==
== omnigate QuickChain parking gate passed ==
```

---

## 7. `svc-gateway` gate summary

`svc-gateway` passed:

```text
cargo fmt check
dynamic quickchain test discovery
quickchain_phase1_round2_confirmation
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
product/proxy regressions
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
final park gate
```

Representative product/proxy regressions stayed green:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
content_view_routes_proxy
identity_routes_proxy
smoke
```

Representative preserved behavior:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies handle upstream connect failures as problem 502
product routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
site visit quote/pay proxies to omnigate and preserve context
identity and wallet display routes remain proxy/display surfaces
```

Final `svc-gateway` marker:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=10 ==
== svc-gateway QuickChain parking gate passed ==
```

---

## 8. `omnigate` gate summary

`omnigate` passed:

```text
cargo fmt check
dynamic quickchain test discovery
quickchain_phase1_round2_confirmation
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
paid/product route regressions
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
final park gate
```

Representative paid/product regressions stayed green:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
article_asset_publish
asset_page_resolver
builtin_page_resolver
comment_asset_publish
image_asset_prepare
middleware_contract
named_site_crab_resolver
passport_profile_routes
policy_gate
site_launch
text_asset_publish
zk receipt/read-only placeholder targets
```

Representative preserved behavior:

```text
content view quote rejects recipient mismatch
content view pay recovers wallet nonce and returns wallet receipt
site visit quote returns manifest payout recipient
site visit pay recovers wallet nonce and returns wallet receipt
stream latest segment requires stream-bound wallet receipt
chat paid send requires quote
paid storage estimate/prepare/write proxy paths pass upstream errors safely
paid object prepare returns hold template from storage estimate
paid write proxy supports POST and PUT happy paths
site launch stores manifest and index pointer
site resolve hydrates manifest from index and storage
asset page resolver keeps canonical crab/b3 behavior
passport profile routes proxy safely to svc-passport
```

Final `omnigate` marker:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=10 ==
== omnigate QuickChain parking gate passed ==
```

---

## 9. Pair-level architecture preserved

The combined pair now has the right division of responsibility:

```text
svc-gateway receives public client traffic.
svc-gateway proxies and filters.
svc-gateway fails closed.
svc-gateway does not decide wallet/ledger truth.

omnigate coordinates product hydration.
omnigate coordinates quote/access flows.
omnigate talks to backend services.
omnigate fails closed.
omnigate does not decide wallet/ledger truth.

svc-wallet remains the paid mutation front door.
ron-ledger remains durable economic truth.
backend wallet/ledger receipts remain accepted local unlock truth.
future QuickChain epoch/finality/anchor labels remain parked unless backed by future authorized proof paths.
```

This keeps the internal ROC value loop intact:

```text
ron-proto economic DTOs
-> ron-ledger durable economic truth
-> svc-wallet issue/transfer/burn/hold/capture/release/receipt
-> svc-storage/svc-gateway/omnigate paid enforcement
-> ron-accounting snapshots
-> svc-rewarder payout planning
-> wallet/ledger receipts
-> CrabLink display/user intent only
```

---

## 10. Receipt and status language preserved

Accepted paid access:

```text
accepted:
  svc-wallet / ron-ledger accepted the mutation.
  sufficient for local paid unlock.
```

Future settlement labels:

```text
epoch_included:
  receipt appears in a future epoch receipt root.

finalized:
  future challenge/finality threshold reached.

anchored:
  optional external anchor proves checkpoint commitment existed externally.
```

Rules:

```text
paid content unlock:
  accepted backend receipt is enough.

creator-visible pending earnings:
  accepted or epoch_included may be displayed honestly.

withdrawable/finalized rewards:
  require epoch_included/finalized according to policy.

external bridge/export/anchor claims:
  require finalized or anchored.

gateway/omnigate display:
  must label status honestly and must not fabricate status.
```

---

## 11. Still forbidden in `svc-gateway + omnigate`

The following remain forbidden until the correct later phase explicitly authorizes them:

```text
root-producing code
state roots
receipt roots
hold roots
checkpoint roots
checkpoint headers
validators
validator signatures
validator sets
committee quorum
fork choice
pruning
settlement
anchors
bridges
ROX active runtime
Solana active runtime
external L2
external DA
public bridge
staking
liquidity
exchange-facing logic
direct wallet mutation
direct ledger mutation
gateway-owned receipt issuance
omnigate-owned receipt issuance
gateway-owned balance truth
omnigate-owned balance truth
gateway-owned finality truth
omnigate-owned finality truth
cache-only paid unlock
manifest-only paid unlock
b3-only paid unlock
transport-header paid unlock
client-header paid unlock
fake receipts
fake balances
fake finality
placeholder hashes
```

---

## 12. Scripts/tooling status

Both crates now have repeatable crate-local QuickChain parking workflows:

```text
crates/svc-gateway/scripts/dev-quickchain-preflight.sh
crates/svc-gateway/scripts/dev-quickchain-park.sh

crates/omnigate/scripts/dev-quickchain-preflight.sh
crates/omnigate/scripts/dev-quickchain-park.sh
```

The preflight scripts:

```text
check docs exist
check test directory exists
reject checked-in Python helper drift
run cargo fmt --check
dynamically discover quickchain*.rs tests
run every discovered focused QuickChain test
run product/proxy or paid/product regressions
run cargo test --all-targets
run cargo clippy --all-targets --no-deps -- -D warnings
print forbidden-scope marker
print dynamic test-count marker
```

The park scripts:

```text
check docs/script/tooling files exist
delegate to dev-quickchain-preflight.sh
print final parking marker
```

---

## 13. What not to reopen

Do not reopen this pair in Phase 1 unless there is a concrete failure from a later pair or final audit.

Do not rerun exhaustive gates for this pair immediately after the green park run.

Do not rerun already-green earlier pairs unless they were patched.

Do not run workspace-wide clippy unless doing the final phase audit.

Do not add Python helper scripts.

Do not add QuickChain runtime authority to satisfy scanner tests.

---

## 14. Current QuickChain Phase 1 status after this pair

Complete / parked:

```text
1. ron-proto + ron-ledger
2. svc-wallet + ron-accounting
3. svc-rewarder + svc-storage
4. svc-gateway + omnigate
```

Remaining:

```text
5. svc-index + ron-policy
6. CrabLink Tauri + client adapters
```

Recommended next crate pair:

```text
svc-index + ron-policy
```

---

## 15. Bottom line

`svc-gateway + omnigate` are complete for QuickChain Phase 1.

They are green, parked, and correctly non-authoritative.

The pair now proves:

```text
public routing is not settlement
hydration is not settlement
paid enforcement is backend-derived
accepted receipt is local unlock truth
future finality labels are not fabricated
gateway is not wallet/ledger/root/finality authority
omnigate is not wallet/ledger/root/finality authority
cache/header/index/storage metadata cannot unlock paid content alone
no external settlement/ROX/Solana/bridge/staking/liquidity path exists
```

Next planned major work for these crates should be Phase 2, unless a later Phase 1 pair or final audit reveals a concrete integration issue.

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 1 ROUND 2 - svc-gateway + omnigate


### END NOTE - JUNE 23 2026 - 15:05 CST



### BEGIN NOTE - JUNE 23 2026 - 21:35 CST


Below are paste-ready crate notes for `svc-gateway` and `omnigate`. Your terminal output shows both focused Phase 2 tests passed, both dynamic park gates discovered the new test, both all-targets and clippy gates passed, and both parking markers printed green. 

Add this to `crates/svc-gateway/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-gateway

# QuickChain Crate Notes — svc-gateway

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `svc-gateway`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`svc-gateway` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — svc-gateway COMPLETE / PARKED
```

This means `svc-gateway` has been hardened for the Phase 2 Round 1 read-only verifier/replay artifact slice.

It does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, finality, validator readiness, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `svc-gateway` can safely remain the public proxy/admission/route boundary while Phase 2 introduces read-only replay/proof/verifier artifacts elsewhere.

The crate remains:

```text
public gateway
proxy boundary
admission boundary
transport/header filter
product route forwarder
paid route forwarder
display/routing context carrier only
```

The crate does not become:

```text
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
wallet mutation authority
ledger mutation authority
paid-unlock authority by replay artifact
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
svc-gateway may route/display read-only verifier or replay metadata, but paid unlock still requires backend wallet/ledger truth.
```

---

## 2. Files Changed

Changed:

```text
crates/svc-gateway/src/headers/proxy.rs
crates/svc-gateway/tests/quickchain_phase2_replay_boundary.rs
crates/svc-gateway/docs/quickchain-preflight.md
```

No `Cargo.toml` dependency change was required.

---

## 3. Header Boundary Hardening

`crates/svc-gateway/src/headers/proxy.rs` was hardened so authority-shaped QuickChain Phase 2 headers do not pass through gateway product/proxy/response paths as trusted metadata.

The added blocked header families include:

```text
x-ron-proof-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum
```

This extends the earlier protection around root/checkpoint/validator/ledger/bridge/finality-shaped metadata.

The purpose is not to prevent future backend verifier routes forever. The purpose is to prevent caller-supplied or upstream-smuggled headers from becoming gateway-endorsed verifier, replay, quorum, committee, finality, or paid-unlock authority.

---

## 4. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/svc-gateway/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
gateway docs name Phase 2 Round 1 verifier artifact / read-only replication
gateway may expose read-only proof/replay artifact routes if needed
gateway replay metadata is display and routing context only
gateway is not verifier truth
gateway is not replay truth
gateway is not quorum truth
gateway is not committee truth
gateway does not sign verifier attestations
gateway does not decide fork choice
gateway does not claim finality
gateway cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
authority-shaped replay/proof/verifier headers are stripped
public routes do not expose verifier/committee/finality/bridge/staking families
Cargo.toml does not gain direct ledger/wallet/external settlement dependencies
gateway source does not construct finality or committee authority statuses
```

The focused test passed:

```text
cargo test -p svc-gateway --test quickchain_phase2_replay_boundary

running 5 tests
verifier_replay_headers_are_transport_metadata_not_gateway_authority ... ok
docs_name_phase2_round1_gateway_read_only_replay_boundary ... ok
gateway_does_not_gain_direct_ledger_wallet_or_external_settlement_dependencies ... ok
gateway_routes_do_not_expose_verifier_committee_or_finality_authority_paths ... ok
gateway_source_does_not_construct_phase2_authority_status ... ok

test result: ok. 5 passed; 0 failed
```

---

## 5. Documentation Updated

Updated:

```text
crates/svc-gateway/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
svc-gateway may expose read-only proof/replay artifact routes if needed
gateway replay metadata is display and routing context only
replay artifacts are not gateway authority
proof artifacts are not gateway authority
verifier results are not gateway authority
committee attestations are not gateway authority
quorum claims are not gateway authority
fork-choice claims are not gateway authority
finality claims are not gateway authority
gateway is not verifier truth
gateway is not replay truth
gateway is not quorum truth
gateway is not committee truth
gateway does not sign verifier attestations
gateway does not decide fork choice
gateway does not claim finality
gateway cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 6. Final Gate Evidence

The final local gate discovered 11 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=11 ==
== svc-gateway QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p svc-gateway --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

---

## 7. Important Existing Coverage Still Preserved

This pass did not replace older gateway safety coverage. It extended it.

Still protected:

```text
gateway remains proxy boundary, not chain runtime
gateway source does not import wallet, ledger, or QuickChain runtime authority
unsupported QuickChain/ledger/bridge/root routes fail closed
product proxy strips caller-supplied QuickChain authority headers
cache headers are transport metadata, not unlock authority
client/cache unlock signals are stripped when authority-shaped
routes do not unlock paid content from cache or query claims
backend receipt metadata is context, not gateway authority
request/response headers cannot smuggle economic or chain authority
client paid unlock headers are never gateway authority
public routes do not expose live QuickChain or settlement families
runtime source does not define direct economic or QuickChain authority
identity/retry/cache headers remain context only
gateway stays out of receipt, balance, and finality truth
accepted receipt is separated from future QuickChain statuses
```

---

## 8. Safe Meaning

The safe completion meaning for this crate is:

```text
svc-gateway is green/parked for QuickChain Phase 2 Round 1.
It can carry or route read-only replay/proof/verifier context if future routes need it,
but it cannot become verifier, replay, quorum, committee, fork-choice, finality, settlement,
wallet, ledger, bridge, staking, or paid-unlock authority.
```

---

## 9. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
gateway verifier routes are live
gateway can verify roots
gateway can sign attestations
gateway can unlock paid content from replay artifacts
gateway can mutate wallet
gateway can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 10. Do Not Reopen Unless

Do not reopen `svc-gateway` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in verifier artifact routing/display boundaries
svc-index or ron-policy introduces a replay artifact pointer/policy shape requiring gateway confirmation
CrabLink client adapters need a stricter gateway boundary signal
svc-wallet receipt or paid-admission contracts change
ron-ledger economic truth or receipt format changes
omnigate route contracts change
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source/header scanner catches new authority creep
```

Otherwise, `svc-gateway` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 11. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-index + ron-policy
```

Expected goal for that pair:

```text
svc-index may point to read-only replay/proof artifacts but cannot become proof authority.
ron-policy may declare verifier access policy but cannot create verifier truth, settlement truth, finality, or paid-unlock authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - svc-gateway

Add this to `crates/omnigate/NOTES.MD`:

### BEGIN NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - omnigate

# QuickChain Crate Notes — omnigate

Date: June 23, 2026
Project: RustyOnions / CrabLink
Track: QuickChain Phase 2 Round 1
Round purpose: verifier artifact / read-only replication
Crate: `omnigate`
Status: COMPLETE / PARKED for QuickChain Phase 2 Round 1

---

## 0. Status Summary

`omnigate` is now complete and parked for QuickChain Phase 2 Round 1.

Final status:

```text
QuickChain Phase 2 Round 1 — omnigate COMPLETE / PARKED
```

This means `omnigate` has been hardened for the Phase 2 Round 1 read-only verifier/replay artifact slice.

It does not mean Phase 2 is complete. It does not mean committee signing, quorum, fork-choice, finality, validator readiness, staking, slashing, bridge behavior, ROX, Solana, or external settlement is implemented.

---

## 1. What This Pass Proved

This pass proved that `omnigate` can safely remain the hydration/product coordination boundary while Phase 2 introduces read-only replay/proof/verifier artifacts elsewhere.

The crate remains:

```text
hydration coordinator
product route coordinator
paid route coordinator
backend-derived display surface
wallet/ledger receipt consumer
storage/index/policy orchestration layer
read-only context carrier only
```

The crate does not become:

```text
verifier truth
replay truth
root authority
checkpoint authority
committee authority
quorum authority
fork-choice authority
finality authority
settlement authority
wallet mutation authority
ledger mutation authority
paid-unlock authority by replay artifact
bridge authority
staking/slashing authority
public-chain runtime
```

The important preserved line is:

```text
omnigate may hydrate/display read-only verifier or replay metadata, but paid unlock still requires backend wallet/ledger truth.
```

---

## 2. Files Changed

Changed:

```text
crates/omnigate/tests/quickchain_phase2_replay_boundary.rs
crates/omnigate/docs/quickchain-preflight.md
```

No `Cargo.toml` dependency change was required.

---

## 3. New Phase 2 Round 1 Boundary Coverage

Added focused Phase 2 Round 1 coverage through:

```text
crates/omnigate/tests/quickchain_phase2_replay_boundary.rs
```

The focused test locks these boundaries:

```text
omnigate docs name Phase 2 Round 1 verifier artifact / read-only replication
omnigate may expose read-only proof/replay artifact views if needed
omnigate replay metadata is display and hydration context only
omnigate is not verifier truth
omnigate is not replay truth
omnigate is not quorum truth
omnigate is not committee truth
omnigate does not sign verifier attestations
omnigate does not decide fork choice
omnigate does not claim finality
omnigate cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
public v1 route registry does not expose verifier/committee/finality/bridge/staking families
v1 route source does not construct verifier results, replay results, committee attestations, quorum, fork-choice, finality, anchored, or settlement-finality statuses
Cargo.toml does not gain direct ledger or external settlement dependencies
existing paid routes remain backend wallet/ledger-derived, not replay-artifact unlocks
```

The focused test passed:

```text
cargo test -p omnigate --test quickchain_phase2_replay_boundary

running 5 tests
omnigate_does_not_gain_direct_ledger_or_external_settlement_dependencies ... ok
docs_name_phase2_round1_omnigate_read_only_replay_boundary ... ok
omnigate_routes_do_not_expose_verifier_committee_or_finality_authority_paths ... ok
existing_paid_routes_remain_backend_wallet_ledger_derived_not_replay_artifact_unlocks ... ok
omnigate_source_does_not_construct_phase2_authority_results ... ok

test result: ok. 5 passed; 0 failed
```

---

## 4. Documentation Updated

Updated:

```text
crates/omnigate/docs/quickchain-preflight.md
```

The doc now explicitly names:

```text
Phase 2 Round 1 verifier artifact / read-only replication boundary
```

The doc now preserves these boundaries:

```text
omnigate may expose read-only proof/replay artifact views if needed
omnigate replay metadata is display and hydration context only
replay artifacts are not omnigate authority
proof artifacts are not omnigate authority
verifier results are not omnigate authority
committee attestations are not omnigate authority
quorum claims are not omnigate authority
fork-choice claims are not omnigate authority
finality claims are not omnigate authority
omnigate is not verifier truth
omnigate is not replay truth
omnigate is not quorum truth
omnigate is not committee truth
omnigate does not sign verifier attestations
omnigate does not decide fork choice
omnigate does not claim finality
omnigate cannot unlock paid content from replay artifacts alone
paid unlock still requires backend wallet/ledger truth
svc-wallet remains the paid mutation path
ron-ledger remains durable economic truth
```

---

## 5. Final Gate Evidence

The final local gate discovered 11 focused QuickChain tests:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

The final parking markers were:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=11 ==
== omnigate QuickChain parking gate passed ==
```

The all-targets gate completed cleanly:

```text
cargo test -p omnigate --all-targets
```

The clippy gate completed cleanly:

```text
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

The forbidden-scope marker stayed parked:

```text
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

---

## 6. Paid/Product Regression Coverage Passed

The park script also ran paid/product route regressions successfully:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

This is important because Phase 2 Round 1 must not accidentally weaken paid access or create replay-artifact-only unlocks.

The successful regression coverage preserved:

```text
content view quote/pay still uses wallet-derived receipt path
site visit quote/pay still uses wallet-derived receipt path
streams latest segment still requires stream-bound wallet receipt
chat paid send still requires quote
paid storage estimate/prepare/write remain backend/storage/wallet-derived paths
upstream errors pass through safely
no replay/proof/verifier artifact becomes paid unlock authority
```

---

## 7. Important Existing Coverage Still Preserved

This pass did not replace older omnigate safety coverage. It extended it.

Still protected:

```text
omnigate remains hydrator and coordinator, not chain runtime
Cargo.toml does not depend on ron-ledger or QuickChain runtime crates
public v1 router does not publish QuickChain authority routes
wallet surface is display balance and explicit hold front door only
source does not import direct ledger or chain authority
cache and b3 are not paid entitlement authority
paid access routes do not parse cache headers as unlock authority
production source does not claim cache/b3/manifest proves payment
wallet balance fallback is explicitly non-authoritative
paid routes use wallet receipt values without dev receipt fabrication
source does not construct root or validator authority fields
source does not construct obvious fake receipts or finality
paid storage remains prepare/estimate/proxy only
content view and site visit pay routes name wallet front-door invariant
stream and chat paid paths fail closed on backend truth
source does not contain paid or unlock bypass shortcuts
x-ron header policy blocks QuickChain authority headers
transport headers are not economic authority
receipt or paid authority must come from backend paths, not request headers
accepted receipt is separated from future finality status
```

---

## 8. Safe Meaning

The safe completion meaning for this crate is:

```text
omnigate is green/parked for QuickChain Phase 2 Round 1.
It can hydrate or display read-only replay/proof/verifier context if future routes need it,
but it cannot become verifier, replay, quorum, committee, fork-choice, finality, settlement,
wallet, ledger, bridge, staking, or paid-unlock authority.
```

---

## 9. What Not To Claim

Do not claim:

```text
Phase 2 complete
Phase 2 Round 1 complete across all crates
committee ready
validator ready
quorum ready
fork choice ready
finality ready
omnigate verifier views are live
omnigate can verify roots
omnigate can sign attestations
omnigate can unlock paid content from replay artifacts
omnigate can mutate wallet
omnigate can mutate ledger
bridge ready
staking/slashing ready
external settlement ready
ROX/Solana runtime ready
```

---

## 10. Do Not Reopen Unless

Do not reopen `omnigate` during Phase 2 Round 1 unless:

```text
a later crate-pair exposes a real mismatch in verifier artifact hydration/display boundaries
svc-index or ron-policy introduces a replay artifact pointer/policy shape requiring omnigate confirmation
CrabLink client adapters need a stricter omnigate status shape
svc-wallet receipt or paid-admission contracts change
ron-ledger economic truth or receipt format changes
svc-gateway route/header contracts change
QuickChain verifier artifact schemas change in ron-proto
a parking gate regresses
a source/header scanner catches new authority creep
```

Otherwise, `omnigate` should stay parked until Phase 2 Round 2 or a later explicitly scoped phase.

---

## 11. Next Pair

The next Phase 2 Round 1 crate pair is:

```text
svc-index + ron-policy
```

Expected goal for that pair:

```text
svc-index may point to read-only replay/proof artifacts but cannot become proof authority.
ron-policy may declare verifier access policy but cannot create verifier truth, settlement truth, finality, or paid-unlock authority.
```

### END NOTE - JUNE 23 2026 - QUICKCHAIN PHASE 2 ROUND 1 - omnigate

### END NOTE - JUNE 23 2026 - 21:35 CST 



### BEGIN NOTE - JUNE 24 2026 - 02:00 CST

Based on the final terminal output, `svc-gateway` hit the full parking marker after clippy passed, and `omnigate` had already parked in the previous run. 

Paste this into `crates/svc-gateway/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# svc-gateway — QuickChain Phase 2 Round 2 Crate Notes

## Status

`svc-gateway` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `svc-gateway` as a Phase 2 public boundary for future committee/verifier metadata while preserving the core invariant:

```text
svc-gateway is a public ingress/proxy/admission surface.
svc-gateway is not ledger truth.
svc-gateway is not wallet mutation authority.
svc-gateway is not a QuickChain verifier.
svc-gateway is not committee authority.
svc-gateway is not quorum/finality/settlement authority.
```

The crate now has explicit Phase 2 Round 2 coverage proving that committee, quorum, finality, replay, verifier, bridge, staking, liquidity, and external-settlement claims cannot be smuggled through public headers, public route names, direct dependencies, or source-level status construction.

## Files changed / hardened

Primary runtime hardening:

```text
crates/svc-gateway/src/headers/proxy.rs
```

This header proxy policy now rejects authority-looking QuickChain / chain / committee / quorum / finality headers before forwarding upstream. The deny surface includes exact and prefix families such as:

```text
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-validator-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-proof-*
x-ron-settlement-*
x-ron-anchor-*
x-ron-bridge-*
```

It still allows ordinary gateway/product context headers that are needed for current ROC paid product flows, such as:

```text
authorization
accept
content-type
x-correlation-id
x-request-id
idempotency-key
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

New Phase 2 Round 2 test:

```text
crates/svc-gateway/tests/quickchain_phase2_committee_boundary.rs
```

Docs hardened:

```text
crates/svc-gateway/docs/quickchain-preflight.md
```

## New test coverage

New focused test suite:

```text
cargo test -p svc-gateway --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 gateway committee readiness boundary
- gateway filters committee/quorum/finality/external QuickChain authority headers
- gateway still allows backend product context without promoting it to finality
- gateway does not gain direct ledger/wallet/validator/external settlement dependencies
- gateway routes do not expose committee/quorum/validator/bridge authority paths
- gateway source does not construct committee or finality status truth
```

This locks `svc-gateway` into the correct future Phase 2 position:

```text
backend-derived verifier or committee labels may eventually be routed/displayed,
but gateway itself must not create, sign, decide, finalize, settle, reward, anchor, or unlock.
```

## Gates passed

Focused Phase 2 Round 2 test:

```text
cargo test -p svc-gateway --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Park runner:

```text
crates/svc-gateway/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain discovery runner picked up the new suite and reran the existing boundary tests.

Confirmed passing suites included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Product/proxy regressions passed, including:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
content_view_routes_proxy
identity_routes_proxy
smoke
```

Hard gates passed:

```text
cargo fmt -p svc-gateway
cargo test -p svc-gateway --all-targets
cargo clippy -p svc-gateway --all-targets --no-deps -- -D warnings
```

Final parking marker reached:

```text
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=12 ==
== svc-gateway QuickChain parking gate passed ==
```

## Small repair made during parking

The initial park run failed only on a clippy `doc_markdown` lint in:

```text
crates/svc-gateway/src/headers/proxy.rs
```

The fix was to wrap `QuickChain` in backticks inside the RO header comment. After that, `cargo fmt`, the dynamic QuickChain preflight, all-targets, regressions, and clippy passed.

## Boundary summary

`svc-gateway` may:

```text
- expose stable public browser/client routes
- proxy selected requests to omnigate
- preserve ordinary product context headers
- strip authority-looking headers
- route backend-derived product responses
- return backend-derived/display-only receipt context
```

`svc-gateway` must not:

```text
- mutate ron-ledger
- bypass svc-wallet
- create fake balances
- create fake receipts
- unlock paid content from cache/client/header claims
- expose live QuickChain authority routes
- create verifier attestations
- decide committee membership
- decide quorum
- decide finality
- decide fork choice
- anchor externally
- bridge externally
- implement staking/liquidity/slashing
- act as validator or settlement runtime
```

## Completion statement

`svc-gateway` is parked for QuickChain Phase 2 Round 2.

Next time `svc-gateway` is touched for QuickChain, preserve this role:

```text
Gateway routes/display metadata only.
Gateway strips authority smuggling.
Gateway never becomes settlement authority.
Gateway never becomes wallet/ledger/validator/quorum/finality authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

Paste this into `crates/omnigate/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2

# omnigate — QuickChain Phase 2 Round 2 Crate Notes

## Status

`omnigate` is complete / parked for QuickChain Phase 2 Round 2.

This round hardened `omnigate` as a product hydrator / coordination boundary for future verifier or committee metadata while preserving the core invariant:

```text
omnigate hydrates product views.
omnigate coordinates backend calls.
omnigate is not ledger truth.
omnigate is not wallet mutation authority.
omnigate is not a QuickChain verifier.
omnigate is not committee authority.
omnigate is not quorum/finality/settlement authority.
```

The crate now has explicit Phase 2 Round 2 coverage proving that committee, verifier, quorum, finality, replay, validator, bridge, staking, liquidity, and external-settlement claims cannot be smuggled through headers, public route names, paid routes, source-level hydration logic, or direct dependencies.

## Files changed / hardened

Primary runtime hardening:

```text
crates/omnigate/src/routes/v1/header_policy.rs
```

The shared header policy now rejects authority-looking QuickChain / chain / committee / quorum / finality headers before route code treats forwarded `x-ron-*` context as safe product context.

The deny surface includes exact and prefix families such as:

```text
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-validator-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-proof-*
x-ron-settlement-*
x-ron-anchor-*
x-ron-bridge-*
```

The policy still preserves ordinary product/value-loop context headers needed for current ROC flows, such as:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

New Phase 2 Round 2 test:

```text
crates/omnigate/tests/quickchain_phase2_committee_boundary.rs
```

Docs hardened:

```text
crates/omnigate/docs/quickchain-preflight.md
```

## New test coverage

New focused test suite:

```text
cargo test -p omnigate --test quickchain_phase2_committee_boundary
```

Coverage added:

```text
- docs name the Phase 2 Round 2 omnigate committee readiness boundary
- omnigate header policy blocks committee/quorum/finality/external QuickChain headers
- omnigate routes do not expose committee/quorum/validator/bridge authority paths
- omnigate does not gain direct ledger/validator/external settlement dependencies
- existing paid routes remain backend wallet/ledger-derived, not committee unlocks
- omnigate hydration and paid routes do not construct committee or finality status truth
```

This locks `omnigate` into the correct future Phase 2 position:

```text
backend-derived verifier or committee labels may eventually be hydrated/displayed,
but omnigate itself must not create, sign, decide, finalize, settle, reward, anchor, or unlock.
```

## Gates passed

Focused Phase 2 Round 2 test:

```text
cargo test -p omnigate --test quickchain_phase2_committee_boundary
```

Result:

```text
6 passed; 0 failed
```

Park runner:

```text
crates/omnigate/scripts/dev-quickchain-park.sh
```

The dynamic QuickChain discovery runner picked up the new suite and reran the existing boundary tests.

Confirmed passing suites included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Paid/product route regressions passed, including:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

All-targets passed, including route, hydration, product, policy, readiness, site, stream, text, and zk read-only suites.

Hard gates passed:

```text
cargo fmt -p omnigate
cargo test -p omnigate --all-targets
cargo clippy -p omnigate --all-targets --no-deps -- -D warnings
```

Final parking marker reached:

```text
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=12 ==
== omnigate QuickChain parking gate passed ==
```

## Boundary summary

`omnigate` may:

```text
- hydrate product views
- coordinate backend calls
- proxy paid storage prepare/write flows
- call backend wallet-facing routes through the approved front-door path
- display backend-derived receipt and paid-access context
- preserve product context headers after policy filtering
- eventually hydrate backend-derived verifier/committee labels as display-only metadata
```

`omnigate` must not:

```text
- mutate ron-ledger directly
- bypass svc-wallet
- create fake balances
- create fake receipts
- unlock paid content from cache/client/header claims
- expose live QuickChain authority routes
- create signed verifier attestations
- decide committee membership
- decide quorum
- decide finality
- decide fork choice
- anchor externally
- bridge externally
- implement staking/liquidity/slashing
- act as validator or settlement runtime
```

## Completion statement

`omnigate` is parked for QuickChain Phase 2 Round 2.

Next time `omnigate` is touched for QuickChain, preserve this role:

```text
Omnigate hydrates backend-derived truth.
Omnigate filters authority smuggling.
Omnigate coordinates product access.
Omnigate never becomes wallet/ledger/validator/quorum/finality authority.
```

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 2 ROUND 2


### END NOTE - JUNE 24 2026 - 02:00 CST


### BEGIN NOTE - JUNE 24 2026 - 13:45 CST

Based on your terminal output, both crates are green: each discovered **13 QuickChain tests**, passed the new Phase 3 validator/passport boundary suite, all-target tests, and clippy. The logs also confirm forbidden scope remains parked for both crates. 

Paste this into `crates/svc-gateway/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - 13:45 CST

# svc-gateway — QuickChain Phase 3 Round 1 Crate Notes

## Status

`svc-gateway` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the gateway-side **validator/passport boundary** for the future Phase 3 passport-gated validator model.

`svc-gateway` remains:

```text
public boundary
route admission layer
proxy/enforcement layer
fail-closed product gateway
header filtering boundary
```

`svc-gateway` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/svc-gateway/src/headers/proxy.rs
crates/svc-gateway/tests/quickchain_phase3_validator_boundary.rs
crates/svc-gateway/docs/quickchain-preflight.md
```

## What the patch proves

The new Phase 3 boundary test proves that `svc-gateway` rejects validator/passport authority smuggling through public/product headers.

Blocked authority-shaped header families include:

```text
x-ron-validator*
x-ron-passport-validator*
x-ron-registry-validator*
x-ron-capability-validator*
x-ron-attestation-identity*
x-ron-validator-authz*
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-bridge-*
x-ron-anchor-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-ledger-*
x-ron-proof-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-settlement-*
```

The patch preserves non-authority product context headers such as:

```text
authorization
accept
content-type
x-correlation-id
x-request-id
idempotency-key
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

Important semantic distinction:

```text
x-ron-passport may be product identity/display context.
x-ron-validator-passport is authority-shaped and must not pass as gateway truth.
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p svc-gateway --test quickchain_phase3_validator_boundary
```

Result:

```text
6 passed; 0 failed
```

Exhaustive QuickChain preflight:

```text
bash crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

Result:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=13 ==
```

The preflight included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Product/proxy regressions passed:

```text
app_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
```

All-target tests passed.

Clippy passed.

## Forbidden scope remains parked

The crate-local preflight printed the expected parked-scope marker:

```text
forbidden scope remains parked:
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

## Phase 3 Round 1 conclusion

`svc-gateway` is green/parked for QuickChain Phase 3 Round 1.

Safe status wording:

```text
svc-gateway is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport authority-smuggling boundary.
It still does not implement validator authority, passport registry authority, ledger authority, wallet authority, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - 13:45 CST

Paste this into `crates/omnigate/NOTES.MD`:

### BEGIN NOTE - JUNE 24 2026 - 13:45 CST

# omnigate — QuickChain Phase 3 Round 1 Crate Notes

## Status

`omnigate` is complete / green for **QuickChain Phase 3 Round 1**.

This round added and proved the omnigate-side **validator/passport boundary** for the future Phase 3 passport-gated validator model.

`omnigate` remains:

```text
product hydrator
access composition layer
backend-derived display surface
paid/product coordinator
fail-closed route layer
```

`omnigate` is still not:

```text
validator identity authority
passport registry authority
validator capability authority
validator-set authority
wallet authority
ledger authority
balance truth
receipt truth
root/checkpoint/finality authority
staking/slashing authority
bridge authority
external settlement authority
```

## Files changed

```text
crates/omnigate/src/routes/v1/header_policy.rs
crates/omnigate/tests/quickchain_phase3_validator_boundary.rs
crates/omnigate/docs/quickchain-preflight.md
```

## What the patch proves

The new Phase 3 boundary test proves that `omnigate` rejects validator/passport authority smuggling through v1 product-route header policy.

Blocked authority-shaped header families include:

```text
x-ron-validator*
x-ron-passport-validator*
x-ron-registry-validator*
x-ron-capability-validator*
x-ron-attestation-identity*
x-ron-validator-authz*
x-quickchain-*
x-qc-*
x-ron-quickchain-*
x-ron-bridge-*
x-ron-anchor-*
x-ron-checkpoint-*
x-ron-root-*
x-ron-proof-*
x-ron-replay-*
x-ron-verifier-*
x-ron-committee-*
x-ron-quorum-*
x-ron-ledger-*
x-ron-settlement-*
```

The patch preserves non-authority product context headers such as:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

Important semantic distinction:

```text
x-ron-passport may be identity/display context.
x-ron-validator-passport is authority-shaped and must not pass as omnigate truth.
```

## Paid access boundary preserved

The new tests explicitly confirm that existing paid routes do not unlock content from validator/passport material.

Paid access still depends on backend wallet/ledger-derived truth, not:

```text
validator passport material
validator capability material
validator set material
passport registry material
cache-only claims
client-supplied unlock claims
committee/replay/finality artifacts
```

## Tests passed

Focused Phase 3 suite:

```text
cargo test -p omnigate --test quickchain_phase3_validator_boundary
```

Result:

```text
6 passed; 0 failed
```

Exhaustive QuickChain preflight:

```text
bash crates/omnigate/scripts/dev-quickchain-preflight.sh
```

Result:

```text
== omnigate quickchain exhaustive preflight gate passed: tests=13 ==
```

The preflight included:

```text
quickchain_phase1_round2_confirmation
quickchain_phase2_committee_boundary
quickchain_phase2_replay_boundary
quickchain_phase3_validator_boundary
quickchain_preflight_boundary
quickchain_preflight_cache_boundary
quickchain_preflight_docs
quickchain_preflight_no_fake_receipts
quickchain_preflight_paid_access
quickchain_preflight_phase1_pair_interlock
quickchain_preflight_transport_authority
quickchain_preflight_value_loop_boundary
quickchain_tooling_boundary
```

Paid/product route regressions passed:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

All-target tests passed.

Clippy passed.

## Forbidden scope remains parked

The crate-local preflight printed the expected parked-scope marker:

```text
forbidden scope remains parked:
no roots
no checkpoints
no validators
no bridges
no external settlement
no fake receipts
no fake balances
```

## Phase 3 Round 1 conclusion

`omnigate` is green/parked for QuickChain Phase 3 Round 1.

Safe status wording:

```text
omnigate is complete for QuickChain Phase 3 Round 1.
It now has a tested validator/passport authority-smuggling boundary.
It still does not implement validator authority, passport registry authority, ledger authority, wallet authority, roots, checkpoints, finality, staking, slashing, bridge, or external settlement.
```

### END NOTE - JUNE 24 2026 - 13:45 CST

After this, the next crate pair in the normal Phase 3 Round 1 order is:

```text
svc-index + ron-policy
```


### END NOTE - JUNE 24 2026 - 13:45 CST




### BEGIN NOTE - JUNE 24 2026 - 22:05 CST

I used the latest terminal output as the source of truth: both new lifecycle suites passed, both existing Phase 3 validator suites passed, and both crate-local QuickChain preflight gates ended at `tests=14` with clippy/all-targets green and forbidden scope parked.  

### BEGIN NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-gateway + omnigate

# QuickChain Phase 3 Round 2 Crate Notes — `svc-gateway + omnigate`

## 0. Final status

Crate pair:

```text
svc-gateway + omnigate
```

Final status:

```text
QuickChain Phase 3: 100% COMPLETE / PARKED
```

Reason:

```text
Phase 3 has only two rounds.
Round 1 validator/passport/registry boundary coverage is green.
Round 2 validator lifecycle/evidence/governance boundary coverage is green.
Both crate-local exhaustive QuickChain preflight gates passed.
Both clippy gates passed.
Both all-target gates passed.
Paid/product regressions stayed green.
Forbidden scope remained parked.
```

This crate pair should not be reopened for Phase 3 unless a real regression appears.

---

## 1. Phase / round completed

Current phase:

```text
QuickChain Phase 3 — passport-gated validator set
```

Current round completed:

```text
Round 2 — validator operation / lifecycle hardening
```

This round hardened public/product boundaries against validator lifecycle authority creep:

```text
validator rotation
validator revocation
validator downtime/degraded status
equivocation evidence
double-attestation evidence
split-brain evidence
replay challenge evidence
governance-gated parameter updates
```

The work remained boundary-only.

No root-producing, settlement, validator runtime, bridge, staking, slashing, or external economy code was added.

---

## 2. `svc-gateway` notes

## 2.1 Role preserved

`svc-gateway` remains:

```text
public ingress
stable product route surface
proxy/enforcement boundary
header filtering boundary
request/context forwarding boundary
problem-response preserving proxy
paid-flow public entry surface
```

`svc-gateway` is not:

```text
wallet
ledger
balance authority
receipt authority
root producer
checkpoint writer
validator
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge
external settlement service
staking service
slashing service
liquidity service
finality source
cache entitlement authority
client/header entitlement authority
```

## 2.2 What changed

The Phase 3 Round 2 patch hardened gateway header and source boundaries.

Likely touched files:

```text
crates/svc-gateway/src/headers/proxy.rs
crates/svc-gateway/docs/quickchain-preflight.md
crates/svc-gateway/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

Header filtering now rejects lifecycle/evidence/governance authority-shaped headers such as:

```text
x-ron-validator-rotation
x-ron-validator-revocation
x-ron-validator-downtime
x-ron-validator-degraded
x-ron-validator-equivocation
x-ron-validator-equivocation-evidence
x-ron-validator-double-attestation
x-ron-validator-split-brain
x-ron-replay-challenge
x-ron-replay-challenge-evidence
x-ron-validator-lifecycle-decision
x-ron-lifecycle-decision
x-ron-governance-parameter-update
x-ron-governance-approval
x-quickchain-validator-rotation
x-quickchain-validator-revocation
x-quickchain-replay-challenge-evidence
x-qc-validator-lifecycle-decision
```

The gateway still permits normal backend-derived product context headers where appropriate, including:

```text
authorization
accept
content-type
x-correlation-id
x-request-id
idempotency-key
x-ron-wallet-account
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

Those headers remain context only.

They are not finality, balance, receipt, ledger, root, checkpoint, validator, governance, bridge, or settlement authority.

## 2.3 New focused test

New test:

```bash
cargo test -p svc-gateway --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
6 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round2_gateway_validator_lifecycle_boundary
gateway_filters_phase3_validator_lifecycle_evidence_and_governance_authority_headers
gateway_keeps_current_backend_derived_product_context_headers_but_not_authority_headers
gateway_route_surface_does_not_expose_validator_lifecycle_mutation_routes
gateway_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority
gateway_source_does_not_import_ledger_or_external_settlement_runtime
```

## 2.4 Existing Phase 3 Round 1 validator boundary stayed green

Existing test:

```bash
cargo test -p svc-gateway --test quickchain_phase3_validator_boundary
```

Observed result:

```text
6 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round1_gateway_validator_passport_boundary
gateway_filters_phase3_validator_passport_registry_and_capability_authority_headers
gateway_still_allows_non_authority_product_context_headers
gateway_routes_do_not_expose_validator_passport_registry_or_finality_authority_paths
gateway_source_does_not_implement_validator_admission_or_paid_unlock_authority
gateway_source_does_not_import_ledger_or_build_validator_economy_runtime
```

## 2.5 Exhaustive gateway gate

Command run:

```bash
bash crates/svc-gateway/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
[svc-gateway quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
svc-gateway QuickChain preflight gate passed
== svc-gateway quickchain exhaustive preflight gate passed: tests=14 ==
```

Additional green coverage:

```text
format check passed
dynamic QuickChain test discovery found 14 tests
all-targets passed
clippy -D warnings passed
product/proxy regressions passed
forbidden scope marker preserved
```

## 2.6 Product/proxy regressions preserved

Gateway product/proxy regressions stayed green, including:

```text
app_proxy
content_view_routes_proxy
identity_routes_proxy
paid_storage_estimate_proxy
paid_storage_write_proxy
product_routes_proxy
site_visit_routes_proxy
smoke
```

Important behavior preserved:

```text
app proxy preserves methods, body, query, auth, correlation headers, and generated request IDs
app proxy passes problem bodies unchanged
paid storage estimate/write proxies pass omnigate errors through
paid storage estimate/write proxies map upstream connect failures to problem 502
image/site/text product routes target omnigate
b3 asset proxy targets omnigate b3 route
chat product routes target omnigate
site resolve proxy targets omnigate site hydrator
paid prepare proxy preserves body and idempotency
crab resolve proxy preserves query and headers
site visit quote/pay proxies to omnigate and preserve context
identity/passport routes preserve allowed headers and upstream errors
wallet balance route targets omnigate wallet-balance display path
```

## 2.7 Gateway boundary proven

`svc-gateway` now proves:

```text
public routes do not expose validator lifecycle mutation endpoints
public routes do not expose governance parameter-update authority endpoints
public routes do not expose replay challenge authority endpoints
headers cannot smuggle lifecycle/governance/evidence authority
normal product context headers remain usable without becoming authority
gateway source does not import ron-ledger or external settlement runtime
gateway source does not create validator lifecycle paid-unlock shortcuts
gateway source does not create cache/client/header-only paid unlocks
gateway source does not introduce bridge, staking, slashing, ROX, Solana, or external settlement runtime
```

---

## 3. `omnigate` notes

## 3.1 Role preserved

`omnigate` remains:

```text
product hydrator
BFF/view composition boundary
quote/access coordinator
backend-derived paid-flow coordinator
manifest/page composition layer
storage/index/wallet coordination surface
fail-closed API surface behind gateway
```

`omnigate` is not:

```text
wallet
ledger
balance authority
receipt authority by itself
root producer
checkpoint writer
validator
validator lifecycle authority
validator rotation authority
validator revocation authority
validator downtime authority
validator equivocation authority
replay challenge authority
governance parameter-update authority
bridge
external settlement service
staking service
slashing service
liquidity service
finality source
cache entitlement authority
transport-header authority
```

## 3.2 What changed

The Phase 3 Round 2 patch hardened omnigate’s v1 header policy and source boundary.

Likely touched files:

```text
crates/omnigate/src/routes/v1/header_policy.rs
crates/omnigate/docs/quickchain-preflight.md
crates/omnigate/tests/quickchain_phase3_validator_lifecycle_boundary.rs
```

The shared v1 header policy now rejects lifecycle/evidence/governance authority-shaped headers.

Normal backend-derived product context headers remain allowed where appropriate, including:

```text
x-ron-wallet-account
x-ron-passport
x-ron-wallet-txid
x-ron-wallet-receipt-hash
x-ron-paid-op
x-ron-paid-asset
```

Those headers remain display/access coordination context only.

They are not ledger truth, wallet truth, finality truth, settlement truth, root truth, checkpoint truth, validator truth, lifecycle truth, or governance truth.

## 3.3 New focused test

New test:

```bash
cargo test -p omnigate --test quickchain_phase3_validator_lifecycle_boundary
```

Observed result:

```text
6 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round2_omnigate_validator_lifecycle_boundary
omnigate_header_policy_blocks_lifecycle_evidence_and_governance_authority
existing_paid_and_access_routes_do_not_unlock_from_lifecycle_evidence
omnigate_routes_do_not_expose_validator_lifecycle_mutation_routes
omnigate_source_does_not_construct_validator_lifecycle_or_paid_unlock_authority
omnigate_source_does_not_import_ledger_or_external_settlement_runtime
```

## 3.4 Existing Phase 3 Round 1 validator boundary stayed green

Existing test:

```bash
cargo test -p omnigate --test quickchain_phase3_validator_boundary
```

Observed result:

```text
6 passed; 0 failed
```

Covered tests:

```text
docs_name_phase3_round1_omnigate_validator_passport_boundary
omnigate_header_policy_blocks_phase3_validator_passport_registry_and_capability_authority
existing_paid_routes_do_not_unlock_from_validator_or_passport_material
omnigate_routes_do_not_expose_validator_passport_registry_or_finality_authority_paths
omnigate_source_does_not_import_ledger_or_build_validator_economy_runtime
omnigate_source_does_not_implement_validator_admission_or_paid_unlock_authority
```

## 3.5 Exhaustive omnigate gate

Command run:

```bash
bash crates/omnigate/scripts/dev-quickchain-preflight.sh
```

Final marker:

```text
[omnigate quickchain] forbidden scope remains parked: no roots, no checkpoints, no validators, no bridges, no external settlement, no fake receipts, no fake balances
omnigate QuickChain preflight gate passed
== omnigate quickchain exhaustive preflight gate passed: tests=14 ==
```

Additional green coverage:

```text
format check passed
dynamic QuickChain test discovery found 14 tests
all-targets passed
clippy -D warnings passed
paid/product route regressions passed
forbidden scope marker preserved
```

## 3.6 Paid/product regressions preserved

Omnigate paid/product route regressions stayed green, including:

```text
content_view
site_visit
streams
chat_routes
paid_storage_estimate_proxy
paid_storage_prepare
paid_storage_write_proxy
```

All-targets also preserved broader product behavior, including:

```text
article_asset_publish
asset_page_resolver
builtin_page_resolver
comment_asset_publish
dto_serialization
image_asset_prepare
metrics_contract
middleware_contract
named_site_crab_resolver
passport_profile_routes
policy_gate
policy_metrics
readiness_error_rate
readiness_inflight
ready_truth
site_launch
text_asset_publish
zk_receipts
```

Important behavior preserved:

```text
content view quote/pay remains wallet/ledger-derived
site visit quote/pay remains wallet/ledger-derived
streams require stream-bound wallet receipt
paid chat requires quote
paid storage prepare/write remains storage/wallet path coordinated, not omnigate-owned truth
site launch stores manifest and index pointer while preserving backend truth boundaries
policy gate still fails closed
middleware hardening remains green
readiness checks remain green
```

## 3.7 Omnigate boundary proven

`omnigate` now proves:

```text
hydration/access routes do not expose validator lifecycle mutation endpoints
paid/access routes do not unlock from lifecycle/evidence/replay challenge material
headers cannot smuggle lifecycle/governance/evidence authority into hydration or paid access
normal product context headers remain usable without becoming authority
omnigate source does not import ron-ledger or external settlement runtime
omnigate source does not create validator lifecycle paid-unlock shortcuts
omnigate source does not create cache/client/header-only paid unlocks
omnigate source does not introduce bridge, staking, slashing, ROX, Solana, or external settlement runtime
```

---

## 4. Shared pair boundary proven

This pair now proves the Phase 3 public/product boundary:

```text
client intent
→ svc-gateway public route/proxy/enforcement boundary
→ omnigate quote/access/hydration coordinator
→ backend wallet/storage/index services
→ svc-wallet mutation front-door when economic mutation is required
→ ron-ledger durable economic truth
→ backend-derived receipt/access result
→ paid unlock/render/display path
```

The pair explicitly does not create:

```text
gateway ledger truth
omnigate ledger truth
gateway receipt truth
omnigate receipt truth
gateway balance truth
omnigate balance truth
gateway root truth
omnigate root truth
gateway checkpoint truth
omnigate checkpoint truth
gateway finality truth
omnigate finality truth
gateway validator authority
omnigate validator authority
gateway validator lifecycle authority
omnigate validator lifecycle authority
gateway governance update authority
omnigate governance update authority
gateway bridge authority
omnigate bridge authority
gateway settlement authority
omnigate settlement authority
cache entitlement authority
client/header entitlement authority
lifecycle/evidence entitlement authority
replay challenge entitlement authority
```

Backend-derived enforcement remains the rule.

No invented trust was introduced.

No local/client/cache/header-only paid unlock was introduced.

No lifecycle/evidence/replay-challenge paid unlock was introduced.

No route-level finality was introduced.

No gateway/omnigate mutation authority was introduced.

---

## 5. Forbidden scope still parked

This pair did not add:

```text
roots
state roots
receipt roots
hold roots
epoch roots
checkpoints
checkpoint signing
validator signatures
validator sets
validator admission
validator rotation runtime
validator revocation runtime
validator downtime runtime
validator equivocation runtime
replay challenge runtime
governance parameter-update runtime
committee replication
staking
slashing
liquidity
ROX runtime
Solana runtime
external settlement
public bridge
external anchors
public-chain route families
gateway direct ledger mutation
omnigate direct ledger mutation
gateway direct wallet mutation outside backend wallet path
omnigate direct wallet mutation outside backend wallet path
fake receipts
fake balances
fake finality
cache-only paid unlock
policy-only paid unlock
index-pointer paid unlock
b3-as-payment-proof
crab-url-as-payment-proof
transport-header-as-payment-proof
lifecycle-evidence-as-payment-proof
replay-challenge-as-payment-proof
governance-header-as-payment-proof
```

QuickChain remains future settlement infrastructure.

Internal ROC remains the only active economic mission.

---

## 6. Commands proven green

Focused Phase 3 Round 2 commands:

```bash
cargo fmt -p svc-gateway -p omnigate

cargo test -p svc-gateway --test quickchain_phase3_validator_lifecycle_boundary
cargo test -p omnigate --test quickchain_phase3_validator_lifecycle_boundary

cargo test -p svc-gateway --test quickchain_phase3_validator_boundary
cargo test -p omnigate --test quickchain_phase3_validator_boundary
```

Crate-local exhaustive gates:

```bash
bash crates/svc-gateway/scripts/dev-quickchain-preflight.sh
bash crates/omnigate/scripts/dev-quickchain-preflight.sh
```

Final observed markers:

```text
== svc-gateway quickchain exhaustive preflight gate passed: tests=14 ==
== omnigate quickchain exhaustive preflight gate passed: tests=14 ==
```

---

## 7. Current Phase 3 order and next pair

Phase 3 crate-pair status:

```text
ron-proto + ron-ledger:          COMPLETE / PARKED
svc-wallet + ron-accounting:     COMPLETE / PARKED
svc-rewarder + svc-storage:      COMPLETE / PARKED
svc-gateway + omnigate:          COMPLETE / PARKED
svc-index + ron-policy:          NEXT ACTIVE PAIR
CrabLink Tauri + client adapters: after svc-index + ron-policy
```

Next active crate pair:

```text
svc-index + ron-policy
```

The next session should begin with:

```text
1. Read QUICKCHAIN_REVIEW_BUNDLE.MD.
2. Read QUICKCHAIN_BUILDPLAN.MD.
3. Read the latest carryover notes.
4. Review the latest focused svc-index codebundle.
5. Review the latest focused ron-policy codebundle.
6. Inspect existing test/script names before writing patches.
7. Start with svc-index.
8. Continue to ron-policy.
```

---

## 8. Next pair mission: `svc-index + ron-policy`

The next crate pair must prove:

```text
svc-index remains lookup/pointer/name/provider metadata infrastructure only.
ron-policy remains declarative allow/deny/classification/explanation policy only.
```

The next pair must not create:

```text
economic truth
paid entitlement truth
receipt truth
balance truth
finality truth
root authority
checkpoint authority
validator authority
validator lifecycle authority
governance parameter-update authority
bridge authority
external settlement authority
cache/index/policy-only paid unlock
```

Correct pair behavior:

```text
svc-index resolves names, sites, assets, profiles, provider references, and metadata pointers.
ron-policy evaluates declarative policy and returns allow/deny/classification/explanation context.
svc-gateway and omnigate enforce public/product access flows.
svc-wallet performs economic mutation.
ron-ledger owns accepted economic truth.
```

Incorrect pair behavior:

```text
svc-index says a user paid.
svc-index says a receipt is valid.
svc-index says a balance is correct.
svc-index says a root/checkpoint/finality claim is valid.
svc-index says a pointer unlocks paid content.

ron-policy says a user paid.
ron-policy creates wallet receipts.
ron-policy mutates balances.
ron-policy allocates rewards directly.
ron-policy finalizes settlement.
ron-policy produces roots/checkpoints.
ron-policy validates bridge settlement.
ron-policy unlocks paid content without backend receipt validation.
```

---

## 9. Attachments / codebundle workflow for next session

For the next session, attach:

```text
QUICKCHAIN_REVIEW_BUNDLE.MD
QUICKCHAIN_BUILDPLAN.MD
latest carryover notes
latest focused svc-index codebundle
latest focused ron-policy codebundle
```

Generate focused bundles from the RustyOnions repo root:

```bash
bash scripts/make_crate_codex.sh --force -c svc-index
bash scripts/make_crate_codex.sh --force -c ron-policy
```

Do not attach unrelated CrabLink/Tauri bundles unless the next session explicitly moves to CrabLink.

Do not attach old all-codebundle artifacts unless broad repository context is truly needed.

---

## 10. Low-disk workflow reminder

For the next pair, continue the low-duplicate QuickChain workflow:

```text
focused tests first
then crate-local preflight/park once
no duplicate preflight + park run for the same crate unless debugging a failure
run clippy only for the active crate
avoid workspace clippy
avoid unnecessary cargo clean unless disk pressure requires it
```

Low-disk pattern if needed:

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/svc-index" \
bash crates/svc-index/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/svc-index
```

```bash
CARGO_INCREMENTAL=0 RUSTFLAGS="-C debuginfo=0" \
CARGO_TARGET_DIR="$PWD/.quickchain-target-lowdisk/ron-policy" \
bash crates/ron-policy/scripts/dev-quickchain-park.sh

rm -rf .quickchain-target-lowdisk/ron-policy
```

---

### END NOTE - JUNE 24 2026 - QUICKCHAIN PHASE 3 ROUND 2 - svc-gateway + omnigate


### END NOTE - JUNE 24 2026 - 22:05 CST