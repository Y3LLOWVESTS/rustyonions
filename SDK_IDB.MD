# IDB TEMPLATE FOR SDK
>Grok wrote this


### SDK Tier & Profile
- **Tier:** [1 | 2 | 3]  
  - Tier 1: Core/maintained (Rust, TS, Python, Go, Java, C#)—full rigor on tests, benchmarks, CI.  
  - Tier 2: Popular/community (Ruby, PHP, Kotlin, Dart, Elixir, Scala)—balanced, grow over time.  
  - Tier 3: Niche/experimental (Haskell, Pony, Nim, Crystal)—baseline only, stretch goals.  
- **Usage profile:**  
  * [ ] Front-end  
  * [ ] Backend/facet  
  * [ ] Dual-use  
  * [ ] CLI/tools  

This tier gates:  
- §3.4 sandboxing expectations  
- §4.3 idempotency helpers  
- §5.3 benchmark rigor  
- §10.3 property/chaos tests  
- §9.3 CI/support levels  

---

### 0) RO:WHAT / WHY / INTERACTS
**RO:WHAT —**  
One-liner: what this SDK is, in this language, in the RON-CORE universe.  

**RO:WHY —**  
* Why this language matters (TS, Python, Go, etc.).  
* Primary use cases (front-end, facets, CLI, data pipelines, etc.).  
* How it helps ship apps faster on RON-CORE.  

**RO:INTERACTS —**  
Core RON-CORE components this SDK talks to:  
* `svc-gateway`  
* `svc-edge`  
* `svc-admin`  
* `svc-registry`  
* `svc-passport` / `ron-auth`  
* `svc-storage` / `svc-index`  
* Other services if applicable.  

---

### 1) Scope & Target Profiles
**1.1 Supported usage modes**  
* [ ] Front-end only (browser / mobile)  
* [ ] Backend / facets only  
* [ ] Dual-use (browser + Node / server)  
* [ ] CLI / tools  
Explain which combinations apply (e.g., TS SDK = browser + Node facets).  

**1.2 Supported node profiles**  
* [ ] Micronode  
* [ ] Macronode  
* [ ] Both (default)  
Note any profile-specific assumptions (e.g., some admin APIs only exist on macronode; warn on persistence ops in amnesia mode).  

**1.3 Feature scope (MVP vs future)**  
**MVP:**  
* [ ] Auth (attach macaroon / token)  
* [ ] Basic request/response calls  
* [ ] Streaming (SSE/WebSocket) subscribe  
* [ ] Error handling (idiomatic for this language)  
* [ ] Minimal logging / tracing hooks  

**Future:**  
* [ ] Schema-generated DTOs from `ron-proto`  
* [ ] Auto-reconnect & backoff  
* [ ] Built-in metrics hooks  
* [ ] Facet lifecycle helpers (register, heartbeat, etc.)  
* [ ] Higher-level convenience APIs (domain-specific)  

---

### 2) API Design & DX Basics
**2.1 Primary abstraction**  
Describe the main entrypoint:  
* `Client` / `Session` / builder pattern, etc.  
Minimal “hello world” example (this should be **copy-paste runnable**):  
```<lang>  
// Smallest possible example that:  
// 1) creates a client  
// 2) makes one simple call  
// 3) handles error in the idiomatic way  
```  
Goal: **you should not have to read the manual** to understand this snippet.  

**2.2 Core operations**  
List and briefly define:  
* `connect(...)` / `create_client(...)`  
* `call(service, method, payload)` or strongly typed ops:  
* `get_resource(...)`  
* `put_resource(...)`  
* `emit_event(topic, payload)`  
* `subscribe(topic, handler)` (streaming)  
* `close()` / cleanup semantics  

**2.3 Error model**  
* Representation (exceptions, `Result`, tagged union, etc.).  
* Mapping of RON-CORE errors:  
* network vs auth vs policy vs app errors.  
* Idiomatic error handling patterns for this language.  

**2.4 Configuration**  
What’s configurable:  
* Gateway/base URL  
* TLS options (verify, CA, mTLS?)  
* Timeouts (connect / read / write / overall)  
* Retries/backoff (if any)  
* Logging/tracing hooks  
How config is loaded:  
* Env vars (with magic detection/fallbacks, e.g., `localhost:8080` if unset)  
* Code-based config (builder)  
* Optional config file  

**2.5 Language-idiomatic patterns**  
* Async model: `async/await`, futures, callbacks, coroutines, etc.  
* Naming conventions: methods / modules / packages.  
* Integration pattern:  
* e.g., for TS: Promises all the way down, no callbacks.  
* for Python: `async def` + `await`, or sync API wrapper.  
The rule here: **this SDK must “feel native” to someone who lives in this language**.  

---

### 3) Security & Capabilities (SEC)
**3.1 Auth flow**  
* How the SDK accepts credentials:  
* macaroon string, bearer token, token provider, etc.  
* How tokens are stored in memory:  
* no accidental logging, redaction in debug output.  

**3.2 Capability boundaries**  
* Helpers to **encourage scoped tokens**:  
* “request a token with these caveats” convenience APIs (if applicable).  
* Clear docs/examples for:  
* tenant-scoped tokens,  
* read-only vs admin tokens,  
* short-lived tokens.  

**3.3 Secure defaults**  
Checklist:  
* [ ] TLS on by default; insecure HTTP requires explicit opt-in.  
* [ ] Credentials never logged.  
* [ ] Reasonable timeouts; no infinite hangs.  
* [ ] Debug-only features gated / off in production builds.  
* [ ] PQ-ready toggle: `pq_mode: off|hybrid` (future/optional: mirrors RON-CORE; implement only if lang TLS stack supports, post RON-CORE PQ profile stable).  

**3.4 Sandboxing for Facets/UI**  
* Invariants for isolating untrusted code (per language/tier):  
* SDK **MUST** be sandbox-friendly (no global singletons, supports use from workers/isolates).  
* **SHOULD** provide docs/recipes for sandbox patterns (e.g., TS: Web Workers; Python: multiprocessing).  
* Tier 1: **MUST** include basic helpers if idiomatic (e.g., TS worker wrapper).  
* Tier 2/3: **MAY** add helpers later; focus on docs.  
* Mandate: Least-privilege execution; no globals that leak across sandboxes.  
* Docs: Examples for "sandbox a facet handler" (copy-paste).  
* Note: Sandbox runtimes (containers/seccomp/WASM) belong at facet host/node level, not SDK.  

---

### 4) Reliability & Resilience (RES)
**4.1 Connection behavior**  
* Auto-reconnect behavior (if any).  
* How the SDK handles:  
* DNS resolution failures,  
* timeouts,  
* connection drops.  
* PQ-hybrid handshake support (future/optional: if lang HTTP lib allows, post RON-CORE stable).  

**4.2 Streaming resilience**  
For subscriptions:  
* Policy on disconnect:  
* immediate error vs transparent reconnect with backoff.  
* Idempotency / at-most-once vs at-least-once semantics (as far as SDK can guarantee).  

**4.3 Backoff & rate limiting**  
* Any built-in throttling/backpressure.  
* Recommendations for avoiding:  
* “one client per request” anti-pattern,  
* floods from misconfigured loops.  
* Idempotency key helpers for writes (e.g., auto-generate UUID keys):  
* Tier 1: **MUST** include.  
* Tier 2: **SHOULD** include.  
* Tier 3: **MAY** include as stretch.  

---

### 5) Performance (PERF)
**5.1 Expected usage profile**  
* Typical request rates (low, med, high).  
* Typical payload sizes.  

**5.2 Implementation choices**  
* HTTP client library choice (and why).  
* WebSocket/SSE client choice.  
* Serialization formats supported:  
* JSON baseline,  
* plus CBOR/proto/other (if/when added).  

**5.3 Benchmarks**  
* Simple benchmarks:  
* single call latency baseline,  
* sustained RPS for simple call,  
* streaming overhead.  
* Hardware + env notes (dev laptop vs server).  
* Prove: SDK overhead <5% vs raw HTTP.  
* Rigor by tier:  
* Tier 1: **MUST** include in CI.  
* Tier 2: **SHOULD** have baselines.  
* Tier 3: **MAY** add later.  

---

### 6) Economics (ECON)
**6.1 Dev productivity**  
* How much boilerplate is removed vs raw HTTP/WS.  
* Minimal lines of code to:  
* call an API,  
* subscribe to a stream,  
* handle basic auth.  

**6.2 Operational cost**  
* Guidance on:  
* long-lived clients vs short-lived,  
* pooling/sharing clients per process.  
* Any known memory/CPU tradeoffs.  
* Invariants: Auto-close idle clients; no resource leaks (e.g., goroutine budgets in Go).  

---

### 7) Developer Experience (DX) — Onboarding & Familiarity
**7.1 “No manual” quickstart**  
* **3 code snippets** that must be present in README:  
1. “Hello world” call (copy-paste).  
2. “Subscribe to a stream” (copy-paste).  
3. “Facet-style worker loop” (for backend languages).  
Each snippet should:  
* Fit on 10–20 lines.  
* Require **minimal config** (ideally environment variables + one function call).  

**7.2 Framework adapters / helpers**  
For this language, what’s “familiar” dev experience?  
Examples:  
* TS:  
* React hooks (`useRonClient`, `useSubscription`),  
* Next.js integration notes,  
* simple widget helpers for admin GUI (can be in a separate `*-ui` package).  
* Python:  
* FastAPI/Flask middleware,  
* async vs sync integration patterns.  
* Go:  
* context patterns,  
* minimal examples with `net/http`.  
List planned or existing adapters:  
* [ ] Web framework middleware (name it)  
* [ ] CLI helper(s)  
* [ ] Admin GUI hooks/widgets (if applicable)  
* [ ] Logging/tracing integration (language-standard)  

**7.3 Language-idiomatic docs**  
* Code samples follow community style guides.  
* Errors and return types look “normal” for that ecosystem.  
* No weird Rust-isms leaking into the API design.  

---

### 8) Extensibility & Plugins
**8.1 Extension points**  
List the designed extension hooks:  
* Request/response middleware / interceptors:  
* before/after each call,  
* for logging, custom headers, tracing.  
* Event hooks:  
* `on_connect`, `on_disconnect`, `on_error`, `on_reconnect`.  
* Pluggable transports (if we ever support alt transports like gRPC/QUIC).  
Mark which are part of MVP vs future:  
* MVP:  
* [ ] Basic “request middleware” or callback hooks.  
* Future:  
* [ ] Fully pluggable pipeline (middleware stack),  
* [ ] Custom transport adapters.  

**8.2 UI widgets / components (for GUI-friendly SDKs)**  
If this SDK is used in GUIs (TS, Dart, Swift, etc.):  
* Decide where UI helpers live:  
* In-core SDK?  
* Or separate `ron-app-sdk-<lang>-ui` package?  
* List planned or existing UI primitives, e.g.:  
* `MetricsPanel`  
* `NodeHealthBadge`  
* `EventLogViewer`  
* Design goal:  
* GUI builders should be able to **drop in a widget** and be live against a node with minimal wiring.  

**8.3 Plugin model (future)**  
* Consider whether 3rd-party extensions should:  
* register as middleware,  
* extend a “plugin registry” on the client,  
* or live purely as “SDK + utility libs.”  
* Note any constraints:  
* Keep core light; heavy plugins live in separate packages.  
* Avoid making plugin loading a security footgun.  

---

### 9) Governance & Versioning (GOV)
**9.1 Version scheme**  
* How SDK versions track RON-CORE protocol versions:  
* e.g., `sdk 0.x` targets `RON-CORE protocol v0.x`.  
* Policy on breaking changes:  
* Major version bump required?  
* Deprecation period? (e.g., warn 2 minor versions before removal).  

**9.2 Compatibility matrix**  
| SDK version | Protocol version | Min node version | Notes |  
| ----------- | ---------------- | ---------------- | --------- |  
| 0.1.x | v0.1 | X.Y.Z | MVP slice |  

**9.3 Support expectations**  
Classify this SDK:  
* [ ] Tier 1 (core, always up to date)  
* [ ] Tier 2 (popular but not core)  
* [ ] Tier 3 (niche / experimental, community-heavy)  
* Tie to CI:  
* Tier 1: Extended suite (concurrency stress, memory checks, property tests).  
* Tier 2: Standard CI (lint, unit/integration).  
* Tier 3: Baseline (lint, unit).  

---

### 10) Testing & QA
**10.1 Test types**  
* Unit tests:  
* serialization, config, error mapping, helper logic.  
* Integration tests:  
* hitting a local test node (micronode),  
* basic call and subscription.  
* E2E samples:  
* sample app that runs in CI (where practical).  

**10.2 CI requirements**  
* Lint / format  
* Unit tests  
* Integration tests (can be optional/flagged if node is required)  
* Build/distribution artifacts (bundle, wheel, jar, etc.)  

**10.3 Property/Chaos Tests**  
* Property-based: e.g., fuzz auth flows, error models with proptest/quickcheck equiv.  
* Chaos: Simulate flakes (toxiproxy for net drops); prove resilience invariants.  
* Rigor by tier:  
* Tier 1: **MUST** include property (auth/errors) + minimal chaos in CI/script.  
* Tier 2: **SHOULD** grow over time.  
* Tier 3: **MAY** add as stretch.  

---

### 11) Interop & Schema (`ron-proto`)
**11.1 Schema sync**  
* How this SDK stays in sync with `ron-proto`:  
* **Preferred:** Codegen from OpenAPI/Proto/JSON Schema (automate to prevent drift; non-negotiable for Tier-1/2).  
* Fallback: Hand-maintained DTOs with version markers (but audit in CI; OK for Tier-3).  

**11.2 Type mapping**  
* Any tricky mappings (e.g., `u64` to bigint/string, timestamps, decimal types).  
* Document rules:  
* Lossless representation where possible.  
* Explicit escape hatches for raw payload access.  

---

### 12) Future Extensions & Nice-to-Haves
Bulleted list of ideas not in MVP, **each gated by RON-CORE milestones** (mandatory: tag post-Mx or drop):  
* Post-M3: Local mock/testing server for offline dev.  
* Post-M4: Code generators (e.g., CLIs that scaffold minimal apps using the SDK).  
* Post-M5: Domain-specific helpers (e.g., “taxi app helpers” when we dogfood).  
* Post-M3: More powerful UI widgets (admin GUI building blocks).  

---

### 13) Platform-Specific Gotchas
* List per-platform issues + mitigations:  
* e.g., Browser: CORS, polyfills for BigInt/u64.  
* Mobile: Cert pinning (Android/iOS).  
* Server: Bundler pitfalls (tree-shaking).  
* Ensure: Amnesia mode warnings for persistence attempts in micronode.  

