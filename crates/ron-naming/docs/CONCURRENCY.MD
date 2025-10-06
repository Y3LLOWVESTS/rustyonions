---

title: Concurrency Model — ron-naming
crate: ron-naming
owner: Stevan White
last-reviewed: 2025-10-06
status: draft
template_version: 1.1
msrv: 1.80.0
tokio: "N/A (library is sync); CLI uses blocking I/O only"
loom: "0.7+ (dev-only; minimal models)"
lite_mode: "This crate is a small library + optional one-shot CLI; §§2,6,7 are N/A"
-----------------------------------------------------------------------------------

# Concurrency Model — ron-naming

`ron-naming` is a **pure, synchronous library** for name normalization/validation and DTOs for TLD governance artifacts, plus an **optional one-shot CLI** (`tldctl`) that wraps those functions for offline authoring (lint/pack/sign/verify). There are **no background tasks, no async runtime, no channels**, and **no shared mutable state** beyond short-lived local variables. This document makes that explicit and provides copy-paste patterns to keep it that way.

> Golden rule (still applies): if any future async enters this crate, **never hold a lock across `.await`** and prefer message passing over shared mutability.

---

## 0) Lite Mode

This crate qualifies for **Lite Mode**:

* Completed: **§1 Invariants**, **§3 Channels**, **§4 Locks**, **§5 Timeouts**, **§10 Validation**, **§11 Code Patterns**, **§15 CI & Lints**.
* Marked **N/A**: **§2 Runtime**, **§6 Shutdown**, **§7 I/O framing** (no network), **§8 Error taxonomy (RPC flavor)** trimmed, **§9 Metrics** trimmed, **§12 Config hooks** summarized, **§14 diagrams** simplified.

---

## 1) Invariants (MUST)

* **[I-1] No async runtime required.** Library APIs are **pure/sync**. The CLI is **one-shot**, uses **blocking std I/O**, and must not spawn background tasks.
* **[I-2] No global mutable state.** No `static mut`, no `lazy_static` with interior mutability for live mutation. Policy/Unicode tables are **read-only** after construction.
* **[I-3] Single-writer rule for caches (if introduced).** Any optional cache must be **construct-then-freeze** (e.g., `OnceCell<Arc<Table>>`). No post-publish mutation.
* **[I-4] Reentrancy & thread safety.** All public functions are **reentrant** and **`Send + Sync` friendly** (no shared interior mutability visible to callers).
* **[I-5] Determinism under concurrency.** Given the same inputs and policy tables, results are **bit-for-bit identical** regardless of caller concurrency.
* **[I-6] No blocking on shared locks.** If any lock is ever introduced (discouraged), guard sections must be **micro-scoped** and **never** cross FFI or hypothetical `.await`.
* **[I-7] Bounded memory.** No unbounded queues/buffers. Parsers validate sizes; canonical encoders allocate proportionally to input with hard caps.
* **[I-8] Cancel/Abort safety.** CLI operations are **idempotent** and can be aborted at process level without leaving partial in-memory state (outputs go to temp path then atomically rename if we ever add file writing beyond stdout).
* **[I-9] No task leaks.** (Trivial here) The CLI must **not** detach background work.
* **[I-10] Platform independence.** No reliance on thread-locals for correctness.

---

## 2) Runtime Topology — **N/A**

* No Tokio runtime, listeners, workers, or supervisors.

---

## 3) Channels & Backpressure

There are **no channels** in the library or CLI by design.

| Name   | Kind | Capacity | Producers → Consumers | Backpressure | Drop Semantics |
| ------ | ---- | -------: | --------------------- | ------------ | -------------- |
| *None* | —    |        — | —                     | —            | —              |

Guidelines retained (should this ever change):

* Prefer **bounded** `mpsc` with `try_send()` and an explicit `Busy` error over buffering.
* If a broadcast is ever introduced (unlikely here), track lag/drop metrics.

---

## 4) Locks & Shared State

**Allowed (only if needed later):**

* `OnceCell<Arc<T>>` to build read-only policy/Unicode tables at first use; no mutation after set.
* `Arc<StateSnapshot>` copies shared read-only views between threads.

**Forbidden:**

* Holding any lock across blocking I/O or hypothetical `.await`.
* Nested locks; lock hierarchies.
* Global mutable registries.

**Current state:** No locks used.

---

## 5) Timeouts, Retries, Deadlines

* **Library:** Not applicable (pure/sync, in-memory transformations).
* **CLI:** Uses standard blocking file/stdin/stdout; no network/RPC. Timeouts are not needed; if they are ever introduced (e.g., for very large inputs), they must be **opt-in** and enforced with a **total deadline** per command.

Backoff/retry policies are **N/A** (no external I/O other than local files/stdio).

---

## 6) Cancellation & Shutdown — **N/A**

* No long-running tasks. Process exit is immediate. If we ever write to files, we use **temp file + atomic rename** to ensure partial writes are not observed.

---

## 7) I/O & Framing — **N/A (no sockets)**

* CLI reads/writes **files/stdin/stdout** only. Encodings are **CBOR/JSON** (canonicalized deterministically in the library). No streaming network framing.

---

## 8) Error Taxonomy (Concurrency-Relevant)

Minimal, tailored to this crate:

| Error                 | When                                  | Retry? | Notes                              |
| --------------------- | ------------------------------------- | ------ | ---------------------------------- |
| `Error::Busy`         | (future) if a bounded queue existed   | N/A    | Not used presently                 |
| `Error::Timeout`      | Only if an optional deadline is added | N/A    | Not used presently                 |
| `Error::Canceled`     | CLI interrupted by user (signal)      | N/A    | Leaves no partial in-memory state  |
| `Error::Poisoned`     | (future) if locks are added           | N/A    | Avoid by not using poisoning locks |
| `Error::SizeExceeded` | Input exceeds documented cap          | N/A    | Validation guard, not concurrency  |

---

## 9) Metrics (Concurrency Health) — **Trimmed**

* **Not emitted here.** If the workspace has global metrics, the CLI may log structured events only (no Prometheus endpoints).

---

## 10) Validation Strategy

**Unit / Property (library)**

* **Determinism under threads:** Spawn N threads, concurrently call `normalize_name` on the same corpus; assert outputs are identical and stable (no allocation panics, no races).
* **Idempotence:** `normalize(normalize(x)) == normalize(x)` across diverse Unicode corpora (ASCII/Latin/Cyrillic/CJK/Emoji/mixed scripts).
* **Immutability checks:** If `OnceCell` is used for tables, verify `set()` is called at most once (negative tests ensure double-set fails predictably).

**Loom (dev-only; minimal)**

* Model a hypothetical **single-writer initialization** of a `OnceCell` while multiple readers call `get_or_init` concurrently; assert no deadlock/livelock and visibility after set.
* Model a **no-mutation after publish** invariant using a small state struct with a read handle cloned before/after initialization.

**Fuzz**

* Feed arbitrary Unicode into normalization; assert **no panics**, errors are typed, and resulting normalized strings meet policy (confusables/mixed scripts as configured).

**Golden vectors**

* Canonical encodings (CBOR/JSON) of `TldMap`/`SignedTldMap` are **byte-stable** across runs/platforms.

---

## 11) Code Patterns (Copy-Paste)

**Read-only table with single initialization**

```rust
use once_cell::sync::OnceCell;
use std::sync::Arc;

static POLICY_TABLES: OnceCell<Arc<PolicyTables>> = OnceCell::new();

pub fn policy_tables() -> &'static Arc<PolicyTables> {
    POLICY_TABLES.get_or_init(|| {
        // Build from embedded/bundled data; no external I/O.
        Arc::new(PolicyTables::from_builtin())
    })
}
```

**No global mutability; pass snapshots explicitly**

```rust
pub fn normalize_with(input: &str, tables: &PolicyTables) -> Result<CanonicalName, NameError> {
    // Pure function: depends only on input + provided tables.
    normalize_impl(input, tables)
}

pub fn normalize(input: &str) -> Result<CanonicalName, NameError> {
    normalize_with(input, policy_tables())
}
```

**Atomic output (future-proofing if writing files)**

```rust
use std::{fs, io, path::Path};

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    // On Unix, this is atomic if same filesystem.
    fs::rename(tmp, path)?;
    Ok(())
}
```

**Thread-safe reentrancy test scaffold**

```rust
#[test]
fn normalize_is_thread_safe() {
    use std::sync::Arc;
    use std::thread;

    let inputs = Arc::new(vec!["ExAmPle", "café", "РУС", "😀", "xn--caf-dma"]);
    let mut handles = Vec::new();

    for _ in 0..8 {
        let inputs = inputs.clone();
        handles.push(thread::spawn(move || {
            for s in inputs.iter() {
                let a = crate::normalize(s).unwrap();
                let b = crate::normalize(&a.0).unwrap();
                assert_eq!(a, b, "idempotence failed on {s}");
            }
        }));
    }
    for h in handles { h.join().unwrap(); }
}
```

---

## 12) Configuration Hooks (Quick Reference)

* None affect concurrency: config toggles normalization strictness, policy bundles, signing behavior (via external KMS profile), PQ mode flags for signature algorithms.
* If a future cache size were configurable, it must have a **hard cap** and **documented memory bound**.

See `docs/CONFIG.md` for the full schema.

---

## 13) Known Trade-offs / Nonstrict Areas

* **OnceCell vs eager build:** We choose `OnceCell` to avoid startup cost; the first call pays the build. This adds minimal synchronization but no ongoing contention.
* **No timeouts in CLI:** Simplicity beats complexity; very large inputs should be handled by upstream tooling (or we add an **opt-in** deadline later, with tests).

---

## 14) Mermaid Diagrams (Lightweight)

### 14.1 Initialization & Use (no tasks/channels)

```mermaid
flowchart LR
  A[First normalize()] --> B{POLICY_TABLES set?}
  B -- no --> C[Build from builtin]
  C --> D[Store OnceCell<Arc<_>>]
  B -- yes --> D
  D --> E[Normalize input (pure)]
```

**Text description:** The first call constructs policy tables from built-ins and stores them in a `OnceCell<Arc<_>>`. Subsequent calls reuse the read-only snapshot.

---

## 15) CI & Lints (Enforcement)

* **Clippy (crate-level):**

  * `-D clippy::unwrap_used`
  * `-D clippy::expect_used`
  * `-D clippy::await_holding_lock` *(kept on even though we’re sync; guards future async drift)*
  * `-D warnings`

* **Loom job (optional, PR-only):**

  * Build tests with `--cfg loom` and run the minimal `OnceCell` model.

* **Fuzz job:**

  * `cargo fuzz` target for normalization inputs; assert no panics and policy compliance.

* **Golden vectors:**

  * Check CBOR/JSON stability for `TldMap`/`SignedTldMap` across platforms.

---

### Definition of Done (Concurrency)

* No background tasks, channels, or locks are introduced.
* Any initialization is one-time and read-only via `OnceCell<Arc<_>>`.
* Determinism and idempotence validated under parallel callers.
* CI enforces lints; (optional) Loom model guards future drift.
