

````markdown
---
title: ron-app-sdk — Invariant-Driven Blueprint (IDB)
version: 1.0.0
status: ready
last-updated: 2025-10-16
audience: contributors, ops, auditors
msrv: 1.80.0
crate-type: sdk
concerns: [DX, SEC, RES]
pillar: 7 # App BFF & SDK
---

# ron-app-sdk — IDB

## 0) Scope & Role

`ron-app-sdk` is the **thin, profile-agnostic client** for applications to speak to RustyOnions nodes (Micronode & Macronode) using **capability-based** auth and **OAP/1** envelopes. The SDK is **client-only** (no servers), enforces canon limits (frame/chunk), offers **resilient defaults** (deadlines, idempotency, full-jitter retries), and guarantees **schema hygiene** via `ron-proto` types. It exposes an ergonomic async API with **typed DTOs**, **trace propagation**, and **consistent errors**.

---

## 1) Invariants (MUST)

- [I-1] **Profile parity**  
  Behavior at the public API is identical for Micronode (amnesia) and Macronode (persistent). No semantic branches by profile.

- [I-2] **Capabilities-only**  
  Every outbound request carries a valid capability; no ambient credentials are used by default.

- [I-3] **OAP/1 fidelity**  
  Enforce `max_frame = 1 MiB`; stream large payloads in ~64 KiB chunks; refuse to construct or send envelopes beyond bounds.

- [I-4] **Canonical content addressing**  
  Object IDs are `b3:<hex>`; when the SDK is the terminal consumer (e.g., local verify), compute and verify the full BLAKE3 digest.

- [I-5] **Resilient-by-default**  
  Mutating calls attach an idempotency key; retries use **full-jitter exponential backoff** with defaults:  
  `base=100ms`, `factor=2.0`, `cap=10s`, `max_attempts=5`; all attempts respect per-call deadlines.

- [I-6] **DTO hygiene in both directions**  
  Deserialization uses `serde(deny_unknown_fields)`; serialization emits no extraneous fields (round-trip stable).

- [I-7] **Transport-agnostic client**  
  Bind via `ron-transport` (TLS; optional Tor via `arti` feature). SDK never opens server sockets or runs service loops.

- [I-8] **Deadlines everywhere**  
  Every call requires an explicit or default deadline and is executed with non-blocking async I/O.

- [I-9] **Error taxonomy is stable**  
  Public errors are represented by `SdkError` with `#[non_exhaustive]`. No string matching for control flow.

- [I-10] **Versioning & compatibility**  
  Public API follows SemVer; breaking changes are major-only with a migration note. DTO evolution is **additive-first** and tracked in a **compat matrix**.

- [I-11] **No persistent local state**  
  The SDK maintains only opt-in ephemeral caches (bounded + TTL) that never bypass capability checks and respect amnesia expectations.

- [I-12] **Canon discipline**  
  Only canonical crates are referenced; names and versions are pinned in the workspace policy.

---

## 2) Design Principles (SHOULD)

- [P-1] **Small surface, strong defaults**: Safe-by-default (caps, deadlines, retries, idempotency). Power tuning requires explicit opt-in.
- [P-2] **Deterministic envelopes & errors**: Map wire/OAP errors exactly; never “guess” recovery on non-retriables.
- [P-3] **Transparency without leaks**: Expose `SdkContext { profile, amnesia }` as metadata; do **not** change semantics based on it.
- [P-4] **Zero kernel creep**: Protocol rules and DTOs live in `oap`/`ron-proto`. SDK is a client library, not a policy engine.
- [P-5] **Pluggable observability**: Provide hooks for tracing/log enrichment, redaction, and custom metrics without forking core.

---

## 3) Implementation (HOW)

### 3.1 Cargo Features & Dependency Posture
- **Features**: `default = ["tls"]`; optional: `tor` (via `arti`), `metrics`, `serde_json` (examples/dev only).
- **No** server deps; **no** kernel/service loops.
- **Tokio** required (`rt-multi-thread`, `time`, `net`, `io-util`).

```toml
[dependencies]
tokio = { version = "1.47", features = ["macros","rt-multi-thread","time","net","io-util"] }
ron-transport = { version = "x.y", default-features = false, features = ["tls"] }
ron-proto = "x.y"
tracing = "0.1"
blake3 = "1"
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", optional = true }

[features]
default = ["tls"]
tls = []
tor = ["ron-transport/tor"]
metrics = []
````

### 3.2 Public API Surface (sketch)

```rust
pub struct SdkConfig {
    pub transport: TransportCfg,
    pub timeouts: Timeouts { connect: Duration, read: Duration, write: Duration, overall: Duration },
    pub retry: RetryCfg { base: Duration, factor: f32, cap: Duration, max_attempts: u32, jitter: Jitter },
    pub idempotency: IdemCfg { enabled: bool, key_prefix: Option<String> },
    pub cache: CacheCfg { enabled: bool, max_entries: usize, ttl: Duration },
    pub tracing: TracingCfg { propagate: bool, redaction: RedactionPolicy },
}

pub struct RonAppSdk { /* ... */ }

impl RonAppSdk {
    pub async fn new(cfg: SdkConfig) -> Result<Self, SdkError>;

    // Mailbox (at-least-once)
    pub async fn mailbox_send(&self, cap: Capability, msg: Mail, deadline: Duration, idem: Option<IdemKey>)
        -> Result<Receipt, SdkError>;
    pub async fn mailbox_recv(&self, cap: Capability, deadline: Duration)
        -> Result<Vec<Mail>, SdkError>;
    pub async fn mailbox_ack(&self, cap: Capability, ack: Ack, deadline: Duration)
        -> Result<(), SdkError>;

    // Edge (HTTP-ish fetch with byte-range)
    pub async fn edge_get(&self, cap: Capability, path: &str, range: Option<ByteRange>, deadline: Duration)
        -> Result<Bytes, SdkError>;

    // Storage (content addressed)
    pub async fn storage_get(&self, cap: Capability, addr_b3_hex: &str, deadline: Duration)
        -> Result<Bytes, SdkError>;
    pub async fn storage_put(&self, cap: Capability, blob: Bytes, deadline: Duration, idem: Option<IdemKey>)
        -> Result<AddrB3, SdkError>;

    // Index (resolve)
    pub async fn index_resolve(&self, cap: Capability, key: &IndexKey, deadline: Duration)
        -> Result<AddrB3, SdkError>;

    pub fn context(&self) -> SdkContext; // { profile: Micronode|Macronode, amnesia: bool }
}
```

### 3.3 End-to-End Pseudocode: Mutating Call (Idempotent + Backoff)

```rust
pub async fn storage_put(
    &self,
    cap: Capability,
    blob: Bytes,
    deadline: Duration,
    idem: Option<IdemKey>,
) -> Result<AddrB3, SdkError> {
    let idem = idem.unwrap_or_else(IdemKey::generate);
    let started = Instant::now();
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        let attempt_deadline = self.deadline_slice(deadline, started)?;
        let span = tracing::info_span!(
            "sdk.call",
            endpoint = "storage.put",
            attempt,
            deadline_ms = attempt_deadline.as_millis() as i64,
            payload_len = blob.len() as i64,
            node_profile = %self.ctx.profile,
            amnesia = self.ctx.amnesia
        );
        let _e = span.enter();

        // 1) Build OAP/1 envelope (frame-bound checked)
        let env = build_envelope_put(&cap, &blob, &idem).map_err(SdkError::OapViolation)?;

        // 2) Send with per-attempt deadline
        let send_res = tokio::time::timeout(attempt_deadline, self.tx.send(env)).await;
        let res = match send_res {
            Err(_) => Err(SdkError::DeadlineExceeded),
            Ok(inner) => inner,
        };

        match res {
            Ok(reply) => {
                let dto = decode_and_validate::<PutReceipt>(reply)
                    .map_err(|e| SdkError::SchemaViolation { path: e.path, detail: e.detail })?;
                let addr = AddrB3::try_from(dto.addr)?;
                // Optional terminal verify if configured (I-4)
                if self.cfg.cache.enabled && self.cfg.cache.verify_puts {
                    verify_blake3(&blob, &addr)?;
                }
                tracing::info!(target: "sdk", "ok");
                return Ok(addr);
            }
            Err(e) => {
                if !self.is_retriable(&e) || attempt >= self.cfg.retry.max_attempts {
                    tracing::warn!(target = "sdk", error = %e, "fail");
                    return Err(self.map_error(e));
                }
                let delay = self.backoff_delay(attempt);
                tracing::info!(target = "sdk", ?delay, "retry");
                tokio::time::sleep(delay).await;
                continue;
            }
        }
    }
}
```

**Retry classes**

* `Retriable`: transport timeouts, transient 5xx, explicit `Retry-After`.
* `Maybe`: overload/backpressure signals (respect hints).
* `NoRetry`: capability failure/expired, OAP bound breach, schema violation, 4xx that indicate caller error.

**Backoff (full-jitter) formula**
`delay_n = min(cap, base * factor^(n-1)) * (1 + rand[0,1))`

### 3.4 Error Taxonomy (public, stable)

```rust
#[non_exhaustive]
pub enum SdkError {
  DeadlineExceeded,
  Transport(std::io::ErrorKind),
  Tls,
  TorUnavailable,
  OapViolation { reason: &'static str },
  CapabilityExpired,
  CapabilityDenied,
  SchemaViolation { path: String, detail: String },
  NotFound,                 // 404/resolve miss
  Conflict,                 // 409
  RateLimited { retry_after: Option<std::time::Duration> }, // 429
  Server(u16),              // 5xx code
  Unknown(String),
}
```

**Mapping (illustrative):**

| Wire/Condition                 | SdkError variant                 | Retry?      |
| ------------------------------ | -------------------------------- | ----------- |
| Timeout (connect/read/write)   | `Transport(ErrorKind::TimedOut)` | Yes         |
| TLS handshake fail             | `Tls`                            | No (config) |
| Tor SOCKS not available        | `TorUnavailable`                 | Maybe       |
| OAP frame > 1 MiB              | `OapViolation{...}`              | No          |
| 401/403 cap invalid/expired    | `CapabilityDenied/Expired`       | No          |
| 404 resolve miss               | `NotFound`                       | No          |
| 409 conflict on idempotent PUT | `Conflict`                       | No          |
| 429 with `Retry-After`         | `RateLimited{retry_after}`       | Yes (hint)  |
| 5xx (generic)                  | `Server(code)`                   | Yes         |
| DTO parse/unknown field        | `SchemaViolation{...}`           | No          |

### 3.5 Tracing & Metrics

**Spans/fields (minimum):**

* `sdk.call`: `endpoint`, `attempt`, `deadline_ms`, `payload_len`, `frame_count`, `node_profile`, `amnesia`, `idem_key[redacted]`
* `sdk.retry`: `attempt`, `delay_ms`, `class` (`retriable|maybe`)
* `sdk.decode`: `dto`, `bytes`, `elapsed_ms`

**Metrics (Prometheus names):**

* `sdk_requests_total{endpoint, outcome="ok|error", code}`
* `sdk_request_latency_seconds{endpoint, quantile}` (histogram → summarize p50/p95/p99)
* `sdk_retries_total{endpoint, reason}`
* `sdk_oap_violations_total{endpoint, reason}`
* `sdk_deadline_exceeded_total{endpoint}`

### 3.6 Configuration Schema (env + builder parity)

| Key                          | Type  | Default | Notes                   |
| ---------------------------- | ----- | ------- | ----------------------- |
| `RON_SDK_TRANSPORT`          | enum  | `tls`   | `tls` or `tor`          |
| `RON_SDK_TIMEOUT_OVERALL_MS` | u64   | 5000    | Per-call outer deadline |
| `RON_SDK_RETRY_BASE_MS`      | u64   | 100     | I-5                     |
| `RON_SDK_RETRY_FACTOR`       | f32   | 2.0     | I-5                     |
| `RON_SDK_RETRY_CAP_MS`       | u64   | 10000   | I-5                     |
| `RON_SDK_RETRY_MAX_ATTEMPTS` | u32   | 5       | I-5                     |
| `RON_SDK_IDEM_ENABLED`       | bool  | true    | Mutations               |
| `RON_SDK_CACHE_ENABLED`      | bool  | false   | Ephemeral only          |
| `RON_SDK_CACHE_MAX_ENTRIES`  | usize | 256     | LRU                     |
| `RON_SDK_CACHE_TTL_MS`       | u64   | 30000   | 30s                     |
| `RON_SDK_TRACING_PROPAGATE`  | bool  | true    | B3/W3C as supported     |

---

## 4) Acceptance Gates (PROOF)

### 4.1 Tests & Property Checks

* [G-1] **Interop**: TLS + Tor round-trips for: `mailbox_send/recv/ack`, `edge_get` (range), `storage_get/put`, `index_resolve`.
* [G-2] **OAP bounds**: Proptests reject frames > 1MiB; storage streaming slices at ~64 KiB; fuzz harness covers envelope/decoder.
* [G-3] **DX**: Doc examples compile (`cargo test --doc`) and pass against Micronode (amnesia ON); mutation examples verify idempotency under injected retries.
* [G-4] **Security**: All tests attach a capability; short-TTL rotate path covered; no ambient calls in test suite.
* [G-5] **Concerns**: CI labels `concern:DX|SEC|RES` pass; `cargo clippy -D warnings`, `cargo fmt -- --check`.
* [G-6] **Public API**: `cargo public-api` + `cargo semver-checks` are green; MSRV checked in CI with `1.80.0`.
* [G-7] **Perf SLO (SDK overhead)**: Loopback, warm TLS, ≤64 KiB: median ≤ 2ms, p95 ≤ 5ms; report includes CPU, core count, and TLS session reuse settings.
* [G-8] **Chaos retries**: With 20% transient faults + 2% timeouts injected, p95 success ≤ 3 attempts; **no duplicate side effects** for idempotent ops.
* [G-9] **Error conformance**: Wire errors map to stable `SdkError` variants; forbid substring matching for control flow in lints.
* [G-10] **Compat matrix**: CI runs the SDK against `ron-proto` (N, N-1) and wire (N, N-1) schemas; fails on non-additive changes without major bump.

### 4.2 Invariants → Gates Traceability

| Invariant | Gate(s)                                                              |
| --------- | -------------------------------------------------------------------- |
| I-1       | G-1, G-3 (Micronode vs Macronode parity)                             |
| I-2       | G-4                                                                  |
| I-3       | G-2                                                                  |
| I-4       | G-1, G-2 (terminal verify path)                                      |
| I-5       | G-8 (chaos), G-7 (perf impact), G-3 (examples)                       |
| I-6       | G-9, G-10                                                            |
| I-7       | G-1 (TLS/Tor), code review checklists                                |
| I-8       | G-1/G-7 (deadline enforcement), clippy lints disallow blocking calls |
| I-9       | G-9                                                                  |
| I-10      | G-6, G-10                                                            |
| I-11      | Code scan + tests; cache TTL tests                                   |
| I-12      | G-6 (deps audit), repo policy script                                 |

---

## 5) Anti-Scope (Forbidden)

* No profile-specific semantic branching.
* No ambient credentials; no cache that bypasses capability checks.
* No servers/accept loops/kernel logic inside the SDK.
* No DTOs without strict (de)serialization guards.
* No persistent local state (ephemeral caches must be bounded+TTL).
* No stringly-typed error control flow.

---

## 6) Reviewer & CI Checklists

**Reviewer quick pass**

* [ ] New APIs are `async` and accept deadlines.
* [ ] Mutations accept or generate idempotency keys.
* [ ] OAP frame enforcement present prior to send.
* [ ] `SdkError` mapping added; no string matching.
* [ ] DTOs derive `serde` with `deny_unknown_fields`.
* [ ] No server sockets/loops; no blocking I/O.

**CI jobs**

* [ ] Linux/macOS builds at MSRV=1.80.0 and stable.
* [ ] `cargo test --all-features` including `tor`.
* [ ] `cargo public-api`, `cargo semver-checks`.
* [ ] Proptests + fuzz (envelope/decoder).
* [ ] Perf microbench: report p50/p95.
* [ ] Compat matrix job (proto N, N-1).

---

## 7) Appendix: Example Integration (docs test)

```rust
/// Basic storage PUT with deadlines, idempotency, and trace propagation.
#[tokio::test]
async fn docs_storage_put_example() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = RonAppSdk::new(SdkConfig::default()) .await?;
    let cap = Capability::from_env()?;
    let payload = Bytes::from_static(b"hello onions");
    let receipt = sdk.storage_put(cap, payload, Duration::from_secs(5), None).await?;
    assert!(receipt.to_string().starts_with("b3:"));
    Ok(())
}
```

---

## 8) References (canon anchors)

* App Integration canon — SDK contract, flows, SLOs (v2.0)
* OAP/1 framing & envelope rules (v1.0)
* Six Concerns gating rubric (DX, SEC, RES) (v1.3)
* Pillar 7 — App BFF & SDK (v2.0)
* Canonical crate list & naming discipline (v2.0)
* Versioning & compat matrix governance (v1.1)

```

