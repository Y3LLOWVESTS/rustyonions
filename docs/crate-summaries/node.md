---

crate: node
path: crates/node
role: lib (primarily a CLI binary with a tiny placeholder lib)
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A developer-facing CLI that runs a single-process RustyOnions **overlay listener** and offers **PUT/GET** helpers (TCP and optional Tor), plus a tiny JSON stats socket for quick local testing.

## 2) Primary Responsibilities

* **Serve**: start an overlay listener bound to a local TCP address (or Tor HS when enabled) using the embedded store path.
* **Client ops**: PUT a local file and GET a blob by `b3:<hex>` from a listener for demos/smoke tests.
* **Dev stats**: expose minimal, ad-hoc JSON counters (bytes in/out, store size) for local inspection.

## 3) Non-Goals

* Not a microkernel-managed “service”; no supervision tree or bus integration.
* Not the production gateway (no HTTP API surface, no auth, no multi-tenant features).
* Not a persistence authority beyond the overlay’s own sled store; no index, no naming resolution.

## 4) Public API Surface

* **Re-exports**: none (the lib target is intentionally empty/placeholder).
* **Key functions (binary/CLI)**

  * `Serve { bind, transport, store_db }` → runs overlay listener (`TCP` today; legacy Tor path in old commands).
  * `Put { to, path }` → connect and upload file; prints content hash.
  * `Get { from, hash, out }` → fetch by hex hash; write to file.
* **Legacy (unwired code under `src/commands/*`)**

  * `serve(config, transport = "tcp"|"tor")` using `TcpTransport` or `ArtiTransport`, spawns a tiny metrics socket on `dev_inbox_addr`.
  * `put/get(..., transport)`, `tor_dial`, `init` (writes a default `config.toml`), `stats_json`.
* **Events / HTTP / CLI**: CLI only (clap). The dev stats socket speaks a minimal HTTP/1.1 response with a JSON body; there’s **no** Prometheus endpoint here.

## 5) Dependencies & Coupling

* **Internal crates**

  * `overlay` (tight, replaceable: **yes**): provides `run_overlay_listener`, `client_put/get`, `Store` (sled) and transport-agnostic variants in legacy path.
  * `common` (loose, replaceable: **yes**): reads `Config` (used by legacy commands).
  * `transport` (loose, replaceable: **yes**): TCP transport; Tor control helpers in legacy path.
* **External crates (top)**

  * `tokio` (multithread + macros + net + signal) — runtime for Ctrl-C and async clients.
  * `clap` — CLI parser (derive).
  * `tracing`, `tracing-subscriber` — logs and `RUST_LOG` env filter.
  * `color-eyre`/`anyhow` — human-friendly error context.
  * *(Legacy code)* `arti_transport` — Tor SOCKS + HS publish; currently **not in Cargo.toml**, indicating drift.
* **Runtime services**:

  * **Network**: TCP listener on `bind`; optional Tor SOCKS + control when using the legacy path.
  * **Storage**: sled DB directory (overlay store).
  * **OS**: signals (Ctrl-C), threads for periodic counter logs in legacy serve.

## 6) Config & Feature Flags

* **CLI flags (current `main.rs`)**: `--bind`, `--transport` (accepts only `tcp`), `--store-db`.
* **Legacy config (via `common::Config`)**:

  * `data_dir`, `overlay_addr`, `dev_inbox_addr`, `socks5_addr`, `tor_ctrl_addr`, `chunk_size`, `connect_timeout_ms`, optional `hs_key_file` (via `RO_HS_KEY_FILE` env).
* **Cargo features**: none today (opportunity: `tor`, `metrics`, `legacy-commands`).
* **Env**: `RUST_LOG` for logging; `RO_HS_KEY_FILE` honored by legacy Tor serve.

## 7) Observability

* **Current**: `tracing` logs; if using legacy `serve`, a tiny TCP socket on `dev_inbox_addr` returns JSON: `{ store: {n_keys,total_bytes}, transport: {total_in,total_out} }`.
* **Gaps**: no Prometheus metrics, no standardized `/healthz`/`/readyz` hooks, and the JSON stats endpoint is non-standard and ad-hoc.

## 8) Concurrency Model

* **Serve path (current)**: overlay listener runs in its own async stack (inside `overlay`); the CLI awaits Ctrl-C via `tokio::signal::ctrl_c()`.
* **Legacy path**: spawns a thread every 60s to log counters; simple accept loop for the stats socket (blocking `TcpListener`).
* **Backpressure**: bounded by overlay/transport internals; CLI PUT/GET calls are synchronous from the user’s perspective.
* **Timeouts/Retries**: client ops rely on overlay/transport defaults; legacy Tor path sets connect timeout via config.

## 9) Persistence & Data Model

* **Store**: sled, rooted at `--store-db` (current) or `config.data_dir` (legacy), chunk size configurable via `chunk_size`.
* **Schema**: managed by `overlay::Store` (content-addressed blobs); node does not define its own keys.
* **Artifacts**: files written during GET; `config.toml` scaffolding via `init`.

## 10) Errors & Security

* **Error taxonomy**: `anyhow` contexts for CLI failures; PUT/GET surface “NOT FOUND” vs IO/transport errors; legacy Tor path differentiates socket vs Tor control errors.
* **Security posture**: no TLS for TCP; optional Tor HS for privacy when legacy serve is used; no authentication/authorization; no secret management beyond HS key file env.
* **PQ**: N/A here—no crypto primitives in this crate proper; relies on transport/overlay choices.

## 11) Performance Notes

* **Hot paths**: overlay read/write (delegated), file IO for PUT/GET, simple JSON stats generation.
* **Targets**: dev-grade—suitable for local smoke (tens to low hundreds of MB/s on loopback via TCP). Tor adds expected latency (hundreds of ms).
* **Tips**: align `chunk_size` to overlay defaults; put sled DB on SSD; avoid running two processes against the same DB (sled lock).

## 12) Tests

* **Present**: none visible in the package; behavior is exercised indirectly by smoke scripts (`gwsmoke`, etc.) outside this crate.
* **Recommended**:

  * **Integration**: spin a temporary listener on an ephemeral port, PUT/GET a temp file (happy path + 404).
  * **Transport matrix**: behind a `tor` feature, mock or noop Arti to validate CLI dispatch without requiring a Tor daemon.
  * **Stats**: ensure the JSON stats socket returns valid JSON and includes monotonic counters.
  * **Config**: `init` writes example config and refuses overwrite.

## 13) Improvement Opportunities

**A. Eliminate drift (highest priority)**

* Two CLIs coexist: the **new** minimal `main.rs` (TCP-only) and a **legacy** `src/cli.rs` + `src/commands/*` tree (TCP+Tor+stats).
* Choose one surface:

  * **Option 1 (lean)**: keep `main.rs` only; delete `src/cli.rs` and `src/commands/*`.
  * **Option 2 (featureful)**: merge the legacy commands into the new clap surface and add features:

    * `--transport {tcp,tor}` with a `tor` Cargo feature, bringing `arti_transport` into `Cargo.toml`.
    * Promote the stats socket to standard `/metrics` Prometheus (reuse `ron-kernel::Metrics` patterns).
* At minimum, remove unused module stubs in `lib.rs` (today it exports an empty `cli` module just to avoid external breakage).

**B. Config & feature hygiene**

* Introduce a crate feature `tor` (default off). When enabled, compile the Arti path; otherwise exclude Tor code.
* Replace ad-hoc env var `RO_HS_KEY_FILE` with a documented config key (`hs_key_file`) and pass it through explicitly.

**C. Observability**

* Replace dev JSON stats with standard **Prometheus** counters/gauges/histograms and `/healthz`/`/readyz`.
* Publish overlay byte counters via the established metrics registry (labels: transport=`tcp|tor`).

**D. Safer sled access**

* Warn or refuse to open the sled store if another process holds the lock; the legacy `stats_json` tries to cope by hitting the stats socket—make this the **only** code path when locked to avoid footguns.

**E. Boundaries & naming**

* Consider renaming the package/binary to `ronode-dev` or `ronode-cli` to signal its dev-only intent vs. kernel services.

**F. Remove dead code**

* If Option 1 is chosen, delete `src/cli.rs` and `src/commands/*` and add a README note that Tor testing moved to a separate example or to `svc-overlay` with flags.

## 14) Change Log (recent)

* **2025-09-14** — Review found **CLI drift** (TCP-only new CLI vs legacy TCP/Tor commands), non-standard stats endpoint, and missing `arti_transport` pin; proposed feature-gated unification.
* **2025-09-05 … 09-12** — Used in local e2e/smoke runs; JSON stats endpoint referenced by `stats_json`.

## 15) Readiness Score (0–5 each)

* **API clarity:** 2 — binary interface is clear, but duplicated/legacy code confuses the crate’s surface.
* **Test coverage:** 1 — no direct tests; relies on external smoke scripts.
* **Observability:** 2 — logs exist; stats endpoint is non-standard; no Prometheus/healthz.
* **Config hygiene:** 2 — two config styles (flags vs file), ad-hoc env var, missing Tor feature pin.
* **Security posture:** 2 — plain TCP; Tor path exists but not unified; no auth.
* **Performance confidence:** 3 — overlay delegates the heavy lifting; adequate for dev.
* **Coupling (lower is better):** 2 — coupled to `overlay` (intentional) but otherwise standalone from kernel services.

