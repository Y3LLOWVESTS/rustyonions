

---

## 1) File tree (scaffold)

```text
sdk/ron-app-sdk-py/
├─ pyproject.toml
├─ README.md
├─ LICENSE-MIT
├─ LICENSE-APACHE
├─ CHANGELOG.md
├─ SDK_IDB.MD
├─ SDK_SECURITY.MD
├─ SDK_SCHEMA_IDB.MD
├─ .gitignore
├─ .editorconfig
├─ .ruff.toml
├─ mypy.ini
├─ tests/
│  ├─ __init__.py
│  ├─ unit/
│  │  ├─ __init__.py
│  │  ├─ test_client_basic.py
│  │  ├─ test_errors_problem.py
│  │  ├─ test_config_env.py
│  │  ├─ test_pagination.py
│  │  └─ test_security_headers.py
│  ├─ integration/
│  │  ├─ __init__.py
│  │  ├─ test_healthz_readyz.py
│  │  ├─ test_app_hello_roundtrip.py
│  │  └─ test_pagination_roundtrip.py
│  ├─ property/
│  │  ├─ __init__.py
│  │  ├─ test_idempotency_keys_hypothesis.py
│  │  └─ test_error_parsing_fuzz.py
│  └─ chaos/
│     ├─ __init__.py
│     ├─ test_timeouts_and_retries_toxiproxy.py
│     └─ test_streaming_disconnects.py
├─ examples/
│  ├─ __init__.py
│  ├─ hello_async.py
│  ├─ hello_sync.py
│  ├─ list_paginated_items.py
│  └─ watch_events_async.py
├─ docs/
│  ├─ arch.mmd
│  ├─ sequence_call.mmd
│  ├─ state_client.mmd
│  ├─ arch.svg
│  ├─ sequence_call.svg
│  └─ state_client.svg
└─ ron_app_sdk_py/
   ├─ __init__.py
   ├─ _version.py
   ├─ config.py
   ├─ client.py
   ├─ client_sync.py
   ├─ errors.py
   ├─ models.py
   ├─ pagination.py
   ├─ streaming.py
   ├─ facets.py
   ├─ logging_.py
   ├─ metrics.py
   ├─ codecs.py
   └─ _types.py
```

---

## 2) File-by-file descriptions

### Repo root

* **`pyproject.toml`** — Python packaging + tooling config (name, version, deps like `httpx`, `pydantic`, and dev deps like `pytest`, `hypothesis`, `ruff`, `mypy`).
* **`README.md`** — Main SDK README you just defined (overview, examples, diagrams, SLOs, security notes).
* **`LICENSE-MIT`** — MIT license text for dual-licensing.
* **`LICENSE-APACHE`** — Apache-2.0 license text for dual-licensing.
* **`CHANGELOG.md`** — SemVer-aligned changelog tracking API/protocol changes and migration notes.
* **`SDK_IDB.MD`** — Invariant-Driven Blueprint for `ron-app-sdk-py` (WHAT/WHY/SCOPE/RES/SEC/PERF/DX/GOV).
* **`SDK_SECURITY.MD`** — Full security checklist and threat model for the Python SDK (caps, timeouts, DoS guardrails).
* **`SDK_SCHEMA_IDB.MD`** — Unified schema & facet contract for SDKs, including Python type mappings and manifest rules.
* **`.gitignore`** — Ignore rules for venvs, build artifacts, cache dirs, and editor junk.
* **`.editorconfig`** — Consistent editor settings (indent, line endings, charset) across tools.
* **`.ruff.toml`** — Linter configuration (rules, ignores, line length) for `ruff`.
* **`mypy.ini`** — Static type checking configuration for `mypy` (strictness levels, plugins).

### `tests/` — test suites

* **`tests/__init__.py`** — Marks `tests` as a package; usually empty.

#### `tests/unit/`

* **`tests/unit/__init__.py`** — Package marker for unit tests.
* **`tests/unit/test_client_basic.py`** — Unit tests for `RonClient`/`RonClientSync` core behavior (base URL resolution, simple GET/POST, timeouts wiring).
* **`tests/unit/test_errors_problem.py`** — Tests canonical `Problem` parsing, `RonProblemError` behavior, and mapping from HTTP responses to exceptions.
* **`tests/unit/test_config_env.py`** — Verifies env-var loading semantics (`RON_APP_URL`, `RON_SDK_*`) and precedence vs explicit config.
* **`tests/unit/test_pagination.py`** — Tests `Page[T]`, `iter_pages`, and pagination helpers with mocked responses.
* **`tests/unit/test_security_headers.py`** — Ensures auth headers are set correctly and that caps never appear in logs, URLs, or non-auth headers.

#### `tests/integration/`

* **`tests/integration/__init__.py`** — Package marker for integration tests.
* **`tests/integration/test_healthz_readyz.py`** — Calls `/healthz` and `/readyz` on a real local node (micronode/macronode via `svc-gateway`) to prove basic connectivity.
* **`tests/integration/test_app_hello_roundtrip.py`** — End-to-end `/app/hello` example that hits a real gateway and validates the decoded response.
* **`tests/integration/test_pagination_roundtrip.py`** — Exercises a real paginated endpoint to confirm pagination envelope and helper semantics.

#### `tests/property/`

* **`tests/property/__init__.py`** — Package marker for property-based tests.
* **`tests/property/test_idempotency_keys_hypothesis.py`** — Hypothesis tests that check idempotency key generation is stable and collision-resistant under random inputs.
* **`tests/property/test_error_parsing_fuzz.py`** — Fuzzes error payloads (random/malformed JSON) to ensure the SDK never crashes or leaks sensitive data when parsing failures occur.

#### `tests/chaos/`

* **`tests/chaos/__init__.py`** — Package marker for chaos-style tests.
* **`tests/chaos/test_timeouts_and_retries_toxiproxy.py`** — Uses Toxiproxy (or equivalent) to simulate latency, drops, and half-open connections and verifies timeout and retry logic.
* **`tests/chaos/test_streaming_disconnects.py`** — Simulates streaming/SSE disconnects and asserts that the SDK reconnects/backoffs according to invariants.

### `examples/` — runnable examples

* **`examples/__init__.py`** — Package marker; usually empty.
* **`examples/hello_async.py`** — Minimal async “hello world” example that calls `/app/hello` using `RonClient` with `asyncio.run`.
* **`examples/hello_sync.py`** — Minimal sync “hello world” example using `RonClientSync` for cron jobs or CLI tools.
* **`examples/list_paginated_items.py`** — Example showing use of pagination helpers (`iter_pages`) to list all items from a paginated endpoint.
* **`examples/watch_events_async.py`** — Example for streaming/SSE usage: subscribes to an event feed and prints events as they arrive.

### `docs/` — diagrams and dev docs

* **`docs/arch.mmd`** — Mermaid source for high-level architecture (Python app → SDK → svc-gateway → omnigate → node).
* **`docs/sequence_call.mmd`** — Mermaid sequence diagram for a typical `RonClient.get("/app/hello")` call.
* **`docs/state_client.mmd`** — Mermaid state diagram for `RonClient` lifecycle (unconfigured → configured → ready → backoff → closed).
* **`docs/arch.svg`** — Rendered SVG for the architecture diagram (generated from `arch.mmd`).
* **`docs/sequence_call.svg`** — Rendered SVG for the sequence diagram (from `sequence_call.mmd`).
* **`docs/state_client.svg`** — Rendered SVG for the state diagram (from `state_client.mmd`).

### `ron_app_sdk_py/` — main package

* **`ron_app_sdk_py/__init__.py`** — Package entrypoint; exports public API surface (`RonClient`, `RonClientSync`, `Problem`, `ClientConfig`, version).
* **`ron_app_sdk_py/_version.py`** — Central version declaration (e.g., `__version__ = "0.1.0"`), used by packaging and `__init__`.
* **`ron_app_sdk_py/config.py`** — Defines `ClientConfig` and helpers to load from environment, apply defaults, and validate URLs/timeouts.
* **`ron_app_sdk_py/client.py`** — Core async `RonClient` implementation using `httpx.AsyncClient` (GET/POST/PUT/DELETE, idempotency keys, retries, timeouts, correlation IDs).
* **`ron_app_sdk_py/client_sync.py`** — Synchronous wrapper (`RonClientSync`) built on top of `RonClient` using `asyncio`/`anyio` to expose blocking methods.
* **`ron_app_sdk_py/errors.py`** — Canonical `Problem` Pydantic model and custom exception hierarchy (`RonProblemError`, `RonAuthError`, `RonNetworkError`, etc.), plus mapping from HTTP responses to errors.
* **`ron_app_sdk_py/models.py`** — Shared DTO models for common app-plane payloads (if any) generated or aligned with `SDK_SCHEMA_IDB.MD` (can start as just `Problem` + pagination if you keep it minimal).
* **`ron_app_sdk_py/pagination.py`** — Generic `Page[T]` Pydantic model and helpers like `iter_pages` and `list_all` for paginated endpoints.
* **`ron_app_sdk_py/streaming.py`** — Streaming/SSE support (async iterator-based subscription, reconnect/backoff logic, event parsing).
* **`ron_app_sdk_py/facets.py`** — Future facet manifest builder/validator producing TOML that conforms to `SDK_SCHEMA_IDB.MD` facet schema.
* **`ron_app_sdk_py/logging_.py`** — Logging setup utilities and middleware-like hooks that add method/path/status/duration/correlation ID while scrubbing secrets.
* **`ron_app_sdk_py/metrics.py`** — Optional hooks or thin interfaces so apps can plug in Prometheus/StatsD-style metrics for SDK call tracking.
* **`ron_app_sdk_py/codecs.py`** — Helper functions for encoding/decoding special types (base64url for bytes, ISO8601 timestamps, u64 string↔int conversions).
* **`ron_app_sdk_py/_types.py`** — Internal type aliases and protocol interfaces (e.g., `TokenProvider`, `MetricsHook`, `Logger`) to keep imports clean and typing centralized.

