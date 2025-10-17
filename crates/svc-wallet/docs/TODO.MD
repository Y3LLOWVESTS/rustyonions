
---

# File tree (scaffold only, no code)

```
svc-wallet/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ .cargo/
│  └─ config.toml
├─ .gitignore
├─ LICENSE-APACHE
├─ LICENSE-MIT
├─ README.md
├─ CHANGELOG.md
├─ configs/
│  ├─ svc-wallet.example.toml
│  └─ policies.sample.json
├─ docs/
│  ├─ ALL_DOCS.md
│  ├─ openapi/
│  │  └─ svc-wallet.yaml
│  ├─ arch.mmd
│  ├─ sequence.mmd
│  └─ state.mmd
├─ scripts/
│  ├─ dev-run.sh
│  ├─ chaos-ledger-stall.sh
│  └─ reload-policy.sh
├─ .github/
│  └─ workflows/
│     ├─ ci.yml
│     ├─ render-mermaid.yml
│     ├─ fuzz.yml
│     └─ loom.yml
├─ benches/
│  ├─ balance_read.rs
│  └─ transfer_commit.rs
├─ fuzz/
│  └─ fuzz_targets/
│     ├─ dto_spend_parser.rs
│     ├─ spend_request_roundtrip.rs
│     └─ sequence_machine.rs
├─ testing/
│  └─ load/
│     ├─ k6-transfer.js
│     └─ vegeta-balance.sh
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ supervisor.rs
│  ├─ readiness.rs
│  ├─ config.rs
│  ├─ errors.rs
│  ├─ metrics.rs
│  ├─ middleware/
│  │  ├─ mod.rs
│  │  ├─ request_id.rs
│  │  ├─ tracing_log.rs
│  │  ├─ limits.rs
│  │  ├─ decompress_cap.rs
│  │  ├─ timeouts.rs
│  │  ├─ rate_limit.rs
│  │  └─ shedder.rs
│  ├─ routes/
│  │  ├─ mod.rs
│  │  ├─ health.rs
│  │  ├─ metrics.rs
│  │  └─ v1/
│  │     ├─ mod.rs
│  │     ├─ balance.rs
│  │     ├─ issue.rs
│  │     ├─ transfer.rs
│  │     ├─ burn.rs
│  │     └─ receipt.rs
│  ├─ dto/
│  │  ├─ mod.rs
│  │  ├─ requests.rs
│  │  ├─ responses.rs
│  │  └─ errors.rs
│  ├─ auth/
│  │  ├─ mod.rs
│  │  └─ caps.rs
│  ├─ policy/
│  │  ├─ mod.rs
│  │  └─ enforce.rs
│  ├─ ledger/
│  │  ├─ mod.rs
│  │  ├─ client.rs
│  │  └─ types.rs
│  ├─ accounting/
│  │  ├─ mod.rs
│  │  └─ client.rs
│  ├─ seq/
│  │  ├─ mod.rs
│  │  └─ nonce.rs
│  ├─ idem/
│  │  ├─ mod.rs
│  │  └─ store.rs
│  ├─ cache/
│  │  ├─ mod.rs
│  │  ├─ balance_cache.rs
│  │  └─ invalidator.rs
│  └─ util/
│     ├─ blake3_receipt.rs
│     ├─ headers.rs
│     └─ parsing.rs
└─ tests/
   ├─ harness.rs
   ├─ api_contract.rs
   ├─ i_1_doublespend.rs
   ├─ i_2_nonnegativity.rs
   ├─ i_3_conservation.rs
   ├─ i_4_caps_policy.rs
   ├─ i_5_dto_hygiene.rs
   ├─ i_7_ledger_primacy.rs
   ├─ i_8_observability.rs
   ├─ i_9_amnesia.rs
   ├─ i_11_transport_bounds.rs
   ├─ i_12_overflow_ceilings.rs
   ├─ readiness_and_shedding.rs
   ├─ vectors_receipt_hash.rs
   └─ loom_invariants.rs
```

---

## What each file/dir is for (one-liners)

### Root & configs

* **Cargo.toml** — Crate manifest; pins MSRV, features (`tls`, `arti`, `legacy_pay`) and workspace deps; HTTP is the public API, Rust surface stays private. 
* **rust-toolchain.toml** — Locks MSRV to 1.80.0 as per docs. 
* **.cargo/config.toml** — Local build/test aliases (e.g., extra clippy lints, llvm-cov) per BUILDPLAN gates. 
* **README.md** — Canonical overview, endpoints, SLOs, chaos/fuzz/loom commands, and acceptance gates (already authored). 
* **CHANGELOG.md** — Records API/behavior changes; `/v2` bumps required on breaking HTTP changes. 
* **configs/svc-wallet.example.toml** — Example runtime configuration: bind addresses, downstream URLs, amnesia/PQ toggles, reload grace.  
* **configs/policies.sample.json** — Sample capability/policy doc used by tests & reload drills. 

### Docs & diagrams

* **docs/ALL_DOCS.md** — Your combined canon; kept beside code for single-source auditing.
* **docs/openapi/svc-wallet.yaml** — Source of truth for the HTTP contract (`/v1` paths and schemas). 
* **docs/arch.mmd · sequence.mmd · state.mmd** — Mermaid sources for architecture, a common transfer path, and lifecycle; rendered in CI. 

### Scripts & CI

* **scripts/dev-run.sh** — Spins the service with sane defaults (metrics on random port); developer convenience.
* **scripts/chaos-ledger-stall.sh** — Invokes the chaos drill matching `/readyz` write-shedding KPI. 
* **scripts/reload-policy.sh** — Sends SIGHUP or `/admin/reload` to exercise policy hot-reload. 
* **.github/workflows/ci.yml** — fmt/clippy/tests/coverage/benches & API diff gates. 
* **render-mermaid.yml** — Renders `.mmd → .svg` in CI as per README policy.
* **fuzz.yml** — Nightly fuzz smoke (≤60s per target) plus long-haul job. 
* **loom.yml** — Concurrency test job (RUSTFLAGS `--cfg loom`). 

### Perf & fuzz

* **benches/** — Criterion microbenches for hot paths (balance read; transfer commit). Ties to SLO/regression baselines. 
* **fuzz/fuzz_targets/dto_spend_parser.rs** — DTO parser fuzz target guarding payload size/ratio & panics. 
* **…/spend_request_roundtrip.rs** — Valid write requests roundtrip without loss; rejects malformed. 
* **…/sequence_machine.rs** — Fuzzes nonce/idempotency sequencing under replays/out-of-order. 
* **testing/load/** — k6/vegeta harnesses to reproduce p95/p99 targets in docs. 

### Core service (src/)

* **main.rs** — Binary entrypoint: builds router/middleware, spawns supervised tasks, exposes `/healthz`, `/readyz`, `/metrics`. Keep Rust API private (service only).  
* **lib.rs** — Empty/minimal to avoid accidental public surface (service, not library). 
* **supervisor.rs** — Owns shutdown/watch, child task lifecycle, and readiness snapshots (lock-free reads). 
* **readiness.rs** — Implements green→degraded transitions; sheds **writes first** when upstream stalls. 
* **config.rs** — Env/flag parsing, validation, and SIGHUP reload integration (grace window). 
* **errors.rs** — Typed error taxonomy → HTTP mapping (409 conflicts, 401/403, 429, 503 deterministic). 
* **metrics.rs** — Prometheus registry + canonical metrics (latency histogram, conflicts, quota, policy version, pq_mode). 
* **middleware/** —

  * **request_id.rs** — Correlation ID propagation.
  * **tracing_log.rs** — JSON logs with high-signal fields (`route`, `status`, `account`).
  * **limits.rs** — Enforces body ≤1 MiB. 
  * **decompress_cap.rs** — Enforces ≤10× decompression ratio. 
  * **timeouts.rs** — Idle/read/write timeouts on all I/O. 
  * **rate_limit.rs** — Local per-route or token bucket; sets Retry-After headers when applicable.
  * **shedder.rs** — Readiness-aware writer shedding under pressure (commit queue saturation). 
* **routes/** —

  * **health.rs** — `/healthz` liveness.
  * **metrics.rs** — `/metrics` exporter.
  * **v1/** — HTTP contract surface (stable):

    * **balance.rs** — `GET /v1/balance` (bounded staleness cache). 
    * **issue.rs** — `POST /v1/issue` (Idempotency-Key required). 
    * **transfer.rs** — `POST /v1/transfer` (idempotent writes; deterministic receipts). 
    * **burn.rs** — `POST /v1/burn` (Idempotency-Key; policy-gated). 
    * **receipt.rs** — `GET /v1/tx/{txid}` (short-TTL cache). 
* **dto/** — JSON DTOs with `serde(deny_unknown_fields)`; string-encoded `u128` amounts; stable schema mirrors OpenAPI. 

  * **requests.rs · responses.rs · errors.rs** — Wire types & structured error body.
* **auth/** — Capability verification (macaroons); scope checks for `{account, asset, action, ceilings}`. 

  * **caps.rs** — Primitive checkers and error mapping (401/403).
* **policy/** — Policy client + cache; supports SIGHUP reload and grace window. 

  * **enforce.rs** — Admission control (limits, ceilings, allowed opcodes).
* **ledger/** — Client to `ron-ledger` for append-commit and receipt fetch; retry/circuit settings. 

  * **types.rs** — Minimal types for commit/receipt mapping.
* **accounting/** — Counters/metering interactions (non-authoritative). 
* **seq/** — Per-account nonce sequencing (single-writer discipline). 
* **idem/** — Idempotency-Key store (bounded, amnesia-aware); deterministic replay semantics. 
* **cache/** — Balance snapshot cache + invalidator listening to ledger bus (losy-ok). 
* **util/** —

  * **blake3_receipt.rs** — Short receipt hash helper.
  * **headers.rs** — Header parsing (Idempotency-Key, Retry-After).
  * **parsing.rs** — Safe parsers with size/ratio guards. 

### Tests (acceptance/gates)

* **tests/harness.rs** — Spins in-process server with example config; boots doubles/mocks for downstreams. 
* **tests/api_contract.rs** — Exercises OpenAPI parity for required paths & status codes. 
* **tests/i_*.rs** — Invariant suites (no doublespends, non-negativity, conservation; caps; DTO hygiene; ledger primacy; amnesia; transport bounds; ceilings). 
* **tests/readiness_and_shedding.rs** — Verifies writes are shed during upstream stall and recover correctly. 
* **tests/vectors_receipt_hash.rs** — Golden vectors for receipt determinism.
* **tests/loom_invariants.rs** — Loom model for sequencing/idempotency races (dev-only). 

---

## Why this layout?

* **Binary-first, private Rust surface** — we keep code modular yet *not* a public library; HTTP + OpenAPI is the consumer contract. 
* **Backpressure-first concurrency** — bounded queues, single-writer sequences, fail-fast timeouts; supervisor/readiness separated. 
* **Rigor baked in** — fuzz/loom/chaos and SLO harnesses live beside code with CI gates mirroring ALL_DOCS. 
* **Security posture** — amnesia mode honored; size/ratio caps; capability checks occur early in the chain.  

