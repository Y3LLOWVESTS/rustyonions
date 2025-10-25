### BEGIN NOTE - ryker - OCTOBER 25 2025 - 14:11 CST

Absolutely—here’s a tight, drop-in “carryover notes” pack for **ryker** that you can paste into `crates/ryker/NOTES.md` (or your per-crate NOTES log). It captures what we shipped, why the benches were hanging, what we fixed, the current benchmark numbers, and the exact next steps (including the clippy cleanup you hit).

---

# Carryover Notes — `ryker` (Actor & Bounded Mailbox Runtime)

**Date:** 2025-10-25 (America/Chicago)
**Profile:** RON-CORE (no Web3), aligns with RON-CORE.MD invariants and CODECHECK.MD

## What we landed (since start of `ryker`)

**Architecture & API**

* `Mailbox<T>`: bounded single-consumer queue on `tokio::mpsc` with **reject-new (Busy)** backpressure and **per-message deadlines** (timeout on `send` and `pull`).
* **Observer hooks** (non-blocking): `MailboxObserver` with `on_enqueue`, `on_drop(Capacity|Closed)`, `on_timeout`, `on_restart`. Exposed via `ryker::mailbox::observer::*` to avoid leaking internal modules.
* **Error taxonomy**: crate-level `Error` + mailbox-local `MailboxError`/`MailboxResult` (clean DX imports under `ryker::mailbox::*`).
* **Runtime**: `Runtime::new(cfg)`, `runtime.mailbox::<T>("actor").builder…`, plus `mailbox_default()` convenience.
* **Config**: `RykerConfig` with strict validation; builder > env > file > defaults precedence (file path blocked in prod unless allowed). Keys: capacity, max_msg_bytes, deadline, fairness knobs, supervisor backoff.
* **Supervisor**: crash-only restart wrapper with **decorrelated jitter** backoff (base/cap), no locks across `.await`.
* **Prelude** re-exports for common types.

**Bench + example harness**

* Benches: `enqueue.rs`, `dequeue.rs`, `batch.rs`.
  Root cause of prior hangs fixed by:

  * Adding `Mailbox::split() -> (MailboxTx, MailboxRx)` and **dropping TX after prefill** so drains terminate (no open-but-empty waits).
  * Prefilling with `send().await` (not `try_send`) to guarantee exact message counts.
  * Using a **current-thread** Tokio RT with `enable_time()` and occasional `yield_now()` in drain loops to keep timers advancing under Criterion.
* Examples: `actor_loop.rs` (bounded Busy + timeout demo), `config_dump.rs` (pretty-prints effective config).
* Script: `scripts/public_api_snapshot.sh` (uses `cargo public-api` if available; rustdoc fallback).

**Safety & invariants (checked)**

* **Zero unsafe**; respects I-7 (no locks across `.await`) and I-2 (bounded backpressure).
* Deadlines enforced on produce/consume; clean `Closed` signaling post-split drop.
* Amnesia: no persistence; config path reading blocked in prod unless explicitly allowed.

## Current issues & quick fixes (carryover)

1. **Clippy error (dead code):** `max_msg_bytes` stored but not enforced.

   * **Options (pick one next session):**
     A) Remove for now to keep warnings clean (re-add when we actually enforce).
     B) Keep the field and add `#[allow(dead_code)]` on the struct fields (temporary).
     C) Start enforcing size for byte-like messages via a light trait:

     ```rust
     pub trait MsgSize { fn size_bytes(&self) -> usize; }
     impl MsgSize for Vec<u8> { fn size_bytes(&self) -> usize { self.len() } }
     // Gate in MailboxTx::try_send / send:
     if msg.size_bytes() > self.max_msg_bytes { return Err(MailboxError::TooLarge { max: self.max_msg_bytes }); }
     ```

     For generic T, this would be opt-in (trait bound on builder or a wrapper type).
   * **Recommendation right now:** choose A (remove) to get green CI, then schedule C when we wire bytes-carrying actors.

2. **Tokio RT feature variance:** we used `new_current_thread()` to avoid requiring `rt-multi-thread`. If we later enable `rt-multi-thread` at workspace level, benches can switch to `new_multi_thread()` for better timer behavior.

3. **Depth metrics:** `tokio::mpsc` doesn’t expose queue depth directly. Our `on_enqueue` passes a best-effort `0`. If needed, add a small **instrumented wrapper** in a follow-up (keep it off the hot path by default).

## Benchmark snapshot (dev laptop)

Command series you ran:

```
cargo fmt -p ryker
cargo clippy -p ryker -- -D warnings
cargo bench -p ryker --bench dequeue
cargo bench -p ryker --bench batch
cargo bench -p ryker --bench enqueue
```

Observed (Criterion):

* **dequeue (CAP=1024, N=1024):** ~**404 µs** per iter (100 samples; some high outliers)
* **batch (CAP=2048, N=2048, BATCH=64):** ~**353 µs** per iter
* **enqueue (CAP=2048, N_TRIES=4096, try_send burst):** ~**144 µs** per iter

Notes:

* These are **functional sanity baselines**, not tuned performance runs. They confirm **no hangs** and that deadlines/closure paths behave as designed. We’ll tune parameters (batch sizes, deadlines, Criterion config) when we move into perf passes.

## Root cause post-mortem (why benches hung before)

* Prefill used `try_send` with `N_MSGS > capacity` and ignored `Busy`. Drains then awaited **forever** on an **open but empty** channel (no more sends, TX still alive).
* Fixes: **prefill with `send().await`**, **drop TX** (`split()`), add occasional `yield_now()` under current-thread RT.

## What to do next (actionable checklist)

**A. Clean up warnings (gate to green)**

1. **Remove or enforce `max_msg_bytes`**:

   * Fast path: remove field & references (lib + builder) to clear clippy.
     Files: `src/mailbox/queue.rs` (struct fields, constructors), `src/mailbox/builder.rs` (builder override), `src/config/model.rs` (can keep cfg for future).
   * Or add `#[allow(dead_code)]` to `max_msg_bytes` fields temporarily (least desirable).
2. `cargo clippy -p ryker -- -D warnings` (target: zero warnings).

**B. Add minimal tests to lock behavior**

* `tests/mailbox_busy.rs`:

  * Create mailbox(cap=2); `try_send` 3rd -> `Busy`.
* `tests/mailbox_timeout.rs`:

  * deadline=10ms; `pull()` on empty -> `Timeout`; `send()` with no receiver also times out if you use our blocking path.
* `tests/mailbox_closed.rs`:

  * Use `split()`, drop TX, drain until `Closed`.
* `tests/supervisor_restart.rs`:

  * Actor future returns Err once, assert it restarts (can inject a counter).

**C. Bench harness polish (Silver gate)**

* Add `CRITERION_QUICK=1` path to benches for CI speed.
* Parameterize CAP/N/BATCH via env (e.g., `RYKER_BENCH_CAP`, etc.) and print effective values once per bench.
* Optionally add a **hard per-iteration guard** (manual timeout) to guarantee no future hang.

**D. Observability hooks (optional, fast win)**

* Add a tiny example observer in `examples/` that increments counters (just print for now).
* Ensure hooks do **no allocation** and are O(1) (already true).

**E. Docs & README hooks**

* Update `README.md` with:

  * `split()` usage example and explanation (why/when to close TX in tests/benches).
  * Short “bench gotcha” blurb (open-but-empty vs closed channel behavior).

**F. Future (when we want max_msg_bytes enforcement)**

* Introduce `MsgSize` trait or a `BytesMsg(Vec<u8>)` newtype implementing size checks.
* Gate size enforcement behind a builder flag to keep generic T cost at zero.

## Repro recipe (fresh instance)

```
# format & lint
cargo fmt -p ryker
cargo clippy -p ryker -- -D warnings

# benches (functional sanity)
cargo bench -p ryker --bench dequeue
cargo bench -p ryker --bench batch
cargo bench -p ryker --bench enqueue

# examples
cargo run -p ryker --example config_dump
cargo run -p ryker --example actor_loop
```

## Risks & watch-outs

* Avoid re-introducing hangs by always **closing TX** in any drain-style test/bench (use `split()` and `drop(tx)`).
* If we later move to multi-thread RT for benches, ensure workspace enables `tokio`’s `rt-multi-thread` feature.
* Do not log message bodies or sizes in observers; keep hooks constant-time.

## Definition of Done (Bronze → Silver for `ryker`)

* **Bronze (today):** API complete, benches run w/o hangs, examples work, zero unsafe.
* **Silver (next session):**

  * Clippy clean (no warnings), tests added (Busy/Timeout/Closed/Supervisor), README updated with split/bench notes, bench env-param knobs in place.
* **Gold (later with perf pass):**

  * Optional depth instrumentation wrapper, size enforcement for byte payload variants, Criterion perf profiles captured and checked into `docs/benches/`.

---

### END NOTE - OCTOBER 25 2025 - 14:11 CST

### BEGIN NOTE - OCTOBER 25 2025 - 14:18 CST

alright—here’s the “god-tier” wrap on **ryker** with crisp next steps to get this crate buttoned up for RON-CORE. i’m grounding this in your uploaded NOTES/README/TODO/ALL_DOCS so it stays canon-aligned. citations inline.

---

# ryker status — what’s already strong

* **Scope & boundaries are tight.** Library crate only (no sockets/HTTP/DB), single-consumer bounded mailbox, deterministic deadlines, supervision wrapper, and observability hooks. Hosts own the runtime and endpoints. This matches Pillar-1 and keeps security boundary clean.  
* **Config surface & precedence are well-defined** (builder > env > file > defaults), with prod guard for `RYKER_CONFIG_PATH`. Hot vs cold reload rules are spelled out.  
* **Mailbox/backpressure model is explicit**: bounded mpsc, `try_send → Busy`, drops counted; fairness via batch/yield knobs; starve-avoid guidance is written. 
* **Security posture is appropriate for a lib**: no secrets, no network; focus on memory/backpressure/log hygiene; amnesia hooks present. Threat model and hardening checklist are drafted.  
* **Observability is specified**: canonical metrics and spans (depth, dropped, busy, handler latency, restarts). 
* **Public API discipline** and SemVer checks are planned with `cargo public-api` (snapshots in `docs/api-history/ryker/`). 
* **Bench/examples/test scaffolding** are laid out (enqueue/dequeue/batch benches; examples; loom area). Your recent bench hang root cause and fixes are documented (split/drop TX, `send().await`, single-thread RT + `yield_now()`).  

---

# gaps to close (small, high-leverage)

1. **Clippy dead-code on `max_msg_bytes`**
   Field exists in docs/config but isn’t enforced in code, producing a `dead_code` error (you saw it in the last run). Fastest close: temporarily **remove the unused field** in structs/builders and leave the config key documented for future enforcement—or keep the field and add a short-term `#[allow(dead_code)]`. Your NOTES already framed the options A/B/C. For shipping now, choose **A: remove**, then add enforcement when you wire a `MsgSize` path.  

2. **Make the contract executable (tests)**
   You have the matrix sketched (Busy/Timeout/Closed/Supervisor/Reload). Convert that into a minimal test set so the invariants “bite”.  

3. **Queue depth sampling**
   Metrics list includes a depth gauge; the docs/todo mention “sampled depth.” Add the tiny sampler (off the hot path) so the README metrics match reality.  

4. **Docs: finalize Bronze→Silver gates**
   README has Roadmap TODO bullets; the ALL_DOCS “Hardening checklist” shows unchecked boxes. Close those by tying each to a test and marking status.  

---

# ship plan — Bronze → Silver (today)

> Goal: **“Silver”** = clippy clean, minimal tests, examples run, metrics wired, benches don’t hang. ryker is optional in RON-CORE, so we can defer deeper perf to later without blocking the release.

## A) code hygiene to green

* **Remove the unused `max_msg_bytes` field** from the mailbox builder/queue structs now to silence clippy; keep the config key documented for forward compatibility, but mark it “cold-only; enforcement pending”. (This matches your NOTES option A.) 
* Re-run the standard block: `fmt / clippy (deny) / tests / deny / bench`. Commands are already captured in README. 

## B) minimal test suite (lock in behavior)

Add these four tests under `tests/integration/` (names match your folder skeleton):

* **`backpressure.rs`**: capacity=2; push 3rd with `try_send` → expect `Busy`. Count `busy_rejections_total`. 
* **`deadline.rs`**: default deadline (e.g., 1s); `pull()` on empty with deadline elapsing → expect `Timeout`. 
* **`supervisor_backoff.rs`**: actor panics once → restart with decorrelated jitter; increment `ryker_actor_restarts_total`. 
* **`reload_hot_cold.rs`**: change a **hot** knob (deadline) applies immediately; change a **cold** knob (capacity) applies only to new mailbox. 

> Bonus (quick loom smoke): `loom_mailbox_basic.rs` ensures no deadlocks on simple send/recv; you already have the loom folder scaffolded. 

## C) observability glue (tiny)

* Implement **`ryker_mailbox_depth`** sampling behind a flag (`RYKER_QUEUE_DEPTH_SAMPLING=true` default) with coarse interval or event-based sampling on enqueue/dequeue. Export counters/gauges named exactly as in README. 
* Wire default `MailboxObserver` to bump:
  `ryker_mailbox_dropped_total{reason}`, `ryker_busy_rejections_total`, and optionally `ryker_handler_latency_seconds{outcome}` in example handler. 

## D) benches — keep them “functional green”

* Ensure every bench **prefills with `send().await`** (not `try_send`), then **drops TX** via `split()` so drains terminate. Keep current-thread RT + `yield_now()` guards. This is already captured in your carryover NOTES; just confirm the files match the doc. 

## E) docs & README alignment

* In **README:**

  * Add a short “Bench gotchas” callout about open-but-empty vs closed channel and the `split()/drop(tx)` pattern. 
  * Tick the Roadmap items you’ll finish today: **Bronze→Silver gates**, **depth gauge**, **fairness docs**. 
* In **docs/TESTS.md**: mark which hardening checklist items are proven by which tests (turn those “[ ]” to “[x]” where applicable). 

---

# near-term (defer OK) — “Gold prep” items

These are nice-to-haves you can schedule after CORE ships (ryker is optional), but they’ll make the crate feel finished:

1. **Enforce `max_msg_bytes` safely** (generic T)
   Introduce an opt-in `MsgSize` trait or a `BytesMsg(Vec<u8>)` newtype that implements `size_bytes()`, enforce in the builder/send path, and keep it **cold-only** so existing mailboxes don’t change beneath actors. Your docs already list `max_msg_bytes ∈ [1 KiB, 1 MiB]` and cold-only semantics.  

2. **Public API snapshot**
   Run `scripts/public_api_snapshot.sh` and commit `docs/api-history/ryker/<v>.txt` once you declare the surface “frozen enough” for CORE.  

3. **Fairness examples**
   Document `RYKER_BATCH_MESSAGES` and `RYKER_YIELD_EVERY_N` with tiny before/after examples in examples/. 

4. **Config schema**
   Keep `schemas/ryker.config.schema.json` synced with README’s table so external validators can lint env/file snapshots in CI. 

---

# acceptance checklist to close the crate (today)

* clippy: **clean** (no `dead_code` on `max_msg_bytes`) ✔
* tests: add **4 integration tests** listed above (Busy/Timeout/Restart/Reload) ✔ 
* observability: **depth gauge** + counters wired ✔ 
* benches: **no hangs** (split/drop TX; current-thread RT; yields) ✔ 
* README & docs updated: bench gotchas + ticked Roadmap items ✔ 

> After that, stamp **“Silver”** for ryker and move on. Perf tuning can wait; CORE doesn’t block on ryker’s benches being perfect.

---

# quick commands (copy/paste)

```
cargo fmt -p ryker
cargo clippy -p ryker -- -D warnings
cargo test -p ryker --all-features
cargo bench -p ryker
```

(These are the exact flows already documented in README; keep them as your ritual.) 

---

## why this is enough for RON-CORE right now

* ryker’s responsibilities (bounded mailboxes + supervision + deadlines + hooks) are **complete at Bronze**, and the **Silver** items above make the guarantees executable. Because ryker is **optional** in the CORE launch, we can ship now and return later for strict `max_msg_bytes` enforcement and perf polish without endangering other CORE acceptance gates. The reload semantics, hot/cold boundaries, and observability contract already match the CORE invariants we’ve been enforcing elsewhere.  


### END NOTE - OCTOBER 25 2025 - 14:18 CST