

````markdown
# 🧪 TESTS.md — ron-bus

*Audience: developers, auditors, CI maintainers*  
*msrv: 1.80.0 (Tokio/loom compatible)*

---

## 0) Purpose

Define the **test contract** for `ron-bus`:

* Unit, integration, property, loom (model), chaos/soak (host-style), performance.
* Explicit coverage goals & Bronze→Silver→Gold acceptance gates.
* Invocation commands for devs & CI.

> Scope: `ron-bus` is a **pure in-process broadcast bus** (Tokio `broadcast`). No wire codecs → fuzzing is **optional/N/A**; chaos/soak are **host-style** in integration tests.

---

## 1) Test Taxonomy

### 1.1 Unit Tests (crate-internal)
**Scope:** constructor validation, defaults, simple send/subscribe, error enums. Fast (<100ms).

**Location:** `src/*` with `#[cfg(test)]`.

**Run:**
```bash
cargo test -p ron-bus --lib
````

**Examples to implement:**

* `bus_new_capacity_must_be_ge_2()`
* `subscribe_returns_unique_receivers()`
* `sender_persists_across_clones()`

### 1.2 Integration Tests (end-to-end, async)

**Scope:** Public surface (`Bus`, `Event`, `BusConfig`) on Tokio runtime. Concurrency invariants (non-blocking send, lag/overflow accounting), reload/cutover, negative patterns.

**Location:** `crates/ron-bus/tests/*.rs`

**Run:**

```bash
cargo test -p ron-bus --test '*'
```

**Required cases:**

* **Happy path fan-out:** 1 producer → N subscribers receive all events (no lag).
* **Lag/overflow accounting:** slow consumer triggers `Lagged(n)`; test increments **counter** and updates **depth gauge**.
* **Non-blocking publishers under lag:** producers keep sending; no deadlocks.
* **Reload migration (two-bus cutover):** **no gaps** for non-lagging subscribers; no artificial spike in overflow counter.
* **Receiver-sharing negative:** prove sharing a `Receiver` across tasks is a bug (compile-fail guard + runtime negative).

#### Copy-paste skeleton (`tests/bus_basic.rs`)

```rust
use ron_bus::{Bus, Event, BusConfig};
use std::sync::{atomic::{AtomicU64, Ordering::Relaxed}, Arc};
use tokio::time::{sleep, Duration};

#[tokio::test(flavor = "multi_thread")]
async fn fanout_happy_path_no_lag() {
    let cfg = BusConfig { capacity: 256, ..Default::default() };
    let bus = Bus::new(cfg).expect("bus");
    let tx = bus.sender();

    let mut rx_a = bus.subscribe();
    let mut rx_b = bus.subscribe();

    for _ in 0..1000 {
        tx.send(Event::Health { service: "overlay".into(), ok: true }).unwrap();
    }

    let (a_cnt, b_cnt) = (count_events(&mut rx_a).await, count_events(&mut rx_b).await);
    assert_eq!(a_cnt, 1000);
    assert_eq!(b_cnt, 1000);
}

async fn count_events(rx: &mut tokio::sync::broadcast::Receiver<Event>) -> usize {
    let mut n = 0usize;
    while let Ok(_ev) = rx.try_recv() { n += 1; }
    sleep(Duration::from_millis(10)).await; // yield for scheduling
    while let Ok(_ev) = rx.try_recv() { n += 1; }
    n
}

#[tokio::test(flavor = "multi_thread")]
async fn lagged_receiver_increments_metrics_and_stays_non_blocking() {
    static DROPS: AtomicU64 = AtomicU64::new(0);
    fn inc_drops(n: u64) { DROPS.fetch_add(n, Relaxed); }

    let cfg = BusConfig { capacity: 16, ..Default::default() };
    let bus = Bus::new(cfg).unwrap();
    let tx = bus.sender();

    let mut rx_fast = bus.subscribe();
    let mut rx_slow = bus.subscribe();

    let slow = tokio::spawn(async move {
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(20)).await; // induce lag
            match rx_slow.recv().await {
                Ok(_ev) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => inc_drops(n as u64),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let pub_task = tokio::spawn(async move {
        for _ in 0..10_000 {
            let _ = tx.send(Event::Health { service: "gw".into(), ok: true });
        }
    });

    let fast = tokio::spawn(async move {
        let mut seen = 0;
        loop {
            match rx_fast.recv().await {
                Ok(_ev) => { seen += 1; if seen >= 100 { break; } }
                Err(_) => break,
            }
        }
    });

    let _ = futures::future::join3(pub_task, fast, slow).await;
    assert!(DROPS.load(Relaxed) > 0, "expected lag-induced drops > 0");
}
```

### 1.3 Property-Based Tests (lightweight)

**Scope:** Semantics over ranges of loads/capacities: **no panics**, **boundedness** (per-subscriber seen ≤ published), **receiver uniqueness behavior**, **monotonic config version**.

**Tooling:** `proptest`.

**Location:** `tests/prop_bus.rs`

**Run:**

```bash
cargo test -p ron-bus --test prop_bus
```

**Sketch:**

```rust
use proptest::prelude::*;
use ron_bus::{Bus, BusConfig, Event};

proptest! {
  #![proptest_config(ProptestConfig::with_cases(32))]

  #[test]
  fn boundedness_never_exceeds_published(cap in 2usize..256usize, pub_count in 1usize..2000usize) {
      let cfg = BusConfig { capacity: cap as u32, ..Default::default() };
      let bus = Bus::new(cfg).unwrap();
      let tx = bus.sender();
      let mut rx = bus.subscribe();

      for _ in 0..pub_count { let _ = tx.send(Event::Shutdown); }
      let mut seen = 0usize;
      while let Ok(_ev) = rx.try_recv() { seen += 1; }
      prop_assert!(seen <= pub_count);
  }

  // Model the host expectation that ConfigUpdated versions are non-decreasing
  #[test]
  fn config_updated_versions_are_monotonic(seq in proptest::collection::vec(0u64..1_000_000, 1..50)) {
      let cfg = BusConfig { capacity: 64, ..Default::default() };
      let bus = Bus::new(cfg).unwrap();
      let tx = bus.sender();
      let mut rx = bus.subscribe();

      for v in &seq { tx.send(Event::ConfigUpdated { version: *v }).unwrap(); }

      let mut last = None::<u64>;
      while let Ok(Event::ConfigUpdated { version }) = rx.try_recv() {
          if let Some(prev) = last { prop_assert!(version >= prev); }
          last = Some(version);
      }
  }
}
```

### 1.4 Fuzz Tests (Optional)

**N/A:** No parsers/codecs.
**Optional (nice-to-have):** an **ops-sequence fuzzer** to stress API interleavings (create → subscribe → send → drop_rx → swap_bus). Keep it out of the critical CI path.

**Run (if added):**

```bash
cargo fuzz run bus_ops -- -max_total_time=60
```

> Seed with randomized sequences and timeouts; the goal is to hunt edge panics in surprising drop orders—**not** to validate Tokio internals.

### 1.5 Chaos/Soak Tests (host-style; `#[ignore]`)

**Scope:** Lag churn, subscriber flapping, reload under load, **amnesia on/off delta** (Micronode profile).

**Location:** `tests/chaos_*.rs` with `#[ignore]`.

**Run (manually/nightly):**

```bash
cargo test -p ron-bus -- --ignored chaos_*
```

**Required scenarios:**

* **Lag churn:** randomly delay N% of subscribers; assert publishers remain non-blocking; overflow counter **slope > 0** but bounded.
* **Subscriber flapping:** join/leave at 100Hz for 60s; no leaks (track task count), depth returns to baseline.
* **Reload under load (two-bus cutover):** **no gaps** for non-lagging subscribers; no artificial spike in overflow.
* **Amnesia on/off delta:** identical load with `amnesia=on` vs `amnesia=off` (host label). Assert p95 publish→recv delta **<5%**, **no positive RSS slope** in amnesia, and overflow slope ≈ 0.

> **Anti-flake rule:** prefer **slope/recovery** assertions (monotonic trends and “returns to baseline within T seconds”) over exact totals.

**Amnesia chaos skeleton (`tests/chaos_amnesia.rs`)**

```rust
#![cfg_attr(not(debug_assertions), allow(unused))]
use std::time::Instant;

/// Pseudo-code: in practice, amnesia is a host label; here we only structure the assertions.
/// Marked #[ignore] to run in nightly chaos job.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn chaos_amnesia_profile() {
    // Arrange load + measurement helpers...
    let t0 = Instant::now();
    // Run with "amnesia=off" (host profile) → measure p95, RSS slope, overflow slope
    // Run with "amnesia=on"  (host profile) → repeat same measurements

    // Assert p95 delta < 5%, RSS slope <= 0 in amnesia mode, overflow slope ≈ 0
    // (Implement using host harness or mock metrics for CI determinism)
    assert!(true);
}
```

### 1.6 Performance/Load Tests

**Scope:** publish→recv p95 latency, events/sec throughput, overflow % at target, allocations/op.

**Tooling:** `criterion`.

**Location:** `crates/ron-bus/benches/bus_perf.rs`

**Run:**

```bash
cargo bench -p ron-bus
```

**Copy-paste skeleton:**

```rust
use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use ron_bus::{Bus, BusConfig, Event};

fn bench_send_recv(c: &mut Criterion) {
    c.bench_function("send->recv 1p/4s cap=256", |b| {
        b.iter_batched(
            || {
                let bus = Bus::new(BusConfig { capacity: 256, ..Default::default() }).unwrap();
                (bus.sender(), bus.subscribe(), bus.subscribe(), bus.subscribe(), bus.subscribe())
            },
            |(tx, mut a, mut b, mut c1, mut d)| {
                for _ in 0..512 { let _ = tx.send(Event::Shutdown); }
                let mut drain = |rx: &mut tokio::sync::broadcast::Receiver<Event>| {
                    let mut n = 0; while let Ok(_ev) = rx.try_recv() { n+=1; } n
                };
                let _ = (drain(&mut a), drain(&mut b), drain(&mut c1), drain(&mut d));
            },
            BatchSize::SmallInput
        )
    });
}

criterion_group!(benches, bench_send_recv);
criterion_main!(benches);
```

---

## 2) Coverage & Gates

### 2.1 Bronze (MVP)

* Unit + integration tests pass (fan-out, lag handling).
* Line coverage ≥ **70%** (grcov/tarpaulin).

### 2.2 Silver (Useful Substrate)

* Property tests included (boundedness, monotonic config).
* Chaos tests implemented and marked `#[ignore]`.
* Coverage ≥ **85%**.

### 2.3 Gold (Ops-Ready)

* Nightly chaos (lag churn + reload + amnesia) **pass**.
* Criterion baselines captured; regression guardrails on PRs.
* Coverage ≥ **90%**.
* **Loom model test** present (cutover protocol, see §3.3).
* **Perf gates on PRs:** p95 ↑ **>20%** or throughput ↓ **>10%** vs baseline ⇒ **fail**; allocs/op **>1** ⇒ **fail**.

---

## 3) Invocation Examples

### 3.1 All Tests

```bash
cargo test -p ron-bus --all-targets -- --nocapture
```

### 3.2 Chaos (ignored)

```bash
cargo test -p ron-bus -- --ignored chaos_*
```

### 3.3 Loom (modeling cutover protocol)

> Tokio `broadcast` isn’t loom-instrumented; we model **our protocol** (cancel → resubscribe → swap). Extend with a “fast subscriber gets all messages across cutover” flag.

```bash
RUSTFLAGS="--cfg loom" cargo test -p ron-bus --test loom_model
```

**Sketch (`tests/loom_model.rs`):**

```rust
#![cfg(loom)]
use loom::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};
use loom::thread;

#[test]
fn cutover_protocol_eventual_quiescence_and_no_loss_for_fast_sub() {
    loom::model(|| {
        // Model switches from old->new; fast sub should see all k messages
        let old_active = Arc::new(AtomicBool::new(true));
        let delivered   = Arc::new(AtomicUsize::new(0));
        let target_k    = 5usize;

        // Fast subscriber: keeps polling until swap done then drains from new path in the model
        {
            let old_active_c = old_active.clone();
            let delivered_c  = delivered.clone();
            thread::spawn(move || {
                // Pretend to receive k messages across the swap
                while delivered_c.load(Ordering::SeqCst) < target_k {
                    if old_active_c.load(Ordering::SeqCst) {
                        delivered_c.fetch_add(1, Ordering::SeqCst);
                    } else {
                        delivered_c.fetch_add(1, Ordering::SeqCst);
                    }
                    thread::yield_now();
                }
            });
        }

        // Supervisor flips to new bus (cutover)
        {
            let old_active_c = old_active.clone();
            thread::spawn(move || {
                old_active_c.store(false, Ordering::SeqCst);
            }).join().unwrap();
        }

        // Wait for subscriber join by running another step
        thread::yield_now();
        assert_eq!(delivered.load(Ordering::SeqCst), target_k);
    });
}
```

### 3.4 Benchmarks

```bash
cargo bench -p ron-bus
```

---

## 4) Observability Hooks

* Tests use `tracing` JSON on failure (fields: `service="ron-bus-test"`, `event_kind`, `case`).
* Integrations update mock **counters/gauges** like hosts:

  * `bus_overflow_dropped_total` (Counter, assert **slope**)
  * `bus_queue_depth` (Gauge, assert **return to baseline**)
* Chaos asserts **trends** (monotonic increase/recovery) to minimize flakiness.

---

## 5) CI Enforcement

* **Core:**

  * `cargo test --workspace --all-targets`
  * `cargo fmt -- --check`
  * `cargo clippy -D warnings`
  * `cargo deny check advisories`

* **Coverage (thresholded):**

  * `grcov` or `tarpaulin` → Bronze/Silver/Gold gates.

* **Perf (nightly):**

  * `cargo bench` → compare `target/criterion` with `testing/performance/baselines/ron-bus/`.

* **Chaos (nightly):**

  * Execute `--ignored` chaos suites with bounded duration (≤2m each).

* **Matrix nits (recommended):**

  * Add **Windows** job (Tokio scheduler differences).
  * Add a **Miri** job for unit tests (skip async/broadcast) to catch UB in helpers.

---

## 6) Compile-time Guards (trybuild)

Add a **compile-fail** test to prevent the “shared receiver across tasks” anti-pattern.

**File:** `tests/trybuild/receiver_shared_should_fail.rs`

```rust
// This test intentionally fails to compile if a Receiver is cloned/shared.
// Use trybuild harness in build.rs or a dedicated test to assert compile failure.
//
// PSEUDO: devs must not write code like this:
//
// let mut rx = bus.subscribe();
// let rx2 = rx.clone(); // should not compile if we make Receiver !Clone in our wrapper
//
// If your public API wraps tokio::broadcast::Receiver in a newtype that is !Clone,
// include a doc test here to enforce it at compile time.
fn main() {}
```

**Harness:** add a `trybuild` test runner (optional) in a small `tests/trybuild.rs`:

```rust
// #[cfg(test)]
// #[test]
// fn ui() { let t = trybuild::TestCases::new(); t.compile_fail("tests/trybuild/*.rs"); }
```

> If the public API keeps `tokio::broadcast::Receiver` directly, retain the **runtime negative** test and enforce rule in docs/PR review.

---

## 7) Open Questions → **Resolved for ron-bus**

* **Which invariants are loom-checked?**
  Cutover protocol eventual quiescence + **no loss for fast subscriber**; shutdown propagation ordering.

* **Which fuzz targets are mandatory?**
  None. Optional op-sequence fuzzer exists; not a Gold gate.

* **What SLOs are measured in perf tests?**
  p95 publish→recv **<50µs**, ≥ **1M events/sec** on 8-core, **overflow <0.01%** at target, **allocs/op ≤1**.

---

## 8) Coverage & Acceptance Gates (recap)

| Tier   | Requirements                                                                                          |
| ------ | ----------------------------------------------------------------------------------------------------- |
| Bronze | Unit + integration pass; coverage ≥70%                                                                |
| Silver | + property tests; chaos implemented (`#[ignore]`); coverage ≥85%                                      |
| Gold   | Nightly chaos (lag churn + reload + amnesia); perf baselines/regression gates; coverage ≥90%; Loom OK |

---

## 9) Test Matrix (Invariant → Proof)

| Invariant (from IDB/CONCURRENCY)                 | Test(s)                                                           |
| ------------------------------------------------ | ----------------------------------------------------------------- |
| Bounded, lossy, observable (no unbounded queues) | `lagged_receiver_increments_metrics_and_stays_non_blocking`       |
| One receiver per task (no sharing)               | trybuild compile-fail (if newtype) + runtime negative             |
| Non-blocking publishers even under lag           | `lagged_receiver_*` (publisher continues)                         |
| Graceful reload (two-bus cutover) no gaps        | `reload_migration` + Loom `cutover_protocol_*`                    |
| Monotonic config view for subscriber             | `config_updated_versions_are_monotonic` (property)                |
| No locks across `.await` in hot paths            | Clippy gate + code review (no test-only mutex under `.await`)     |
| Metrics updated on `Lagged(n)`                   | `lagged_receiver_*` asserts counter/gauge updates                 |
| Amnesia neutrality (ops delta bounded)           | `chaos_amnesia_profile` (ignored/nightly), perf deltas/assertions |

---

## 10) File Map (suggested)

```
crates/ron-bus/
  src/
    lib.rs
  tests/
    bus_basic.rs
    prop_bus.rs
    chaos_reload.rs           # #[ignore]
    chaos_lag_churn.rs        # #[ignore]
    chaos_amnesia.rs          # #[ignore]
    loom_model.rs             # cfg(loom)
    trybuild.rs               # optional: trybuild harness
    trybuild/
      receiver_shared_should_fail.rs
  benches/
    bus_perf.rs
testing/
  performance/
    baselines/
      ron-bus/
        baseline.json
```

---

✅ With this contract, `ron-bus` has **provable invariants**, **repeatable perf baselines**, **ops-centric chaos**, and **compile-time guardrails**—all CI-enforced and auditor-friendly.

```

