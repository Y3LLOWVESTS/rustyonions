**ron-policy**

```text
crates/
└─ ron-policy/
   ├─ Cargo.toml
   ├─ README.md
   ├─ CHANGELOG.md
   ├─ LICENSE-APACHE
   ├─ LICENSE-MIT
   ├─ CODEOWNERS
   ├─ rust-toolchain.toml
   ├─ deny.toml
   ├─ .cargo/
   │  └─ config.toml
   ├─ src/
   │  ├─ lib.rs
   │  ├─ model.rs
   │  ├─ errors.rs
   │  ├─ features.rs
   │  ├─ ctx/
   │  │  ├─ mod.rs
   │  │  ├─ normalize.rs
   │  │  └─ clock.rs
   │  ├─ parse/
   │  │  ├─ mod.rs
   │  │  ├─ json.rs
   │  │  ├─ toml.rs
   │  │  └─ validate.rs
   │  ├─ engine/
   │  │  ├─ mod.rs
   │  │  ├─ eval.rs
   │  │  ├─ index.rs
   │  │  ├─ obligations.rs
   │  │  ├─ reason.rs
   │  │  └─ metrics.rs
   │  └─ explain/
   │     ├─ mod.rs
   │     └─ trace.rs
   ├─ docs/
   │  ├─ IDB.md
   │  ├─ INTEROP.md
   │  ├─ CONFIG.md
   │  ├─ SECURITY.md
   │  ├─ OBSERVABILITY.md
   │  ├─ PERFORMANCE.md
   │  ├─ CONCURRENCY.md
   │  ├─ GOVERNANCE.md
   │  └─ QUANTUM.md
   ├─ schema/
   │  └─ policybundle.schema.json
   ├─ specs/
   │  └─ governance.md
   ├─ tests/
   │  ├─ unit_model_serde_strict.rs
   │  ├─ unit_eval_determinism.rs
   │  ├─ unit_tighten_only.rs
   │  ├─ unit_churn_protection.rs
   │  ├─ golden_reasons.rs
   │  ├─ vectors/
   │  │  ├─ deny_region.json
   │  │  ├─ body_too_large.json
   │  │  └─ decompress_guard.json
   │  └─ helpers/
   │     └─ bundle_load.rs
   ├─ benches/
   │  └─ eval_throughput.rs
   ├─ fuzz/
   │  ├─ fuzz_targets/
   │  │  ├─ fuzz_bundle_parse.rs
   │  │  └─ fuzz_eval.rs
   │  └─ Cargo.toml
   ├─ examples/
   │  └─ minimal_allow_deny.rs
   ├─ .github/
   │  └─ workflows/
   │     └─ policy.yml
   └─ scripts/
      └─ ci_invariants.sh
```

### What each file/directory is for (and why it’s small)

* **Cargo.toml** — Library crate metadata + MSRV + features (`json`, `toml`, `geo`, `quota`, `explain`). No service deps (tokio/axum) to keep purity and tiny compile units. Perf/CI knobs live behind features. 
* **README.md** — Your canonical quickstart, public API sketch (doc-tested snippet), module map, env toggles (`RON_POLICY_STRICT`, `RON_POLICY_EXPLAIN`), metrics names, and perf SLOs.    
* **CHANGELOG.md** — SemVer-tracked wire/schema changes; denial reason changes must be called out (ops contract).
* **CODEOWNERS** — Review gates for SEC/GOV surfaces (policy schema/reasons/validators need OPO/SPR/RM eyes).
* **rust-toolchain.toml** — Pin for CI parity; bump = breaking per MSRV section. 
* **deny.toml** — cargo-deny clean per workspace policy; protects supply chain. 
* **.cargo/config.toml** — Lint wall (`-D warnings`, select clippy lints) to keep drift low. 

#### `src/` (hot path, but each file is tiny)

* **lib.rs** — Re-exports only (`engine::Evaluator`, `parse::{from_json_str, from_toml_str}`, `ctx::RequestCtx`, `model::*`, `Decision`). Keeps crate’s public surface crisp, matches README examples. 
* **model.rs** — DTOs for the *policy bundle* (versioned, content-addressed `id_b3`, residency/quotas/routes/features/metadata) with `#[serde(deny_unknown_fields)]` to enforce strictness. Keep predicates/actions simple; stable denial reasons enumerated separately.  
* **errors.rs** — `ParseError`, `ValidationError`, `EvalError` taxonomy and mapping expectations (service: 400/409/422/503). Tiny enums to avoid bloat.  
* **features.rs** — Doc-only mapping of cargo features and their compile-time effects; avoids scattering cfgs.
* **ctx/** — Request context (builder + normalizers), pure and `Send+Sync`.

  * `mod.rs`: `RequestCtx` builder (tenant/region/method/path/bytes_in/out/cap scope/etc.).
  * `normalize.rs`: upper-case methods, canonicalize paths, ensure deterministic inputs.
  * `clock.rs`: caller-provided vs system-time selection (library prefers “provided”). 
* **parse/** — Zero-I/O parsers + validators.

  * `mod.rs`: `from_json_str` / `from_toml_str` API; always normalize to canonical JSON before hashing.
  * `json.rs` / `toml.rs`: thin decode shims (TOML as authoring convenience).
  * `validate.rs`: **tighten-only**, **churn protection**, perf budgets, schema strictness. Each check as a tiny function you can test in isolation.  
* **engine/** — Pure evaluator, kept in narrow files.

  * `mod.rs`: `Evaluator` constructor, `set_explain(bool)` toggle.
  * `eval.rs`: core `evaluate(&RequestCtx) -> Decision` loop (no I/O).
  * `index.rs`: precomputed route/class indices; immutable; swapped atomically via `Arc`. 
  * `obligations.rs`: structured obligations (e.g., **DegradeWritesFirst**, **RequireProof{…}**) that hosts interpret; no ECON logic here. 
  * `reason.rs`: canonical, *stable* denial reasons as tiny enums mapped to strings (`region.denied`, `body.too_large`, `decompress.guard`, …). 
  * `metrics.rs`: **traits only** for hosts to export counters/histograms with agreed names (`policy_decisions_total{service,decision,rule}`, etc.). 
* **explain/** — Optional decision trace (behind env/feature).

  * `mod.rs`: opt-in capture guard (allocates only if enabled).
  * `trace.rs`: compact path of matched rules/branches for debugging and ops explain. Toggle via `RON_POLICY_EXPLAIN` or `--features explain`. 

#### `docs/` (you already wrote these; we keep them split by topic)

* **IDB.md** — MUSTs (pure lib, tighten-only, deterministic eval, amnesia-aware) and their CI gates. 
* **INTEROP.md** — Canonical wire = JSON; TOML for authoring; no endpoints; bus/event expectations are *service* contracts. 
* **CONFIG.md** — Explicit env/config matrix (library is pure; env keys are documented for **consumers**), including strict-tighten-only + break-change settings aligned with Hardening.  
* **SECURITY.md** — Fail-closed, no secrets, capability-relative, amnesia-safe (no disk). 
* **OBSERVABILITY.md** — Reason codes, metric names/labels, tracing span fields (`policy.evaluate`, `matched_rule`, etc.). 
* **PERFORMANCE.md** — P50/P99 targets and CI regression gates; guidance on index sizing and route scoping.  
* **CONCURRENCY.md** — Snapshot-swap pattern and “evaluate first, then await I/O” sequence for **consumers**.  
* **GOVERNANCE.md** — Roles, multi-sig, freeze/rollback runbooks, auditability & economic boundaries. 
* **QUANTUM.md** — PQ posture is advisory hooks only (no crypto here); consumers/KMS own keys.

#### `schema/` & `specs/`

* **schema/policybundle.schema.json** — Versioned JSON Schema for canonical wire; keeps non-Rust consumers honest. 
* **specs/governance.md** — The signed-bundle/governance notes your docs reference for interop. 

#### `tests/` (kept out of `src/` to protect hot paths)

* **unit_model_serde_strict.rs** — Unknown fields → error when strict; aligns with env default. 
* **unit_eval_determinism.rs** — Same `(bundle, ctx, now)` ⇒ same `Decision`. 
* **unit_tighten_only.rs** — Rejects attempts to relax hard caps (1 MiB body, ≤10× decompress). 
* **unit_churn_protection.rs** — Residency widening requires `break_change=true` + runbook; otherwise error. 
* **golden_reasons.rs** — Stable denial reasons snapshots (e.g., `region.denied`, `body.too_large`, `decompress.guard`). 
* **tests/vectors/** — Canonical `(bundle, ctx) → reason` fixtures exercised in CI to keep behavior stable. 

#### `benches/` & `fuzz/`

* **benches/eval_throughput.rs** — Criterion bench to guard p99 ≤ 100 µs (CI fails on >10% regression). 
* **fuzz/fuzz_targets/** — `fuzz_bundle_parse.rs` and `fuzz_eval.rs` exactly as your README prescribes. 

#### `examples/` (DX only)

* **minimal_allow_deny.rs** — The doc-tested snippet as a runnable example, matching README’s API. 

#### CI & scripts

* **.github/workflows/policy.yml** — Lint + test + deny + benches (thresholds) + fuzz smoke.
* **scripts/ci_invariants.sh** — Greps to enforce tighten-only + churn hints (e.g., reject residency widening without `break_change`), mirrors examples in docs. 

---

### Canon alignment & boundaries (why this shape)

* **Pure library, no endpoints/I-O** — ron-policy is used *by services* (gateway, index, storage, overlay). This crate only parses, validates, evaluates, and exposes *names* for metrics/tracing.  
* **Strict schema + explain toggles** — `RON_POLICY_STRICT` and `RON_POLICY_EXPLAIN` are host-read flags; library behavior is documented for consumers. 
* **Tighten-only & hardening bounds** — Policy cannot relax platform limits (OAP/HTTP body cap 1 MiB; decompress ≤ 10×). CI lints included. 
* **Canon wire = JSON; TOML for authoring** — We normalize TOML → JSON before hashing; `id_b3` binds content.  
* **SEC/GOV mapping** — ron-policy’s concerns are Security & Governance; it stays out of ECON/transport. 
* **Small files, clear seams** — Each rule/validator/obligation is in its own short module to keep compile times low and unit tests focused (matches your “no large code files” rule). The snapshot-swap pattern ensures consumers don’t hold locks across awaits. 
