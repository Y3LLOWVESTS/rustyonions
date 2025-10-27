### BEGIN NOTE - OCTOBER 26 2025 - 22:05 CST
---

# What to carry over from `svc-overlay` (and friends)

## 1) Transport & framing

* **Overlay is the pipe.** `svc-dht` should talk over the overlay TCP session; don’t bind its own sockets.
* **Use OAP frames.** Send and receive via `FrameKind::Data` only (no new kind needed for beta).
* **Serialization.** Put a typed message in `Frame.payload` using `serde` (`serde_cbor` or `serde_json`). Keep it self-describing (e.g., an enum `ron_proto::DhtMsg`) so future multiplexing stays easy.

## 2) Handshake & caps

* **No new handshake.** Reuse overlay’s `version=1, Caps(GOSSIP_V1)`. DHT doesn’t add capabilities for beta; it rides the data channel.

## 3) Backpressure & queues

* **Best-effort delivery.** Overlay writer drops when the per-peer TX queue is full (and bumps `overlay_peer_dropped_total`). `svc-dht` must be tolerant: retries, timeouts, idempotency.
* **Don’t block.** Use bounded channels on your side too; never hold locks across `.await`.

## 4) Readiness & admin HTTP

* **Endpoints parity.** Expose the same three endpoints: `/healthz`, `/readyz`, `/metrics`.
* **Readiness gates (DHT):**

  * `listeners_bound` equivalent for your internal services (e.g., RPC or control plane if you add one).
  * `routing_ready` when: at least N bootstrap peers connected and routing table has ≥K entries.
  * `queues_ok` based on your own TX queue depth (mirrors overlay pattern).
* **Return 503 until all gates are true.** Keep JSON body with field-level truth for debugging.

## 5) Metrics conventions

* **Namespace** your metrics as `dht_*`.
* **Counters (at minimum):**
  `dht_put_total`, `dht_get_total`, `dht_found_total`, `dht_timeout_total`, `dht_error_total{kind=...}`.
* **Histograms:**
  `dht_get_latency_seconds`, `dht_put_latency_seconds`.
* **Gauges:**
  `dht_routing_table_size`, `dht_active_sessions`, `dht_pending_queries`.
* **Build info:** mirror overlay’s `*_build_info{version,git}=1`.
* **Export path.** Reuse the overlay approach: `once_cell::Lazy` + `prometheus` types, register on first use, `TextEncoder` for exposition.

## 6) Config & tunables (env for now)

* **Prefix:** `RON_DHT_*` (mirrors `RON_OVERLAY_*`).
* **Recommended envs (beta):**

  * `RON_DHT_BOOTSTRAP` (CSV of overlay addresses or logical peer IDs you resolve to overlay endpoints).
  * `RON_DHT_K` (routing table bucket size; default 20).
  * `RON_DHT_ALPHA` (parallelism for lookups; default 3).
  * `RON_DHT_TIMEOUT_MS` (per-RPC timeout; default 1500–3000ms).
  * `RON_DHT_REPLICA` (replication factor; default 3).
* **Clamping.** Apply sane clamps in a small `tuning.rs` (same pattern we used).

## 7) Logging/tracing

* **Consistent fields.** Include `peer`, `key` (hex or base32), `op` (`put|get|pong|…`), and `ttl|hops` when applicable.
* **Levels.** `info!` for state transitions, `warn!` for expected failures/timeouts, `error!` for bugs/IO failures.
* **No panics.** Return errors; let supervisors restart tasks if needed (see `ron-kernel` patterns).

## 8) Code style, lints, docs

* **Keep workspace lints** (clippy pedantic, etc.). Add `Default`, `#[must_use]`, `# Panics`/`# Errors` docs where flagged.
* **Comment style.** Follow `CODECOMMENTS.MD`: top-of-file RO blocks (`RO:WHAT`, `RO:WHY`, etc.) and function-level brief docs where non-obvious.
* **Feature gates.** Mirror overlay’s pattern: keep placeholder features (`pq`, `quic`) but avoid dead cfg trees. Gate examples like we did for `pq_embed`.

## 9) Scripts & CI

* **Roundtrip harness.** Provide `crates/svc-dht/scripts/roundtrip_dht.sh` that:

  1. starts `svc-overlay` (or assumes it’s running)
  2. launches 2× DHT nodes (A,B) pointed at overlay
  3. runs a `put` on A, `get` on B, asserts equality
* **Soak harness.** A lightweight soak similar to overlay’s `soak_overlay.sh` (optional for beta, handy for smoke).
* **Minimal CI step.** `fmt + clippy + roundtrip_dht.sh`.

## 10) Dependencies to reuse

* `ron-proto`: add `DhtMsg` here to keep message types central/shared.
* `ron-metrics`: use its registry and helper patterns if you prefer; overlay showed direct `prometheus` is fine too—choose one and be consistent.
* `ron-kernel`: for bus/supervisor patterns if you spawn internal services; not required for initial client+server loop.

---

## Paste into `NOTES.MD` (carryover for svc-dht)

```
## svc-dht — carryover contracts from svc-overlay (beta)

- Transport: use svc-overlay sessions; send OAP `FrameKind::Data` with serialized `ron_proto::DhtMsg` (serde_cbor/json).
- Handshake: reuse overlay v1 (`Caps(GOSSIP_V1)`); no new caps for beta.
- Backpressure: treat overlay as best-effort. Writers drop on full queues; handle retries/timeouts idempotently.
- Admin HTTP: expose `/healthz`, `/readyz` (503 until routing+queues OK), `/metrics` (Prometheus).
- Metrics (prefix `dht_*`):
  - counters: `dht_put_total`, `dht_get_total`, `dht_found_total`, `dht_timeout_total`, `dht_error_total{kind}`
  - histograms: `dht_get_latency_seconds`, `dht_put_latency_seconds`
  - gauges: `dht_routing_table_size`, `dht_active_sessions`, `dht_pending_queries`
  - `dht_build_info{version,git}=1`
- Tunables (env, clamp values; move to Config later):
  - `RON_DHT_BOOTSTRAP`, `RON_DHT_K` (default 20), `RON_DHT_ALPHA` (default 3),
    `RON_DHT_TIMEOUT_MS` (default 2000), `RON_DHT_REPLICA` (default 3)
- Logging: include `peer`, `op`, `key`, `hops/ttl`; `info` for transitions, `warn` timeouts, `error` IO/bugs. No panics.
- Lints/docs: keep clippy pedantic hygiene; add `Default`, `#[must_use]`, `# Panics/# Errors` where flagged; top-of-file RO blocks.
- Features: keep placeholders but avoid unused cfg branches; gate examples that aren’t ready (`[[example]] required-features`).
- Scripts/CI:
  - `roundtrip_dht.sh`: start overlay, run 2 nodes (A,B), `put` on A, `get` on B, assert value.
  - optional soak for smoke.
  - CI: `fmt + clippy + roundtrip_dht.sh`.

Rationale: keeps the beta path narrow; no new network surface; leverages overlay’s ready/metrics/transport contracts; lets us iterate on DHT logic without transport churn.
```

---

### END NOTE - OCTOBER 26 2025 - 22:05 CST