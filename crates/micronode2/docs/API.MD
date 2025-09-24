
---

# 📖 API.md — micronode

---

title: API Surface & SemVer Reference
status: draft
msrv: 1.80.0
last-updated: 2025-09-23
audience: contributors, auditors, API consumers
-----------------------------------------------

## 0. Purpose

This document captures the **public API surface** of `micronode`:

* Snapshot of exported functions, types, traits, modules.
* SemVer discipline: what changes break vs. extend.
* Alignment with CHANGELOG.md (behavioral vs. surface changes).
* CI-enforceable via `cargo public-api` (and semver checks).
* Acts as the **spec** for external consumers of this crate.

**Profile**: `micronode` is the **DX-lean, single-tenant** node profile with **amnesia ON by default**. It exposes the standard admin surfaces (`/metrics`, `/healthz`, `/readyz`, `/version`) plus a trimmed **object** and **(feature-gated) mailbox** surface. Global limits are strict and uniform (timeouts 5s, inflight 512, RPS 500, body 1 MiB, decompress ≤10×; OAP frame 1 MiB; streaming chunk 64 KiB).

---

## 1. Public API Surface

Generate an authoritative snapshot in CI:

```bash
cargo public-api --simplified --deny-changes > docs/api-history/micronode/${GIT_TAG}.txt
```

### 1.1 Rust (crate) surface

`micronode` is a **service (bin)**. It does **not** expose a stable library API beyond its CLI entry and configuration glue. Keep public items minimal to avoid type leakage.

> **Expected public-api snapshot (illustrative; keep CI-generated):**

```text
pub fn main() -> ()
pub mod cli
```

### 1.2 HTTP (canonical service surface)

| Path        | Method | Purpose                                     | Auth                             | Limits                              | Status / Error Taxonomy                                                |
| ----------- | ------ | ------------------------------------------- | -------------------------------- | ----------------------------------- | ---------------------------------------------------------------------- |
| `/metrics`  | GET    | Prometheus scrape                           | none                             | n/a                                 | 200                                                                    |
| `/healthz`  | GET    | Liveness                                    | none                             | n/a                                 | 200                                                                    |
| `/readyz`   | GET    | Readiness                                   | none                             | n/a                                 | 200 when ready; 503 degraded JSON (`{degraded,missing[],retry_after}`) |
| `/version`  | GET    | Build/version                               | none                             | n/a                                 | 200                                                                    |
| `/o/{addr}` | GET    | **OBJ\_GET** by BLAKE3 address (`b3:<hex>`) | macaroon (short TTL) or dev mode | stream in 64 KiB chunks             | 200, 404 NotFound, 502 IntegrityFail, 503 NotReady                     |
| `/put`      | POST   | **OBJ\_PUT** (simple form)                  | macaroon                         | **1 MiB** body cap; decompress ≤10× | 201 Created, 413 FrameTooLarge, 429 QuotaExceeded, 503 NotReady        |

**Readiness keys** (non-exhaustive): `bus_attached`, `db_open` (if persistence enabled), `tor_bootstrap`/`hs_published` (if onion transport/profile enabled).
**Semantics**: **fail-open reads** / **fail-closed writes** during degradation.

#### 1.2.1 OpenAPI outline (kept in-sync via CI)

Create `/docs/openapi/micronode.yaml` and keep it authoritative. Skeleton:

```yaml
openapi: 3.1.0
info: { title: Micronode API, version: 0.1.0 }
paths:
  /metrics:
    get: { summary: Prometheus metrics, responses: { '200': { description: OK } } }
  /healthz:
    get: { summary: Liveness, responses: { '200': { description: OK } } }
  /readyz:
    get:
      summary: Readiness
      responses:
        '200': { description: Ready }
        '503': { description: Degraded, content: { application/json: { schema: { $ref: '#/components/schemas/Degraded' } } } }
  /version:
    get: { summary: Version, responses: { '200': { description: OK } } }
  /o/{addr}:
    get:
      summary: Get object by BLAKE3 address
      parameters: [{ name: addr, in: path, required: true, schema: { type: string, pattern: '^b3:[0-9a-f]{64}$' } }]
      responses:
        '200': { description: OK, content: { application/octet-stream: {} } }
        '404': { description: NotFound }
        '502': { description: IntegrityFail }
        '503': { description: NotReady }
  /put:
    post:
      summary: Put object (simple)
      requestBody: { required: true, content: { application/octet-stream: { schema: { type: string, format: binary, maxLength: 1048576 } } } }
      responses:
        '201': { description: Created }
        '413': { description: FrameTooLarge }
        '429': { description: QuotaExceeded }
        '503': { description: NotReady }
components:
  schemas:
    Degraded:
      type: object
      required: [degraded, missing]
      properties:
        degraded: { type: boolean }
        missing: { type: array, items: { type: string } }
        retry_after: { type: integer, minimum: 0 }
```

### 1.3 Mailbox surface (feature-gated)

When `mailbox` feature is enabled, expose a **single-tenant** minimal queue:

| Path         | Method | Purpose           | Notes                                      |
| ------------ | ------ | ----------------- | ------------------------------------------ |
| `/mbox/send` | POST   | enqueue message   | requires `Idempotency-Key` header          |
| `/mbox/recv` | POST   | lease one message | request may include `visibility_timeout_s` |
| `/mbox/ack`  | POST   | ack message       | DLQ after N retries; poison quarantined    |

**Facet gating**: mailbox backs **Feed fanout** and **Graph change events** in the Developer Suite; enabling it must not alter object endpoints’ semantics. Metrics: `mailbox_visibility_timeout_total`, `mailbox_dlq_total`. Errors: 400 (schema), 404 (missing/late ack), 409 (duplicate idempotency), 429 (quota).

---

## 2. SemVer Discipline

### 2.1 Additive (Minor / Non-Breaking)

* New endpoints under **new** paths.
* New optional response fields (with `#[serde(default)]`).
* New feature flags (`arti`, `pq-hybrid`, `mailbox`) default-off.
* Extend enums/objects marked `#[non_exhaustive]`.

### 2.2 Breaking (Major)

* Remove/rename endpoints or change HTTP methods.
* Change status codes or error taxonomy of existing routes.
* Tighten global **limits** below canon (e.g., cap < 1 MiB) or change OAP framing rules.
* Remove/alter admin surfaces (`/metrics`, `/healthz`, `/readyz`, `/version`).
* Rust: remove/rename public symbols; signature/return type changes.

### 2.3 Patch

* Pure doc fixes; internal improvements with no externally observable change; non-semantic log copy changes.

---

## 3. Stability Guarantees

* **MSRV**: 1.80.0.
* **Unsafe**: forbidden without an explicit safety section.
* **No type leakage**: internal transports/sockets/pools are private; no surprise re-exports.
* **Error taxonomy is contract** (HTTP):

  * `413 FrameTooLarge`, `429 QuotaExceeded`, `503 NotReady`, `404 NotFound`, `502 IntegrityFail`.

---

## 4. Invariants

* **Profile**: single-tenant, **amnesia-first** (RAM only by default; no disk spill). Any change here is **major**.
* **Admin surfaces**: `/metrics`, `/healthz`, `/readyz`, `/version` are mandatory.
* **Limits** (apply to **all ingress**): **5s** timeout, **512** inflight, **500 rps**, **1 MiB** body, **≤10×** decompress.
* **OAP**: non-stream frames **≤1 MiB**; streaming chunks **64 KiB**.
* **Integrity**: strict BLAKE3 verification on reads; corrupted chunk → **502** and `integrity_fail_total`++.
* **DTO hygiene**: `#[serde(deny_unknown_fields)]` with schema-compat tests in CI.

---

## 5. Tooling

* **cargo public-api** → detect surface diffs; snapshot to `/docs/api-history/micronode/{version}.txt`.
* **cargo semver-checks** (optional) → semver compliance.
* **cargo doc** with doctests for HTTP/OAP usage and error taxonomy.
* **Self-test container** (used in CI and locally):

  1. Wait for `/readyz` = 200.
  2. `curl -X POST --data-binary @2MiB.bin /put` → expect **413**.
  3. Scrape `/metrics` → `request_latency_seconds` present; golden labels exist (`service`, `route`, `method`, `amnesia`).

---

## 6. CI & Gates

* PR runs `cargo public-api`; **fail** on unacknowledged breaking diffs (bot posts symbol diff).
* Hardening acceptance: admin surfaces exist; limits enforced; DTO guard on; **amnesia honored**.
* Feature matrix build/test: `arti` on/off; `amnesia` on/off; `mailbox` on/off.
* OpenAPI sync check: CI verifies `/docs/openapi/micronode.yaml` matches implemented routes.

---

## 7. Acceptance Checklist (DoD)

* [ ] API snapshot generated & stored under `/docs/api-history/micronode/{version}.txt`.
* [ ] SemVer discipline reviewed (additive/breaking/patch).
* [ ] CI gate passes (`cargo public-api`).
* [ ] CHANGELOG updated for any surface or behavior changes.
* [ ] Docs/tests updated (HTTP examples, OAP streaming).
* [ ] `/docs/openapi/micronode.yaml` updated & validated.

---

## 8. OAP Streaming Examples (DX)

### 8.1 Rust (chunked PUT, 64 KiB frames)

```rust
use std::io::{Read};
use std::fs::File;
use std::time::Duration;
use reqwest::blocking::Client;

fn put_streamed(path: &str, url: &str, macaroon: &str) -> anyhow::Result<()> {
    let mut f = File::open(path)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // 64 KiB streaming chunks (do not exceed OAP non-stream 1 MiB cap per frame)
    let mut buf = vec![0u8; 64 * 1024];
    let mut body: Vec<u8> = Vec::new();
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        body.extend_from_slice(&buf[..n]);
        // In a real OAP client, you'd frame & send incrementally; this example
        // mirrors chunk sizing discipline while using plain HTTP POST for brevity.
    }

    let resp = client
        .post(format!("{}/put", url))
        .header("Authorization", format!("Macro {}", macaroon))
        .body(body)
        .send()?;

    match resp.status().as_u16() {
        201 => Ok(()),
        413 => anyhow::bail!("FrameTooLarge: split into smaller chunks"),
        429 => anyhow::bail!("QuotaExceeded"),
        503 => anyhow::bail!("NotReady"),
        s   => anyhow::bail!("Unexpected status {}", s),
    }
}
```

> For full OAP, send framed messages (`HELLO`, `START`, `DATA` 64 KiB, `END`). The rule to remember: **never** exceed 1 MiB per non-stream frame; prefer chunked DATA frames.

### 8.2 curl (simple PUT cap demonstration)

```bash
# OK: 1 MiB (max) body
head -c 1048576 /dev/zero | curl -sS -X POST --data-binary @- http://localhost:8080/put -H "Authorization: Macro <token>" -i

# Rejected: >1 MiB body → 413 FrameTooLarge
head -c 1048577 /dev/zero | curl -sS -X POST --data-binary @- http://localhost:8080/put -H "Authorization: Macro <token>" -i
```

---

## 9. Roadmap (non-normative; informs SemVer)

* **PQ-hybrid defaults**: phase toward X25519+Kyber (KEM) and Ed25519+Dilithium (signatures) as defaults; ensure capability verification remains backward-compatible (minor) and flips only via a major release.
* **ZK hooks**: evaluate zero-knowledge attestations for capability proofs/ledger interop; introduce behind a feature (additive).
* **Facet breadth**: expand mailbox-backed Feed/Graph helpers (SDK-side), keeping server API stable; add only **new** endpoints for new flows (minor).

---

## 10. Appendix

**References**

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

**Perfection Gates**

* **Gate G**: No undocumented API surface.
* **Gate H**: Breaking changes require **major** bump + migration notes.
* **Gate J**: CHANGELOG alignment required for any surface/behavior change.

**History** (maintain)

* 0.1.0: Initial admin + object + (gated) mailbox surfaces; limits codified; OpenAPI stub added.

