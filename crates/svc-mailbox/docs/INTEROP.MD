````markdown
# 🔗 INTEROP.md — svc-mailbox

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `svc-mailbox`:

* Wire protocols & message formats (HTTP/1.1 + TLS, OAP/1 framed gRPC).
* DTOs & schemas (Envelope, Send/Recv/Ack requests).
* Bus topics and events.
* Canonical test vectors.

This ensures all inter-crate and external integrations remain consistent with **GMI-1.6 Omni-Gate** and the `IDB` invariants (at-least-once, bounded queues, visibility, idempotency, tamper-evidence).

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

- **HTTP/1.1 + TLS 1.3** (tokio-rustls). JSON request/response.  
- **HTTP/1.1 + NDJSON** (`/v1/recv/stream`) for server-sent streaming.  
- **gRPC** over HTTP/2 (OAP/1 framed). Proto package: `ron.mailbox.v1`.  
- **UDS** (optional): identical HTTP surface via Unix socket (peer cred enforced).

> **Transport Invariants**
>
> - `max_frame = 1 MiB` (OAP/1 hard cap)  
> - **Streaming chunk** (gRPC HTTP/2): ≈ **64 KiB** target  
> - TLS uses **`tokio_rustls::rustls::ServerConfig`** only  
> - All DTOs use strict JSON with `deny_unknown_fields`

### 1.2 Exposed Endpoints (HTTP)

Base path: `/v1`

| Method | Path               | Purpose                         | Auth (cap scope) |
|-------:|--------------------|----------------------------------|------------------|
| POST   | `/send`            | Enqueue message (idempotent)     | `send` + topic   |
| POST   | `/recv`            | Batch receive with visibility    | `recv` + topic   |
| POST   | `/recv/stream`     | NDJSON stream of envelopes       | `recv` + topic   |
| POST   | `/ack/{msg_id}`    | Acknowledge (finalize)           | `ack`            |
| POST   | `/nack/{msg_id}`   | Negative-ack (requeue/backoff)   | `nack`           |
| POST   | `/dlq/reprocess`   | Operator reprocess from DLQ      | `admin`          |
| GET    | `/metrics`         | Prometheus exposition            | local/gateway    |
| GET    | `/healthz`         | Liveness                         | open (read-only) |
| GET    | `/readyz`          | Readiness                        | open (read-only) |

**Canonical Headers**

- `Authorization: Bearer <macaroon>`  
- `X-Corr-Id: <uuidv7>` (optional input; always output)  
- `Retry-After: <seconds>` (429/503)  
- `X-Idempotency-Mode: 200-flag|409-conflict` (optional; default `200-flag`)

### 1.3 gRPC Service

```proto
service Mailbox {
  rpc Send       (SendRequest) returns (SendResponse);
  rpc Recv       (RecvRequest) returns (RecvResponse);
  rpc RecvStream (RecvRequest) returns (stream Envelope);
  rpc Ack        (AckRequest)  returns (AckResponse);
  rpc Nack       (NackRequest) returns (AckResponse);
}
````

* **Framing:** OAP/1; per-message frame ≤ 1 MiB.
* **Backpressure:** retry with jitter on `UNAVAILABLE` / `RESOURCE_EXHAUSTED`.

---

## 2) DTOs / Schemas

### 2.1 Envelope (wire-level, JSON / Proto)

```json
{
  "msg_id": "ulid|snowflake",
  "topic": "string",
  "ts": "rfc3339",
  "idem_key": "string",
  "payload_hash": "b3:<hex64>",
  "attrs": { "k": "v" },
  "corr_id": "uuidv7",
  "shard": 123,
  "attempt": 1,
  "hash_chain": "b3:<hex64>",
  "sig": null
}
```

* **Integrity:** `payload_hash = blake3_256(payload)`;
  `hash_chain = H(topic || ts || idem_key || payload_hash || attrs)`.
* **Ordering:** best-effort FIFO **per shard**; no cross-shard ordering guarantees.

### 2.2 HTTP DTOs

**SendRequest**

```json
{ "topic":"user:42:inbox", "idem_key":"email-2025-10-12-01", "payload_b64":"<base64>", "attrs":{ } }
```

**SendResponse**

```json
{ "msg_id":"01JAE6...", "duplicate":false }
```

**RecvRequest**

```json
{ "topic":"user:42:inbox", "visibility_ms":5000, "max_messages":32, "max_bytes":524288 }
```

**RecvResponse**

```json
{ "messages":[ <Envelope> ... ] }
```

**Ack/Nack**

```json
{ "ok": true }
```

**Error (canonical)**

```json
{ "code":"E_SATURATED", "message":"work queue full", "corr_id":"018f..." }
```

### 2.3 gRPC Messages (summary)

* `Envelope` (fields above; plus optional `bytes payload` for gRPC path)
* `SendRequest { string topic; string idem_key; map<string,string> attrs; bytes payload; }`
* `RecvRequest { string topic; uint32 visibility_ms; uint32 max_messages; uint32 max_bytes; }`
* `AckRequest { string msg_id; }` / `NackRequest { string msg_id; string reason; }`

### 2.4 Validation Rules (interop-critical)

* JSON: **deny unknown fields** (serde).
* `payload_b64` must decode ≤ **1 MiB**.
* `visibility_ms ≥ 250` (configurable min).
* Duplicate `SEND` (same `(topic, idem_key, payload_hash)` within `T_replay`) returns same `msg_id`.
* Integrity mismatch → `422 E_INTEGRITY` and eventual DLQ after attempts.

---

## 3) Bus Topics

> The mailbox is a **transport**; it emits operational events for observability and control. No feed/ranking semantics appear here (see `svc-mod`).

### 3.1 Events Published

* `mailbox.health`

  ```json
  { "service":"svc-mailbox", "ok":true, "profile":"macronode", "ts":"..." }
  ```
* `mailbox.readyz`

  ```json
  { "ready":true, "missing":[],"ts":"..." }
  ```
* `mailbox.dlq.moved`

  ```json
  { "topic_class":"user_inbox","reason":"parse_error","count":42,"ts":"..." }
  ```
* `mailbox.auth.revoked` (burst when auth failures spike)

  ```json
  { "scope":"send","samples":17,"window_s":30,"ts":"..." }
  ```
* `mailbox.scanner.requeue` (rate-limited)

  ```json
  { "shard":7,"count":128,"ts":"..." }
  ```

### 3.2 Events Subscribed

* `config.updated` → refresh runtime config snapshot.
* `bus.shutdown` → initiate graceful shutdown.
* (Optional) `caps.rotate` → purge capability cache immediately.

> **Topics are versionless operational channels.** Payloads use stable JSON keys, `deny_unknown_fields`.

---

## 4) Canonical Test Vectors

> Stored under `/tests/vectors/mailbox/` and referenced by CI contract tests.

### 4.1 Idempotent Enqueue (duplicate recognized)

* **SendRequest A**

  ```json
  {"topic":"user:42:inbox","idem_key":"email-2025-10-12-01","payload_b64":"eyJzIjoiaGkifQ=="}
  ```
* **Response A** → `{"msg_id":"01JAE6...","duplicate":false}`
* **SendRequest B (same as A)**
* **Response B** → `{"msg_id":"01JAE6...","duplicate":true}`

**Assertions**

* `msg_id` equal; second call does not create a second delivery.
* `mailbox_enqueued_total` increments once; `E_DUPLICATE` not emitted in `200-flag` mode.

### 4.2 Visibility Expiry & Reappear

* **Recv** with `visibility_ms=1000`, do not ACK.
* Sleep 1200 ms.
* **Recv** again → same `msg_id`, `attempt >= 2`.

**Assertions**

* `mailbox_visibility_timeout_total` increments; `requeue` path used.

### 4.3 DLQ on Poison

* Consumer fails parse **5×** (`max_attempts=5`).
* Message moves to `dlq/user:42:inbox` with `reason="E_PARSE"`.

**Assertions**

* `mailbox_dlq_total{reason="E_PARSE"}` increments.
* Reprocess endpoint returns to ready with `attempt` reset or policy-defined.

### 4.4 Integrity Failure

* Flip one byte in stored payload before RECV.
* Consumer recomputes BLAKE3 → mismatch.
* Server surfaces `422 E_INTEGRITY` path & DLQ after policy.

**Assertions**

* `integrity_fail_total{reason="hash_chain" | "payload_hash"}` increments.

### 4.5 Replay Window

* Flood N `SEND` within `T_replay=300s` with same `(topic, idem_key, payload_hash)` → single effective delivery.
* After `>T_replay`, key accepted again.

---

## 5) Error Taxonomy (wire contract)

| HTTP | Code                | Semantics                       | gRPC mapping          |
| ---: | ------------------- | ------------------------------- | --------------------- |
|  400 | `E_SCHEMA`          | invalid JSON / unknown fields   | `INVALID_ARGUMENT`    |
|  401 | `E_CAP_AUTH`        | invalid/expired token           | `UNAUTHENTICATED`     |
|  403 | `E_CAP_SCOPE`       | scope mismatch                  | `PERMISSION_DENIED`   |
|  409 | `E_DUPLICATE`       | duplicate (compat mode)         | `ALREADY_EXISTS`      |
|  413 | `E_FRAME_TOO_LARGE` | > 1 MiB                         | `RESOURCE_EXHAUSTED`  |
|  422 | `E_INTEGRITY`       | hash/hash_chain/signature fails | `FAILED_PRECONDITION` |
|  429 | `E_SATURATED`       | backpressure                    | `RESOURCE_EXHAUSTED`  |
|  503 | `E_UNAVAILABLE`     | not ready / degraded            | `UNAVAILABLE`         |

* All error bodies include `corr_id`.
* `Retry-After` advertised on 429/503 (seconds).

---

## 6) Interop Guarantees

* **No Kernel Drift:** mailbox kernel hooks (metrics/health/config) use stable names; updates are additive.
* **SemVer Discipline:** Any **breaking** schema or behavior change (HTTP, gRPC, error codes, defaults) → **major version** bump and `/vN/` path.
* **Backward Compatibility:**

  * JSON: unknown **input** fields rejected (secure strictness).
  * JSON: **output** may add fields (clients must ignore extras).
  * gRPC: new fields use **new field numbers**; existing numbers never repurposed.
* **Auth Consistency:** All ops require macaroon capability; no ambient trust.
* **Auditability:** Canonical vectors live under `/tests/vectors/` and are exercised in CI on every PR.

---

## 7) References

* **Interop Blueprint GMI-1.6** — `docs/Interop_Blueprint.md`
* **OAP/1 Spec** — `docs/specs/OAP-1.md`
* **OpenAPI** — `api/openapi.yaml` (HTTP contract)
* **Proto** — `proto/ron/mailbox/v1/mailbox.proto` (gRPC contract)
* **IDB** — `docs/IDB.md` (invariants → gates mapping)
* **OBSERVABILITY** — `docs/OBSERVABILITY.md` (corr_id, metrics)
* **CONCURRENCY** — `docs/CONCURRENCY.md` (readiness semantics)

---

✅ With this document, `svc-mailbox` declares a **precise wire-level contract**: protocols, DTOs, events, vectors, and errors. SDKs and sibling services can integrate confidently without drift, and CI keeps the contract stable over time.

```
```
