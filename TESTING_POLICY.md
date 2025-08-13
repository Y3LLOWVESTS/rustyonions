# RustyOnions Testing Policy (Hybrid Model)
>In progress - this file used for reference purposes

This repository uses a **hybrid testing model** that balances small, white‑box unit tests with clear, black‑box integration tests. The goals are:
- Keep production code lean.
- Make behavior obvious to contributors.
- Encourage API stability and regression safety.
- Support **verbose** debugging when needed.

---

## 1) Directory Layout (per crate)

```
crates/<crate>/
  src/
    lib.rs                  # library entry (or internal modules)
    tests/                  # unit tests that can access crate internals
      mod.rs
      common.rs             # logging + helpers (compiled only for tests)
      unit_*.rs             # focused, white‑box tests
  tests/                    # black‑box integration tests (public API only)
    api_*.rs
    regression_*.rs
examples/                   # runnable examples; also used as living docs
```

> **Note**: For binary crates (like `crates/node`), prefer to extract functional logic into a small library module so both `main.rs` and tests can reuse it. Where that’s not yet possible, use `assert_cmd` to test the binary behavior directly.

---

## 2) Unit Tests (white‑box)

- Live under `src/tests/` and are compiled **only** when running `cargo test`.
- Can see private items by importing `crate::*` or individual modules.
- Keep them **small and fast**. Prefer 1–2 happy paths + a couple of edge cases.
- Shared helpers go in `src/tests/common.rs` and are guarded by `#[cfg(test)]`.

**Example layout**
```rust
// src/lib.rs
#[cfg(test)]
mod tests;

// src/tests/mod.rs
mod common;
mod unit_sanity;
mod unit_store_edges;

// src/tests/common.rs
use std::sync::Once;
static INIT: Once = Once::new();
pub fn init_test_logging() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}
```

---

## 3) Integration Tests (black‑box)

- Live under `tests/` (sibling of `src/`) and build as **separate crates**.
- Only use the crate’s **public** API. (Use `pub(crate)` items or a `testutils` feature if you must expose helpers.)
- Prefer scenario‑oriented names: `api_roundtrip.rs`, `api_error_paths.rs`, `regression_1234.rs`.

**Optional helper feature**
```toml
# Cargo.toml
[features]
testutils = []  # exposes extra test helpers

# src/lib.rs
#[cfg(any(test, feature = "testutils"))]
pub(crate) mod testkit { /* fixtures/builders */ }
```

---

## 4) Verbose Test Output

- Show crate/test logs:  
  `RUST_LOG=debug cargo test -- --nocapture`
- Extra cargo chatter:  
  `cargo test -vv`
- Filter by test name/path:  
  `cargo test overlay::roundtrip`

The logger in `src/tests/common.rs` enables `tracing` output across all tests without global re‑init panics.

---

## 5) Dev Dependencies

Add these per crate as needed (keep **prod deps** clean):

```toml
[dev-dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
anyhow = "1"
tempfile = "3"

# For binary testing (e.g., crates/node)
assert_cmd = "2"
predicates = "3"
```

---

## 6) Binary Crate Guidance (e.g., `crates/node`)

Prefer:
1. Move core logic into a library module (e.g., `src/lib.rs`), import it from `src/main.rs`.
2. Integration tests import the lib and assert behavior.

Until then, write CLI‑level tests using `assert_cmd`:
```rust
// crates/node/tests/cli_help.rs
use assert_cmd::Command;

#[test]
fn prints_help() {
    let mut cmd = Command::cargo_bin("ronode").unwrap();
    cmd.arg("--help").assert().success();
}
```

---

## 7) Workspace Conventions

- Tests must be **deterministic** and run offline by default.
- Networked tests (e.g., Tor/socket) must be opt‑in: gate with a feature or env flag, and **skip** by default.
- Each new crate gets the same layout and `common.rs` logger helper.
- Place runnable samples in `examples/`; reference them from docs and CI.

---

## 8) Scripts in `/testing`

Keep convenience scripts (`build.sh`, `lint.sh`, `test.sh`, `run_overlay_demo.sh`). They should call standard cargo commands and can pass env flags for verbose logs, e.g.:

```bash
# testing/test.sh
#!/usr/bin/env bash
set -euo pipefail
RUST_LOG=${RUST_LOG:-info} cargo test --workspace --all-targets -- --nocapture
```

> These scripts are **optional** for contributors; plain `cargo test` must always work.

---

## 9) GitHub Actions (CI) – suggested matrix

- Lint: `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -D warnings`
- Build & test: `cargo test --workspace`
- Doc tests: `RUSTDOCFLAGS="-D warnings" cargo test --doc`
- Optional job for feature‑gated network tests (off by default).

A minimal workflow file can be added under `.github/workflows/ci.yml` when you’re ready.

---

## 10) How to add tests to a new crate

1. Add in `src/lib.rs`:
   ```rust
   #[cfg(test)]
   mod tests;
   ```
2. Create `src/tests/mod.rs`, `src/tests/common.rs`, and at least one `unit_*.rs` file.
3. Create `tests/` with at least one `api_*.rs` file.
4. Add dev‑dependencies listed above as needed.
5. Ensure `cargo test` passes locally and in CI.

---

## 11) Naming & Style

- File names: `unit_*` for white‑box, `api_*` or `regression_*` for black‑box.
- Test names: meaningful, snake_case, describe behavior (e.g., `put_then_get_roundtrip`).
- One assert per behavior when possible; multiple asserts fine for a single scenario.
- Prefer `anyhow::Result<()>` returning tests for ergonomic `?` usage.

---

**That’s it.** This policy keeps your crates lean, tests discoverable, and contributors happy.
