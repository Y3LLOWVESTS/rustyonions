### TODO FOR RON-APP-SDK-TS

## File tree (proposed scaffold)

```text
ron-app-sdk-ts/
  package.json
  tsconfig.json
  tsconfig.build.json
  vitest.config.ts
  eslint.config.mjs
  .prettierrc
  .prettierignore
  .gitignore
  README.md
  SDK_IDB.md
  SDK_SECURITY.md
  SDK_SCHEMA_IDB.md
  NOTES.md

  src/
    index.ts
    client.ts
    types.ts
    errors.ts
    config.ts
    transport.ts
    utils/
      headers.ts
      url.ts
      timeouts.ts
      problem.ts
    facets/
      manifest.ts

  test/
    client.basic.test.ts
    transport.timeouts.test.ts
    errors.mapping.test.ts
    security.no_secrets.test.ts
    schema.interop.test.ts

  docs/
    ron-app-sdk-ts-arch.mmd
    ron-app-sdk-ts-state.mmd
    ron-app-sdk-ts-seq-app-call.mmd
```

---

## File-by-file summaries

### Root config & docs

* **`package.json`**
  NPM package definition: name/version, build/test scripts, dependencies (`typescript`, `vitest`, `tsup` or equivalent), and publish configuration for the TS SDK.

* **`tsconfig.json`**
  Main TypeScript config for development (strict mode, module resolution, paths), used by editor tooling and tests.

* **`tsconfig.build.json`**
  Build-focused TS config that extends `tsconfig.json` but targets the final output format (ESM/CJS) and excludes tests/docs.

* **`vitest.config.ts`**
  Test runner configuration (Vitest): test environment (node/jsdom), coverage settings, and path aliases for `src/`.

* **`eslint.config.mjs`**
  ESLint setup for consistent style and basic security rules (no unused vars, no console in library code, etc.), wired to TS parser.

* **`.prettierrc`**
  Prettier formatting rules (indentation, quotes, trailing commas) to keep the TS codebase visually consistent.

* **`.prettierignore`**
  Ignore list for Prettier (e.g., `dist/`, `coverage/`, generated files).

* **`.gitignore`**
  Standard ignore list for Node/TS projects (node_modules, dist, coverage, temp files).

* **`README.md`**
  The God-tier README we just built: role, architecture diagrams, API docs, security notes, SLOs, roadmap, and examples.

* **`SDK_IDB.md`**
  Invariant-Driven Blueprint for the SDK’s behavior and API surface (what `Ron` must do, must not do, and how it interacts with RON-CORE).

* **`SDK_SECURITY.md`**
  Security checklist specialized for `ron-app-sdk-ts` (caps handling, timeouts, error hygiene, browser vs Node rules).

* **`SDK_SCHEMA_IDB.md`**
  Unified schema contract for DTOs, error envelopes, pagination, and facet manifests, with explicit TypeScript mappings.

* **`NOTES.md`**
  Rolling notes / carry-over log for this SDK (what’s implemented, open questions, next steps), mirroring your pattern in Rust crates.

---

### `src/` — library implementation

* **`src/index.ts`**
  Public entrypoint: re-exports `Ron`, core types (`AppRequest`, `AppResponse`, `RonProblem`, `RonOptions`), and any facet helpers so consumers only import from `"ron-app-sdk-ts"`.

* **`src/client.ts`**
  Home of the `Ron` class: constructor, `request<T>()`, convenience methods (`get/post/put/patch/delete`), header injection, and integration with `transport.ts`.

* **`src/types.ts`**
  Canonical TypeScript interfaces and types: `AppRequest`, `AppResponse<T>`, `RonProblem`, `RonOptions`, and shared utility types that are derived from the schema IDB.

* **`src/errors.ts`**
  Defines TS error types and helpers (e.g., `SdkConfigError`, `LocalNetworkError`) used for SDK misconfiguration or catastrophic local failures (distinct from server-side problems).

* **`src/config.ts`**
  Central place for SDK defaults and config resolution: merges constructor options with Node env vars (`RON_SDK_*`) and enforces invariants like HTTPS-only by default.

* **`src/transport.ts`**
  The fetch wrapper: builds full URLs (`baseUrl + "/app" + path`), attaches timeouts via `AbortController`, runs the request, and delegates response parsing to helpers.

* **`src/utils/headers.ts`**
  Utilities for safely merging user headers with SDK-required ones, generating `x-request-id`/`x-correlation-id`, and ensuring no sensitive headers are logged or mutated.

* **`src/utils/url.ts`**
  URL composition primitives: validates `baseUrl`, concatenates `/app` and path, and builds query strings from `Record<string, string>` while ensuring no caps leak into URLs.

* **`src/utils/timeouts.ts`**
  Abstractions around `AbortController` and timers: sets up overall/connect/read timeouts and attaches the resulting `AbortSignal` to fetch.

* **`src/utils/problem.ts`**
  Parsers for canonical error envelopes: takes an HTTP response and tries to construct a `RonProblem` (or a safe fallback) based on JSON body, status, and headers.

* **`src/facets/manifest.ts`**
  Future-facing facet manifest helpers: defines TS types for facet TOML schema and small builders that can eventually output valid manifests for static/proxy facets.

---

### `test/` — tests & verification

* **`test/client.basic.test.ts`**
  Unit tests for the `Ron` client: path handling, header injection, correct `ok/problem` behavior across 2xx/4xx/5xx, and JSON encoding/decoding.

* **`test/transport.timeouts.test.ts`**
  Tests ensuring timeouts work correctly (overall/connect/read), that `AbortController` is triggered, and that timeouts map into appropriate `RonProblem` values.

* **`test/errors.mapping.test.ts`**
  Tests for error envelope parsing and mapping: canonical Problem JSON, malformed JSON bodies, and network errors all produce the expected `problem` shapes.

* **`test/security.no_secrets.test.ts`**
  Security-focused tests: verify that caps/tokens are never placed in URLs, never logged by default, and never show up in error messages or stack traces.

* **`test/schema.interop.test.ts`**
  Schema/interop tests: validate that DTOs produced/consumed by the SDK match the canonical schema (e.g., `u64` as string, pagination envelopes) and don’t drift.

---

### `docs/` — diagrams

* **`docs/ron-app-sdk-ts-arch.mmd`**
  Mermaid architecture diagram source: client app → `ron-app-sdk-ts` → `svc-gateway` → `omnigate` → app handlers, used to render SVGs for docs/README.

* **`docs/ron-app-sdk-ts-state.mmd`**
  State diagram source describing the request lifecycle (Idle → Requesting → Waiting → Completed/Failed), useful for reasoning about retries/backoff.

* **`docs/ron-app-sdk-ts-seq-app-call.mmd`**
  Sequence diagram source for a typical app call: app code → `Ron.request()` → gateway → omnigate → app → error/success response back to the caller.

---

