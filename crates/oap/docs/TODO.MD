
---

# Proposed file tree (no code, just structure)

```
crates/
└─ oap/
   ├─ Cargo.toml
   ├─ rust-toolchain.toml
   ├─ README.md
   ├─ CHANGELOG.md
   ├─ LICENSE-APACHE
   ├─ LICENSE-MIT
   ├─ .gitignore
   ├─ .cargo/
   │  └─ config.toml
   ├─ src/
   │  ├─ lib.rs
   │  ├─ consts.rs
   │  ├─ prelude.rs
   │  ├─ envelope.rs
   │  ├─ frame.rs
   │  ├─ error.rs
   │  ├─ metrics.rs
   │  ├─ seq.rs
   │  ├─ parser/
   │  │  ├─ mod.rs
   │  │  ├─ config.rs
   │  │  └─ state.rs
   │  └─ writer/
   │     ├─ mod.rs
   │     └─ config.rs
   ├─ tests/
   │  ├─ conformance.rs
   │  ├─ split_need_more.rs
   │  ├─ ack_algebra.rs
   │  ├─ config_validation.rs
   │  ├─ metrics_mapping.rs
   │  └─ vectors/
   │     └─ oap1/
   │        ├─ 01_hello_hello.json
   │        ├─ 02_start_data_ack_end.json
   │        ├─ 10_oversize_frame.bin
   │        ├─ 11_missing_obj.json
   │        ├─ 12_unknown_envelope.bin
   │        ├─ 13_truncated_pair.bin
   │        └─ 14_credit_violation.json
   ├─ benches/
   │  ├─ decode_happy.rs
   │  ├─ decode_pathological.rs
   │  └─ encode_ack.rs
   ├─ fuzz/
   │  ├─ fuzz_targets/
   │  │  ├─ parser_fuzz.rs
   │  │  ├─ header_fuzz.rs
   │  │  └─ ack_fuzz.rs
   │  ├─ corpora/
   │  │  ├─ parser_fuzz/
   │  │  ├─ header_fuzz/
   │  │  └─ ack_fuzz/
   │  └─ artifacts/   (CI-minimized crashes; regression corpus)
   ├─ docs/
   │  ├─ specs/
   │  │  └─ oap-1.md
   │  ├─ api-history/
   │  │  └─ oap/
   │  │     └─ 1.0.0.txt
   │  ├─ arch.mmd
   │  ├─ seq.mmd
   │  ├─ state.mmd
   │  ├─ API.md
   │  ├─ CONCURRENCY.md
   │  ├─ CONFIG.md
   │  ├─ GOVERNANCE.md
   │  ├─ IDB.md
   │  ├─ INTEROP.md
   │  ├─ OBSERVABILITY.md
   │  ├─ PERFORMANCE.md
   │  ├─ QUANTUM.md
   │  ├─ RUNBOOK.md
   │  ├─ SECURITY.md
   │  └─ TESTS.md
   ├─ scripts/
   │  └─ perf_compare.sh
   └─ .github/
      └─ workflows/
         ├─ ci.yml
         ├─ render-mermaid.yml
         ├─ public-api.yml
         ├─ fuzz-nightly.yml
         └─ perf-bench.yml
```

---

## What each file/folder is for (and why it exists)

### Root

* **Cargo.toml** — Minimal deps surface; defines features like `pq`, `serde`, `dev-vectors`. Keeps OAP a **pure lib** (no TLS/IO deps), matching scope in README + IDB. 
* **rust-toolchain.toml** — Pins **MSRV 1.80.0** for stability and CI clarity. 
* **README.md** — Your role-aware, proof-gated README (Mermaid + acceptance gates). Lives beside code to enforce the invariants and “prove it” culture. 
* **CHANGELOG.md** — SemVer discipline; any wire-affecting change (frame cap, envelope set) = **major**. 
* **LICENSE-*** — Dual license canon.
* **.cargo/config.toml** — Lints as walls (e.g., `-D warnings`), mirrors CI expectations. 

### `src/` (small, modular, testable)

* **lib.rs** — Re-exports the public surface (`envelope`, `frame`, `parser`, `writer`, `error`, `metrics`, `consts`, `seq`, `prelude`) exactly as the API snapshot specifies. Keeps the surface auditable and small. 
* **consts.rs** — Protocol-frozen constants (`VERSION`, `MAX_FRAME_BYTES=1_048_576`) with doc comments stating that changing them is **major**. 
* **prelude.rs** — Convenience re-exports for embedding hosts and SDKs (matches the snapshot’s prelude). 
* **envelope.rs** — `EnvelopeKind` + `Envelope` enum (HELLO/START/DATA/ACK/END/ERROR), `#[non_exhaustive]` for additive evolution; mirrors the canonical wire grammar. 
* **frame.rs** — `FrameHeader`, `Frame<'a>` with borrowed body slice; encode/decode function boundaries that enforce **1 MiB** cap early. Zero-copy friendliness is explicit. 
* **error.rs** — Typed `OapError` and `OapConfigError` with canonical, **low-cardinality** reason mapping (e.g., `frame_oversize`, `unknown_envelope`, `truncated`). 
* **metrics.rs** — Small trait facade `OapMetricsExt` (cloneable, Send+Sync) with methods for frames/rejects/bytes/inflight/ack window and timers; host wires exporters. 
* **seq.rs** — Sequence/credit algebra helpers; `Seq`, `SeqRolloverPolicy { Error | Wrap }` (default **Error**) to keep monotonicity. 
* **parser/**

  * **mod.rs** — `Parser`, `Progress<'a> { NeedMore | Frame{..} }` surface; **cancel-safe**, **no blocking**, and **no locks across `.await`** by construction. 
  * **config.rs** — `ParserConfig` with validated knobs (`strict_headers`, `enforce_obj_header_on_objects`, `ack_window_frames ∈ [1,1024]`, `allow_pq_hello_flags`, `seq_rollover_policy`). 
  * **state.rs** — Opaque internal cursor/scratch; enforces “length-first refusal” before any allocation (defense against oversize/truncation). 
* **writer/**

  * **mod.rs** — `Writer` encode surface; window/credit-friendly paths; **no timers** (host owns timing/backpressure). 
  * **config.rs** — Placeholder for writer knobs (kept minimal for OAP/1). 

> Design intent: many **small files** over one “god module”; this keeps the protocol surface obvious, test seams crisp, and diffs tiny.

### `tests/` (acceptance gates as code)

* **conformance.rs** — Drives **canonical vectors** (happy path & rejects) and asserts byte-identical round trips (Rust ↔ fixtures), exactly as your INTEROP doc requires. 
* **split_need_more.rs** — Proves **split-read equivalence**: feeding N slices == feeding once; no state drift. 
* **ack_algebra.rs** — Monotone `ack_up_to`; rejects credit violations deterministically. 
* **config_validation.rs** — Bounds check for `ack_window_frames`, strict-headers defaults, PQ flag gating. 
* **metrics_mapping.rs** — Uses a fake metrics handle to assert all rejects hit the **canonical low-cardinality** reasons and frame counters increment per envelope. 
* **vectors/oap1/** — Cross-language fixtures: JSON for structs and binary for on-wire frames; includes: hello↔hello, start/data/ack/end, and the reject suite (oversize, missing_obj, unknown, truncated, credit_violation). 

### `benches/` (perf gates)

* **decode_happy.rs** — HELLO→START→(DATA×N)→END across payload sizes (≤256 B, 4 KiB, 64 KiB). 
* **decode_pathological.rs** — Oversize, truncated at every boundary, unknown envelope; confirms **length-first refusal** and p95 budgets. 
* **encode_ack.rs** — Windowed ACK sequences; measures encode hot path. 

### `fuzz/` (hardening proof)

* **fuzz_targets/** — `parser_fuzz`, `header_fuzz`, `ack_fuzz` as in TESTS.md. Keeps corpora small but effective; nightly runs ≥4h in CI. 
* **corpora/** — Seeded with canonical vectors + minimized rejects; regression corpus checked in (size-capped). 
* **artifacts/** — CI-minimized crashes; ensures “never forget” regressions. 

### `docs/` (single source of truth)

* **specs/oap-1.md** — Normative spec file you reference from the README (“spec of record”). 
* **API.md, CONCURRENCY.md, CONFIG.md, GOVERNANCE.md, IDB.md, INTEROP.md, OBSERVABILITY.md, PERFORMANCE.md, QUANTUM.md, RUNBOOK.md, SECURITY.md, TESTS.md** — Your combined docs broken back into maintainable pieces so each blueprint is diff-able and reviewable on PR. 
* **arch.mmd · seq.mmd · state.mmd** — Mermaid sources (rendered to SVG in CI), satisfying README’s diagram policy. 
* **api-history/oap/1.0.0.txt** — `cargo public-api --simplified` snapshot to enforce SemVer. 

### `scripts/`

* **perf_compare.sh** — Compares Criterion JSON vs stored baselines; fails CI if p95 ↑>10% or TPS ↓>10% (the performance gates you defined). 

### `.github/workflows/` (operational excellence baked in)

* **ci.yml** — fmt, clippy (deny warnings), tests (unit/integration/doc), cargo-deny advisories/licenses, coverage if you enable it. 
* **render-mermaid.yml** — Renders all `.mmd` → `.svg` under `docs/`, exactly like the README snippet. 
* **public-api.yml** — Runs `cargo public-api`/`cargo semver-checks`, posts symbol diffs, and enforces change discipline. 
* **fuzz-nightly.yml** — libFuzzer targets with sanitizers; saves minimized artifacts to `fuzz/artifacts/`. 
* **perf-bench.yml** — Criterion benches + `scripts/perf_compare.sh` fail thresholds (p95, TPS, alloc/op) per PERFORMANCE.md. 

---

## Why this scaffold fits your canon (quick checks)

* **Pure library; no drift:** no sockets/TLS/auth; small modules that mirror the **public API snapshot** (envelope/frame/parser/writer/error/metrics/seq/consts/prelude). 
* **Invariants enforced by structure:** early-refusal framing layout; tests + fuzz + benches reflect **1 MiB cap** and **fixed envelope set**; low-cardinality reject reasons wired through metrics tests. 
* **Observability contract honored:** no endpoints here; hosts export `oap_*` metrics; readiness keys referenced by docs + runbooks. 
* **Polyglot & interop-ready:** `tests/vectors/oap1/` is the lingua franca for Rust/TS/Python bindings; fixtures are the truth for byte-compat. 
* **DX & governance:** Mermaid sources, API snapshots, SemVer gates, and PIP-friendly docs live under `docs/` with CI to keep them fresh. 

---

## Optional tiny additions (still modular)

* **/docs/pip/PIP-TEMPLATE.md** — If you want governance PRs to spawn from a template. 
* **/docs/sbom/** — If you generate SBOMs at release (ties to cargo-deny & reproducibility). 
