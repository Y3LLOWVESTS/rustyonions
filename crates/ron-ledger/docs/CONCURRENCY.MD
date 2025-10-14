
---

# 📄 Paste-ready — `crates/ron-ledger/docs/CONCURRENCY.md`

````markdown
---
title: ron-ledger — Concurrency Model
status: reviewed
msrv: 1.80.0
last-updated: 2025-10-13
audience: contributors, ops, auditors
pillar: P12 — Economics & Wallets
concerns: [SEC, PERF, RES]
owners: [Stevan White]
links:
  - IDB: ./IDB.md
  - SECURITY: ./SECURITY.md
  - CONFIG: ./CONFIG.md
  - OBSERVABILITY: ./OBSERVABILITY.md
---

# 0) Purpose

Define a **deterministic, auditable, backpressure-aware** concurrency model for `ron-ledger` that preserves the IDB invariants:
- I-1 Append-only, I-2 Conservation, I-6 Idempotency, I-7 Ordering, I-13 Crash-recovery monotonicity, I-8 Observability.

This doc is **enforceable** via tests, CI gates, and lints (see §14).

---

# 1) Concurrency Invariants (MUST)

1. **Single Writer (I-7).** Exactly one **Committer** performs the atomic append + accumulator update. No other task mutates the ledger state.
2. **No locks across `.await`.** Never hold a mutex/RwLock guard over an `.await`. Use ownership transfer + channels or interior batching.
3. **Bounded queues.** Every channel is bounded; overflow triggers defined backpressure policy (reject or drop-oldest) and **metrics**.
4. **Deterministic ordering.** One **Sequencer** assigns strictly increasing `seq`; tie-break is stable `(source_ts, entry.id)`.
5. **Idempotent batching (I-6).** Re-ingesting the same idempotency tuple `(account, amount, nonce[, idem_id])` is a no-op.
6. **Crash-safe drain (I-13).** On shutdown, queues are drained to a well-defined boundary; on restart, WAL replay yields the **same** `new_root`.
7. **Backpressure > unbounded parallelism.** We prefer bounded queues + caller rejections over internal unbounded growth.
8. **No task leaks.** All spawned tasks are tracked and joined via `JoinSet`; exported metric `tasks_leaked_total == 0`.
9. **Timeouts & breakers.** External calls (KMS/policy/wallet) have deadlines + circuit breakers; auth failures are not retried blindly.
10. **Poison-free.** A failed prevalidator/worker never poisons global state; supervisor restarts with jittered backoff.
11. **Observability (I-8).** Queue depths, busy rejections, latency histograms, and safe-mode toggles are emitted continuously.
12. **Amnesia compatibility (I-10).** Concurrency behavior is identical for in-mem and persistent engines; only the Committer sink differs.

---

# 2) Runtime Topology

**Tasks**
- `Supervisor` — owns a `JoinSet`, restarts children with jittered backoff, escalates repeated failures.
- `Ingress` — accepts HTTP/UDS requests, enforces body limits, emits to `ingress_tx`.
- `Prevalidator Pool (N)` — N workers run capability+policy+wallet checks; emits valid items to `preval_tx`.
- `Sequencer` — assigns `seq`, groups batches deterministically; emits to `commit_tx`.
- `Committer` — **single writer**; append + compute `new_root` + fsync/checkpoint; emits `root_pub_tx`.
- `RootsPublisher` — publishes roots to subscribers and `/roots`.
- `Health/Ready` — calculates `/healthz` & `/readyz` based on queue pressure and Committer lag.
- `Metrics/OTEL` — scrapes internal gauges, pushes OTLP spans.
- `Admin` — handles drains, safe-mode, snapshot/export.

**Worker sizing (config-driven)**
- `concurrency.prevalidators = max(2, num_cpus / 2)` (override via config)
- `concurrency.ingress_workers = max(1, num_cpus / 4)`
- Always **1** `Sequencer`, **1** `Committer`.

```mermaid
flowchart LR
  A[Ingress] -->|bounded mpsc| B[Prevalidator Pool (N)]
  B -->|bounded mpsc| C[Sequencer (1)]
  C -->|bounded mpsc| D[Committer (1)]
  D --> E[RootsPublisher]
  D --> F[Health/Ready]
  subgraph Supervisor
    A;B;C;D;E;F
  end
````

---

# 3) Queues & Backpressure

| Queue         | Type      | Capacity (cfg)                | Policy on full            | Metrics (min)                                                    |
| ------------- | --------- | ----------------------------- | ------------------------- | ---------------------------------------------------------------- |
| `ingress_tx`  | mpsc      | `queues.ingress` (default 2k) | **Reject** (429/Busy)     | `queue_depth{q="ingress"}`, `busy_rejections_total{q="ingress"}` |
| `preval_tx`   | mpsc      | `queues.prevalidator` (2k)    | **Drop-oldest** validated | `queue_depth{q="preval"}`, `drops_total{q="preval"}`             |
| `commit_tx`   | mpsc      | `queues.commit` (1k)          | **Reject** (escalate)     | `queue_depth{q="commit"}`, `busy_rejections_total{q="commit"}`   |
| `root_pub_tx` | broadcast | n/a (subscriber buffers)      | n/a                       | `roots_published_total`                                          |

* **Rationale:** Reject at the edges (ingress/commit) to bound latency; allow limited drop-oldest on prevalidated queue to absorb micro-bursts without violating ordering (Sequencer consumes in order; drop happens *before* sequencing).

---

# 4) Shared State & Locking

* **Allowed:** `Arc<Atomic*>`, lock-free counters, per-task owned buffers.
* **Forbidden:** Locks held across `.await`; nested locks without a strict hierarchy.
* **Hierarchy (if unavoidable):** config < metrics < small_counters (never take two at once).
* **Commit path:** Committer owns WAL/accumulator exclusively; other tasks never lock ledger state.

---

# 5) Timeouts, Retries, Breakers

| Operation                 | Timeout      | Retries                        | Breaker           |
| ------------------------- | ------------ | ------------------------------ | ----------------- |
| Capability verify (KMS)   | 1s           | 2 (jittered, ≤ 250ms)          | 5xx opens breaker |
| Policy guard              | 800ms        | 1 (≤ 150ms)                    | 5xx opens breaker |
| Wallet double-spend check | 1s           | 1 (≤ 150ms)                    | 5xx opens breaker |
| WAL fsync/checkpoint      | commit-local | none (fail-closed → safe-mode) | n/a               |

* Auth failures (4xx) **do not** retry; respond with structured `RejectReason`.
* Breakers half-open after 5s and require 1 success.

---

# 6) Cancellation & Graceful Shutdown

* **Propagation:** broadcast `shutdown_rx` to all tasks; every `.await` uses `tokio::select!`.
* **Drain order:** Ingress stop → Prevalidators drain → Sequencer drains → Committer drains → publish final root.
* **Deadlines:** Prevalidators/Sequencer have 3s to drain; Committer finishes current batch or marks partial and enters **safe-mode**.
* **Task management:** All tasks registered in `JoinSet`; joined at end; assert `tasks_leaked_total == 0`.

```mermaid
stateDiagram-v2
  [*] --> Running
  Running --> Draining: shutdown signal
  Draining --> SafeMode: commit error or timeout
  Draining --> Stopped: queues empty & root published
  SafeMode --> Stopped: operator ack / restore replay
```

---

# 7) I/O & Framing

* **HTTP/UDS:** Deny bodies over `limits.max_body_bytes` (cfg). UDS uses **PEERCRED** for identity in addition to capability.
* **Framing invariants:** `max_frame = 1 MiB` (OAP/1), streaming chunk size = **64 KiB** (matches INTEROP.md).
* **DoS safety:** Early size checks, read timeouts, and slowloris guards on ingress.

---

# 8) Error Taxonomy (Concurrency-relevant)

| Error                      | Caller Action              | Ledger Action                         | Metric(s)                                    |
| -------------------------- | -------------------------- | ------------------------------------- | -------------------------------------------- |
| `Busy` (ingress full)      | Retry with backoff         | Increment rejection; keep operating   | `busy_rejections_total{q="ingress"}`         |
| `PrevalDropOldest`         | Retry (client optional)    | Drop oldest validated (pre-sequence)  | `drops_total{q="preval"}`                    |
| `CommitBackpressure`       | Retry or shed load         | Escalate; WARN→PAGE if sustained      | `busy_rejections_total{q="commit"}`          |
| `SafeMode`                 | Observe read-only          | Freeze writes; operator repair/replay | `ledger_safe_mode`, `seq_gap_detected_total` |
| DeadlineExceeded (ext svc) | Backoff or alternate route | Trip breaker; log redacted span       | `io_timeouts_total{op}`                      |

---

# 9) Observability & Tracing

**Metrics (additive to IDB §7):**

* `queue_depth{q}`
* `busy_rejections_total{q}`
* `drops_total{q}`
* `tasks_leaked_total`
* `prevalidator_duration_seconds` (histogram)
* `sequencer_batch_size` (histogram)
* `committer_batch_latency_seconds` (histogram)

**Alerts (initial SLOs):**

* p95 `committer_batch_latency_seconds` > **80ms** (5m) → **WARN**
* `queue_depth{q="commit"}` > **0.8 * capacity** (2m) → **PAGE**
* `busy_rejections_total{q="ingress"}` rate > **50/s** (1m) → **WARN**
* `tasks_leaked_total` > 0 → **PAGE**

**OpenTelemetry (OTLP):**

* Spans per task: `ingress`, `prevalidator`, `sequencer`, `committer`.
* Span attributes: `corr_id`, `seq`, `entry_id`, `kid`, `capability_ref`, `queue`, `batch_size`.
* Sampling: 1% baseline; **100% when `ledger_safe_mode=1`**.

---

# 10) Validation Plan

**Unit/Property**

* Deterministic batch ordering: same inputs → same `(seq, root)`.
* Idempotency: duplicate tuples are no-ops.
* Backpressure: fill queues to capacity; assert reject/drop metrics.

**Loom (minimal model sketch)**

```rust
// Pseudocode outline (doc-only):
loom::model(|| {
  let q = BoundedQueue::new(2);
  let produced = Arc::new(AtomicUsize::new(0));
  let consumed = Arc::new(AtomicUsize::new(0));

  // producer
  thread::spawn({
    let q = q.clone(); let p = produced.clone();
    move || { for _ in 0..3 { q.try_send(1).ok(); p.fetch_add(1, SeqCst); } }
  });

  // consumer with shutdown
  let shutdown = loom::sync::atomic::AtomicBool::new(false);
  thread::spawn({
    let q = q.clone(); let c = consumed.clone(); let s = &shutdown;
    move || { while !s.load(SeqCst) { if let Some(_v) = q.try_recv() { c.fetch_add(1, SeqCst); } } }
  });

  // simulate shutdown at any point
  shutdown.store(true, SeqCst);

  // Assert: consumed <= produced; no deadlock; queue bounded.
});
```

**Fuzz**

* Ingress parsers, batch boundaries, reject taxonomy.

**Chaos**

* SIGKILL Committer mid-fsync (expect safe-mode + monotonic recovery).
* OOM prevalidator (expect supervisor restart, no poisoning).
* Disk-full during append (fail-closed; alert).

**TLA+ (state sketch)**

* States: `Validated -> Sequenced -> Committed`.
* Invariants: single writer; no duplication; conservation preserved across transitions.

---

# 11) Code Patterns (copy-paste)

**Spawn with shutdown:**

```rust
let (tx, rx) = tokio::sync::mpsc::channel(cfg.queues.ingress);
joinset.spawn(async move {
  loop {
    tokio::select! {
      Some(req) = rx.recv() => handle(req).await?,
      _ = shutdown.recv() => break,
    }
  }
});
```

**Non-blocking send with metrics:**

```rust
match tx.try_send(item) {
  Ok(()) => {}
  Err(TrySendError::Full(_)) => {
    METRICS.busy_rejections.inc(&["ingress"]);
    return Err(Error::Busy);
  }
  Err(e) => return Err(e.into()),
}
```

**Retry with jittered backoff (external check):**

```rust
let mut delay = std::time::Duration::from_millis(50);
for attempt in 0..=cfg.retries.policy_guard.max_retries {
  match policy_guard(&req).await {
    Ok(ok) => break Ok(ok),
    Err(Error::Auth(_)) => break Err(Error::Auth(_)), // don't retry 4xx
    Err(_) if attempt < cfg.retries.policy_guard.max_retries => {
      tokio::time::sleep(jitter(delay)).await;
      delay = delay.saturating_mul(2).min(cfg.retries.policy_guard.max_backoff);
    }
    Err(e) => break Err(e),
  }
}
```

**Never hold a lock across `.await`:**

```rust
// BAD
let mut g = state.lock();
g.update();
some_async().await; // ❌

/* GOOD */
let delta = { let mut g = state.lock(); g.compute_delta() }; // drop before await
apply_async(delta).await;
```

**Testing shutdown injection:**

```rust
let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
let task = tokio::spawn(run_with_shutdown(shutdown_rx));
shutdown_tx.send(()).ok();
task.await?;
```

---

# 12) Config Hooks (minimum)

```toml
[concurrency]
prevalidators = 4         # default = max(2, num_cpus/2)
ingress_workers = 2

[queues]
ingress = 2000
prevalidator = 2000
commit = 1000

[limits]
max_body_bytes = "1MiB"

[timeouts]
kms_ms = 1000
policy_ms = 800
wallet_ms = 1000

[retries.policy_guard]
max_retries = 1
max_backoff = "150ms"
```

---

# 13) Trade-offs

* **Reject vs drop-oldest:** we reject at ingress/commit to bound tail latency; `preval_tx` may drop-oldest pre-sequence to ride micro-bursts without violating ordering.
* **Single threaded writer:** simplifies invariants and recovery at minor throughput cost; we scale by upstream batching, not multi-writer.
* **Tokio multi-threaded runtime:** chosen for I/O; Committer remains logically single-threaded.

---

# 14) CI & Lints Gates

* `cargo clippy -- -D clippy::await_holding_lock -D clippy::mutex_atomic`
* Loom model tests in `loom/` job (runs under `--cfg loom`).
* Fuzz targets build and smoke-run (libFuzzer/AFL as configured).
* Concurrency SLO checks (queue depth, busy rejections, commit latency) run in perf CI and must stay under thresholds (see §9 alerts).

---

# 15) Review & Drift Control

* Any change that adds a new task, channel, or backpressure policy **must** update:

  * This doc (§2–§4), CONFIG defaults (§12), OBSERVABILITY dashboards/alerts (§9), and TESTS (Loom/fuzz/chaos).
* PR template includes a “Concurrency Checklist” with these items.

```
```

---

## Why this addresses Grok’s nits

* **Task-leak enforcement**: explicit `JoinSet` requirement + `tasks_leaked_total` metric.
* **Sizing tied to config**: worker counts derived from CPU with config overrides.
* **Loom/TLA+**: included concrete Loom model sketch and TLA+ state outline.
* **SLO-backed alerts**: queue/latency alert thresholds added in §9; matches IDB perf targets.
* **OTEL tracing**: spans, attributes, and sampling policy (100% in safe-mode).

