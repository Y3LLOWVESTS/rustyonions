
>Short list
>Build and refine later

## Merge plan (single PR)

1. **Add/ensure files** (exact paths from the blueprint):

   * `testing/lib/ready.sh`
   * `testing/ci_invariants.sh`
   * `testing/ci_sanitizers_run.sh`
   * `ci/crate-classes.toml`
   * `.github/workflows/ci-invariants.yml`
   * `.github/workflows/ci-sanitizers-mandatory.yml`
   * `.github/workflows/ci-rt-flavors.yml`
   * `.github/workflows/ci-loom-kernel.yml`
   * `specs/readyz_dag.tla`, `specs/readyz_dag.cfg`
   * `specs/readyz_dag_failures.tla`, `specs/readyz_dag_failures.cfg`
   * `fuzz/Cargo.toml`, `fuzz/fuzz_targets/oap_roundtrip.rs`, `fuzz/dictionaries/oap.dict`
   * (helper) `crates/common/src/test_rt.rs`
2. **Wire crate features** (`rt-multi-thread`, `rt-current-thread`) in crates that have async tests.
3. **Kernel loom**: add the provided `loom_health.rs` test and `loom` dev-dep/feature to `ron-kernel`.
4. **Health degraded mode**: return 503 JSON as specified when any readiness key times out.
5. **Lock in lints**: add the deny prelude at crate roots.

## Quick local smoke (paste to shell)

```
chmod +x testing/*.sh
testing/ci_invariants.sh
ENABLE_MIRI=0 testing/ci_sanitizers_run.sh || true
cargo test --all --features rt-multi-thread
cargo test --all --features rt-current-thread
cargo test -p ron-kernel --features loom --test loom_health
cargo install cargo-fuzz
cargo fuzz run oap_roundtrip -- -dict=fuzz/dictionaries/oap.dict
```

## Repo hygiene

* Protect `main`: require all four CI checks (Invariants, TSan, RT-Flavors, Loom).
* Set `PHASE` to **Silver** by default; bump to **Gold/Platinum** on release branches.
* Add the PR checklist from the blueprint to `.github/PULL_REQUEST_TEMPLATE.md`.

## Nice-to-haves (post-merge)

* Implement real OAP `encode/parse` and hook the fuzz target.
* Make readiness timeouts configurable per key; include `retry_after` in 503 payloads.
* Bake the TLA+ spec SHA into build logs for Platinum releases.


# Strategy for identifying issues

You’re right: doing this by hand across the whole repo would be overwhelming. The good news is your new Concurrency & Aliasing blueprint already gives us a high-leverage path that catches 60–70% of issues automatically, and then narrows the rest to a short, targeted review. Here’s the battle plan I’d run—fast first, deep second—mapped directly to the blueprint.

# Fast pass (automated; same-day wins)

1. **Turn on the Golden Rules as “tripwires”**
   Most concurrency/aliasing bugs in RustyOnions cluster around a few patterns. We’ll scan for them first, exactly as the blueprint prescribes: never hold a lock across `.await`; single writer per connection; one broadcast receiver per task; owned bytes on hot paths; no magic sleeps; short-lived sled iterators; immutable TLS config; register metrics once (via `OnceLock`) (rules 1–10). &#x20;

Run a quick sweep (ripgrep is enough to surface suspects in minutes):

```
# lock across await (both orders)
rg -n "(\.await.*(lock|read|write))|((lock|read|write)\(\)\.await.*\n.*\.await)" -S crates/

# shared broadcast Receiver (each task needs its own)
rg -n "broadcast::Receiver" -S crates/

# multi-writer risks (look for split() and any cloned writers)
rg -n "tokio::io::split" -S crates/
rg -n "write_all\(" -S crates/

# magic sleeps (replace with readiness gates)
rg -n "tokio::time::sleep|std::thread::sleep" -S

# sled iterators held across awaits (manual eyeball after surfacing)
rg -n "iter\(|scan\(|range\(" -S crates/

# global mut / ad-hoc singletons
rg -n "static mut|lazy_static!" -S

# metrics double-registration (must be in Metrics::new() once)
rg -n "register_(counter|histogram|gauge)|prometheus::register" -S crates/

# HTTP bodies should use Bytes on hot paths
rg -n "Body::from\(" -S crates/
```

2. **Clippy + “await\_holding\_lock” + deny warnings**
   Clippy will flag several of these automatically (e.g., `await_holding_lock`). Run it repo-wide with warnings as errors:

```
cargo clippy --all-targets --all-features -- -D warnings
```

(Why: it’s a CI-enforced expectation in the blueprint’s phase gates.)&#x20;

3. **Run tests under both Tokio runtimes**
   Bugs that don’t appear in `multi_thread` show up in `current_thread`. The blueprint provides a tiny `test_both_runtimes!` helper macro and feature flags (`rt-multi-thread`, `rt-current-thread`). Add once and reuse in async tests. &#x20;

4. **Loom for readiness ordering (kernel)**
   Add the provided `loom_health` test to `ron-kernel` and run it. It proves “ready only after all gates are set; never in degraded mode.” This catches startup races that normal tests miss. &#x20;

5. **Replace any “sleep until ready” with real gates**
   Anywhere a script or service “waits a bit,” switch to the blueprint’s readiness contract: health keys (config/db/net/bus\[/tor]) must be true before `/readyz` returns 200; scripts poll `/readyz` or the exact object URL. This removes the entire class of “it flakes on CI” bugs. &#x20;

# Put it in CI (so it stays fixed)

Wire three small jobs the blueprint already defines:

* **CI Invariants (grep bans + Clippy)** — Phase *Bronze*: run the ripgreps above and `cargo clippy -D warnings`.&#x20;
* **ThreadSanitizer** — Phase *Silver* for critical crates: catches UB races that Rust’s borrow checker can’t see. (Run in Linux CI where TSan is supported.)&#x20;
* **Loom (kernel)** — Phase *Gold*: use the workflow snippet from the blueprint to execute loom tests on PRs that touch `ron-kernel`.&#x20;

This “adaptive strictness” is baked into the phase gates so we can dial it up as we approach release.&#x20;

# Targeted manual review (short, high-yield)

Once the scanners flag hotspots, do a 1–2 hour pass where it matters most:

* **Transport (TCP/TLS/OAP):** ensure **one writer per connection** (don’t clone the writer side of `split` across tasks), and keep the rustls `ServerConfig` in an `Arc` immutable after startup. &#x20;
* **Bus (kernel):** each task must `subscribe()` to get **its own** `broadcast::Receiver` (never share). Check overflow handling and that metrics are registered once. &#x20;
* **Axum handlers (gateway/services):** **don’t hold locks across I/O/await**; gate on `health.all_ready()`; use `bytes::Bytes` for bodies/ETag correctness. &#x20;
* **Index/Storage (sled):** copy iterator items into owned data before any `.await`; keep iterators **short-lived**.&#x20;
* **Config hot-swap:** use `ArcSwap` (or `RwLock<Arc<Config>>>`) to avoid reader stalls and aliasing hazards on live config.&#x20;

# “Is there an easy way?”

**Yes—partially.** If you do just these three things, you’ll surface most bugs quickly and continuously:

1. **Run the greps + Clippy today** (15–30 min to get a hit list). The rules are tuned to your repo and workflows.&#x20;
2. **Add the loom test for readiness in `ron-kernel`** (copy the snippet; it’s small) and keep it in CI. &#x20;
3. **Replace sleeps with readiness gates** in scripts and handlers (poll `/readyz`/object URL; return 503 with `Retry-After` until ready).&#x20;

Those steps are fast and eliminate the flakiest classes of failures (startup races, shared receivers, lock-across-await). The deeper items—TSan, structure-aware fuzzing for the protocol—slot into CI as time allows (the blueprint includes starter fuzz target and dictionary). &#x20;

# Drop-in assets (optional, copy-paste)

If you want me to, I can spit out paste-ready files for:

* `.github/workflows/ci-loom-kernel.yml` from the blueprint, unchanged.&#x20;
* `crates/common/src/test_rt.rs` with `test_both_runtimes!`.&#x20;
* `crates/ron-kernel/tests/loom_health.rs` (ready to run).&#x20;
* `testing/lib/ready.sh` to replace sleeps in shell flows with real readiness checks.&#x20;

---

## TL;DR

Start automated: run the blueprint greps + Clippy, add the kernel loom test, and gate everything on `/readyz`. Then do a short, targeted pass on transport (single writer), bus (no shared receivers), handlers (no lock-across-await), sled (no iterators across await), and metrics registration. This converts a daunting audit into a tight loop that CI enforces going forward.  &#x20;




# NOTES CURRENT - 11:38 - 9-7-2025

Love it—let’s turn that hit-list into a clean, finite plan. Below is a **step-by-step, do-this-then-that** roadmap that fixes what your run surfaced and locks it in so it stays fixed. No code changes here—just the plan. When you want the actual patches, say the word and I’ll produce paste-ready files per step.

---

# Phase 0 — Guardrails stay on

Keep running exactly this before/after each step:

```
cargo clippy --all-targets --all-features -- \
  -D warnings \
  -D clippy::await_holding_lock \
  -D clippy::await_holding_refcell_ref \
  -D clippy::mutex_atomic \
  -D clippy::unwrap_used \
  -D clippy::expect_used
```

Then the tripwires:

```
rg -nU '(\.await.*(lock|read|write))|((lock|read|write)\(\).*\n.*\.await)' -S crates/
rg -n 'tokio::io::split' -S crates/
rg -n 'broadcast::Receiver.*clone' -S crates/
rg -n 'tokio::time::sleep|std::thread::sleep' -S
rg -n 'iter\(|scan\(|range\(' -S crates/
rg -n 'static mut|lazy_static!' -S
rg -n 'register_(counter|histogram|gauge)' -S crates/
rg -n 'Body::from\([^B]' -S crates/
```

The goal is: **Clippy green + no high-risk grep hits in runtime code** (tests/demos may be exempt).

---

## Step 1 — Unblock Clippy (the “gatekeeper”)

**Why:** Clippy stopped at `kameo::Context::new()` → “new without Default”.
**Action:** Add `Default` for that type (or remove `new()` if redundant).
**Exit criteria:** `cargo clippy ...` proceeds past `kameo`.
**Notes:** This is a style gate; once clear, you’ll see any deeper lints in other crates.

---

## Step 2 — Eliminate “magic sleeps” from runtime

Your grep shows one **runtime** sleep in `ron-kernel/src/transport.rs` (others are demos/tests/old code).

**Why:** Sleeps in runtime mask scheduling bugs, add latency, and create flakiness.
**Action plan:**

* Replace the runtime sleep with an event-based mechanism appropriate to that line’s intent (readiness, backpressure, retry budget, or a queue/waker).
* Keep sleeps only in demos/tests; gate them behind `#[cfg(any(test, feature = "demo"))]` or similar, or move them out entirely.
* Optional: if you *must* keep a demo sleep, add a `// allow-sleep (demo)` comment so your human reviewer knows it’s intentional.

**Exit criteria:**

* `rg 'tokio::time::sleep|std::thread::sleep' -S` returns **no hits in non-demo runtime paths** (bins/examples/tests may remain).
* Clippy still green.

---

## Step 3 — Single-writer discipline on I/O

You had no `tokio::io::split` hits (great), but verify the same invariant in your wrappers.

**Why:** Concurrency bugs often show up as two tasks writing the same stream.
**Action plan (review):**

* In transport/overlay paths, ensure the write half is **owned by exactly one async task** (non-`Clone`, moved not shared).
* If multiple tasks need to write, funnel through a **single writer task** via an mpsc channel.
* Confirm `IoEither` style enums do not end up with aliasable mutable access.

**Exit criteria:**

* Code review confirms “one writer task per connection” contract.
* (Optional) Add a small test that tries to clone the writer newtype and ensure it **doesn’t compile** or panics a debug assertion.

---

## Step 4 — “No lock across await” sweep

Your first ripgrep is a **triage** list, not proof. We’ll harden the few hot spots that do take locks.

**Why:** Holding a Mutex/RwLock guard across `.await` can deadlock or stall.
**Action plan:**

* For each async function that acquires a guard, **shrink the critical section** so the guard drops **before** any `.await`.
* When mutation spans awaits (rare), refactor to an **actor/message** or copy the needed data out while locked.

**Exit criteria:**

* Clippy with `await_holding_lock` is green.
* Manual code scan shows guards end before `.await` in runtime code.

---

## Step 5 — Bus: one broadcast receiver per task

You had no `Receiver.*clone` hits (good). We still lock in the contract.

**Why:** Sharing a single `broadcast::Receiver` among tasks causes missed messages.
**Action plan:**

* Ensure every consumer calls `bus.subscribe()` itself, not via a shared `Receiver`.
* Add a tiny test that spawns two tasks, each with its **own** receiver, and proves both see a sent event.

**Exit criteria:**

* Review confirmed; test passes.
* (Optional) Document “one receiver per task” in the bus module docs.

---

## Step 6 — Sled/DB iterators don’t cross awaits

Your iterator grep is noisy by design; we only care about **sled**/DB ranges.

**Why:** Long-lived DB iterators captured across awaits can alias internal state or hold locks too long.
**Action plan:**

* In overlay/index/storage code, ensure any `tree.range(..)`/`iter()` results are **fully collected into owned `Vec<_>`** (or processed) **before** an `.await` happens.
* If you stream results to clients, perform the DB work in a dedicated task and send chunks over a channel.

**Exit criteria:**

* Review confirms no DB iterator spans an `.await`.
* Add a quick async test that interleaves “produce next item” with other awaits to ensure no panics/poisoning.

---

## Step 7 — Metric registration occurs exactly once

Your grep shows `register_*` in multiple modules (may be fine, may duplicate).

**Why:** Prometheus `register_*` panics on double-registration.
**Action plan:**

* Centralize registration in `Metrics::new()` (or equivalent) and **store handles** (e.g., `IntCounterVec`) there.
* Other modules should **take the handles** (inject via state) instead of registering themselves.
* Use `OnceLock`/`OnceCell` only at a single rendezvous point—avoid “surprise singletons.”

**Exit criteria:**

* One registration site per metric family; app runs twice in tests without panic.
* `/metrics` endpoint still exposes everything expected.

---

## Step 8 — HTTP hot paths return `Bytes`

Your single hit was on a **/metrics** response (low importance). Keep the principle for hot paths.

**Why:** `Bytes` avoids copies and aligns with stable ETag hashing behavior.
**Action plan (policy):**

* In handlers that serve objects/content, ensure bodies are `Bytes` (or `Body` constructed from `Bytes`).
* `/metrics` can remain as is (it isn’t hot), unless you want uniformity.

**Exit criteria:**

* Object/overlay/gateway hot paths use `Bytes`.
* ETags (if used) match `b3:<hex>` on exact bytes.

---

## Step 9 — Lock the process into CI

You’ve added **CI Invariants** already. Now add the other two gates so regressions can’t sneak back.

**Why:** Humans forget; CI doesn’t.
**Action plan:**

* Add **TSan workflow** for critical crates (`ron-kernel`, `transport`, `index`, `overlay`, `gateway`).
* Add **Tokio runtime flavors** workflow (`rt-current-thread` and `rt-multi-thread`).
* (Optional but excellent) Add the **kernel loom test** for readiness DAG.

**Exit criteria:**

* All three workflows green on your PRs.
* New PRs can’t merge if any of these regress.

---

## Step 10 — Sanity tests (end-to-end safety nets)

**Why:** We want behavior-level confidence after refactors.
**Add small tests:**

1. **Readiness contract**: `/readyz` returns 503 until {config, db, net, bus} all true; then 200.
2. **Single writer**: attempt to create two writers; ensure only one exists (compile-time or runtime assert).
3. **Bus fanout**: two subscribers both receive the same broadcast.
4. **Overlay range**: range scan materializes before the first await; interleave artificial yields and confirm no panic.

**Exit criteria:**

* All new tests pass on both Tokio flavors and under TSan.

---

# Fix order, owners, and expected effort

| Order | Scope                                  | Effort (est.) | Risk | Owner (suggest)  |
| ----: | -------------------------------------- | ------------: | ---: | ---------------- |
|     1 | `kameo::Context` Default               |      5–10 min |  Low | kameo maintainer |
|     2 | Runtime sleep in transport             |     30–90 min | High | kernel/transport |
|     3 | Single-writer audit                    |     30–60 min |  Med | transport        |
|     4 | Lock-across-await shrink               |     30–60 min |  Med | module authors   |
|     5 | Bus receiver contract + test           |     20–40 min |  Med | kernel           |
|     6 | Sled iterator audit                    |     30–60 min |  Med | overlay/index    |
|     7 | Metrics registration centralization    |     20–40 min |  Med | kernel           |
|     8 | HTTP body policy (Bytes on hot paths)  |     15–30 min |  Low | gateway/overlay  |
|     9 | Add TSan + RT-flavors (+loom optional) |     20–40 min |  Low | CI maintainer    |
|    10 | Add sanity tests                       |     30–60 min |  Med | respective mods  |

If you want to move fast with minimal context switches, do **1 → 2 → 7 → 3/4/6 in parallel → 9 → 10**.

---

## Acceptance checklist (green light to merge)

* `cargo clippy ...` is **green** on workspace.
* **No runtime sleeps** outside demos/tests.
* **No lock held across `.await`** in runtime.
* **Single writer per connection** enforced by design.
* **Each task has its own broadcast receiver**.
* **No DB iterator spans awaits**.
* **Metrics registered once**; no panics on double init.
* **Hot HTTP** paths return `Bytes`.
* **CI**: Invariants + TSan + RT-flavors (and loom if added) all green.

---

When you’re ready for implementation, tell me which step you want first and I’ll return **full, paste-ready files** (or diffs) tailored to your repo layout.
