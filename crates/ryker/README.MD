

---

````markdown
# ryker

> **Role:** library (embedded actor & mailbox runtime)  
> **Owner:** Stevan White  
> **Status:** draft  
> **MSRV:** 1.80.0  
> **Last reviewed:** 2025-09-28

Badges:  
[![Build](https://img.shields.io/badge/build-CI-green)]() [![Crates.io](https://img.shields.io/crates/v/ryker)]() [![Docs.rs](https://img.shields.io/docsrs/ryker)]() [![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]() [![MSRV 1.80](https://img.shields.io/badge/MSRV-1.80.0-blue.svg)]() [![SemVer](https://img.shields.io/badge/semver-stable-informational)]()

---

## 1) Overview

**What it is (one paragraph):**  
`ryker` is a minimal, embedded actor & mailbox runtime that provides bounded single-consumer mailboxes, a small supervision wrapper, deterministic deadlines, and observability hooks. It never runs its own async runtime or background daemons; hosts create actors/mailboxes and own task lifecycles. All queues are bounded (drops are counted), deadlines are first-class, and cancellation is cooperative—focusing on resilience and predictable backpressure over buffering.

**How it fits (RustyOnions topology):**  
- **Pillar:** 1 — Kernel & Orchestration.  
- **Upstream callers:** `ron-kernel` (supervision patterns), service crates embedding actors.  
- **Downstream deps:** in-proc channels, `tokio`, optional `metrics`/`tracing`; no network/DB backends.  
- **Data it touches:** in-memory only (no persistence), with amnesia/zeroize hooks available.  
- **Security boundary:** library (no PII). Hosts control endpoints & custody; amnesia mode forbids disk spill.

### 1.1 High-Level Architecture

```mermaid
flowchart LR
  subgraph Host Service / ron-kernel
    A[Caller task(s)] -->|enqueue msg| B(ryker::Mailbox<T>)
    B -->|pull/handle| C[Actor task]
    A -. oneshot reply .- C
  end

  C -->|metrics hooks| E[[Prometheus via host]]
  C -->|logs/spans| F[[Tracing]]

  style B fill:#0b7285,stroke:#083344,color:#fff
  style C fill:#0b7285,stroke:#083344,color:#fff
````

*Alt (Architecture): Single-consumer mailbox; host owns runtime, ryker exposes hooks only.*

---

## 2) Responsibilities & Boundaries

**MUST do:**

* [ ] Provide bounded single-consumer mailboxes with explicit Busy/Drop semantics and counters.
* [ ] Expose deterministic deadlines/cancellation and fairness knobs (batch/yield).
* [ ] Offer a supervision wrapper with jittered backoff and restart accounting.

**MUST NOT do:**

* [ ] Own a global runtime, bind sockets, or ship HTTP endpoints.
* [ ] Implement broadcast/fan-out (use the kernel Bus instead).
* [ ] Parse network protocols, handle TLS/keys, or persist data.

**Acceptance Gates (PROOF you did it):**

* [ ] Backpressure property test → `tests/prop_mailbox_backpressure.rs::prop_full_queue_busy`
* [ ] Deadline enforcement → `tests/loom_deadline.rs::loom_deadline_observed`
* [ ] No-locks-across-`.await` → `clippy` gate + `tests/loom_no_lock.rs::no_lock_across_await`
* [ ] Restart/backoff bounds → `tests/supervisor_backoff.rs::decorrelated_jitter_bounds`

---

## 3) Public Interfaces

### 3.1 Rust API (library)

Key modules/types:

* `ryker::mailbox::Mailbox<T>` — bounded queue with `try_send`, `send_with_deadline`.
* `ryker::supervisor::Supervisor` — spawn/restart wrapper with jittered backoff.
* `ryker::config::RykerConfig` — validated snapshot (env/file/builder).
* `ryker::observe::MailboxObserver` — overflow/drain hooks for metrics.

#### Minimal usage

```rust
use ryker::config::RykerConfig;

fn main() -> anyhow::Result<()> {
    let cfg = RykerConfig::from_env_validated()?;
    let rt = ryker::Runtime::new(cfg);
    let inbox = rt.mailbox::<String>("worker").capacity(256).build();
    inbox.try_send("hello".to_string()).map_err(|e| anyhow::anyhow!(e))?;
    Ok(())
}
```

#### Actor loop example (full)

```rust
/// Minimal actor loop: pulls messages until cancellation; enforces deadlines.
use ryker::{config::RykerConfig, mailbox::Mailbox};
use std::time::Duration;

#[derive(Debug)]
enum Msg { Work(u64), Quit }

fn main() -> anyhow::Result<()> {
    let cfg = RykerConfig::from_env_validated()?;
    let rt  = ryker::Runtime::new(cfg);
    let inbox: Mailbox<Msg> = rt.mailbox("worker")
        .capacity(256)
        .deadline(Duration::from_millis(500))
        .build();

    inbox.try_send(Msg::Work(42))?;
    inbox.try_send(Msg::Quit)?;

    loop {
        match inbox.pull()? {
            Msg::Work(n) => { /* handle */ let _ = n; }
            Msg::Quit => break,
        }
    }
    Ok(())
}
```

---

## 4) Configuration

| Variable                         | Type     | Default | Description                  |
| -------------------------------- | -------- | ------- | ---------------------------- |
| `RYKER_DEFAULT_MAILBOX_CAPACITY` | int      | 256     | Per-mailbox capacity (cold)  |
| `RYKER_DEFAULT_MAX_MSG_BYTES`    | size     | 64KiB   | Max message size (cold)      |
| `RYKER_DEFAULT_DEADLINE`         | duration | 1s      | Default per-message deadline |
| `RYKER_BACKOFF_BASE_MS`          | int      | 100     | Supervisor base backoff ms   |
| `RYKER_BACKOFF_CAP_MS`           | int      | 5000    | Supervisor cap ms            |
| `RYKER_BATCH_MESSAGES`           | int      | 32      | Fairness: batch size         |
| `RYKER_YIELD_EVERY_N`            | int      | 64      | Fairness: yield interval     |
| `RYKER_ENABLE_METRICS`           | bool     | true    | Enable metric emission       |
| `RYKER_QUEUE_DEPTH_SAMPLING`     | bool     | true    | Depth sampling               |
| `RYKER_AMNESIA`                  | bool     | false   | Amnesia mode (zeroize/drop)  |

**Feature flags:**

| Feature | Default | Effect                                   |
| ------- | ------- | ---------------------------------------- |
| metrics | on      | emits counters/histograms via hooks      |
| tracing | on      | span fields: actor, deadline_ms, outcome |
| amnesia | on      | zeroize-on-drop; no disk spill           |
| loom    | off     | loom tests (dev only)                    |
| dev-cli | off     | `ryker config print` helper              |

---

## 5) Build, Run, Test

```bash
cargo build -p ryker
cargo test -p ryker --all-features
cargo test -p ryker --doc
cargo fmt --all
cargo clippy -p ryker -- -D warnings -W clippy::await_holding_lock
cargo deny check
cargo bench -p ryker
```

---

## 6) Observability

**Canonical Metrics:**

* `ryker_mailbox_depth{actor}` (gauge)
* `ryker_mailbox_dropped_total{actor,reason}` (counter)
* `ryker_busy_rejections_total{actor}` (counter)
* `ryker_handler_latency_seconds{actor,outcome}` (histogram)
* `ryker_actor_restarts_total{actor}` (counter)

**Tracing:** spans: `ryker.mailbox.enqueue`, `ryker.actor.handle`, `ryker.config.reload`.

---

## 7) Performance & SLOs

| Metric              | Target      | Notes                     |
| ------------------- | ----------- | ------------------------- |
| Handler p50 latency | ≤ 2 ms      | steady-state              |
| Handler p99 latency | ≤ 20 ms     | with batch size 32        |
| Enqueue allocs      | ~0 alloc/op | amortized path            |
| Span overhead       | p95 ≤ 5×    | debug vs info span levels |

---

## 8) Data & Schema

No wire schemas or persistence. Prefer `bytes::Bytes`. Avoid PII in logs/metrics.

---

## 9) Security & Privacy

* **Threat model:** bounded queues prevent overload; no PII.
* **Amnesia:** optional; zeroize RAM-only.
* **PQ:** N/A (see QUANTUM.md in repo).

---

## 10) Error Taxonomy

| Code/Variant           | When it happens    | Host mapping                  |
| ---------------------- | ------------------ | ----------------------------- |
| `Error::Busy`          | queue at capacity  | HTTP 429 / RESOURCE_EXHAUSTED |
| `Timeout`              | deadline exceeded  | HTTP 504 / DEADLINE_EXCEEDED  |
| `ConfigError::Invalid` | bad env/file value | startup fail/reload rejected  |

---

## 11) Concurrency Model

* **Rules:** never hold a lock across `.await`; single writer/consumer; cooperative cancellation.

```mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Running: Supervisor::spawn(actor)
  Running --> Backoff: panic/failure
  Backoff --> Running: jittered restart
  Running --> Shutdown: cancel/ctrl_c
  Shutdown --> [*]
```

*Alt (State): Actor lifecycle with restart/backoff and shutdown.*

---

## 12) Compatibility

* Rust 1.80.0+
* macOS, Linux, containers safe
* No HTTP/TLS/DB deps exposed

---

## 13) Examples

```rust
use ryker::config::RykerConfig;

let cfg = RykerConfig::from_env_validated()?;
let rt  = ryker::Runtime::new(cfg);
let inbox = rt.mailbox::<MyMsg>("worker").capacity(256).deadline_ms(1000).build();
```

---

## 14) Troubleshooting

* **Busy errors:** queue full → adapt producer rate or increase capacity.
  Next action:

  ```bash
  cargo test -p ryker prop_mailbox_backpressure -- --nocapture
  ```
* **Deadline timeouts:** increase per-call deadline or reduce handler work.
  Next action:

  ```bash
  RYKER_DEFAULT_DEADLINE=1500 cargo bench -p ryker
  ```
* **Locks across .await:** fix clippy lint; drop guards before `.await`.

---

## 15) Development Notes

* No broadcast here—use the kernel Bus.
* Supervisor uses decorrelated jitter; restarts counted.
* Config reload: fairness/deadline hot; capacity/size cold.

```bash
cargo fmt --all
cargo clippy -- -D warnings -W clippy::await_holding_lock
cargo test -p ryker --all-features
cargo deny check
```

---

## 16) Mermaid Policy

* Architecture, Sequence, and State diagrams included.
* Render locally:

  ```bash
  npm i -g @mermaid-js/mermaid-cli
  mmdc -i docs/arch.mmd -o docs/arch.svg
  ```

---

## 17) Roadmap & TODO

* [ ] Bronze → Silver → Gold test gates
* [ ] Add queue depth gauge & hist buckets
* [ ] Expand fairness knobs doc/examples

---

## 18) Changelog

See [CHANGELOG.md](./CHANGELOG.md).

---

## 19) License

Dual-licensed under **MIT** or **Apache-2.0**.

---

## 20) Contributing

PRs welcome! Please run fmt/clippy/tests and include:

* Proof of backpressure/deadline invariants in tests/benches
* Updated diagrams if concurrency behavior changed
* `cargo public-api` / semver checks clean

```

---
