# ron-kernel — Test Quickstart

This mini‑guide shows how to run the **ron‑kernel** test suite and individual integration tests.

> Location of tests: `crates/ron-kernel/tests/` (file‑scoped integration tests).
> File name → test target name (e.g., `bus_load.rs` → target `bus_load`).

---

## Prerequisites

- Rust toolchain (stable) installed (`rustup`).
- From the **workspace root** (top of the RustyOnions repo), use these commands.

Optional but helpful:
- Show logs from tests with `RUST_LOG=trace`.
- See test output with `--nocapture`.

---

## Run all tests for ron-kernel

Run unit + integration tests:
```
cargo test -p ron-kernel -- --nocapture
```

With verbose logs:
```
RUST_LOG=trace cargo test -p ron-kernel -- --nocapture
```

Single-threaded (useful for order-sensitive tests):
```
cargo test -p ron-kernel -- --nocapture --test-threads=1
```

Release mode (faster, optimizations on):
```
cargo test -p ron-kernel --release -- --nocapture
```

---

## Run a single integration test file

General form:
```
cargo test -p ron-kernel --test <file_stem> -- --nocapture
```

Examples:
```
cargo test -p ron-kernel --test bus_basic -- --nocapture
cargo test -p ron-kernel --test bus_topic -- --nocapture
cargo test -p ron-kernel --test bus_load -- --nocapture
cargo test -p ron-kernel --test event_snapshot -- --nocapture
cargo test -p ron-kernel --test http_index_overlay -- --nocapture
```

From inside the `crates/ron-kernel/` directory you can drop `-p ron-kernel`:
```
cd crates/ron-kernel
cargo test --test bus_load -- --nocapture
```

---

## Run a single test function within a file

Pattern: `cargo test -p ron-kernel --test <file_stem> <fn_pattern> -- --nocapture`

Example (replace with your function name):
```
cargo test -p ron-kernel --test bus_load bus_reports_lag -- --nocapture
```

Partial match works (e.g., `bus_reports` will match `bus_reports_lag`).

---

## What each integration test covers

- `bus_basic.rs` — basic publish/subscribe sanity on the monomorphic Bus.
- `bus_topic.rs` — topic‑style filtering behavior.
- `bus_load.rs` — bounded capacity & lag behavior; ensures `bus_lagged_total` increments.
- `event_snapshot.rs` — `KernelEvent` serde JSON snapshot & roundtrip.
- `http_index_overlay.rs` — index PUT/RESOLVE + overlay echo; end‑to‑end roundtrips.

Additional guard (sometimes in this crate or workspace):
- `no_sha256_guard.rs` — repo guard to keep **BLAKE3 (`b3:<hex>`)** canonical; rejects stray SHA‑256 mentions.

---

## Troubleshooting

**“bash ron-kernel/tests/bus_load.rs” doesn’t work**  
`.rs` files are Rust sources, not shell scripts. Use Cargo:
```
cargo test -p ron-kernel --test bus_load -- --nocapture
```

**“error: no test target named 'bus_load'”**  
Ensure the file exists at `crates/ron-kernel/tests/bus_load.rs`. The test target is the file stem (`bus_load`).

**Port in use (rare) or hanging tests**  
Try single‑threaded test execution:
```
cargo test -p ron-kernel -- --nocapture --test-threads=1
```

**Need more logs**  
Set `RUST_LOG` to `debug` or `trace`:
```
RUST_LOG=trace cargo test -p ron-kernel --test http_index_overlay -- --nocapture
```

---

## Handy one‑liners

List available integration test files:
```
ls crates/ron-kernel/tests
```

Run every integration test file explicitly (helpful for CI smoke):
```
for t in bus_basic bus_topic bus_load event_snapshot http_index_overlay; do   cargo test -p ron-kernel --test "$t" -- --nocapture || exit 1; done
```

---

### Example outputs (what “OK” looks like)

```
running 1 test
test bus_basic_pubsub ... ok

running 1 test
test bus_topic_filtering ... ok

running 1 test
test bus_reports_lag ... ok

running 2 tests
test overlay_echo_roundtrip ... ok
test index_put_resolve_roundtrip ... ok

running 2 tests
test kernel_event_serde_snapshot ... ok
test kernel_event_json_roundtrip ... ok
```

---

## Notes

- Use `--release` for more realistic performance characteristics.
- For load‑sensitive runs, prefer `--test-threads=1` and consider repeating tests (`for i in {1..20} ; do …`).

---

**TL;DR**  
Run everything with logs:
```
RUST_LOG=trace cargo test -p ron-kernel -- --nocapture
```
Run a specific file:
```
cargo test -p ron-kernel --test bus_load -- --nocapture
```
Run a specific test:
```
cargo test -p ron-kernel --test bus_load bus_reports_lag -- --nocapture
```
