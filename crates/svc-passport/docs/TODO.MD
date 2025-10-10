

> notes
>
> * targets the service role described in your docs/README (HTTP is canonical; bus RPC is feature-gated).
> * prioritizes amnesia (RAM-first), minimal persistence, clean PQ-hybrid seam, and testability.
> * each Rust file aims to be **<300 LOC**; split whenever a module grows past that.

---

### file tree

```text
crates/
└── svc-passport/
    ├── Cargo.toml
    ├── README.md
    ├── CHANGELOG.md
    ├── LICENSE-APACHE
    ├── LICENSE-MIT
    ├── .gitignore
    ├── rust-toolchain.toml
    ├── src/
    │   ├── lib.rs
    │   ├── main.rs
    │   ├── config.rs
    │   ├── error.rs
    │   ├── metrics.rs
    │   ├── health.rs
    │   ├── bootstrap.rs
    │   ├── http/
    │   │   ├── mod.rs
    │   │   ├── router.rs
    │   │   ├── middleware.rs
    │   │   └── handlers/
    │   │       ├── issue.rs
    │   │       ├── verify.rs
    │   │       ├── revoke.rs
    │   │       ├── healthz.rs
    │   │       └── readyz.rs
    │   ├── dto/
    │   │   ├── mod.rs
    │   │   ├── issue.rs
    │   │   ├── verify.rs
    │   │   └── revoke.rs
    │   ├── token/
    │   │   ├── mod.rs
    │   │   ├── macaroon.rs
    │   │   ├── caveat.rs
    │   │   ├── attenuate.rs
    │   │   └── encode.rs
    │   ├── kms/
    │   │   ├── mod.rs
    │   │   ├── client.rs
    │   │   ├── rotation.rs
    │   │   └── keyslot.rs
    │   ├── verify/
    │   │   ├── mod.rs
    │   │   └── preflight.rs
    │   ├── state/
    │   │   ├── mod.rs
    │   │   ├── issuer.rs
    │   │   └── audit.rs
    │   ├── policy/
    │   │   ├── mod.rs
    │   │   └── eval.rs
    │   ├── bus/                    # feature = "bus-rpc"
    │   │   ├── mod.rs
    │   │   └── rpc.rs
    │   ├── telemetry/
    │   │   ├── mod.rs
    │   │   ├── tracing_init.rs
    │   │   └── prometheus.rs
    │   └── util/
    │       ├── mod.rs
    │       ├── time.rs
    │       ├── id.rs
    │       └── hashing.rs
    ├── tests/
    │   ├── api_issue.rs
    │   ├── api_verify.rs
    │   ├── api_revoke.rs
    │   ├── readiness.rs
    │   └── invariants.rs
    ├── loom/
    │   └── rotation_loom.rs
    ├── fuzz/
    │   └── fuzz_targets/
    │       ├── envelope_parse.rs
    │       └── caveat_mutation.rs
    ├── benches/
    │   └── issue_bench.rs
    ├── examples/
    │   └── issue_cli.rs
    ├── docs/
    │   ├── API.md
    │   ├── CONFIG.md
    │   ├── CONCURRENCY.md
    │   ├── GOVERNANCE.md
    │   ├── IDB.md
    │   ├── OBSERVABILITY.md
    │   ├── QUANTUM.md
    │   ├── TESTS.md
    │   ├── INTEROP.md
    │   ├── arch.mmd
    │   ├── sequence.mmd
    │   └── state.mmd
    ├── scripts/
    │   ├── inject_kms_latency.sh
    │   ├── rotate_under_load.sh
    │   └── load_issue_profile.sh
    ├── testing/
    │   ├── profiles/
    │   │   └── issue_80_20_local.json
    │   └── data/                  # any test fixtures
    ├── config/
    │   ├── local.env.example
    │   └── default.toml
    ├── docker/
    │   ├── Dockerfile
    │   └── docker-compose.yml
    └── .github/
        └── workflows/
            ├── ci.yml
            ├── render-mermaid.yml
            └── obs.yml
```

---

### what each file/dir is for (and why it’s separate)

#### top-level

* **Cargo.toml** — crate metadata, features (`pq-hybrid`, `bus-rpc`), pinned deps, benches/fuzz config.
* **README.md** — the contract for contributors/operators; diagrams, SLOs, and acceptance gates.
* **CHANGELOG.md** — SemVer history + migration notes for any breaking changes.
* **LICENSE-APACHE / LICENSE-MIT** — dual-license required by RustyOnions.
* **rust-toolchain.toml** — pin MSRV for reproducible builds.
* **.gitignore** — standard Rust + local env/secrets ignores.

#### `src/` (library-first surface even for services)

* **lib.rs** — re-exports the public surface: `Config`, `Metrics`, `Health`, `Bus` (if enabled), `wait_for_ctrl_c`. Keeps bin thin.
* **main.rs** — binary entry; parses env/flags, `bootstrap::run()` then awaits shutdown.
* **bootstrap.rs** — wires config, telemetry, KMS client, router, and background tasks (rotation, config watcher). Single place to “start the world”.
* **config.rs** — env + file config load, validation, hot-reload signal. Enforces TTL bounds and amnesia toggles.
* **error.rs** — unified error taxonomy (`ConfigError`, `KeyError`, `MintError`, …) with `IntoResponse` mappings for HTTP.
* **metrics.rs** — metric definitions/registries (counters, histograms, gauges) and helpers for request spans.
* **health.rs** — liveness/readiness state machine snapshot, used by `/healthz` and `/readyz`.

##### HTTP stack

* **http/mod.rs** — Axum integration glue: layers, uniform error responses, request IDs.
* **http/router.rs** — declares routes: `/v1/passport/issue`, `/verify`, `/revoke`, `/healthz`, `/readyz`, `/metrics`.
* **http/middleware.rs** — tracing, timeouts, body limits, optional auth for issuance endpoint.
* **http/handlers/**

  * **issue.rs** — POST `/v1/passport/issue`; validates DTO; assembles caveats; mints token via `token::macaroon`.
  * **verify.rs** — POST `/v1/passport/verify` preflight decoder; does **not** claim authoritativeness (delegates to `ron-auth` in production).
  * **revoke.rs** — POST `/v1/passport/revoke`; supports token ref or KID revocation hints; bumps epoch for propagation.
  * **healthz.rs** — fast, dependency-light check.
  * **readyz.rs** — keys loaded, rotation thread alive, policy cache (if enabled) ok.

##### DTOs (keep types small and gather serde here)

* **dto/mod.rs** — shared response envelopes, error bodies.
* **dto/issue.rs** — request/response types for `/issue` (includes `accept_algs`, `proof`).
* **dto/verify.rs** — request/response types for `/verify`.
* **dto/revoke.rs** — request/response types for `/revoke`.

##### Token logic (macaroon-style capabilities)

* **token/mod.rs** — trait `Signer`, `Passport`, and common invariants.
* **token/macaroon.rs** — signing/verification primitives, caveat encoding, detached sigs.
* **token/caveat.rs** — whitelist + parser for supported caveats (`svc`, `route`, `scope`, `region`, `budget.bytes`, `rate.rps`, `exp`).
* **token/attenuate.rs** — safe attenuation rules producing a *new* reduced token, never widening authority.
* **token/encode.rs** — base64/url-safe encoding/decoding, size enforcement.

##### KMS layer (separate to mock and chaos test)

* **kms/mod.rs** — interfaces for key access; `KmsProvider` abstraction.
* **kms/client.rs** — concrete `ron-kms` client with timeouts/retries.
* **kms/rotation.rs** — rotation worker; atomic swap of active keyslot; jittered retry; emits metrics and health transitions.
* **kms/keyslot.rs** — `KeySlot { kid, alg, not_before, not_after, … }` and zeroization-on-drop.

##### Preflight verification

* **verify/mod.rs** — trait for local decode/shape checks; shared warning enums.
* **verify/preflight.rs** — cheap structural checks to help clients/debug; always defers authority to `ron-auth`.

##### Service state & minimal audit (no PII)

* **state/mod.rs** — service-level shared state container; builders for components.
* **state/issuer.rs** — in-RAM registry of active/old KIDs; grace windows; epoch counter for revocation hints.
* **state/audit.rs** — minimal, non-identifying audit events (time, kid, op, audience_hash) honoring Amnesia Mode.

##### Optional policy hooks

* **policy/mod.rs** — interface for optional `ron-policy` checks.
* **policy/eval.rs** — adapter to call policy engine (feature-gated in Cargo).

##### Bus RPC (feature = `"bus-rpc"`)

* **bus/mod.rs** — topic names, message schemas (if/when enabled).
* **bus/rpc.rs** — experimental `mint_cap` RPC handlers; off by default.

##### Telemetry

* **telemetry/mod.rs** — exports telemetry setup surface.
* **telemetry/tracing_init.rs** — tracing subscriber wiring, log levels.
* **telemetry/prometheus.rs** — exporter `/metrics` and registry plumbing.

##### utilities

* **util/mod.rs** — shared helpers prelude.
* **util/time.rs** — TTL math, monotonic clocks, deadline helpers.
* **util/id.rs** — correlation/request IDs, token refs.
* **util/hashing.rs** — salted hashing of audience labels for privacy-preserving audit.

#### tests & verification

* **tests/api_issue.rs** — black-box tests for mint happy paths and TTL bounds.

* **tests/api_verify.rs** — preflight decode shape + warnings.

* **tests/api_revoke.rs** — revocation by token ref and by KID; epoch propagation expectations.

* **tests/readiness.rs** — readiness gates (no mint until key ready, block during rotation).

* **tests/invariants.rs** — property tests: attenuation never widens authority; caveat whitelist enforced.

* **loom/rotation_loom.rs** — loom concurrency model: rotation+mint interleavings; no double-swap; no leaks.

* **fuzz/fuzz_targets/envelope_parse.rs** — fuzz token envelope parsing/len limits.

* **fuzz/fuzz_targets/caveat_mutation.rs** — fuzz caveat parser against malformed/oversized inputs.

* **benches/issue_bench.rs** — criterion benches for `/issue` p50/p95 under local conditions.

* **examples/issue_cli.rs** — tiny CLI using HTTP client to request a passport (copy-pasteable for app teams).

#### docs

* **docs/*.md** — the source of truth mirrored from your combined docs: API, CONFIG, CONCURRENCY, QUANTUM, etc.
* **docs/arch.mmd, sequence.mmd, state.mmd** — mermaid source co-located with README diagrams; rendered in CI.

#### scripts / testing

* **scripts/inject_kms_latency.sh** — chaos: tc/netem wrapper to inject KMS jitter for weekly runs.

* **scripts/rotate_under_load.sh** — rotates KIDs while issuing to probe race/backoff behavior.

* **scripts/load_issue_profile.sh** — drives a load profile from `testing/profiles/*`.

* **testing/profiles/issue_80_20_local.json** — canonical local load profile (80% issue, 20% verify/revoke mix).

* **testing/data/** — any fixtures (e.g., canned tokens for decode tests).

#### config

* **config/local.env.example** — env template with sane defaults (BIND, METRICS_ADDR, DEFAULT_TTL_SECS=900, MAX_TTL_SECS=3600, AMNESIA=true).
* **config/default.toml** — optional file-based config used if present; env wins.

#### containerization

* **docker/Dockerfile** — distroless or alpine with non-root user; exposes service port + metrics.
* **docker/docker-compose.yml** — local stack wiring to `ron-kms`/`ron-auth` test doubles.

#### CI

* **.github/workflows/ci.yml** — fmt, clippy (`-D warnings`), tests, fuzz smoke, llvm-cov ≥90%, cargo-deny, benches (threshold gates).
* **.github/workflows/render-mermaid.yml** — render `docs/*.mmd` to SVG artifacts.
* **.github/workflows/obs.yml** — run PromQL checks against canary run; fail on SLO regressions.

---

## why this structure works (and how it maps to your docs)

* **Clear separation of concerns:** HTTP surface (router/handlers) is distinct from **token** logic and **KMS** access, so we can fuzz/bench internals without a server in the way.
* **Concurrency safety:** rotation and mint live behind `kms::rotation` + `state::issuer` with a **single-writer discipline** and loom tests dedicated to swap correctness.
* **PII avoidance baked in:** `state::audit` hashes audiences with a salt in **util/hashing** and never logs raw identifiers.
* **Observability-first:** `metrics.rs`, `telemetry/` and **health/readiness** are not mixed with business logic; handlers are thin.
* **Future-proof PQ seam:** KID/alg handling sits in `kms::keyslot` so `pq-hybrid` is a feature flip, not a refactor.
* **Modularity & maintainability:** each module stays small (<300 LOC). as complexity grows, we split further (e.g., `/issue` path can move builder bits into `token/`).

