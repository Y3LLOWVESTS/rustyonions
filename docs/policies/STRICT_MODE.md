# STRICT_MODE Checklist

Purpose: crank code quality to “release-grade” while keeping contributor friction reasonable. Use this file as a quick runbook when preparing releases or hardening a crate.

---

## 1) Lint Gates (add at top of `src/lib.rs` or `src/main.rs`)

```rust
#![forbid(unsafe_code)]
#![deny(
    warnings,
    clippy::all,
    clippy::cargo,
    clippy::complexity,
    clippy::perf,
    clippy::style,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_debug_implementations,
    unsafe_op_in_unsafe_fn
)]
#![warn(clippy::pedantic)]
// Common pedantic relaxations while ramping up strictness:
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc, clippy::missing_panics_doc)]
```

> For binary crates, consider relaxing `missing_docs`. For libraries, keep it strict.

---

## 2) CI Commands (fail fast)

- Format: `cargo fmt --all --check`
- Clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Docs: `RUSTDOCFLAGS="-D warnings -D rustdoc::broken-intra-doc-links" cargo test --doc`
- Tests: `RUST_LOG=debug cargo test --workspace --all-targets -- --nocapture`

Optional gates (enable progressively):
- Treat rustdoc lints as deny in code via `#![deny(rustdoc::broken_intra_doc_links)]` (requires nightly on some lints).

---

## 3) Security / License / Hygiene

- Vulnerabilities: `cargo audit`
- Licenses/Dups/Bans: `cargo deny check`
- Unused deps: `cargo +stable udeps`
- Outdated: `cargo outdated -R`
- Minimal versions (optional): `cargo update -Z minimal-versions` (nightly cargo feature)

Recommended: add a `.cargo-deny.toml` and pin audit to CI. Keep allowlists minimal with comments.

---

## 4) Coverage & API Stability

- Coverage (Linux/macOS): `cargo llvm-cov --workspace --lcov --output-path lcov.info`
  - Gate on: `--fail-under-lines 80`
- Public API checks for libraries: `cargo semver-checks`
  - Run before tagging releases.

---

## 5) UB & Fuzz (core crates)

- Undefined Behavior checks: `cargo +nightly miri test`
  - Env: `MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-retag-fields"`
- Fuzzing: `cargo fuzz run <target>`
  - Budget CI time weekly or pre-release only.

---

## 6) Test Discipline

- **Deterministic** by default; no network or time-of-day flakes.
- Gate slow/networked tests:
  ```rust
  #[cfg_attr(not(feature = "net-tests"), ignore)]
  #[test]
  fn tor_integration() { /* ... */ }
  ```
  Run with: `cargo test --features net-tests`.
- Keep unit tests fast (<100ms each); move heavy scenarios to integration tests.
- Use the shared logger helper (`src/tests/common.rs`) for consistent, verbose logs.

---

## 7) MSRV & Toolchain

- Pin MSRV in `Cargo.toml` (workspace root recommended):
  ```toml
  [package]
  rust-version = "1.78" # example
  ```
- CI matrix:
  - stable (latest)
  - MSRV
  - nightly (smoke: fmt + clippy + build)

---

## 8) Release Checklist (TL;DR)

- [ ] `cargo fmt --all --check` clean
- [ ] `cargo clippy ... -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] `cargo audit` clean (or documented allow)
- [ ] `cargo deny check` clean (or documented allow)
- [ ] Coverage >= target (e.g., 80% lines)
- [ ] `cargo semver-checks` green (libraries)
- [ ] Changelog updated; docs build without warnings
- [ ] Tags/version bumped; CI green on all jobs

---

## 9) Local Convenience

Consider a `Makefile` or `/testing/strict.sh` that runs the full strict suite:
```bash
#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings -D rustdoc::broken-intra-doc-links" cargo test --doc
RUST_LOG=debug cargo test --workspace --all-targets -- --nocapture
cargo audit
cargo deny check
# Optional:
# cargo +nightly miri test
# cargo llvm-cov --fail-under-lines 80
# cargo semver-checks
```
