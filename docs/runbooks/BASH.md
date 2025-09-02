# BASH Strict Build Command

This file documents a single Bash-friendly one-liner for running the **strict build + lint + tests + security checks** workflow we use.

This version avoids `zsh` parse errors by wrapping the pipeline in `bash -lc`, which executes the whole command sequence inside a clean Bash shell.

---

## Strict Build Command (Stable Toolchain)

```bash
env RUSTFLAGS="-D rust_2018_idioms -D unreachable_pub -D missing_debug_implementations" \
    RUSTDOCFLAGS="-D warnings -D rustdoc::broken-intra-doc-links" \
    bash -lc 'set -euo pipefail;
      cargo build  --workspace --all-targets --all-features -vv --future-incompat-report --locked &&
      cargo clippy --workspace --all-targets --all-features -vv -- -D warnings -W clippy::pedantic -W clippy::nursery -W clippy::cargo &&
      cargo test   --workspace --all-targets -- --nocapture &&
      cargo audit --deny warnings &&
      cargo deny  check'
```

### What this does

1. **Build (all targets, all features)** with extra verbosity and the `--future-incompat-report` for early warning about changes in upcoming Rust versions.
2. **Clippy** with `pedantic`, `nursery`, and `cargo` lint groups enabled, plus warnings treated as errors.
3. **Tests** for all targets, verbose (`--nocapture`).
4. **Security checks**:
   - `cargo audit` for vulnerabilities.
   - `cargo deny` for license/compliance/ban checks.

**Flags explained**:
- `-D rust_2018_idioms`: Deny outdated 2018 edition idioms.
- `-D unreachable_pub`: Disallow items marked `pub` but unreachable from outside.
- `-D missing_debug_implementations`: Ensure `Debug` is implemented where possible.
- `-D warnings`: Treat all warnings as errors.
- `-W clippy::pedantic`, `-W clippy::nursery`: Turn on extra lint groups.
- `-W clippy::cargo`: Catch Cargo.toml and packaging issues.

---

## Nightly-Only Extras (Optional)

Run these individually **after** the strict stable run for deeper checks.

### Miri (UB Detector)
```bash
cargo +nightly miri setup && \
MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-retag-fields" cargo +nightly miri test
```

### AddressSanitizer (Memory Errors)
```bash
RUSTFLAGS="-Z sanitizer=address" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

### ThreadSanitizer (Data Races)
```bash
RUSTFLAGS="-Z sanitizer=thread" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

### MemorySanitizer (Uninitialized Reads — Linux only)
```bash
RUSTFLAGS="-Z sanitizer=memory" \
cargo +nightly test -Zbuild-std --target x86_64-unknown-linux-gnu
```

---

**Tip**: You can also integrate this into `testing/strict.sh` with a `--nightly` flag so you can trigger the Nightly checks without retyping commands.
