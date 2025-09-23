
# 🔗 INTEROP.md — **macronode**

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `macronode`:

* Wire protocols & message formats (**OAP/1**, HTTP/1.1, SSE).
* DTOs & schemas (strict, versioned; CBOR at rest, JSON at edges).
* Bus topics and events (publish/subscribe) over the kernel **Bus**.
* Canonical test vectors (frames, manifests, capabilities).

This binds macronode to **GMI-1.6 Omni-Gate** and the kernel’s runtime invariants—eliminating drift across crates and SDKs.

---

## 1) Protocols & Endpoints

### 1.1 Ingress Protocols

* **HTTP/1.1 + TLS (rustls)** — control & object IO.
* **OAP/1 (framed over TCP)** — low-latency internal RPC / streaming.
* **SSE over HTTP** — live event feeds for ops/audit/dev.
* **(Optional)** Unix domain socket mirror for localhost ops.

> **TLS constraint:** only `tokio_rustls::rustls::ServerConfig` is permitted.

### 1.2 Bindings (typical; configurable via `TransportConfig`)

* `:8443` — HTTPS control surface (`/healthz`, `/readyz`, `/metrics`, `/version`, `/o/*`, `/put`, `/events`).
* `:34000` — OAP/1 framed RPC (intra-cluster).
* `unix:/var/run/rustyonions/macronode.sock` — local ops mirror (optional).

### 1.3 HTTP Endpoints

* `GET /healthz` → 200 when **ready**, else 503.
* `GET /readyz` → readiness (deps/indices).
* `GET /metrics` → Prometheus text exposition.
* `GET /version` → `{ crate, semver, git, build }`.
* `GET /schemas` → JSON list of schema URLs + versions.
* `GET /schemas/{name}.json` → JSON Schema for DTO `name`.
* `GET /o/{addr}` → content-addressed object.

  * `addr`: `b3:<hex>` or `b3:<hex>@<size>`.
  * `ETag: "<addr>"`, content type from manifest when present.
* `HEAD /o/{addr}` → headers only.
* `POST /put` → store object; **requires capability** (macaroon or signed token).

  * Body: raw bytes **or** `multipart/form-data` with `file` and optional `manifest` (JSON or CBOR).
  * `201 Created` → `{ "id":"b3:<hex>", "size":<u64>, "chunks":<u32> }`.

### 1.4 SSE Endpoint

* `GET /events` → `text/event-stream`

  * Format:

    ```
    event: <topic>
    id: <corr_id>
    data: <json>

    ```
  * Topics include: `macronode.health`, `overlay.obj_put`, `overlay.obj_get`, `audit.http`, `audit.oap`.

### 1.5 OAP/1 Verbs

* `OBJ_GET { addr }` → `OBJ_DATA { manifest?, stream }`
* `OBJ_PUT { manifest?, stream }` → `OBJ_ACK { id, accepted }`
* `CFG_SNAP {}` → `CFG_IMG { version, bytes }`
* `HEALTH_PING {}` → `HEALTH_PONG { ready, snapshot }`

**Transport invariants**

* `max_frame = 1 MiB` (OAP/1).
* Streaming **chunk size = 64 KiB**.
* `TransportConfig` enforces: `max_conns`, `read_timeout`, `write_timeout`, `idle_timeout`.

---

## 2) DTOs / Schemas

> All JSON representations MUST use `#[serde(deny_unknown_fields)]` in Rust and the JSON Schemas MUST mark extra properties disallowed.

### 2.1 `ObjectManifestV2`

```rust
/// DAG manifest describing chunk layout and integrity.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectManifestV2 {
  /// Content ID: "b3:<hex>"
  pub id: String,
  /// Total logical size in bytes.
  pub size: u64,
  /// Fixed-size chunks (64 KiB) except the last.
  pub chunks: Vec<ChunkV2>,
  /// Optional media type.
  pub content_type: Option<String>,
  /// Optional bounded user metadata.
  pub meta: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkV2 {
  /// Chunk digest: "b3:<hex>"
  pub id: String,
  /// Zero-based logical offset.
  pub offset: u64,
  /// Chunk length (<= 65536).
  pub len: u32,
}
```

* **Encoding at rest:** **DAG-CBOR**, canonical key order.
* **Validation:**

  * `sum(ch.len) == size`.
  * Root `id` = BLAKE3(content bytes); **not** digest of manifest.
  * Each `chunk.id` = BLAKE3(chunk bytes).

### 2.2 Canonical Envelope (OAP/1)

| Field       | Type | Description                                 |
| ----------- | ---- | ------------------------------------------- |
| `len`       | u32  | Remaining length (excludes this field)      |
| `ver`       | u8   | Protocol version (`1`)                      |
| `flags`     | u16  | Bitset: `REQ, RESP, EVENT, CONT, END, COMP` |
| `tenant_id` | u128 | ULID/UUID; `0` if unused                    |
| `corr_id`   | u64  | Correlation ID (tracing/span link)          |
| `verb`      | u16  | Operation code (e.g., `OBJ_GET`)            |
| `payload`   | \[]  | App bytes; may be chunk-framed and/or LZ4   |

* **Compression:** `COMP` → OAP/1-LZ4 block.
* **Streaming:** `CONT` frames continue; final frame carries `END`.

### 2.3 Capability Token (Macaroon / Signed Token)

```json
{
  "typ": "macaroon",
  "ver": 1,
  "iss": "ron-kms",
  "aud": "macronode",
  "alg": "Ed25519|Dilithium3|Hybrid-Ed25519+Dilithium3",
  "kid": "key-id",
  "nbf": 1732406400,
  "exp": 1732406460,
  "caveats": [
    "ttl=60s",
    "method=POST",
    "path=/put",
    "tenant=00000000-0000-0000-0000-000000000000",
    "max-bytes=104857600"
  ],
  "sig": "base64url(signature)"
}
```

* **Verification:** via `ron-kms`; Hybrid means verify **both** signatures (fail if either fails).
* **Clock skew:** ≤ 60s.
* **Caveats:** unknown **critical** caveats MUST cause rejection (capability parser is conservative).

### 2.4 HTTP Error Envelope

```json
{
  "error": {
    "code": "FrameTooLarge",
    "message": "exceeds 1 MiB",
    "corr_id": "18446744073709551615"
  }
}
```

---

## 3) Bus Topics (Kernel Broadcast)

### 3.1 Published

* `macronode.health` → `KernelEvent::Health { service:"macronode", ok:bool }`
* `macronode.config.updated` → `KernelEvent::ConfigUpdated { version:u64 }`
* `macronode.service.crashed` → `KernelEvent::ServiceCrashed { service:"macronode" }`
* `overlay.obj_put` → `{ id:"b3:<hex>", size:u64, tenant:u128 }`
* `overlay.obj_get` → `{ id:"b3:<hex>", hit:bool, bytes:u64 }`
* `audit.http.access` → sanitized request summary
* `audit.oap.rpc` → OAP verb summary (rate-limited)

### 3.2 Subscribed

* `config.updated` → refresh config snapshot / rebind transports.
* `bus.shutdown` → graceful drain and shutdown.
* `policy.updated` → reload capability rules.
* `kms.key.rotated` → refresh verification keys (PQ + classical).

---

## 4) Canonical Test Vectors

> All vectors live under `macronode/tests/vectors/` and are exercised by CI:
> `cargo test -p macronode --features vectors`.

### 4.1 OAP/1 Frame Round-Trip

**Given** header bytes (little-endian) with `ver=1`, `flags=REQ`, `tenant_id=0`, `corr_id=42`, `verb=OBJ_GET(0x0007)` and JSON payload `{"addr":"b3:<hex>"}`:

* **Input (hex)** → decoder yields:

  ```json
  { "ver":1, "flags":["REQ"], "tenant_id":"0", "corr_id":42, "verb":"OBJ_GET", "payload_len": <n> }
  ```
* **Re-encode** → byte-identical (bit-for-bit).

*(Vector file: `oap1_obj_get_roundtrip.json` + `oap1_obj_get_roundtrip.hex`)*

### 4.2 Manifest Digest — “hello world”

**Payload:** ASCII bytes `hello world` (no newline)
**Expected:**

* `size = 11`
* Single-chunk manifest is accepted; multi-chunk manifests MUST reconstruct identical `id`.
* `root.id = b3:<hex-of(hello world)>` (computed by test harness and compared to embedded golden).

*(Vector files: `hello_world.bytes`, `hello_world.manifest.cbor`, `hello_world.manifest.json`)*

> Note: We store the **computed** BLAKE3 once (golden) and assert future recomputations match; tests will fail if any encoder/decoder drifts.

### 4.3 Capability Acceptance / Rejection

* **Accept:** macaroon with `alg=Hybrid-Ed25519+Dilithium3`, valid `nbf/exp`, `method=POST`, `path=/put`, `max-bytes=100MiB`. → `201 Created`.
* **Reject (403 InvalidCapability):** tampered caveat, bad `kid`, or missing Hybrid leg.

*(Vector files: `cap_ok.json`, `cap_tampered.json`, `cap_missing_pq_leg.json`)*

### 4.4 Quota / Size

* **POST /put** body = 1,048,577 bytes (1 MiB + 1) → `413 FrameTooLarge`.
* **OAP/1** non-stream frame `len > 1 MiB` → `FrameTooLarge`.

---

## 5) Error Taxonomy

| Code                | When                                | HTTP    | OAP/1            |
| ------------------- | ----------------------------------- | ------- | ---------------- |
| `BadVersion`        | `ver != 1`                          | 400     | `BadVersion`     |
| `FrameTooLarge`     | frame (non-stream) exceeds 1 MiB    | 413     | `FrameTooLarge`  |
| `ChunkTooLarge`     | a chunk `len > 64 KiB`              | 413     | `ChunkTooLarge`  |
| `QuotaExceeded`     | tenant quota exhausted              | 429     | `QuotaExceeded`  |
| `NotReady`          | readiness gate failed               | 503     | `NotReady`       |
| `NotFound`          | unknown object address              | 404     | `NotFound`       |
| `InvalidCapability` | missing/expired/invalid capability  | 401/403 | `AuthFailed`     |
| `Conflict`          | overwrite divergent existing object | 409     | `Conflict`       |
| `Timeout`           | IO exceeded configured timeout      | 504     | `Timeout`        |
| `Internal`          | unexpected internal error           | 500     | `Internal`       |
| `NotImplemented`    | unknown OAP/1 verb                  | 501     | `NotImplemented` |

**Mapping rule:** Every OAP/1 error maps to the closest HTTP status; HTTP error bodies use §2.4.

---

## 6) Interop Guarantees

* **No Kernel Drift:** Public `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()` are stable across minor releases.
* **SemVer Discipline:** Any wire-visible breaking change (path, DTO, flag, error) bumps **major**.
* **Backward Compatibility:**

  * Unknown **fields** ignored on read; never silently dropped on re-emit.
  * Unknown **flags** → drop with telemetry.
  * Unknown **verbs** → `501 NotImplemented`.
* **Auditability:** Golden vectors in `/tests/vectors/`; JSON Schemas in `/docs/schemas/`. CI ensures round-trip determinism.
* **TLS Uniformity:** TLS always via `tokio_rustls::rustls::ServerConfig`.
* **Determinism:** DAG-CBOR canonicalization; **no floats** in wire DTOs.
* **Observability:** `corr_id` propagates across HTTP/OAP/1 → spans; `/metrics` exports counters/histograms and error cardinalities.
* **Resource Bounds:** `max_frame`, `chunk_size`, `max_conns`, timeouts enforced centrally by `TransportConfig`.

---

## 7) Amnesia Mode (Explicit)

* **Goal:** zero persistent artifacts when `AMNESIA=1` (env/config).
* **Effects:**

  * Storage backend: **RAM-only**; no on-disk chunks or manifests.
  * **Vectors:** test harness uses in-memory vectors; CI writes golden files only when `AMNESIA=0`.
  * Logs/metrics: stdout/stderr only; no file sinks.
  * Capability cache & KMS keys: memory-resident; zeroized on shutdown.
* **Interop Impact:** wire contracts unchanged; only durability semantics differ.

---

## 8) Post-Quantum Hooks

* **Capabilities:** `alg` supports `Ed25519`, `Dilithium3`, or `Hybrid-Ed25519+Dilithium3`.
* **KMS:** key discovery via `kms.key.rotated` bus event (classical + PQ).
* **Handshake:** OAP/1 control frames can carry `pq: true` hint inside payloads for feature negotiation (non-breaking).
* **SDK Guidance:** default to **Hybrid** verification where available; log downgrade to classical.

---

## 9) Facet Interop Map (DX / Macro profile)

* **Overlay / Storage:** `/o/{addr}`, `OBJ_GET/PUT` interop with `svc-overlay`, `svc-storage`.
* **Index / Search:** manifest headers and SSE `overlay.obj_put` feed `svc-index` for Graph/Search hydration.
* **Audit / Metrics:** `audit.*` events → `ron-metrics` ingest; `/metrics` Prom scrape.
* **Auth / KMS:** capabilities verified against `ron-auth` policies; keys via `ron-kms` (PQ-aware).
* **Policy:** `policy.updated` consumption aligns with `ron-policy` runtime rules.

> This keeps macronode facet-clean while enabling Graph/Feed/Search composition via services.

---

## 10) References

* **Interop Blueprint GMI-1.6** (Omni-Gate alignment)
* **OAP/1 Spec** (framing, flags, verbs, LZ4 profile)
* **OBSERVABILITY.md** (tracing/corr\_id propagation, metrics)
* **CONCURRENCY.md** (readiness/shutdown/backpressure)
* **SECURITY.md** (capability rules, PQ policy, amnesia implications)

---

✅ With this, macronode’s **wire-level contract** is explicit, PQ-aware, amnesia-safe, facet-mapped, and test-verifiable. SDKs and auditors get strict DTOs, bounded transports, and golden vectors—no drift.

