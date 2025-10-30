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