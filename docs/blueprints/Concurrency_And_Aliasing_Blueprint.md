
# RustyOnions — Concurrency & Aliasing Blueprint (v1.3, 2025 Refactor Alignment)

*Last updated: 2025-09-22*
**Audience:** RustyOnions contributors (kernel, transport, overlay/DHT, gateway/edge, storage, mailbox, index, node profiles)
**Purpose (explicit):** Kill aliasing bugs, boot-order races, and liveness failures with **architectural invariants**, **code-level patterns**, **ops gates**, and **CI teeth**, so “passes locally” ≙ “behaves in prod”. This version aligns to the **33 canonical crates** and 12 Pillars (2025 edition).&#x20;

## Standard Header

**Scope:** Async runtime discipline, ownership/aliasing rules, readiness and shutdown semantics, bus/actor patterns, per-module guardrails (kernel, transport, overlay, DHT, gateway/edge, index, storage, mailbox).
**Non-scope:** Business policy/economics (see ECON concerns), app semantics, product UX.
**Crates impacted (subset of 33):** `ron-kernel`, `ron-bus`, `ryker`, `ron-transport`, `svc-overlay`, `svc-dht`, `svc-gateway`, `omnigate`, `svc-edge`, `svc-index`, `svc-storage`, `svc-mailbox`, `ron-metrics`, `ron-proto`, `oap`, node profiles (`macronode`, `micronode`).
**Six Concerns touched:** **RES** (primary), **PERF**, **SEC**, **GOV**.&#x20;

---

## What changed in v1.3 (compared to v1.2)

* **Crate map alignment:** references updated to the **exact 33 crates**; no legacy crates. `svc-arti-transport` is merged into **`ron-transport`** (feature `arti`), and `tldctl` was folded into **`ron-naming`** (types/schemas only).&#x20;
* **Overlay/DHT split:** `svc-dht` now owns Kademlia/Discv5; `svc-overlay` focuses on sessions/gossip—guardrails separate accordingly.
* **Naming fixes:** use `svc-gateway` and `svc-index` consistently (no “gateway”/“index” generics).&#x20;
* **OAP constants restated:** frame = **1 MiB**, typical chunk = **64 KiB** on storage path (perf/backpressure).&#x20;

> v1.3 keeps all core rules/patterns introduced in v1.2 (“perfected”) and extends module guardrails to `svc-dht`/`svc-edge`, plus CI lists and examples updated to the new crate names.&#x20;

---

## 0) TL;DR: Golden Rules (unchanged, still non-negotiable)

1. **Never hold a lock across `.await`.** Lock → copy/extract → drop → then await.
2. **One writer per connection; one broadcast receiver per task.**
3. **Owned bytes on hot paths.** Prefer `bytes::Bytes` (zero-copy, ref-counted).
4. **Register metrics once; clone handles everywhere else.**
5. **Readiness is observable.** Gate handlers on `health.all_ready()`; tests/scripts poll `/readyz`/object URLs (no magic sleeps).
6. **Config updates are atomic.** Swap as `Arc<Config>` (use `ArcSwap` or `RwLock<Arc<Config>>>`).
7. **TLS config immutable post-startup.** `Arc<tokio_rustls::rustls::ServerConfig>`, never mutate live.
8. **Sled iterators are short-lived.** Don’t hold them across awaits; copy owned values first.
9. **No global mutable state.** `#![forbid(unsafe_code)]` at crate roots; prefer `Arc<Mutex|RwLock>`/atomics.
10. **No magic sleeps.** Replace with readiness checks and deadlines.

---

## 1) Why this exists (problem → remedy → guarantee)

* **Problem:** Async systems die from *logical races* (startup ordering), *liveness failures* (deadlocks/starvation), and *I/O interleavings* (multi-writer).
* **Remedy:** System-wide **invariants**, **observable readiness**, **ownership discipline**, and **CI gates** that explore interleavings and deny foot-guns.
* **Guarantee:** Code proving these holds **boots deterministically** and **behaves consistently** across laptops, CI, and prod.

---

## 2) Repo-wide Architecture Invariants (refreshed)

* **Content addressing:** **BLAKE3** addresses; ETag format: `"b3:<hex>"`.
* **Protocol vs storage chunking:** **OAP/1** max frame = 1 MiB; storage typically chunks ≤ 64 KiB internally.&#x20;
* **HTTP read path contract:** `GET /o/{addr}` → strong ETag, `Vary: Accept-Encoding`, Range (206/416), stable error envelope.
* **Gated readiness:** `/readyz` returns `200` only after config, DBs, index watchers, listeners, and bus subscribers are up; Tor/HS keys are additive when applicable.
* **Bus is broadcast-first:** `tokio::broadcast` with **one receiver per task**; backpressure observable.
* **Degraded mode:** If a key misses its deadline, **do not** set `ready`; return `503` with `{ "degraded": true, "missing": [...], "retry_after": <s> }`.

---

## 3) Module-Specific Guardrails (updated to crate map)

**Kernel & Bus — `ron-kernel`, `ron-bus`, `ryker`**

* One `broadcast::Receiver` per consumer task; surface overflow via counters (`bus_overflow_dropped_total`, lag).
* Crash-only supervision with jittered backoff; no locks held across awaits in supervisory paths.&#x20;

**Transport — `ron-transport` (Arti merged behind `arti` feature)**

* **Single writer per socket**; split reader/writer tasks explicitly; set explicit read/write/idle timeouts per `TransportConfig`.
* TLS config is **`Arc<tokio_rustls::rustls::ServerConfig>`** and immutable at runtime.&#x20;

**Overlay — `svc-overlay`**

* Owns sessions/gossip only (no DHT).
* Enforce per-peer single write path; bound in-flight frames; graceful cancellation on disconnect.&#x20;

**DHT — `svc-dht`**

* Routing table updates: **single-writer discipline per k-bucket**; apply CAS-style or short, tight write-locks (never across awaits).
* Lookup concurrency parameters (α/β) must be bounded; hedged requests cancel-safe; p99 hop bound **≤ 5** enforced via sims/metrics.&#x20;

**Gateway & Edge — `svc-gateway`, `omnigate`, `svc-edge`**

* Fair-queue DRR on ingress; quotas enforced before heavy work.
* `/readyz` reflects shedding/degraded modes; handlers bail early if not ready.
* `svc-edge` static serving: range/ETag correctness with **owned bytes** only; never borrow temp buffers into responses.&#x20;

**Index & Naming — `svc-index`, `ron-naming`**

* `ron-naming` is types/schemas (no runtime).
* `svc-index` is read-optimized; do not hold sled iterators across awaits; copy values before async I/O; discovery via `svc-dht`.&#x20;

**Storage — `svc-storage`**

* Decompression/size caps; chunk assembly from owned buffers; avoid long-held write locks while awaiting I/O.
* Replication pipelines must bound concurrency per volume/peer.&#x20;

**Mailbox — `svc-mailbox`**

* Bounded mailboxes (Ryker); at-least-once with idempotency keys; ACK/visibility timeouts cancel-safe.&#x20;

**Metrics & Health — `ron-metrics`**

* Register metrics once (guard with `OnceLock`); clone handles only.
* `HealthState` exposes **owned** snapshots; set `ready` last.&#x20;

**Node Profiles — `macronode`, `micronode`**

* Compose services by config only; **no** service logic here.
* Micronode defaults to **amnesia mode** (RAM-only caches, ephemeral logs); readiness semantics identical.

---

## 4) Patterns (copy-paste-ready)

> These remain the canonical idioms from v1.2—kept intact and corrected to current crate names/types.

### 4.1 Ready gating for handlers (Axum)

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
} // dropped
some_async().await;
```

### 4.3 Broadcast fan-out (one receiver per task)

```rust
let tx = bus.sender().clone();
let mut rx_worker = tx.subscribe(); // unique per task
tokio::spawn(async move {
    while let Ok(ev) = rx_worker.recv().await {
        // handle ev
    }
});
```

### 4.4 Single writer per connection

```rust
let (mut rd, mut wr) = tokio::io::split(stream);
let _reader = tokio::spawn(async move {
    let _ = tokio::io::copy(&mut rd, &mut tokio::io::sink()).await;
});
// Writer lives here; do not clone wr to multiple tasks
wr.write_all(&frame).await?;
```

### 4.5 Owned bytes end-to-end (ETag quoting)

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

### 4.7 Cancel-safe & Drop-safe

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
        _ = real_io_work() => {},
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

### 6.2 Smoke tests use the helpers

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

## 7) CI & Tooling Gates (crate-aware)

### 7.1 Clippy lint wall (deny on CI)

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

CI run:

```bash
cargo clippy --all-targets --all-features \
  -D warnings \
  -D clippy::await_holding_lock \
  -D clippy::await_holding_refcell_ref \
  -D clippy::mutex_atomic \
  -D clippy::unwrap_used \
  -D clippy::expect_used
```

### 7.2 Miri (logic-heavy crates)

```bash
rustup toolchain install nightly
cargo +nightly miri test -p ron-kernel
cargo +nightly miri test -p svc-gateway -- --ignored
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
  cargo +nightly miri test -p svc-gateway -- --ignored || true
fi

echo "[ok] CI invariants passed."
```

### 7.5 Mandatory ThreadSanitizer for **critical crates**

`ci/crate-classes.toml`

```toml
[critical]
crates = [
  "ron-kernel",
  "ron-transport",
  "svc-gateway",
  "svc-overlay",
  "svc-dht",
  "svc-index",
  "svc-storage",
  "svc-mailbox"
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

### 7.6 Tokio runtime flavors CI (multi-thread + current\_thread)

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

**Helper macro** (e.g., `crates/common/src/test_rt.rs`):

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

**Cargo features**:

```toml
[features]
rt-multi-thread = []
rt-current-thread = []
```

### 7.7 Structure-aware fuzzing (OAP, DHT, mailbox)

Keep the `cargo-fuzz` harness (as in v1.2) and extend dictionaries for:
`"FIND_NODE"`, `"PROVIDE"`, `"ETAG:b3:"`, `"Range: bytes="`.

### 7.8 Loom interleavings (mandatory for kernel readiness)

Keep the kernel Loom test; add bus/health interleavings as needed. (Original example unchanged and valid.)

---

## 8) HealthState: DoD for Readiness (Tor-aware + degraded + profiles)

* `config`, `db`, `net`, `bus` must be true; add `tor_bootstrap`, `hs_published` when HS involved.
* If any deadline misses → **`degraded=true`** and **no `ready`**.
* Profiles (`macronode`, `micronode`) share identical readiness semantics; **micronode** may run with **amnesia** defaults.

**Degraded HTTP 503 example:**

```json
{ "degraded": true, "missing": ["tor_bootstrap"], "retry_after": 5 }
```

---

## 9) Rollout Plan (crate-aware)

1. Convert smoke tests to use `testing/lib/ready.sh`.
2. Gate all external handlers on `health.all_ready()`; implement degraded keys (incl. Tor) where applicable.
3. Centralize metrics registration (`OnceLock`) in `ron-metrics`.
4. Adopt atomic `Arc<Config>` swaps (e.g., `ArcSwap`).
5. Enforce single-writer transport ownership across `ron-transport`, `svc-overlay`, `svc-dht`.
6. Enable CI Invariants, **Mandatory TSan**, Tokio flavor matrix, and Loom (kernel).
7. Keep cancel-safe patterns, panic barriers, and cooperative shutdown everywhere.
8. Extend fuzz targets (OAP frames, DHT messages, mailbox envelopes).
9. Document degraded mode deadlines per service; surface reasons in 503s.

---

## 10) PR Checklist (Concurrency/Aliasing)

* [ ] No lock held across `.await`.
* [ ] **Single writer per connection** enforced (transport/overlay).
* [ ] **One broadcast receiver per task**; bus overflow/lag metrics present.
* [ ] HTTP bodies use `bytes::Bytes`; ETag is `"b3:<hex>"`.
* [ ] Metrics registered once; clones elsewhere.
* [ ] `/readyz` gating implemented; no magic sleeps in tests.
* [ ] Config updates are atomic `Arc<Config>` swaps.
* [ ] No `static mut`/ad-hoc singletons.
* [ ] Sled iterators not held across awaits.
* [ ] **TSan green** for critical crates on PR.
* [ ] **Tests pass under both Tokio flavors**.
* [ ] **Loom (kernel) green** for readiness interleavings.
* [ ] (If HS) Tor bootstrap/HS publish keys + degraded deadlines wired.

---

## 11) Formal model of readiness (TLA+) — kept & still binding

> The v1.2 TLA+ specs remain correct; we keep both the simple and failure-aware versions. (File names unchanged.)

**Basic eventual-readiness — `specs/readyz_dag.tla`** and **failure-aware with deadlines — `specs/readyz_dag_failures.tla`** (cfg files likewise). (See original blueprint for the full modules; content unchanged.)

---

## 12) Phase Gates (adaptive CI strictness)

* **Bronze (default):** CI Invariants + Clippy + grep bans + ready.sh in tests.
* **Silver (PRs touching critical crates):** + **Mandatory TSan**.
* **Gold (pre-release):** + **Loom (kernel)** + structure-aware fuzz targets (time-boxed).
* **Platinum (RC):** TLA+ failure-aware spec checked; print spec SHA in build logs.

---

## 13) Tokio Runtime Guidance (beyond CI)

* **Prod default:** multi-thread runtime.
* **Tests:** must pass under both `multi_thread` and `current_thread`.
* **Ban:** sharing `RefCell` across awaits (deny `await_holding_refcell_ref`).
* **Blocking:** use `spawn_blocking` or move work off runtime.
* **Fairness (single thread):** long loops **yield** periodically.

```rust
while work_left() {
    step();
    tokio::task::yield_now().await;
}
```

---

### Cross-references

* **Canonical 33-crate list / Pillars (2025):** authoritative mapping and boundaries.&#x20;
* **Crate deltas & refactor notes (carry-over):** Arti→`ron-transport`, `svc-dht` creation, overlay slim, `tldctl`→`ron-naming`, OAP constants, global amnesia.&#x20;
* **Prior “perfected” blueprint (v1.2):** source of retained patterns & tests now updated to new crate names.&#x20;

---

**Definition of Done (for this doc):**
This blueprint mentions **only** the canonical crates, reflects the **overlay/DHT split**, the **transport merge**, and the **standard header**/acceptance gates pattern we’re using across the 8 blueprints before condensing into the Six Concerns.
