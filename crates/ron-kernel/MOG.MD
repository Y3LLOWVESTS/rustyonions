Here’s an updated, paste-ready **MOG.MD** incorporating the A3/A4 clarifications, observability metric names (A-track), a tighter bench matrix (with feature combos), and DoD additions—while keeping your original structure and language.

```markdown
# MOG.MD — ron-kernel Performance Enhancement Plan
status: draft
last-updated: 2025-10-20
scope: ron-kernel (microkernel core) — internal perf toggles only (public API frozen)

---

## 0) Baseline (reference for all gates)

Use the tuned results from 2025-10-19 as the **named baseline** so comparisons are apples-to-apples:

- 0 subs: ~49 ns/publish  
- 1 sub (cap≈64): ~55 ns/publish  
- 4 subs (cap≈64): ~61 ns/publish  
- 16 subs (cap≈128): ~170 ns/publish  

Freeze this baseline locally:

```

cargo bench -p ron-kernel -- --save-baseline core-2025-10-19

```

Compare any future runs to it:

```

cargo bench -p ron-kernel -- --baseline core-2025-10-19

````

**Important:** All perf PRs must paste the top deltas (wins & losses) into NOTES.md under a new “MOG Results” subheading.

---

## 1) Cargo feature flags (ALL OFF by default)

Add to `crates/ron-kernel/Cargo.toml`:

```toml
[features]
# Phase A
bus_edge_notify = []   # edge-triggered, coalesced wakeups per subscriber
bus_batch = []         # publish_many(&[T]) single-fence/notify
bus_autotune_cap = []  # cap defaults chosen by N subscribers, warn on oversized caps
metrics_buf = []       # per-thread metrics buffering with periodic flush
# Phase B
bus_soa = []           # SoA ring + per-slot ready bitmask (Arc<T> payload to keep 100% safe)
bus_interest = []      # variant/topic bitmask filtering per subscriber
bus_coalesce = []      # coalesce wakeups by K/Δt thresholds (optional latency trade)
local_dispatch = []    # example helpers/patterns; not a kernel core change
````

**Contract:** These flags **must not** alter public API or external behavior when **OFF**. Perf features that are ON must preserve correctness and observability.

---

## 2) Phase A — High ROI, Low Risk

### A1) Edge-triggered, coalesced notify — `bus_edge_notify`

**What:** Per-subscriber `pending: AtomicBool`. Publisher only notifies on `pending` 0→1. Subscriber drains → `pending=false` → race check → await.
**Why:** Eliminates futile wakes; multiplies with all other gains.

**Tests:**

* Unit/property: no double-delivery; monotonic tails.
* Loom: lost-wake (publisher vs 2 subs), drain-after-clear race.

**Benches:** publish matrix subs {1,4,16}, cap {64,128}.
**Telemetry:** `bus_notify_sends_total`, `bus_notify_suppressed_total`, `bus_sub_pending` gauge per subscriber.
**Gate:** ≥10% win at subs=4 cap=64; ≥20% at subs≥8.
**Gain est.:** +10–25%.

---

### A2) Batch publish API — `bus_batch`

**What:** `publish_many(&[T])`: reserve N seqs once; write N slots; **one** Release fence + **one** notify.
**Why:** Coalesces atomics/fences for bursts.

**Tests:** wraparound correctness; drop accounting over overwritten range; partial batches.
**Benches:** burst variants (1k/5k/10k per iter) vs single publishes.
**Telemetry:** `bus_batch_publish_total`, `bus_batch_len_histogram`.
**Gate:** ≥15% faster on burst benches (subs ∈ {1,4}, cap=64).
**Gain est.:** +10–35% (workload-dependent).

---

### A3) Capacity auto-tune + guard rails — `bus_autotune_cap`

**What:** Select default cap by expected N: `N≤4 → 64`, `N≤16 → 128`; warn (and emit a metric) if cap > 256 (cache-hostile). No runtime resizing.
**Why:** Prevent misconfig regressions; matrices show large caps degrade perf.

**Tests:** respects overrides; emits warning metric/counter for >256; no runtime resize.
**Benches:** matrix sanity (no regressions vs hand-picked sweet spots).
**Gate:** No regressions; guardrail warnings present.
**Gain est.:** +5–20% (by avoiding bad caps).

**API (behind feature):**

```rust
#[cfg(feature = "bus_autotune_cap")]
pub fn autotune_capacity(expected_subs: usize, override_cap: Option<usize>) -> usize { /* impl */ }
```

---

### A4) Per-thread metrics buffering — `metrics_buf`

**What:** Replace per-publish atomic incs with TLS counters flushed every **N** increments or **Δt** (and on drop).
**Why:** Removes atomics from the hot path.

**Knobs:**

* `RON_TLS_FLUSH_THRESHOLD` (default **64**, clamp ≥1)
* Flush interval ≈ 200 ms (internal constant)
* Best-effort flush on drop

**Tests:** no counter loss on drop; time-boxed flush; fuzz flush ordering.
**Benches:** steady 4 subs / cap=64; verify reduction in atomics.
**Telemetry:** `bus_metrics_tls_flush_total` (+ existing metrics) to ensure visibility.
**Gate:** ≥8% faster at subs=4 cap=64.
**Gain est.:** +3–15%.

**Threshold sweep plan:** run with `RON_TLS_FLUSH_THRESHOLD={32,64,128,256}`; record CPU(user%) and publish:flush ratio (expect ≈N:1).

---

### A5) Drain-loop discipline (pairs with A1)

**What:** Drain with `try_recv/now_or_never` → `pending=false` → race check → await; avoid ping-pong wakeups.
**Tests:** race after clear; starvation checks; wake suppression visible in metrics.
**Gate:** ≥5% at subs≥4.
**Gain est.:** +5–15%.

---

## 3) Phase B — Medium Impact (after Phase A)

### B1) SoA ring + per-slot ready bitmask — `bus_soa`

**What:** Safe SoA backend: `slots[cap]` with `{seq: AtomicU64, ready: AtomicU64, msg: Arc<T>}`; `sub_tail[]` is `CachePadded<AtomicU64>`.
**Why:** Flattens N-subscriber bookkeeping; stride-1 cache-friendly.

**Tests:**

* Unit/property: overwrite semantics, single-drop on last-bit clear.
* Loom: last-bit clear; ABA guarded by 64-bit `seq`.

**Benches:** side-by-side group `bus_publish_matrix_soa` vs current backend.
**Telemetry:** `bus_ready_mask_width` histogram (avg width of ready masks).
**Gate:** ≥20% win at subs=4 cap=64 **and** ≥30% at subs=16 cap=128.
**Gain est.:** +15–30% (4 subs), +20–45% (16 subs).

**Safety posture:** Start with `Arc<T>` payloads only. Consider inline `T` **only** if Loom + Miri + fuzz pass and gains justify it.

---

### B2) Topic/interest filtering — `bus_interest`

**What:** Subscribers register a static bitmask over `KernelEvent` variants; publisher ANDs mask into per-slot `ready`.
**Why:** Do not wake uninterested subs.

**Tests:** mask correctness; dynamic change only via epoch’d updater to avoid torn reads.
**Benches:** mixed-topic stream (e.g., 30% relevant).
**Telemetry:** `bus_interest_skips_total`.
**Gate:** ≥15% gain on mixed workloads.
**Gain est.:** +10–40% (selectivity-dependent).

---

### B3) Role-level local dispatch (pattern) — `local_dispatch`

**What:** Keep global subscribers few (one per role); each role re-fans-out locally (unicast channels or SPSC).
**Why:** Reduces global N — strongest macro lever, without kernel changes.

**Artifacts:** Example helper module + docs.
**Gate:** Typical macronode configs reduce global N by ≥50%.
**Gain est.:** +20–60% when N drops (e.g., 16→4).

---

### B4) Lag threshold notify (K / Δt) — `bus_coalesce`

**What:** Optional thresholds to coalesce wakes on noisy streams; bound tail latency.
**Tests:** latency histogram guard (p95 regress ≤10%).
**Benches:** chatty publisher; compare throughput vs p95.
**Gate:** Throughput ↑ with ≤10% p95 hit.
**Gain est.:** +8–20%.

---

### B5) Compact event layout (schema hygiene)

**What:** Keep hot events tiny (`repr(u8)` tag + compact payload); use `Arc<T>` for heavy/rare data.
**Tests:** schema compatibility; migration notes (no public API break).
**Gate:** ≥5% improvement on real-work benches.
**Gain est.:** +5–15% (payload dependent).

---

## 4) Phase C — Micro-polish

### C1) False-sharing eradication

**What:** `CachePadded` per-subscriber tails/counters; align hot arrays to 64B.
**Tests:** Miri layout sanity; perf smoke.
**Gain est.:** +3–10% (more at high N).

### C2) Memory-ordering tightening

**What:** Keep only `Release` on final `seq.store()` and `Acquire` on consumer `seq.load()`; mask ops `Relaxed`.
**Tests:** Loom litmus (message visibility, no reordering regressions).
**Gain est.:** +2–8%.

### C3) CPU pinning / NUMA guidance (ops)

**What:** Doc & optional helpers to co-locate publisher + hot subs under same L3; avoid cross-NUMA.
**Gain est.:** +5–20% throughput stability on busy hosts.

---

## 5) Telemetry (must add/keep so regressions are obvious)

**A-track (Phase A):**

* `bus_notify_sends_total`
* `bus_notify_suppressed_total`
* `bus_batch_publish_total`
* `bus_batch_len_histogram`
* `bus_metrics_tls_flush_total`
* `bus_sub_pending` (gauge per subscriber)

**Always-on / existing bus metrics:**

* `bus_published_total`
* `bus_no_receivers_total`
* `bus_receiver_lag_total` (or per-subscriber lag gauge)
* `bus_dropped_total`
* `bus_overflow_dropped_total`

**Phase B additions (when enabled):**

* `bus_interest_skips_total`
* `bus_ready_mask_width`

**Rule:** Any feature that changes wake/throughput must expose counters that clearly show its effect.

---

## 6) Bench coverage (must-have)

**Authoritative matrix (keep updated):** subs {0,1,4,16} × cap {32,64,128,256}.

**Add (and keep comparing to baseline):**

* `publish_many` burst variants (A2).
* Wakeup counters per run to validate A1/A5 effects.
* `bus_publish_matrix_soa` side-by-side group (B1).
* Mixed-topic stream for interest masks (B2).
* Latency histograms for coalescing (B4).

**Feature-combo runs (compare vs baseline `core-2025-10-19`):**

* (none)
* `metrics_buf`
* `metrics_buf,bus_edge_notify`
* `metrics_buf,bus_edge_notify,bus_batch`

**Commands**

```
cargo bench -p ron-kernel --bench publish_edge_matrix
cargo bench -p ron-kernel --features bus_batch --bench bus_batch
cargo bench -p ron-kernel -- --baseline core-2025-10-19
```

---

## 7) Risks & mitigations

* **Lost wake (A1):** Single atomic `pending` bit + race check after clear; covered by Loom tests.
* **Drop accounting with batches (A2):** Property test balances `overwrites - delivered`.
* **Dynamic sub masks (B2):** Make masks static per subscriber or change only via an epoch’d updater.
* **SoA sequence wrap (B1):** 64-bit `seq`; ABA implausible; Loom asserts.
* **Zero-UB posture:** Start SoA with `Arc<T>` payload; consider inline `T` only with Loom+Miri+fuzz green and proven gains.

---

## 8) CI & governance gates (must be in place before perf changes)

* **Public API freeze gate:** run `cargo public-api` (and optionally `cargo semver-checks`) on `-p ron-kernel`. Fail on surface changes unless accompanied by a changelog entry and approval.
* **Deny-wall:** `cargo deny` (licenses/bans/advisories/sources) must pass.
* **Lint/tests:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` green.
* **Observability contract:** CI should verify `/metrics`, `/healthz`, `/readyz` endpoints in the example bin.
* **Docs sync:** README/ALL_DOCS describe config update semantics and list golden metrics.

**PR template checklist (paste into each PR):**

* [ ] Public API unchanged (CI public-api pass)
* [ ] Feature flag default OFF; perf change behind a flag
* [ ] New metrics added & documented
* [ ] New tests (unit/property/Loom as applicable)
* [ ] Benches updated; baseline diff pasted into NOTES.md
* [ ] README/ALL_DOCS/TODO synced (if needed)

---

## 9) Execution order (so gains compound)

1. **PR-0 (prep):** Add all cargo features (OFF), wire CI gates, sync docs (readiness & config update explainer, golden metrics).
2. **PR-1 — A3:** Cap auto-tune + guardrails (no regressions; warnings for >256).
3. **PR-2 — A1 + A5:** Edge-notify + disciplined drain; counters + Loom; meet gates at subs=4/64 and ≥8 subs.
4. **PR-3 — A2:** `publish_many` + burst benches; meet ≥15% gate.
5. **PR-4 — A4:** TLS metrics buffering; show ≥8% steady-state gain (threshold sweep recorded).
6. **PR-5 — B1:** SoA backend toggle; side-by-side benches; ≥20% (4/64) and ≥30% (16/128).
7. **PR-6 — B2:** Interest filtering; mixed-topic ≥15% gate.
8. **PR-7 — B3:** Local dispatch helpers + docs (no kernel change).
9. **PR-8 — B4/B5 + C-polish:** Coalescing; compact event layout; padding/alignment; memory ordering; ops pinning guide.

---

## 10) Honest all-in projected gains (vs tuned baseline)

| Scenario (draining) | Baseline |        Target |  Total Gain | Throughput (M/s) |
| ------------------- | -------: | ------------: | ----------: | ---------------: |
| 0 subs              |   ~49 ns |  **46–47 ns** |   **~3–6%** |    **21.3–21.7** |
| 1 sub (cap≈64)      |   ~55 ns |  **50–47 ns** |  **~9–15%** |    **20.0–21.3** |
| 4 subs (cap≈64)     |   ~61 ns |  **51–49 ns** | **~16–20%** |    **19.6–20.4** |
| 16 subs (cap≈128)   |  ~170 ns | **120–90 ns** | **~29–47%** |     **8.3–11.1** |

*Against untuned, large caps, apparent gains can look **~60–73%**, but the table above is tuned-vs-tuned.*

---

## 11) Commands (copy-ready)

**Freeze baseline:**

```
cargo bench -p ron-kernel -- --save-baseline core-2025-10-19
```

**Run compare after each PR:**

```
cargo bench -p ron-kernel -- --baseline core-2025-10-19
```

**Smoke + demo:**

```
cargo build -p ron-kernel
cargo test -p ron-kernel
cargo run -p ron-kernel --example kernel_demo
curl -s http://127.0.0.1:9600/metrics | head -n 30
curl -s http://127.0.0.1:9600/healthz
curl -s http://127.0.0.1:9600/readyz
```

---

## 12) Definition of Done (for MOG track)

* All Phase-A features landed behind flags (OFF by default), with tests, telemetry, and benches, and documented gates **met**.
* A3 `bus_autotune_cap` landed; warns on >256; examples may use it; **no** p50/p95 regressions vs fixed caps.
* A4 `metrics_buf` documented with threshold knob + flush pump; threshold sweep logged in NOTES.MD; observed publish:flush ratio ≈ N:1.
* SoA (B1) shipped behind a flag with side-by-side benches and Loom/Miri/fuzz clean.
* CI gates active: public-api freeze, deny-wall, clippy/format/tests, observability checks.
* README/ALL_DOCS/TODO/NOTES synced (baseline deltas pasted).
* No public API changes unless explicitly approved with a versioned changelog entry.

---

```

