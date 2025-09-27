
---

````markdown
---
title: IDB — ron-kernel (Microkernel)
version: 1.0.1
status: draft
last-updated: 2025-09-24
audience: contributors, reviewers, ops, auditors
---

# 🪓 Invariant-Driven Blueprinting — ron-kernel

Pillar: **P1 Kernel & Orchestration**  
Crates impacted: `ron-kernel`, `ron-bus`, `ryker`  
Six Concerns: **SEC, RES, PERF, GOV, DX**  

---

## 1) Invariants (MUST)

- [I-1] **Frozen public API**  
  The kernel re-exports **exactly**:  
  `pub use { Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c };`  
  Any semantic change → **major** bump + migration notes.

- [I-2] **KernelEvent contract (exact shape)**  
  ```rust
  enum KernelEvent {
    Health        { service: String, ok: bool },
    ConfigUpdated { version: u64 },
    ServiceCrashed{ service: String, reason: String },
    Shutdown,
  }
````

* \[I-3] **Crash-only supervision**
  Exponential backoff with full jitter (100ms → 30s cap); **≤ 5 restarts / 60s** per child before quarantine; emits `ServiceCrashed{reason}`.

* \[I-4] **No locks across `.await`** on supervisory or other hot paths.
  (Lint/CI enforced; see §4 Gates.)

* \[I-5] **Bounded queues everywhere**

  * **Bus**: `tokio::broadcast` with bounded capacity (default 4096). Overflow **never blocks kernel**; increments `bus_overflow_dropped_total`.
  * **Mailboxes (Ryker)**: per-service bounded `mpsc` (default 128).
  * **Commands**: request/response via bounded `mpsc` + `oneshot`.

* \[I-6] **TLS type and immutability**
  TLS server config is `Arc<tokio_rustls::rustls::ServerConfig>` and **immutable** post-startup.

* \[I-7] **OAP/1 constants & ownership (delegated enforcement)**
  Protocol **defaults**: `max_frame = 1 MiB` (protocol); storage streaming **chunks \~64 KiB** (I/O detail, not frame size).
  **DATA headers MUST include** `obj:"b3:<hex>"` for object IDs.
  **Enforcement is owned by services/libraries** (`oap`/service handlers). The kernel **does not police payloads**; it **exposes counters/metrics only** (e.g., `rejected_total{reason=oversize}`).

* \[I-8] **Content addressing is BLAKE3**
  Addresses use `b3:<hex>` (BLAKE3-256 of the **unencrypted** object/manifest root). Services **MUST** verify the **full digest** before serving.

* \[I-9] **Global Amnesia Mode**
  A kernel-surfaced, read-only flag (`amnesia = on|off`) present in health snapshots and as a metrics label. **All services must honor it** (RAM-only caches, ephemeral logs, timed key purge, no disk spill).

* \[I-10] **Golden metrics are mandatory**
  Kernel exports:

  * `bus_lagged_total` (counter)
  * `service_restarts_total` (counter)
  * `request_latency_seconds` (histogram)
    `/healthz`, `/readyz` semantics: **liveness** vs **readiness/backpressure**; writes fail first under degradation.

---

## 2) Design Principles (SHOULD)

* \[P-1] **Crash fast, restart clean** — avoid in-loop bespoke retries; rely on supervision/backoff.
* \[P-2] **Message passing > shared mutability** — prefer bus + bounded mailboxes.
* \[P-3] **Deterministic observability** — readiness reflects real capacity; fail closed on writes first.
* \[P-4] **Config as snapshot** — `Arc<Config>` with atomic hot-swaps; **rollback** on invalid updates.
* \[P-5] **Kernel minimalism** — lifecycle/health/config/bus only; **no overlay/DHT/storage/ledger/transport loops**.
* \[P-6] **DX-first** — hermetic tests (no network), stable public API, strict docs/CI gating.
* \[P-7] **PQ neutrality** — kernel passes through PQ/crypto posture (e.g., `pq_hybrid=true`) and surfaces it in health; crypto specifics live outside.

---

## 3) Implementation Patterns (HOW)

* \[C-1] **Bounded broadcast bus**

  ```rust
  let (tx, _rx0) = tokio::sync::broadcast::channel(4096);
  // one Receiver per task; overflow is counted, not blocking
  ```

* \[C-2] **Supervisor jittered backoff**

  ```rust
  use rand::Rng;
  use tokio::time::{sleep, Duration};

  async fn backoff(attempt: u32) {
      let base = Duration::from_millis(100);
      let cap  = Duration::from_secs(30);
      let exp  = base.saturating_mul(2u32.saturating_pow(attempt));
      let jit  = Duration::from_millis(rand::thread_rng().gen_range(0..100));
      sleep(std::cmp::min(exp, cap) + jit).await;
  }
  ```

* \[C-3] **Atomic config hot-swap**

  ```rust
  use arc_swap::ArcSwap;
  use std::sync::Arc;

  pub struct Cfg(ArcSwap<Config>);
  impl Cfg {
      pub fn get(&self) -> Arc<Config> { self.0.load_full() }
      pub fn set(&self, new_cfg: Config) { self.0.store(Arc::new(new_cfg)); }
  }
  ```

* \[C-4] **Readiness gating (Axum sketch)**

  ```rust
  use axum::{http::StatusCode, response::IntoResponse};
  pub async fn readyz(state: AppState) -> impl IntoResponse {
      if state.health.all_ready() {
          (StatusCode::OK, "ready").into_response()
      } else {
          (StatusCode::SERVICE_UNAVAILABLE, [("Retry-After","1")], "degraded").into_response()
      }
  }
  ```

* \[C-5] **Owned bytes end-to-end (ETag)**

  ```rust
  use bytes::Bytes;
  let body: Bytes = Bytes::from(data);
  let etag = format!("\"b3:{}\"", blake3::hash(&body).to_hex());
  ```

---

## 4) Acceptance Gates (PROOF)

**Each invariant maps to at least one gate. “Green CI == green spec.”**

* \[G-1] **Unit/Integration**

  * Bus fan-out under saturation: no deadlock; overflow increments `bus_overflow_dropped_total`. (I-5)
  * Supervisor restarts emit `ServiceCrashed{reason}`; restart intensity cap enforced. (I-3)
  * Config invalidation triggers rollback; emits `ConfigUpdated{version}` only on success. (I-2/I-4)

* \[G-2] **Property tests**

  * “Kernel never blocks”: prove bus overflow does **not** block supervisory loops. (I-5)
  * Health/readiness: once `ready=true`, regressions require explicit cause (state machine property).

* \[G-3] **Fuzzing (structure-aware)**

  * OAP/1 envelope fuzz (HELLO/START/DATA/END/ACK/ERROR) — **kernel does not enforce**, but parsers/telemetry paths must remain robust. (I-7)

* \[G-4] **Loom (interleavings)**

  * Readiness DAG (config→bus→metrics): no deadlocks; `all_ready()` only when prerequisites satisfied. (I-4/I-10)

* \[G-5] **Chaos**

  * Inject panics in supervised tasks: `/readyz` flips to degraded; `service_restarts_total` increases; quarantine after cap. (I-3/I-10)

* \[G-6] **CI Teeth (deny drift)**

  * Clippy wall (`await_holding_lock`, `unwrap_used`, `expect_used`) (I-4).
  * **TSan mandatory** (critical crate).
  * **Tokio flavors**: tests pass for `multi_thread` and `current_thread`.
  * **Miri** on logic-heavy tests.
  * **Public API guard** (`cargo public-api` / semver checks) (I-1/I-2).

* \[G-7] **Coverage & Soak**

  * Coverage ≥ **85%**; 1-hour soak: restart storms & bus overflow produce expected metrics patterns. (I-3/I-5/I-10)

* \[G-8] **Amnesia validation**

  * Matrix run with `amnesia=on/off`: with **on**, assert **no disk artifacts**, logs ephemeral, keys time-boxed; metrics include `amnesia="on"`. (I-9)

---

## 5) Anti-Scope (Forbidden)

* ❌ **No global mutable state** (`static mut`, ad-hoc singletons).
* ❌ **No protocol/payload policing** in kernel (OAP enforcement lives in services/libs; kernel only observes).
* ❌ **No overlay/DHT/storage/ledger/transport loops** inside the kernel.
* ❌ **No unbounded queues** of any kind.
* ❌ **No ambient auth** (capability systems live in `ron-auth`/`svc-passport`).
* ❌ **No SHA-2/MD5** for addressing; **BLAKE3 only** for content address digests.
* ❌ **No direct `rustls::ServerConfig`**; must use `tokio_rustls::rustls::ServerConfig`.

---

## 6) References

* **Microkernel Blueprint (FINAL)** — kernel scope, API freeze, amnesia flag, metrics, supervision.
* **Full Project Blueprint (v2.0)** — 33-crate canon, OAP constants, content addressing, overlay/DHT split, transport merge.
* **Concurrency & Aliasing Blueprint (v1.3)** — no locks across `.await`, bounded queues, single-writer discipline, readiness rules.
* **Hardening Blueprint (v2.0)** — OAP limits, decompression caps, degraded readiness, amnesia mode spec.
* **Six Concerns (2025)** — SEC/RES/PERF/GOV/DX mappings (kernel primary: SEC/RES/PERF/GOV).

---

### Definition of Done (for this IDB)

* Invariants are **executable** (each has at least one gate).
* OAP enforcement is **explicitly delegated** to services; kernel remains **crypto/protocol-neutral** while exporting **observability**.
* Public API re-exports & `KernelEvent` shape are **unchanged**.
* Amnesia is end-to-end verifiable via **metrics + disk-artifact checks**.
* CI blocks **any** drift on the above.

```

