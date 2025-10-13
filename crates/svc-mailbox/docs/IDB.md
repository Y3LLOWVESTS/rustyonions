
```
---
title: svc-mailbox — Store-and-Forward Messaging (IDB)
version: 1.0.1
status: reviewed
last-updated: 2025-10-12
audience: contributors, ops, auditors
pillar: 11 (Messaging & Extensions)
crate-type: service
owners: [RustyOnions Core]
msrv: 1.80.0
---
```

# svc-mailbox — Invariant-Driven Blueprint

## 1) Invariants (MUST)

* **[I-1] At-least-once delivery.** Every message is delivered ≥1× or lands in DLQ; there are no “fire-and-forget” paths.
* **[I-2] Idempotency keys.** Duplicate `SEND` with the same `(topic, idem_key, payload_hash)` yields one effective delivery; subsequent calls return the original `msg_id` with `duplicate=true`.
* **[I-3] Visibility timeout discipline.** `RECV` moves a message to `inflight(deadline=now+visibility_ms)`; on deadline expiry it reappears in `ready`; `ACK` is cancel-safe and final.
* **[I-4] DLQ correctness.** Poison/over-retried messages are quarantined with `{reason, attempt, last_error}`; reprocessing occurs only via explicit operator action or policy job.
* **[I-5] Boundedness & backpressure.** Per-topic shard capacity, a global inflight ceiling, and bounded dedup tables are enforced. When saturated, operations return `429/503` with `Retry-After`.
* **[I-6] OAP/1 constraints.** `max_frame = 1 MiB`; streaming chunk size ≈ 64 KiB; frame size and streaming chunk size are not conflated.
* **[I-7] Addressing & integrity hygiene.** Payload integrity is `payload_hash = blake3_256(payload)`; when surfaced, use the `b3:<hex>` form.
* **[I-8] Capability-only access.** All ops (SEND/RECV/ACK/NACK) require valid, scoped, time-bound macaroon-style capabilities; no ambient trust.
* **[I-9] Amnesia mode.** With `amnesia=ON`, persistence is RAM-only where feasible; transient material is zeroized; no disk spill.
* **[I-10] Golden observability.** Expose latency histograms (enqueue, dequeue, ack), `queue_depth{topic,shard}`, `inflight{topic,shard}`, `saturation{topic,shard}` (0–1), `rejected_total{reason}`, `dlq_total{topic,reason}`, and `/healthz` + `/readyz`.
* **[I-11] Feed neutrality.** Mailbox remains a “dumb pipe.” Fanout/ranking/modding lives in `svc-mod` under `svc-sandbox`.
* **[I-12] Tamper-evidence.** Envelopes carry `hash_chain = H(topic, ts, idem_key, payload_hash, attrs)` verifiable by consumers; integrity failures surface as `E_INTEGRITY`.
* **[I-13] Replay-resistance window.** For each `(topic, idem_key)`, duplicates are rejected for at least `T_replay`. **Default `T_replay = 300s`** (per-topic configurable). The dedup table’s eviction policy must not reduce the effective window below `T_replay`.
* **[I-14] Ordering stance (explicit).** Best-effort FIFO **per shard**; no cross-shard ordering. Under failover, order may degrade but **no message may be lost without a DLQ trace**.
* **[I-15] Capability rotation & revocation.** Expired or revoked caps fail closed (`401/403`) with structured reasons. In-flight ACKs issued before expiry remain valid; new ops fail. **Default `cap_cache_ttl = 30s`** (bounded 5–120s).
* **[I-16] DLQ durability by profile.**

  * *Micronode (amnesia=ON):* DLQ is in-memory and clears on restart; surface `dlq_profile="ephemeral"` in metrics/UI.
  * *Macronode:* DLQ is durable across restarts; reprocessing is explicit.
* **[I-17] Timebase tolerance.** Deadlines use monotonic clocks; the system tolerates at least ±2 minutes wall-clock skew. Wall clock is for telemetry only.

## 2) Design Principles (SHOULD)

* **[P-1] Shard to scale.** Use small, modular shards per topic to avoid head-of-line blocking; scale horizontally by shard count.
* **[P-2] Fail fast.** Verify capabilities, sizes, and quotas at ingress; `/readyz` degrades writes before reads.
* **[P-3] Zero-copy hot path.** Prefer `bytes::Bytes`/owned buffers; do not borrow transient buffers into responses.
* **[P-4] Keep economics out.** Accounting/ledger/rewards are external; mailbox remains pure transport.
* **[P-5] Ops clarity.** Emit structured errors and DLQ reasons; make “reprocess” flows first-class in runbooks and tooling.
* **[P-6] Shard keying.** Default shard key = stable hash of `topic` (or `(topic, partition)`); optionally support sticky keys (e.g., `user_id`) to improve locality while honoring [I-14].
* **[P-7] Backoff with jitter.** Use exponential backoff with full jitter on NACK/retries; **default `backoff_max = 60s`**; formula `rand(0, min(backoff_max, base*2^attempt))`.
* **[P-8] Correlation IDs.** Every envelope includes a stable `corr_id` (UUIDv7) propagated to logs/metrics for end-to-end traces.
* **[P-9] Retention policy.** Per-topic retention windows for `ready/inflight/dlq` in seconds and/or count are explicit and surfaced via config + metrics; never “infinite”. **Defaults:** `ready=7d`, `inflight=visibility_ms*10`, `dlq=14d (macronode) / ephemeral (micronode)`.

## 3) Implementation (HOW)

* **[C-1] Logical API surface.**
  `SEND(topic, bytes, idem_key, attrs?) -> { msg_id, duplicate }`
  `RECV(topic, visibility_ms, max_bytes?, max_messages?) -> [Envelope]`
  `ACK(msg_id) -> { ok:true }`
  `NACK(msg_id, reason?) -> { ok:true }`
  All operations are capability-gated.
* **[C-2] Idempotency guard.** Maintain a bounded `(topic, idem_key) -> {msg_id, payload_hash, ts}` map with TTL ≥ `T_replay` (default 300s).
* **[C-3] Visibility & scanner.** `RECV` moves messages to `inflight(deadline)`; a background scanner requeues overdue `inflight` using `tokio::time::Instant` for deadlines (monotonic).
* **[C-4] DLQ policy & reprocess.** After `attempt > max_attempts` or fatal parse/integrity errors, move to `dlq/<topic>`; emit `dlq_total{topic,reason}`; expose a `dlq:reprocess` operator/job path.
* **[C-5] Backpressure & metrics.** Enforce per-shard capacity and a global inflight ceiling; reject on saturation with `429/503` + `Retry-After`. Expose `queue_depth{topic,shard}`, `inflight{topic,shard}`, `saturation{topic,shard}`, and a **`reorder_depth_max{shard}`** gauge recorded during failover tests.
* **[C-6] Security & capability cache.** Macaroon caps include scoped caveats (topics, ops, quotas, expiry). Cache verifications with TTL `cap_cache_ttl` (default 30s); on `401/403` spikes, purge cache and re-validate; in-flight ACKs permitted until issued cap expiry.
* **[C-7] Deployment profiles.**

  * *Micronode:* single binary; RAM-first shards; `amnesia=ON` by default; ephemeral DLQ as per [I-16].
  * *Macronode:* multi-service fanout via `svc-mod` workers; durable DLQ and larger shard counts.
* **[C-8] Serde & interop guards.** DTOs use `#[serde(deny_unknown_fields)]`; strict types for metrics; OAP/1 framing constants enforced at ingress.
* **[C-9] Envelope schema (concise).**

  ```json
  {
    "msg_id":"ulid|snowflake",
    "topic":"string",
    "ts":"rfc3339",
    "idem_key":"string",
    "payload_hash":"b3:<hex32>",
    "attrs":{"k":"v"},
    "corr_id":"uuidv7",
    "shard":123,
    "attempt":1,
    "hash_chain":"b3:<hex32>",
    "sig":"optional: detached signature"
  }
  ```
* **[C-10] Error taxonomy.**
  `400 E_SCHEMA`, `401 E_CAP_AUTH`, `403 E_CAP_SCOPE`, `409 E_DUPLICATE`, `413 E_FRAME_TOO_LARGE`, `429 E_SATURATED`, `503 E_UNAVAILABLE`, `422 E_INTEGRITY`.
* **[C-11] Timer sources.** Use monotonic time (`tokio::time::Instant`) for deadlines; wall-clock only for telemetry.
* **[C-12] Defaults surfaced.** `T_replay=300s`, `cap_cache_ttl=30s`, `backoff_max=60s`, `visibility_ms_min=250`, retention defaults per [P-9].

## 4) Acceptance Gates (PROOF)

* **[G-1] Crash/at-least-once property tests.** Inject panics mid-flow; assert eventual reappearance or DLQ entry (no silent loss).
* **[G-2] Idempotency tests.** N duplicate `SEND`s with identical `(topic, idem_key, payload_hash)` yield one effective delivery; duplicates return the same `msg_id` with `duplicate=true`.
* **[G-3] Visibility tests.** Un-ACKed messages reappear after `visibility_ms`; `ACK` is final; Loom/TSan race checks pass around `RECV/ACK` boundaries.
* **[G-4] Hardening suite.** 413 on frames >1 MiB; 429/503 under saturation with `Retry-After`; Serde denies unknown fields; quotas enforced.
* **[G-5] Observability SLOs.** Metrics exposed as per [I-10]. Dashboards show **p95 enqueue+dequeue < 50 ms** intra-region; fanout p95 < **2 s** for 10→10k follower pushes (work performed by `svc-mod`).
* **[G-6] Amnesia matrix.** With `amnesia=ON`, no on-disk artifacts after soak; with `amnesia=OFF`, DLQ durability verified across restart.
* **[G-7] Integrity proof.** Bit-flip fault injection causes `E_INTEGRITY`; otherwise `payload_hash` and `hash_chain` verification succeeds end-to-end.
* **[G-8] Replay window tests.** Across `T_replay` (default 300s), duplicates are rejected; after `T_replay + ε`, the key becomes eligible again.
* **[G-9] Ordering tests.** Within a single shard, FIFO holds under normal load; under induced failover, **no loss + bounded reorder** with **`reorder_depth_max{shard} ≤ 32`** (CI fails if exceeded).
* **[G-10] Revocation drills.** Rotate/revoke caps during live traffic; new SEND/RECV fail within `cap_cache_ttl` (default 30s); operations resume with new caps; in-flight ACKs are honored until expiry; no stranded messages.

## 5) Anti-Scope (Forbidden)

* No unbounded queues, retries, or retention windows.
* No ambient authentication or implicit trust at mailbox boundaries.
* No feed ranking/business logic inside mailbox (lives in `svc-mod` + `svc-sandbox`).
* No DHT/overlay/storage semantics creeping in—stay Pillar-11 pure.
* **No “exactly-once” claims.** The system is at-least-once with idempotency.
* **No global ordering promises.** Ordering is best-effort per shard only.
* No silent DLQ loss (except Micronode amnesia by explicit design, surfaced in metrics/UI).

## 6) References

* RustyOnions Pillars (P11), Concurrency & Aliasing, Hardening, Scaling, App Integration blueprints.
* OAP/1 framing constants; BLAKE3 addressing norms; macaroon capability model.
* Internal runbooks: DLQ reprocessing, saturation triage, revocation rotation.

---

**Note:** The HTTP/OpenAPI and Protobuf surfaces (including an NDJSON streaming `recv` and a gRPC `RecvStream`) are provided alongside this IDB as canonical specs; they enforce these invariants on the wire and are validated by the CI test vectors referenced in [G-1]..[G-10].
