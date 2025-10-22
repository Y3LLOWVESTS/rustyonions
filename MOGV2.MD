
# MOGV2 — RON-CORE Efficiency + Fortress Plan

status: active
goal: **≥50% fewer servers** at the same SLOs, with **no security regressions**
scope: all RON-CORE crates (API stable; feature-gated internals only)

---

## 0) Program KPIs (what we measure, not vibes)

**Hard acceptance gates (per crate / PR):**

* **Perf:** ≥15% throughput **or** ≥20% p95 improvement with ≤5% resource change.
* **Tails:** p99 must not regress; p999 ≤ +5% unless justified (documented).
* **Security:** CI fortress gates green (deny/audit/geiger/miri/loom), rate limits + caps enforced.
* **Observability:** `/metrics`, `/healthz`, `/readyz` verified; new counters documented.

**Stack-level target:** blended **1.7–2.5×** throughput and **0.5–0.7×** p99 across busy services ⇒ **~50% fewer servers**.

---

## 1) Global feature flags (OFF by default; API stable)

Add consistently across crates (namespacing per crate):

```toml
[features]
# perf
mog_edge_notify = []        # A1/A5: edge-triggered wakes + disciplined drain
mog_batch = []              # A2: batch submit/drain, single fence/notify
mog_autotune_cap = []       # A3: sane default caps, warn on cache-hostile sizes
metrics_buf = []            # A4: per-thread hot counters + periodic flush
# io/codec
mog_compact_codec = []      # CBOR/MsgPack or simd-json borrowed DOM
mog_writev = []             # vectored IO / coalesced flush
mog_pools = []              # buffer/encoder pools
mog_interest = []           # B2: interest masks / topic filters (authz-aware)
mog_local_dispatch = []     # B3: one global sub per role, local re-fan-out
```

> All new paths are **pure opt-in** and must preserve correctness; public API unchanged.

---

## 2) Telemetry (golden counters)

Add uniformly; namespaced per crate where relevant:

* **Bus/Notify:** `bus_notify_sends_total`, `bus_notify_suppressed_total`, `bus_receiver_lag_total`, `bus_overflow_dropped_total`, `bus_batch_publish_total`, `bus_batch_len_histogram`, `bus_sub_pending{sub}` (optional).
* **IO/Codec:** `<crate>_bytes_out_total`, `<crate>_writev_total`, `<crate>_flush_total`, `<crate>_codec_bytes_saved_total`, `<crate>_serde_fallback_total`.
* **Coalesce/Debounce:** `<crate>_coalesced_total`, `<crate>_debounced_total`.
* **Security:** `<crate>_authz_denied_total`, `<crate>_rate_limited_total`, `<crate>_frame_too_large_total`.
* **Metrics TLS buffer:** `<crate>_metrics_tls_flush_total`.

---

## 3) CI fortress (repo-wide)

```
cargo fmt --check
cargo clippy -D warnings
cargo test
cargo deny check
cargo audit
cargo geiger
cargo +nightly miri test
cargo +nightly test --features loom
```

> Blocks merges if any fail. Public API freeze gate applies to stable surfaces.

---

## 4) Rollout playbook (every crate)

1. **Freeze baseline**

   ```
   cargo bench -p <crate> -- --save-baseline <crate>-YYYY-MM-DD
   ```
2. **Enable features locally** and bench:

   ```
   cargo bench -p <crate> --features mog_edge_notify,mog_batch,metrics_buf -- --baseline <crate>-YYYY-MM-DD
   ```
3. **Canary** enablement via config knob (runtime), watch golden counters + SLOs.
4. **Paste deltas** to NOTES.md → “MOGV2 Results — <crate> (date)”.

---

## 5) Crate-by-crate plan (moves, gates, realistic gains)

### 5.1 ron-http (front door)

**Moves:** short middleware chain; `Bytes` hot path; h2 for M:N, tuned keep-alive; pooled header maps; rustls 1.3 config reuse; optional `mog_compact_codec` for JSON→CBOR or simd-json borrowed.

**Gates:** ≥20% req/s **or** p95 −20%; `*_bytes_out_total` flat/↓ with codec on.

**Expect:** +20–40% throughput, −20–40% p99; CPU/req −15–35%.

---

### 5.2 ron-metrics (exporter)

**Moves:** `metrics_buf`; batch gather+encode+single flush; optional `mog_writev`; backpressure.

**Gates:** exporter throughput +15–35% **or** p95 −20%; `*_flush_total` ↓, `*_writev_total` ↑.

**Expect:** −5–15% CPU under load; tail variance ↓ 10–20%.

---

### 5.3 ron-config (watcher)

**Moves:** content-hash debounce; epoch’d apply; `mog_interest` to wake only listeners.

**Gates:** `*_debounced_total` shows ≥60% fewer events on noisy edits; no increase in stale applies.

**Expect:** −50–80% wakeups during change storms; listener p95 −10–20%.

---

### 5.4 ron-supervisor (lifecycle/readiness)

**Moves:** branchless backoff; short-TTL (1–2ms) cached readiness under probe storms; coalesced state transitions.

**Gates:** background CPU −5–10%; flap storms show ≥50% fewer notifications.

**Expect:** smoother restarts; fewer tail spikes.

---

### 5.5 ron-interop (internal messaging)

**Moves:** `mog_batch` (publish_many), `mog_edge_notify`, `mog_interest` (topic filter), `mog_compact_codec` (internal lanes CBOR/MsgPack; JSON at edges), optional `mog_local_dispatch`.

**Gates:** bytes on internal lanes −20–35%; p95 −10–25%; suppressed wakes ≥30% of total.

**Expect:** +20–50% throughput on chatty paths; CPU/byte −20–40%.

---

### 5.6 app-integration (role gateways)

**Moves:** local dispatch fan-out (B3); coalesce outbound within 0.5–2ms; per-role caps/quotas (also security).

**Gates:** upstream calls −10–30%; bus wake reductions ≥30%; p95 −10–20%.

**Expect:** −20–60% global wakes; better cache locality.

---

### 5.7 telemetry-bridge

**Moves:** adaptive export cadence by depth; columnar/SoA staging; one encode per tick; `mog_writev`.

**Gates:** encode time −15–30%; `*_bytes_out_total` flat/↓; p95 spikes −10–25%.

**Expect:** steady NIC usage; less memory bandwidth.

---

## 6) Shared MOG kit (to avoid copy/paste)

Create `ron-mog-kit` (internal) exposing:

* `edge::EdgeNotify` + `EdgeMetrics` hook (A1/A5).
* Tiny TLS metrics buffer trait (optionally).
* Drain loop helper.

Consumers depend with `features = ["edge_notify"]`, keeping semantics identical across crates.

---

## 7) Security: “Diamond Fortress” overlays (low overhead)

**Always-on (near-zero runtime):**

* `#![forbid(unsafe_code)]` (crate roots), `cargo geiger` in CI.
* SBOM + audit: `cargo auditable`, `cargo audit`, `cargo deny`.
* Race safety where it matters: Loom tests for notify/drain, Miri, fuzz on parsers.

**Process hardening (ops):**

* Non-root users; read-only rootfs; systemd: `NoNewPrivileges=yes`, `ProtectSystem=strict`, `MemoryDenyWriteExecute=yes`, `SystemCallFilter=@system-service @network-io`, `RestrictAddressFamilies=AF_INET AF_INET6`.
* Containers: drop caps, seccomp, AppArmor/SELinux.

**Network & identity:**

* rustls TLS 1.3 only, strong ciphers, session cache, mTLS where needed (SPIFFE optional).
* Rate limits + retry budgets + circuit breakers.

**Data & message safety (fast-path friendly):**

* Frame caps: `{max_frame_bytes, max_batch_len, max_inflight}` enforced at decode+publish; counters wired.
* Authz via `mog_interest` allow-lists per role/topic; deny metrics.

**Observability with restraint:**

* Batched metrics; sampled tracing; PII scrubbing at ingress; budgeted label cardinality.

> Expected overhead of the fortress layer: ~4–8% CPU at edges, ~1–2% elsewhere—fully covered by MOG gains.

---

## 8) Efficiency accounting (Amdahl tracker)

Track per service:

* `p` = % CPU in paths covered by MOG features (kernel, interop, exporter, etc.).
* `s` = measured speedup for those paths under representative load.

Compute overall speedup:

```
S = 1 / ((1 - p) + p / s)
```

**Program goal:** demonstrate blended **S ≈ 2×** on the busiest services → **~50% fewer servers**.

---

## 9) Commands (copy-ready)

**Freeze baseline (per crate):**

```
cargo bench -p <crate> -- --save-baseline <crate>-YYYY-MM-DD
```

**Compare with features:**

```
cargo bench -p <crate> --features mog_edge_notify,mog_batch,mog_autotune_cap,metrics_buf,mog_compact_codec,mog_writev,mog_pools,mog_interest,mog_local_dispatch -- --baseline <crate>-YYYY-MM-DD
```

**Smoke + demo (standard):**

```
cargo build -p <crate>
cargo test  -p <crate>
cargo run   -p <crate> --example <demo_if_any>
curl -s http://127.0.0.1:9600/metrics | head -n 30
curl -s http://127.0.0.1:9600/healthz
curl -s http://127.0.0.1:9600/readyz
```

---

## 10) Documentation & governance

* **PR template** includes: gates checklist, counters added, benches + baseline diff, security notes (caps, rate limits), README/ALL_DOCS updates.
* **ALL_DOCS.md**: add a “MOGV2 switches” section per crate (flags, defaults, knobs, counters).
* **NOTES.md**: append “MOGV2 Results — <crate> (date)” with top wins/losses.

---

## 11) Realistic outcome (why we’ll hit 50%)

* Kernel + interop + exporter usually dominate **35–60%** of CPU in fan-out/IO-heavy stacks.
* They individually see **1.5–2.5×** speedups (edge-notify + batching + compact codecs + writev).
* Blended with the rest (which also get 1.1–1.5×), the macro services land ~**2× overall** → **~50% fewer servers** at same SLOs.
* Fortress controls stay on; overhead is contained and already amortized by the gains.

---

