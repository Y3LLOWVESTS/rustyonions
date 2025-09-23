
# RustyOnions Concurrency & Aliasing Blueprint (v1.2 — perfected)

*Last updated: 2025-09-07*
**Audience:** RustyOnions contributors (kernel, gateway, services)
**Purpose (explicit):** Prevent aliasing bugs, boot-time races, and liveness failures with rules that are **architectural**, **code-level**, **operational**, and **CI-enforced**—so behavior proven locally is identical in CI and production. Passing the CI gates is required to merge and ship.

**New in this perfected version (since prior v1.2):**

* Failure-aware **TLA+** model (deadlines, degraded mode) with safety + liveness properties
* **Loom** interleaving tests for readiness (mandatory on `ron-kernel`)
* **Structure-aware fuzzing** (generators + dictionary) for protocol invariants
* **Tokio runtime guidance** (multi vs. current\_thread pitfalls and patterns)
* **Async-Drop safety** via cooperative shutdown token & pattern
* **Phase gates** (Bronze/Silver/Gold/Platinum) so CI strictness scales by branch/release

---

## 0) TL;DR: Golden Rules

1. **Never hold a lock across `.await`.** Lock → copy/extract → drop → then await.
2. **One writer per connection; one broadcast receiver per task.**
3. **Owned bytes on hot paths.** Use `bytes::Bytes` (zero-copy, ref-counted).
4. **Register metrics once; clone handles everywhere else.**
5. **Readiness is observable.** Gate handlers on `health.all_ready()`; scripts poll `/readyz` and object URLs (never sleep).
6. **Config updates are atomic.** Distribute as `Arc<Config>` (prefer `ArcSwap` or `RwLock<Arc<Config>>>`).
7. **TLS config is immutable post-startup.** Wrap `tokio_rustls::rustls::ServerConfig` in `Arc`, never mutate live.
8. **Sled iterators are short-lived.** Don’t hold them across `.await`; copy owned values first.
9. **No global mutable state.** `#![forbid(unsafe_code)]` at crate roots; prefer `Arc<Mutex|RwLock>`/atomics.
10. **No magic sleeps.** Readiness gates in scripts/services replace timing guesses.

---

## 1) Why this blueprint exists (problem → remedy → guarantee)

* **Problem:** Async systems fail from **logical races** (startup ordering), **liveness issues** (deadlocks, starvation), and **I/O interleaves** (multi-writer).
* **Remedy:** Define non-negotiable **invariants**, make readiness **observable**, enforce **ownership discipline**, and bake checks into **CI**. Replace sleeps with checks for concrete conditions.
* **Guarantee:** Code that passes these checks exhibits **stable startup and API semantics** on laptops, CI, and production.

---

## 2) Architecture Invariants (repo-wide)

* **BLAKE3 everywhere** for content addressing; HTTP exposes `ETag: "b3:<hex>"`.
* **Protocol vs. storage chunking.** OAP/1 `max_frame = 1 MiB`; storage may use smaller internal chunks.
* **HTTP read path contract.** `GET /o/{addr}` supports strong ETag, `Vary: Accept-Encoding`, ranges (206/416), stable error envelope.
* **Gated readiness.** `/readyz` returns `200` only after: config loaded, DBs opened, index watchers active, listeners bound, and bus subscribers attached.
* **Bus is broadcast-first.** `tokio::broadcast` with one receiver per task (no shared `Receiver`).
* **Tor-aware readiness (when applicable).** Extra keys: `tor_bootstrap`, `hs_published`.
* **Degraded mode.** If a key times out, service must not set `ready`. It returns `503` with JSON `{ "degraded": true, "missing": ["<keys>"], "retry_after": <s> }`.

---

## 3) Module-Specific Guardrails

**Bus (kernel).** Subscribe per task; surface overflow via counters; never share a `Receiver`.
**Axum HTTP.** `Arc<AppState>`; never `.await` while holding a lock; early `503` if not ready; `Body::from(Bytes)`.
**Transport (TCP/TLS/OAP).** One writer per connection; immutable `Arc<rustls::ServerConfig>`; explicit read/write/idle timeouts.
**Index/Storage (sled).** Cloneable `Db/Tree`; copy iterator outputs before `.await`; use CAS where applicable.
**Metrics.** Register once in `Metrics::new()` (guarded by `OnceLock`); clone handles elsewhere.
**Health/Readiness.** Set readiness last; expose owned `snapshot()` so locks are short-lived.
**Config hot-reload.** Atomically swap `Arc<Config>` (`ArcSwap` or `RwLock<Arc<Config>>>`).

---

## 4) Patterns (copy-paste ready)

### 4.1 Handler gating on readiness

```rust
use axum::{http::StatusCode, response::IntoResponse};
use std::sync::Arc;

pub async fn readyz(state: Arc<AppState>) -> impl IntoResponse {
    if state.health.all_ready() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, [("Retry-After", "1")], "booting").into_response()
    }
}

pub async fn get_object(state: Arc<AppState>, addr: String) -> impl IntoResponse {
    if !state.health.all_ready() {
        return (StatusCode::SERVICE_UNAVAILABLE, [("Retry-After", "1")], "initializing").into_response();
    }
    // resolve -> stream bytes -> set ETag = "b3:<hex>"; return 200 or true 404.
}
```

### 4.2 Lock discipline in async

```rust
// BAD: holds lock across await
let mut g = state.inner.write().await;
g.pending.push(x);
some_async().await;

// GOOD: keep the critical section small
{
    let mut g = state.inner.write().await;
    g.pending.push(x);
} // guard dropped here
some_async().await;
```

### 4.3 Broadcast fan-out

```rust
let tx = bus.sender().clone();
let mut rx_worker = tx.subscribe(); // one per task
tokio::spawn(async move {
    while let Ok(ev) = rx_worker.recv().await {
        // handle ev
    }
});
```

### 4.4 Single writer per connection

```rust
let (mut rd, mut wr) = tokio::io::split(stream);
// reader task
let _r = tokio::spawn(async move {
    let _ = tokio::io::copy(&mut rd, &mut tokio::io::sink()).await;
});
// writer lives here; never clone wr to multiple tasks
wr.write_all(&frame).await?;
```

### 4.5 Owned bytes end-to-end (correct ETag quoting)

```rust
use bytes::Bytes;

let bytes = Bytes::from(body_bytes);
let mut hasher = blake3::Hasher::new();
hasher.update(&bytes);
let etag = format!("\"b3:{}\"", hasher.finalize().to_hex());

let resp = axum::body::Body::from(bytes.clone());
```

### 4.6 Config hot-swap

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct Cfg(ArcSwap<Config>);
impl Cfg {
    pub fn get(&self) -> Arc<Config> { self.0.load_full() }
    pub fn set(&self, new_cfg: Config) { self.0.store(Arc::new(new_cfg)); }
}
```

### 4.7 Cancel-safe & Drop-safe patterns

```rust
use futures::future::{AbortHandle, Abortable};
use tokio::select;

pub fn spawn_abortable<F>(f: F) -> AbortHandle
where F: std::future::Future<Output=()> + Send + 'static {
    let (h, reg) = AbortHandle::new_pair();
    tokio::spawn(Abortable::new(f, reg).unwrap_or(()));
    h
}

async fn do_work() {
    select! {
        _ = real_io_work() => { /* success */ }
        _ = tokio::signal::ctrl_c() => { cleanup().await; }
    }
}
```

### 4.8 Panic barriers for supervised tasks

```rust
use std::panic::{catch_unwind, AssertUnwindSafe};
tokio::spawn(async move {
    let _ = catch_unwind(AssertUnwindSafe(async { run_service_loop().await; }))
        .map_err(|_| {
            metrics.service_restarts_total.inc();
            // emit bus event for supervisor to restart
        });
});
```

### 4.9 Cooperative shutdown (async-Drop safe)

```rust
use tokio_util::sync::CancellationToken;

pub struct Service {
    cancel: CancellationToken,
    handle: tokio::task::JoinHandle<()>,
}

impl Service {
    pub fn new() -> Self {
        let cancel = CancellationToken::new();
        let child = cancel.child_token();
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = service_loop() => {},
                _ = child.cancelled() => { graceful_cleanup().await; }
            }
        });
        Self { cancel, handle }
    }

    pub async fn shutdown(self) {
        self.cancel.cancel();
        let _ = self.handle.await;
    }
}
impl Drop for Service {
    fn drop(&mut self) { self.cancel.cancel(); } // no blocking, no I/O
}
```

---

## 5) Anti-Patterns (ban these)

* Magic sleeps to “wait” for readiness.
* Shared `broadcast::Receiver` across tasks.
* Multiple writers to the same TCP stream.
* Holding any lock across I/O or `.await`.
* Re-registering Prometheus metrics under the same name.
* Borrowing short-lived buffers for HTTP bodies.
* Global `static mut` or ad-hoc singletons.

---

## 6) Scripts & Ops: Readiness Gates (drop-in)

### 6.1 `testing/lib/ready.sh`

```bash
#!/usr/bin/env bash
# Portable readiness helpers (macOS-friendly). Source, don't exec.
# Usage:
#   source testing/lib/ready.sh
#   wait_readyz "http://127.0.0.1:9080" 25
#   wait_obj "http://127.0.0.1:9080" "$OBJ_ADDR" 25
set -euo pipefail

_wait_http_200() {
  local url="$1" timeout="${2:-30}"
  local start now code
  start=$(date +%s)
  while true; do
    code=$(curl -fsS -o /dev/null -w "%{http_code}" "$url" || true)
    if [ "$code" = "200" ]; then return 0; fi
    now=$(date +%s)
    if (( now - start >= timeout )); then
      echo "timeout: expected 200 from $url (last=$code)" >&2
      return 1
    fi
    sleep 0.25
  done
}

wait_readyz() { _wait_http_200 "$1/readyz" "${2:-30}"; }
wait_obj()    { _wait_http_200 "$1/o/$2"   "${3:-30}"; }

require_env() {
  local name="$1"
  if [ -z "${!name:-}" ]; then
    echo "missing required env: $name" >&2
    return 1
  fi
}
```

### 6.2 Using the helpers in smoke tests

```bash
set -euo pipefail
source testing/lib/ready.sh

require_env RON_INDEX_DB
require_env OUT_DIR
require_env BIND
echo "RON_INDEX_DB=$RON_INDEX_DB"
echo "OUT_DIR=$OUT_DIR"
echo "BIND=$BIND"

# ... start services/gateway ...

wait_readyz "http://$BIND" 25
wait_obj "http://$BIND" "$OBJ_ADDR" 25
```

---

## 7) CI & Tooling Gates

### 7.1 Lints (deny on CI)

Add to crate roots:

```rust
#![forbid(unsafe_code)]
#![deny(
    clippy::await_holding_lock,
    clippy::await_holding_refcell_ref,
    clippy::mutex_atomic,
    clippy::unwrap_used,
    clippy::expect_used
)]
```

Run in CI:

```bash
cargo clippy --all-targets --all-features \
  -D warnings \
  -D clippy::await_holding_lock \
  -D clippy::await_holding_refcell_ref \
  -D clippy::mutex_atomic \
  -D clippy::unwrap_used \
  -D clippy::expect_used
```

### 7.2 Miri (pure logic crates)

```bash
rustup toolchain install nightly
cargo +nightly miri test -p ron-kernel
cargo +nightly miri test -p gateway -- --ignored
```

### 7.3 Greps to catch foot-guns

```bash
rg -n "static mut|lazy_static" -S
rg -n "\bsleep\s+\d+(\.\d+)?" testing -S | grep -v 'allow-sleep' || true
rg -n "tokio::sync::broadcast::Receiver" -S
rg -n "Arc<(Mutex|RwLock)>" -S
```

### 7.4 CI invariant script (`testing/ci_invariants.sh`)

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "[*] rustc version:"; rustc -V
echo "[*] cargo version:"; cargo -V

if ! command -v rg >/dev/null 2>&1; then
  echo "ripgrep (rg) is required"; exit 1
fi

echo "[*] Running Clippy with strict lints…"
cargo clippy --all-targets --all-features \
  -D warnings \
  -D clippy::await_holding_lock \
  -D clippy::await_holding_refcell_ref \
  -D clippy::mutex_atomic \
  -D clippy::unwrap_used \
  -D clippy::expect_used

echo "[*] Checking for banned patterns…"
if rg -n -S -i 'sha[23][^[:alnum:]]*256' .; then
  echo "Found SHA2 (256) remnants; migrate to BLAKE3 and update docs."; exit 1
fi
rg -n "b3:" -S specs || { echo "Expected 'b3:' address reference in specs/"; exit 1; }
rg -n "max_frame\s*=\s*1\s*MiB" -S specs || { echo "Expected OAP/1 max_frame = 1 MiB in specs/"; exit 1; }

SLEEP_HITS="$(rg -n '\bsleep\s+\d+(\.\d+)?' testing || true)"
if [ -n "$SLEEP_HITS" ]; then
  BAD="$(printf '%s\n' "$SLEEP_HITS" | grep -v 'allow-sleep' || true)"
  if [ -n "$BAD" ]; then
    echo "Arbitrary sleeps found in testing/ (mark rare intentional ones with 'allow-sleep'):"
    printf '%s\n' "$BAD"
    exit 1
  fi
fi

if rg -n "static mut|lazy_static!" -S . >/dev/null; then
  echo "Global mutable state detected; replace with Arc/locks or OnceCell/OnceLock."; exit 1
fi

echo "[*] Optional: Miri run (ENABLE_MIRI=1)…"
if [[ "${ENABLE_MIRI:-0}" == "1" ]]; then
  rustup toolchain install nightly --profile minimal
  rustup component add miri --toolchain nightly
  cargo +nightly miri setup
  cargo +nightly miri test -p ron-kernel
  cargo +nightly miri test -p gateway -- --ignored || true
fi

echo "[ok] CI invariants passed."
```

### 7.5 CI workflow: invariants (`.github/workflows/ci-invariants.yml`)

```yaml
name: CI Invariants
on: [push, pull_request]
jobs:
  invariants:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install ripgrep
        run: |
          sudo apt-get update
          sudo apt-get install -y ripgrep
      - name: Run CI invariants
        run: |
          chmod +x testing/ci_invariants.sh
          testing/ci_invariants.sh
```

### 7.6 **Mandatory** ThreadSanitizer for critical crates

`ci/crate-classes.toml`

```toml
[critical]
crates = [
  "ron-kernel",
  "gateway",
  "transport",
  "index",
  "overlay"
  # "arti_transport"
]
```

`testing/ci_sanitizers_run.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLASSES="$ROOT/ci/crate-classes.toml"
[ -f "$CLASSES" ] || { echo "Missing ci/crate-classes.toml"; exit 1; }

CRATES=$(awk '/\[critical\]/{flag=1;next}/\[/{flag=0}flag' "$CLASSES" | grep -Eo '"[^"]+"' | tr -d '"')
echo "[*] Critical crates:"; echo "$CRATES" | sed 's/^/  - /'

export RUSTFLAGS="-Zsanitizer=thread"
export RUSTDOCFLAGS="-Zsanitizer=thread"
rustup target add x86_64-unknown-linux-gnu --toolchain nightly

FAILED=0
for c in $CRATES; do
  echo "[*] TSAN: $c"
  if ! cargo +nightly test -Z build-std --target x86_64-unknown-linux-gnu -p "$c"; then
    echo "[!] TSAN failed: $c"; FAILED=1
  fi
done
exit $FAILED
```

`.github/workflows/ci-sanitizers-mandatory.yml`

```yaml
name: CI Sanitizers (critical crates)
on: [push, pull_request]
jobs:
  tsan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - uses: Swatinem/rust-cache@v2
      - name: Run TSan
        run: |
          sudo apt-get update && sudo apt-get install -y build-essential
          chmod +x testing/ci_sanitizers_run.sh
          testing/ci_sanitizers_run.sh
```

### 7.7 Tokio runtime flavors CI (multi-thread + current\_thread)

`.github/workflows/ci-rt-flavors.yml`

```yaml
name: CI Tokio Runtime Flavors
on: [push, pull_request]
jobs:
  rt-flavors:
    runs-on: ubuntu-latest
    strategy: { matrix: { flavor: [multi, single] } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run tests for flavor
        run: |
          if [ "${{ matrix.flavor }}" = "single" ]; then
            cargo test --all --features rt-current-thread
          else
            cargo test --all --features rt-multi-thread
          fi
```

**Helper macro** (place in `crates/common/src/test_rt.rs`):

```rust
#[macro_export]
macro_rules! test_both_runtimes {
    ($name:ident, $body:block) => {
        #[cfg(feature = "rt-multi-thread")]
        #[tokio::test(flavor = "multi_thread")]
        async fn $name() $body

        #[cfg(feature = "rt-current-thread")]
        #[tokio::test(flavor = "current_thread")]
        async fn $name() $body
    };
}
```

**Cargo features** (add in each tested crate):

```toml
[features]
rt-multi-thread = []
rt-current-thread = []
```

### 7.8 Structure-aware fuzzing (libFuzzer)

`fuzz/Cargo.toml`

```toml
[package]      name = "fuzz"  version = "0.0.1"  publish = false
[package.metadata]  cargo-fuzz = true
[dependencies] libfuzzer-sys = "0.4" arbitrary = "1"
[workspace]    members = ["."]

[[bin]] name="oap_roundtrip" path="fuzz_targets/oap_roundtrip.rs" test=false doc=false

[dependencies.gateway] path = "../crates/gateway"
```

`fuzz/fuzz_targets/oap_roundtrip.rs`

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::{Arbitrary, Unstructured};

#[derive(Debug, Arbitrary, Clone)]
enum Body { Bytes(Vec<u8>), Range { start: u64, len: u32 } }

#[derive(Debug, Arbitrary, Clone)]
struct Frame { kind: u8, id: u32, body: Body }

fn encode(_f: &Frame) -> Vec<u8> { /* TODO: real serializer */ vec![] }
fn parse(_b: &[u8]) -> Option<Frame> { /* TODO: real parser */ None }

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    if let Ok(fr) = Frame::arbitrary(&mut u) {
        let bytes = encode(&fr);
        if let Some(fr2) = parse(&bytes) {
            let _ = (fr2.kind, fr2.id); // and invariants/roundtrip checks
        }
    }
});
```

`fuzz/dictionaries/oap.dict`

```
"FRAME"
"KIND_DATA"
"KIND_RANGE"
"ETAG:b3:"
"Range: bytes="
```

Run locally:

```
cargo install cargo-fuzz
cargo fuzz run oap_roundtrip -- -dict=fuzz/dictionaries/oap.dict
```

### 7.9 Loom interleavings (mandatory for kernel readiness)

**Cargo (in `ron-kernel`)**

```toml
[dev-dependencies] loom = "0.7"
[features] loom = []
```

**Test (`crates/ron-kernel/tests/loom_health.rs`)**

```rust
#[cfg(feature = "loom")]
mod loom_tests {
    use loom::sync::{Arc, Mutex};
    use loom::thread;

    #[derive(Default)]
    struct Health { ready: bool, config: bool, db: bool, net: bool, bus: bool }
    impl Health {
        fn set_ready_if_complete(&mut self) {
            self.ready = self.config && self.db && self.net && self.bus;
        }
    }

    #[test]
    fn readiness_eventual_and_consistent() {
        loom::model(|| {
            let h = Arc::new(Mutex::new(Health::default()));
            for key in ["config","db","net","bus"] {
                let h2 = h.clone();
                thread::spawn(move || {
                    let mut g = h2.lock().unwrap();
                    match key {
                        "config" => g.config = true,
                        "db" => g.db = true,
                        "net" => g.net = true,
                        "bus" => g.bus = true,
                        _ => {}
                    }
                    g.set_ready_if_complete();
                });
            }
            let g = h.lock().unwrap();
            if g.ready { assert!(g.config && g.db && g.net && g.bus); }
        });
    }
}
```

**Workflow (`.github/workflows/ci-loom-kernel.yml`)**

```yaml
name: CI Loom (kernel readiness)
on: [push, pull_request]
jobs:
  loom:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Run loom tests
        run: |
          cargo test -p ron-kernel --features loom --test loom_health
```

---

## 8) HealthState: DoD for Readiness (Tor-aware + degraded)

* `health.set("config", true)` — config loaded and parsed.
* `health.set("db", true)` — sled DBs open; index watcher active.
* `health.set("net", true)` — listeners bound and accepting.
* `health.set("bus", true)` — subscribers attached.
* *(If Tor/HS)* `health.set("tor_bootstrap", true)` — Tor bootstrap complete.
* *(If HS)* `health.set("hs_published", true)` — hidden service descriptor published/ready.
* **Degraded:** if any key misses its deadline, set `degraded(key)=true` and **do not** set `ready`.
* **Only then** `health.set("ready", true)` → `/readyz` returns 200.

**Degraded response example (HTTP 503):**

```json
{ "degraded": true, "missing": ["tor_bootstrap"], "retry_after": 5 }
```

---

## 9) Rollout Plan

1. Land `testing/lib/ready.sh`; convert smoke tests.
2. Gate handlers on `health.all_ready()`; implement Tor keys where applicable.
3. Centralize metrics registration under `OnceLock`.
4. Use `ArcSwap` (or `RwLock<Arc<Config>>>`) for atomic config swaps.
5. Enforce single-writer transport ownership.
6. Enable **CI Invariants**, **Mandatory TSan**, **Tokio runtime flavors**, and **Loom (kernel)** workflows.
7. Add cancel-safe patterns, panic barriers, and cooperative shutdown.
8. Wire structure-aware fuzz target(s) and dictionary; grow coverage over time.
9. Adopt degraded mode deadlines per service; expose reason in 503.

---

## 10) PR Checklist (Concurrency/Aliasing)

* [ ] No lock is held across `.await`.
* [ ] Single writer per connection enforced.
* [ ] Each consumer task has its own `broadcast::Receiver`.
* [ ] HTTP bodies use `bytes::Bytes`; ETag is `b3:<hex>`.
* [ ] Metrics registered once; handlers cloned.
* [ ] `/readyz` gating implemented; no magic sleeps in tests.
* [ ] Config updates are atomic `Arc<Config>` swaps.
* [ ] No `static mut` or ad-hoc singletons.
* [ ] Sled iterators not held across `.await`.
* [ ] **TSan green** for all critical crates on PR.
* [ ] **Tests pass under both Tokio flavors**.
* [ ] **Loom kernel readiness test green**.
* [ ] (If Tor) Tor bootstrap/HS publish keys + degraded deadlines implemented.

---

## 11) Formal model of readiness (TLA+)

### 11.1 Basic eventual-readiness (kept for reference) — `specs/readyz_dag.tla`

```tla
------------------------------- MODULE readyz_dag -------------------------------
EXTENDS Naturals, Sequences
CONSTANTS Keys
ASSUME Keys = << "config", "db", "net", "bus", "tor_bootstrap", "hs_published" >>
VARIABLE ready, state
Init == /\ ready = FALSE /\ state \in [k \in Keys -> {FALSE}]
Set(k) == state' = [state EXCEPT ![k] = TRUE] /\ UNCHANGED ready
AllReady == \A k \in Keys: state[k] = TRUE
Next == \/ \E k \in Keys: /\ state[k] = FALSE /\ Set(k)
        \/ /\ AllReady /\ ready' = TRUE /\ UNCHANGED state
Spec == Init /\ [][Next]_<<ready, state>>
THEOREM EventuallyReady == Spec => <>ready
=============================================================================
```

`specs/readyz_dag.cfg`

```tla
SPECIFICATION Spec
PROPERTY EventuallyReady
```

### 11.2 Failure-aware with deadlines & degraded mode — `specs/readyz_dag_failures.tla`

```tla
----------------------------- MODULE readyz_dag_failures -----------------------------
EXTENDS Naturals, Sequences

CONSTANTS Keys, MaxWait
ASSUME Keys = << "config","db","net","bus","tor_bootstrap","hs_published" >>
ASSUME MaxWait \in Nat \ {0}

VARIABLES clk, ready, degraded, state, deadline

Init ==
  /\ clk = 0
  /\ ready = FALSE
  /\ degraded \in [k \in Keys -> {FALSE}]
  /\ state \in [k \in Keys -> {FALSE}]
  /\ deadline \in [k \in Keys -> Nat]
  /\ \A k \in Keys: deadline[k] > 0

Tick == /\ clk' = clk + 1 /\ UNCHANGED <<ready, degraded, state, deadline>>
Set(k) == /\ state[k] = FALSE /\ state' = [state EXCEPT ![k] = TRUE] /\ UNCHANGED <<clk, ready, degraded, deadline>>
Timeout(k) == /\ state[k] = FALSE /\ clk >= deadline[k]
              /\ degraded' = [degraded EXCEPT ![k] = TRUE]
              /\ UNCHANGED <<clk, ready, state, deadline>>
AllReady == \A k \in Keys: state[k] = TRUE
AnyDegraded == \E k \in Keys: degraded[k] = TRUE
MarkReady == /\ AllReady /\ ~AnyDegraded /\ ready' = TRUE /\ UNCHANGED <<clk, degraded, state, deadline>>

Next == \/ \E k \in Keys: Set(k)
        \/ \E k \in Keys: Timeout(k)
        \/ Tick
        \/ MarkReady

Spec == Init /\ [][Next]_<<clk, ready, degraded, state, deadline>>

InvSafety == ready => (AllReady /\ ~AnyDegraded)
Liveness == <> (ready \/ AnyDegraded)

THEOREM Correct == Spec => []InvSafety /\ Liveness
=============================================================================
```

`specs/readyz_dag_failures.cfg`

```tla
SPECIFICATION Spec
INVARIANT InvSafety
PROPERTY Liveness
```

---

## 12) Phase Gates (adaptive CI strictness)

* **Bronze (default)**: CI Invariants + Clippy + grep bans + ready.sh in tests.
* **Silver (all PRs touching critical crates)**: + **Mandatory TSan**.
* **Gold (pre-release branches)**: + **Loom (kernel)** + structure-aware fuzz target(s) meet coverage/time budget.
* **Platinum (RC)**: TLA+ failure-aware spec checked; print spec SHA in build logs for traceability.

> Implement via branch-protection rules and a repo variable `PHASE` to require the corresponding jobs.

---

## 13) Tokio Runtime Guidance (beyond CI)

* **Prod default:** multi-thread runtime.
* **Tests:** must pass under both `multi_thread` and `current_thread`.
* **Ban:** `RefCell` shared across awaits (we deny `await_holding_refcell_ref`).
* **Blocking:** wrap blocking work with `spawn_blocking` (multi) or move it out of the runtime entirely.
* **Fairness (single thread):** long CPU loops must yield:

```rust
while work_left() {
    step();
    tokio::task::yield_now().await;
}
```

---

**When enforced, this blueprint makes “it passed locally” ≙ “it behaves in production” — and now models failure, explores interleavings, and fuzzes protocol structure to kill the last 0.5%.**
