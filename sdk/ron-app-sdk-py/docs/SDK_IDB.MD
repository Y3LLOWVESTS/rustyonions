
# **IDB — ron-app-sdk-py (PYTHON SDK)**

*(Complete, exhaustive, aligned with TS SDK & RON-CORE protocol)*

---

# SDK Tier & Profile

* **Tier:** **1**
  Python is one of the Tier-1 managed languages (Rust/TS/Python/Go/Java/C#) and receives full rigor:

  * Full test suite
  * Benchmarks
  * Schema interop checks
  * Security & capability compliance
  * CI parity with TS SDK

* **Usage profile:**
  *[x] Backend/facet*
  *[x] CLI/tools*
  *[x] Dual-use (sync + async helpers)*
  *[ ] Front-end* (Python is not browser-side)

Tier-1 gates active:

* §3.4 sandboxing
* §4.3 idempotency helpers
* §5.3 benchmark rigor
* §10.3 property/chaos tests
* §9.3 CI/support guarantees

---

# 0) RO:WHAT / WHY / INTERACTS

## **RO:WHAT —**

A **Python client SDK** for the RON-CORE **/app/** plane.
Provides a `RonClient` (async-first) with sync wrappers, strongly typed Problem responses, TLS-secure HTTP requests, streaming subscriptions, and helpers for app-facet development.

## **RO:WHY —**

Python is critical for:

* **Backend apps** and **facets** written in Python
* **Data pipelines**, ML/AI microservices, cron/worker jobs
* **CLI tooling** and automation scripts
* Rapid prototyping by developers that are not Rust/TS-heavy

This SDK makes Python apps **first-class citizens** of RON-CORE.

## **RO:INTERACTS —**

The SDK communicates with RON-CORE’s app-plane surfaces:

* `svc-gateway` — receives `/app/*`
* `omnigate` — canonicalizes routing to `/v1/app/*`
* `svc-passport` / `ron-auth` — validates tokens & macaroons
* `svc-registry` — for future facet manifest registration
* `svc-storage` & `svc-index` — via app-plane proxy routes
* `svc-edge` — future streaming (WebSocket/SSE) endpoints

Canonically matches the TS SDK call path.

---

# 1) Scope & Target Profiles

## **1.1 Supported usage modes**

* [ ] Front-end only
* [x] Backend / facets
* [x] CLI / tools
* [x] Dual-use (async + sync)

Python SDK is BACKEND-first.
Async-first API (`async def`) with optional synchronous wrapper (`RonClientSync` using `anyio.to_thread.run_sync`).

## **1.2 Supported node profiles**

* [x] Micronode
* [x] Macronode
* [x] Both (default)

Notes:

* Macronode-only admin APIs gated with explicit flags.
* Micronode amnesia mode: warns when persistence ops attempted.

## **1.3 Feature scope (MVP vs Future)**

### **MVP**

* [x] Auth (macaroon / bearer token)
* [x] Basic request/response
* [x] Streaming (SSE)
* [x] Error mapping → `RonProblem`
* [x] Basic tracing hooks

### **Future**

* [ ] Codegen from `ron-proto` schema
* [ ] Auto-reconnect for streaming
* [ ] Built-in metrics hooks
* [ ] Facet manifest builder
* [ ] Domain helpers (app-specific convenience)

---

# 2) API Design & DX Basics

## **2.1 Primary abstraction**

**Async-first client**:

```python
from ron_app_sdk_py import RonClient, RonProblem

async def main():
    client = RonClient(base_url="https://localhost:8080", token="my-token")
    try:
        res = await client.get("/hello")
        print(res)
    except RonProblem as p:
        print("Error:", p)

import asyncio; asyncio.run(main())
```

Sync wrapper:

```python
from ron_app_sdk_py import RonClientSync

client = RonClientSync(base_url="https://localhost:8080", token="my-token")
print(client.get("/hello"))
```

## **2.2 Core operations**

* `await client.get(path, *, query=None)`
* `await client.post(path, json=None)`
* `await client.put(path, json=None)`
* `await client.delete(path)`
* `await client.call(method, path, *, json=None, query=None)`
* `await client.subscribe(topic, callback)` *(SSE)*
* `await client.close()`

## **2.3 Error model**

Python idioms:

* All structured errors raise **`RonProblem`** (maps TS `Problem` type)
* Network errors raise `RonNetworkError`
* Auth failures raise `RonAuthError`
* Problem JSON body → `RonProblem(detail, status, title, type)`

No raw exceptions leak.

## **2.4 Configuration**

Configurable:

* `base_url` (default: env `RON_APP_URL`, else `http://127.0.0.1:8080`)
* TLS verify: `tls_verify=True`
* CA bundle: `tls_ca=None`
* Timeouts:

  * connect timeout
  * read timeout
  * write timeout
  * overall
* Retries: simple linear/jitter backoff
* Logging: integrates with `logging` module

Config loading:

* Env vars `RON_APP_URL`, `RON_APP_TIMEOUT`
* Code-based builder
* Optional `.env` file via python-dotenv (optional dep)

## **2.5 Language-idiomatic patterns**

* Async-first (`asyncio`) with sync wrapper
* Uses `httpx` (async client)
* Uses Pydantic v2 for typed Problem
* Dict-like JSON responses
* No global singletons (supports multiprocessing, sandboxes)

---

# 3) Security & Capabilities (SEC)

## **3.1 Auth flow**

Supported:

* `token="..."`
* `macaroon="..."`
* Token stored only in memory
* Never logged
* Redacted in debug prints

## **3.2 Capability boundaries**

* Helpers for restricting tokens:
  `client.with_caveat(expiry="10m", readonly=True)`
* Docs for tenant-scoped tokens
* No silent escalation: all tokens explicit

## **3.3 Secure defaults**

* [x] TLS ON by default
* [x] Credentials never logged
* [x] Timeouts enforced
* [x] Debug-only mode gated
* [ ] PQ-hybrid mode — postponed until Python TLS lib supports hybrid KEM

## **3.4 Sandboxing**

* No globals
* Client is cheap & isolated
* Documented multiprocessing, thread & async-safe patterns
* Facet handler sandbox example included

---

# 4) Reliability & Resilience (RES)

## **4.1 Connection behavior**

* Graceful retries for transient errors
* Distinguish DNS errors vs gateway errors
* Optional exponential backoff

## **4.2 Streaming resilience**

* SSE client with disconnect detection
* Optional automatic reconnect with jitter
* At-most-once semantics
* User callback receives structured events

## **4.3 Backoff & rate limiting**

* Built-in idempotency key helper:
  `await client.post("/x", json=data, idem=True)`
* Retry budget and jitter
* Warns if a caller loops without sleep

---

# 5) Performance (PERF)

## **5.1 Expected use**

* 50–500 RPS typical
* Payloads usually small (<32 KiB JSON)

## **5.2 Implementation choices**

* HTTP: **httpx.AsyncClient**
* SSE: **aiohttp** or **httpx-SSE** depending on environment
* JSON: stdlib or Pydantic
* Future: CBOR if app-plane adds it

## **5.3 Benchmarks**

Tier-1 benchmark suite:

* Latency baseline: GET /healthz
* Sustained RPS test
* Streaming throughput (messages/sec)
* Overhead <5% vs raw httpx baseline

---

# 6) Economics (ECON)

## **6.1 Dev productivity**

Removes ~15–20 lines of boilerplate per call.
Automatic error mapping and streaming makes it trivial to write Python facets.

## **6.2 Operational cost**

* Reuse clients (keep-alive)
* Avoid client-per-request
* Low memory footprint (httpx)
* All clients close sockets cleanly

---

# 7) Developer Experience (DX)

## **7.1 No-manual Quickstart**

1. Hello world:

```python
client = RonClient(base_url="http://localhost:8080", token="x")
print(await client.get("/hello"))
```

2. Streaming:

```python
async for event in client.subscribe("/events/logs"):
    print(event)
```

3. Facet-style worker loop:

```python
while True:
    job = await client.get("/queue/next")
    await handle(job)
```

## **7.2 Framework adapters**

Planned:

* FastAPI middleware (`RonClientDependency`)
* Flask extension (`RonClient.init_app(app)`)
* Click CLI helpers
* Logging integration via structlog

## **7.3 Idiomatic docs**

* PEP8 style
* clear async examples
* Typed signatures
* Problem examples identical to TS SDK

---

# 8) Extensibility & Plugins

## **8.1 Extension points**

MVP:

* request middleware: `before_request`, `after_response`

Future:

* full plugin stack
* custom transports (QUIC, WebSocket backend)

## **8.2 UI widgets**

Not applicable for Python SDK (CLI widgets only)

## **8.3 Plugin model**

Keep core light.
3rd-party plugins register via `client.add_middleware(x)`

---

# 9) Governance & Versioning (GOV)

## **9.1 Version scheme**

* SDK version tracks protocol version
* `0.1.x` → RON-CORE protocol v0.1
* Breaking change requires major bump

## **9.2 Compatibility matrix**

| SDK version | Protocol | Min node | Notes |
| ----------- | -------- | -------- | ----- |
| 0.1.x       | v0.1     | 0.1.0    | MVP   |

## **9.3 Support expectations**

* Tier-1 = full test suite, property tests, chaos tests, perf gates

---

# 10) Testing & QA

## **10.1 Test types**

* Unit: config, headers, error mapping
* Integration: local micronode via docker or subprocess
* E2E: sample Python facet calling through gateway

## **10.2 CI requirements**

* flake8 + mypy
* pytest
* Integration tests gated behind `RON_TEST_NODE_URL`

## **10.3 Property / Chaos tests**

* Hypothesis-based property tests for Problem mapping
* Toxiproxy tests for network failures
* Required for Tier-1

---

# 11) Interop & Schema

## **RO:WHAT**

Ensure identical DTO, error, pagination semantics across languages.

## **RO:INVARIANTS**

* Python SDK uses canonical schemas from `ron-proto`
* No hand-invented shapes
* Pagination shapes match TS client identically
* Problem shapes match TS & Rust exactly

## **11.1 Canonical sources**

* ron-proto JSON schema
* gateway OpenAPI spec
* SDK_SCHEMA_IDB.md (cross-language rules) 

## **11.2 Requirements**

Tier-1:

* MUST pass interop test suite
* MUST match canonical error envelopes
* MUST regenerate schema & fail CI on drift

## **11.3 Type mapping**

* `u64` → Python int
* timestamps → `datetime` objects
* Problems → Pydantic model

## **11.4 Facet interop**

* Future: Python facet manifest builder
* Must produce identical TOML to Rust & TS SDKs

---

# 12) Future Extensions

* Mock local server (post-M3)
* CLI scaffolding: `ron-app init py`
* Facet registry integration
* Domain helpers for common app patterns
* Async streaming reconnection with exponential backoff

---

# 13) Platform-Specific Gotchas

* Windows: event loop differences handled by asyncio
* macOS: TLS CA bundle path variance (httpx auto-detect)
* Docker kills idle connections → reconnect logic included
* Micronode amnesia mode → warn on persistence calls

---


