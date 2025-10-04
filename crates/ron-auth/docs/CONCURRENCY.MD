````markdown
---
title: Concurrency Model — ron-auth
crate: ron-auth
owner: Stevan White
last-reviewed: 2025-10-04
status: draft
template_version: 1.1
msrv: 1.80.0
tokio: "1.x (pinned at workspace root)"
loom: "0.7+ (dev-only)"
lite_mode: "This is a small pure library. We fully fill §§1,3,4,5,10,11 and mark others N/A."
---

# Concurrency Model — ron-auth

`ron-auth` is a **pure verification library** (no network/disk I/O, no background tasks).
It must be **thread-safe**, **reentrant**, **side-effect-free**, and **deterministic** under
contention. This file makes all concurrency rules explicit and testable.

> **Golden rule:** never hold a lock across `.await`.  
> (This crate is predominantly **sync** and should not `.await` at all.)

---

## 0) Lite Mode (applies)

- We complete **§1 Invariants**, **§3 Channels**, **§4 Locks**, **§5 Timeouts**, **§10 Validation**, **§11 Code Patterns**.
- Sections **§2, §6, §7, §8, §9, §12–§17** are **N/A** to this crate (library has no runtime, tasks, I/O).

---

## 1) Invariants (MUST)

- **[L-1] No `await` in core paths.** All verification/attenuation APIs are synchronous and do not park a runtime.
- **[L-2] Send+Sync everywhere appropriate.** Public verifier types and configs are `Send + Sync`; share via `Arc`.
- **[L-3] No global mutable state.** No `static mut`, no runtime-mutable singletons. If a global registry exists, it is **write-once** before use and read-only thereafter.
- **[L-4] Constant-time compares.** All MAC equality checks use `subtle` and are independent of success/failure (no timing side channels).
- **[L-5] Zeroization on drop.** Any buffer or key material exposed through trait objects is zeroized; no secret-bearing `Debug`/`Display`.
- **[L-6] Lock discipline.** If any lock exists, its critical section is **short** and **never** crosses an `.await`. Nested locks require a documented order (none expected).
- **[L-7] Config snapshots.** Verification uses an **immutable snapshot** of `VerifierConfig` (e.g., `Arc<VerifierConfig>`). Hot-swaps (if a host performs them) are atomic and race-free.
- **[L-8] No task leaks.** The crate does not spawn tasks. If a host supplies async hooks (e.g., key provider), they must own lifecycle.
- **[L-9] Bounded work.** Decoding/evaluation is **O(n_caveats)** with configured caps (`max_token_bytes`, `max_caveats`); no unbounded structures.
- **[L-10] Panic-free.** Concurrency violations, decode errors, and policy failures return typed errors—never `panic!`.
- **[L-11] Reentrancy.** `verify()` may be called concurrently from many threads; results are independent except for externally injected context (time, amnesia flag, policy digest).
- **[L-12] Registrar freeze.** Custom caveat handlers (if used) are **registered before first use** and then considered frozen (no concurrent mutation).

---

## 2) Runtime Topology — **N/A (library)**

No background tasks, listeners, or supervisors are created by this crate.

---

## 3) Channels & Backpressure

**Library inventory:** _none_. `ron-auth` does **not** own channels.

**Host guidance (when integrating):**
- If you propagate verification results via channels, use **bounded** `mpsc` and prefer **rejecting** new work (`try_send` → `Busy`) over buffering.
- For broadcast (e.g., policy/handler updates), prefer **watch** with **write-once/freeze** semantics for this crate (see §11/Registrar).

---

## 4) Locks & Shared State

**Policy:** Prefer **lock-free** reading via `Arc` snapshots and immutable data.

**Allowed**
- `Arc<VerifierConfig>` snapshots (recommended).
- `once_cell::sync::Lazy` or `OnceLock<T>` for **write-once** global registries (see “Registrar” below).
- Tiny, short-lived `Mutex`/`RwLock` only for construction/registration **before** the verifier is used.

**Forbidden**
- Holding any lock across `.await` (should not occur in this crate).
- Mutable global maps that change at runtime after first verification call.
- Secret-bearing data behind `Debug`, `Display`, or `Clone`.

**Recommended patterns**
- **Config hot-swap (host side):** keep `ArcSwap<VerifierConfig>` (or atomically replace an `Arc<VerifierConfig>`) and pass the snapshot into `verify()`; the crate remains oblivious to swaps.
- **Registrar for custom caveats:** provide a **builder** that accepts handler registrations; once `build()` is called, **freeze** the handler map into an `Arc<HashMap<…>>` and do not mutate.

---

## 5) Timeouts, Retries, Deadlines

- **No I/O → no runtime timeouts** inside this crate.
- Time appears only via `RequestCtx::now_unix_s` and **bounded skew** logic. There is **no sleeping**, no retries, no timers.
- If the host calls key providers that potentially block, they must **perform those calls** before calling `verify()` (or adapt with their own timeouts).

---

## 6) Cancellation & Shutdown — **N/A (library)**

No tasks to cancel. Hosts should cancel their own async operations before calling into `ron-auth`.

---

## 7) I/O & Framing — **N/A (library)**

The crate only decodes **in-memory** Base64URL → CBOR and evaluates caveats.

---

## 8) Error Taxonomy (Concurrency-Relevant) — **N/A (library-local queueing)**

All errors are **pure** (decode/semantic/config) and not related to runtime contention.

---

## 9) Metrics (Concurrency Health) — **N/A (library emits none)**

Any metrics are emitted by hosts. Library offers **hooks** to return reason codes so hosts can increment counters.

---

## 10) Validation Strategy

**Unit / Property**
- **Determinism under threads:** run `verify()` from multiple threads with identical inputs; assert **bit-stable** results.
- **Bounds respected:** property tests randomly generate tokens up to bounds; oversize tokens are rejected without allocation explosions.
- **Registrar freeze:** once built, attempts to mutate handler tables must fail at type level (no API) or return a typed error.

**Loom (dev-only)**
- **Model:** two threads calling `verify()` while a third attempts to replace an `Arc<VerifierConfig>` snapshot (host-style). Assert: no data races, decisions use either **old or new** snapshot but never a torn state.
- **No deadlocks:** if a minimal lock is used for *build time only*, loom should never find a lock cycle during verification.

**Fuzz**
- **CBOR fuzzing:** malformed/hostile inputs must not hang, OOM, or panic; total work ≤ O(n).
- **Caveat sequences:** reordering or duplication must not bypass conjunctive semantics.

**Chaos**
- Simulate host hot-swap of config while running verifications at high concurrency; measure no increase in error rate beyond the expected boundary (tokens incompatible with new policy).

---

## 11) Code Patterns (Copy-Paste)

### 11.1 Config snapshots (lock-free read)
```rust
use std::sync::Arc;
use ron_auth::{verify_token, RequestCtx};
use ron_auth::config::VerifierConfig;

let cfg: Arc<VerifierConfig> = Arc::new(VerifierConfig::default());
// in request handlers:
let snapshot = cfg.clone(); // cheap; lock-free
let decision = verify_token(&snapshot, token_b64, &ctx)?;
````

### 11.2 Registrar (write-once, then read-only)

```rust
use std::{collections::HashMap, sync::Arc};
type Handler = Arc<dyn Fn(&serde_cbor::Value, &RequestCtx) -> Result<(), AuthError> + Send + Sync>;

pub struct Registrar {
    table: HashMap<(String, String), Handler>,
}

impl Registrar {
    pub fn new() -> Self { Self { table: HashMap::new() } }
    pub fn register(mut self, ns: &str, name: &str, h: Handler) -> Self {
        self.table.insert((ns.to_owned(), name.to_owned()), h);
        self
    }
    pub fn freeze(self) -> Arc<HashMap<(String,String), Handler>> {
        Arc::new(self.table) // After this, do not mutate; share Arc clones freely.
    }
}
```

### 11.3 No lock across `.await` (defensive pattern for hosts)

```rust
// Acquire data under a lock, drop guard, then await.
let value = { let g = state.lock().unwrap(); g.derive_value() }; // guard dropped here
do_async(value).await;
```

### 11.4 Constant-time equality

```rust
use subtle::ConstantTimeEq;
fn eq_mac(a: &[u8;32], b: &[u8;32]) -> bool {
    a.ct_eq(b).into() // time independent
}
```

### 11.5 ArcSwap (optional host hot-swap)

```rust
// in host crate, not inside ron-auth:
use arc_swap::ArcSwap;
let cfg = ArcSwap::from_pointee(VerifierConfig::default());
// reader:
let snapshot = cfg.load(); // lock-free snapshot
// writer (reload):
cfg.store(Arc::new(new_cfg));
```

---

## 12) Configuration Hooks — **N/A (library concurrency)**

See `docs/CONFIG.md` for schema; concurrency-relevant knobs are host-side (e.g., queue sizes).

---

## 13) Known Trade-offs / Nonstrict Areas

* **Registrar mutability:** We intentionally select a **freeze-then-share** model instead of supporting dynamic handler churn; this removes a whole class of data races.
* **No internal caches:** We avoid caches to keep semantics deterministic and code path short (hosts may add caches outside with their own concurrency controls).

---

## 14) Mermaid Diagrams — **N/A (no tasks/queues)**

Optional diagrams are provided in IDB/CONFIG. None are required here due to lack of runtime.

---

## 15) CI & Lints (Enforcement)

* **Clippy:** `-D warnings`, `-W clippy::await_holding_lock` (defensive), `-W clippy::mutex_atomic`.
* **Forbid:** `#![forbid(unsafe_code)]`.
* **cargo-deny / geiger:** ensure no accidental async executors or I/O deps creep in.
* **Tests:** gate a `loom` job (ignored by default) to model Arc snapshot behavior (dev-only).

---

## 16) Schema Generation — **N/A**

No channels/locks registry needed; the library should remain lock-free at runtime.

---

## 17) Review & Maintenance

* Re-review on any change that introduces interior mutability, global state, or async.
* Keep this file in PR scope whenever `verify()` signatures or config snapshotting changes.

```
```
