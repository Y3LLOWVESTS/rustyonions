

---

````markdown
# 🔗 INTEROP.md — ron-bus

*Audience: developers, auditors, external SDK authors*  
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-bus`:

- **In-proc** event contract (`Event` enum) and subscriber/publisher rules.
- How hosts translate bus events to **wire protocols** (Omni-Gate GMI-1.6: HTTP/OAP/…).
- DTO semantics and **SemVer** policy for event evolution.
- Canonical test scaffolds to keep kernel/service integrations in lock-step.

> Scope reminder: `ron-bus` is **in-process only**. It has **no sockets, endpoints, or wire framing**. All network interop happens **in host services** that consume this crate.

---

## 1) Protocols & Endpoints (N/A in ron-bus)

- **Ingress Protocols:** N/A (no network in this crate).
- **Exposed Endpoints:** N/A.
- **Transport Invariants:** N/A.

**Where this applies instead:** Host crates (e.g., gateway, overlay, kernel HTTP) map in-proc `Event` values to wire envelopes defined in **Omni-Gate (GMI-1.6)** and/or OAP/1. This document focuses on making that mapping **lossless and stable**.

---

## 2) DTOs / Schemas (Event Contract)

`ron-bus` publishes **strongly-typed in-proc events**. The canonical shape is the public `Event` enum (see API.md). It is **`#[non_exhaustive]`** to allow additive growth.

### 2.1 `Event` (kernel-aligned)

```rust
#[non_exhaustive]
pub enum Event {
  /// Basic health signal from a component.
  Health { service: String, ok: bool },

  /// Host’s config changed (e.g., reloaded file/env); version is monotonic.
  ConfigUpdated { version: u64 },

  /// A supervised component crashed (reason is operator-facing).
  ServiceCrashed { service: String, reason: String },

  /// Global shutdown initiation (graceful/cancel-cooperative).
  Shutdown,
}
````

**Encoding:** Not applicable in this crate (no wire).
**Stability:** Additive new variants → **minor** version; changes to existing variants → **major**.

> **Correlation IDs:** `Event` carries **no transport envelope**. Correlation is supplied by hosts (e.g., via `tracing` span fields or event wrapper types local to the host), not by `ron-bus`.

**Polyglot SDK parity:** For externalization, hosts MUST expose a stable JSON schema for the wire-facing `WireEvent`. SDKs (Rust/TS/Python/Swift) MUST round-trip this schema (serialize → parse → serialize) without loss. Store canonical JSON examples under `/tests/interop/vectors/` and mirror them in SDK repos.

---

## 3) Bus Semantics (Topics, Publish/Subscribe)

### 3.1 Topics

* There are **no topics**. `ron-bus` uses a **single broadcast** channel of `Event`.
* Subscribers **filter in code** by `match`ing the `Event` variant(s) they care about.

### 3.2 Publish & Subscribe Patterns (canonical)

```rust
// Publisher (host code)
let tx = bus.sender();
let _ = tx.send(Event::Health { service: "overlay".into(), ok: true });

// Subscriber (unique receiver per task; Lagged handling + metrics hooks)
let mut rx = bus.subscribe();
tokio::spawn(async move {
    loop {
        match rx.recv().await {
            Ok(ev) => handle_event(ev).await,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                metrics::bus_overflow_dropped_total().inc_by(n as u64);
                metrics::bus_queue_depth().set(estimate_bus_depth());
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }
});
```

---

## 4) Host Interop: Mapping Events to Omni-Gate (GMI-1.6) / OAP/1

Although `ron-bus` is in-proc, **hosts** often mirror events to external observers or control planes. Recommended mappings:

| Event Variant               | Wire Surface (example)                                                           | Notes                                                             |
| --------------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `Health { service, ok }`    | HTTP(S) `/events/health` (JSON) or OAP/1 `EVENT` frame with `{ "service","ok" }` | Low cardinality; never include payload bodies.                    |
| `ConfigUpdated { version }` | HTTP(S) `/events/config` (JSON) or OAP/1 `EVENT` `{ "version" }`                 | Use monotonic `version`; consumers can idempotently reconcile.    |
| `ServiceCrashed { … }`      | HTTP(S) `/events/crash` (JSON) or OAP/1 `EVENT` `{ "service","reason" }`         | “reason” must be operator-safe (no secrets/PII).                  |
| `Shutdown`                  | Usually **not** propagated externally; internal control plane only               | Externalizing shutdown is optional and must be access-controlled. |

**Correlation:** Hosts attach `corr_id` and tenant metadata **in the wire envelope**, not to the `Event` itself.

**Security:** Do **not** serialize event payloads containing secrets. Keep wire DTOs audit-safe. **Hosts MUST validate capabilities (e.g., macaroons) and apply policy checks BEFORE mapping and emitting any bus-derived event externally.**

---

## 5) Canonical Test Scaffolds (Interop)

> Since this crate has no wire encoding, canonical **vectors** become **unit tests** that assert the mapping logic hosted elsewhere. We still provide scaffolds here so every host uses the same patterns.

### 5.1 Health Round-Trip (host test outline)

* **Given**: `Event::Health { service:"overlay", ok:true }`
* **When**: host maps to HTTP JSON
* **Then**: body equals

  ```json
  { "event":"health", "service":"overlay", "ok":true }
  ```

  and includes `corr_id` in HTTP header; no payload bodies or secrets appear.

### 5.2 ConfigUpdated Monotonicity

* **Given**: publish versions `[1, 2, 2, 3]`
* **Then**: external stream preserves ordering; consumers are idempotent if duplicates appear.

### 5.3 Crash Reason Redaction

* **Given**: `ServiceCrashed { reason:"disk quota exceeded on /var/data" }`
* **Then**: externalized reason must not contain secrets (e.g., no tokens); redaction in host enforced by test.

### 5.4 Shutdown Non-Export

* Assert that `Shutdown` is **not** surfaced to public endpoints unless the host explicitly opts in and requires admin capability.

### 5.5 External propagation during bus reload (host chaos)

* **Given:** steady publish load; ≥1 non-lagging subscriber; host mirrors events to HTTP/OAP stream.
* **When:** host creates a new `Bus` (different `capacity`), cuts subscribers over, and drops the old bus.
* **Then:** the external stream:

  * Contains **no gaps** for non-lagging consumers around the cutover window.
  * Shows **no artificial spike** in `overflow_dropped_total` attributable solely to migration.
  * Returns `bus_queue_depth` to baseline within T seconds after cutover.

---

## 6) Error & Edge Semantics (Interop-Relevant)

* **Lag/Overflow** (`RecvError::Lagged(n)`): purely in-proc. Hosts **must not** reflect this as an external failure, but may drive a **degraded readiness** mode or emit an operator event (non-customer facing).
* **Closed** (`RecvError::Closed`): end of stream for a subscriber; hosts should resubscribe or recreate `Bus` during reloads.

---

## 7) Interop Guarantees

* **No Kernel Drift:** `Event` semantics align with kernel docs; kernel may re-export `ron_bus::{Bus, Event}` without wrapper shims.
* **SemVer Discipline:**

  * Additive new `Event` variants → **minor**.
  * Changes to existing variants → **major**.
  * `Bus` surface remains **monomorphic** (no public generics).
* **Backwards Compatibility:** Unknown `Event` variants in host code must be handled via a catch-all `match` arm (forward compatible).
* **Auditability:** Host mapping tests live under `/tests/interop/` with fixtures; update when adding new variants.
* **Polyglot SDK guarantee:** The JSON wire schema for `WireEvent` is stable under SemVer; all official SDKs carry round-trip tests against `/tests/interop/vectors/`.

---

## 8) Reference Implementations (Host-Side Snippets)

### 8.1 HTTP JSON Emitter (host)

```rust
#[derive(serde::Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum WireEvent<'a> {
    Health { service: &'a str, ok: bool, corr_id: &'a str },
    ConfigUpdated { version: u64, corr_id: &'a str },
    ServiceCrashed { service: &'a str, reason: &'a str, corr_id: &'a str },
}

fn map_event_to_wire(ev: &ron_bus::Event, corr_id: &str) -> Option<WireEvent<'_>> {
    match ev {
        ron_bus::Event::Health { service, ok } => Some(WireEvent::Health { service, ok: *ok, corr_id }),
        ron_bus::Event::ConfigUpdated { version } => Some(WireEvent::ConfigUpdated { version: *version, corr_id }),
        ron_bus::Event::ServiceCrashed { service, reason } => {
            Some(WireEvent::ServiceCrashed { service, reason, corr_id })
        }
        ron_bus::Event::Shutdown => None, // typically internal-only
        _ => None, // forward-compatible: unknown variants not emitted unless explicitly supported
    }
}
```

### 8.2 OAP/1 Frame Wrapper (illustrative)

```rust
struct OapEvent<'a> {
    ver: u8,       // 1
    flags: u16,    // EVENT
    tenant_id: u128,
    corr_id: u64,
    payload_json: &'a [u8], // JSON of WireEvent
}
// The host is responsible for frame size caps and checksums per OAP/1 spec.
```

> Tip: If multiple hosts/SDKs need the same wire DTOs, consider centralizing them in a `ron-proto` crate (or shared JSON schema) to prevent drift.

---

## 9) Compatibility Matrix

| Change Type                                 | Bus Impact | Host Mapping Impact              | SemVer |
| ------------------------------------------- | ---------- | -------------------------------- | ------ |
| Add new `Event` variant                     | None       | Add new match arm + wire DTO     | Minor  |
| Add new field to existing variant           | Breaking   | Update mapping + external schema | Major  |
| Rename/remove existing variant              | Breaking   | Update mapping + external schema | Major  |
| Change `Bus` method signature               | Breaking   | None (host compile error)        | Major  |
| Add new host wire endpoint for existing evs | None       | Host-only change                 | Minor  |

---

## 10) References

* **Omni-Gate (GMI-1.6) Blueprint** — transport, envelopes, and readiness contracts (host level).
* **OAP/1 Spec** — framing and limits (host level).
* **ron-bus**: [`API.md`](./API.md), [`CONCURRENCY.md`](./CONCURRENCY.md), [`OBSERVABILITY.md`](./OBSERVABILITY.md), [`CONFIG.md`](./CONFIG.md).

---

✅ With this interop spec, `ron-bus` stays **pure and stable** in-proc, while hosts have **unambiguous, testable** mappings to external protocols. That prevents kernel drift, keeps SemVer honest, and gives auditors concrete fixtures to verify.

```

---
