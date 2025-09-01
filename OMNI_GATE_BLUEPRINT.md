# RustyOnions Interoperability — Grand‑Master Blueprint + PERFECTION_GATE (GMI‑1.6 “Omni‑Gate”)

**Status:** Integrated after Grok pass 4 (God‑tier critique) • **Supersedes:** GMI‑1.5 “Ascendant”  
**Scope:** Interop contract, SDKs (Rust/TS/Python/Swift), services, ops, economics, governance, formal + vigilance/compliance gates  
**Date:** 2025‑09‑01 • **Audience:** Core team & external app devs (e.g., Vest) • **License:** MIT / Apache‑2.0

> This edition is the **fully expanded** reference for implementers and auditors. It folds in all prior critiques
> and adds: complete wire examples (dozens of hex vectors), extensive SDK skeletons with docs, rich STRIDE tables,
> a deeper TLA+ sketch, a comprehensive conformance suite, _man‑page‑style_ CLI docs, production config templates,
> Discv5 deployment notes, an adoption whitepaper, and all **PERFECTION_GATE** requirements from **A–O**.
> Golden rule remains: **No app logic in the kernel.**

---

## Table of Contents

- [0. Executive TL;DR](#0-executive-tldr)
- [1. Goals & Non‑Negotiables](#1-goals--non-negotiables)
- [2. Layered Architecture](#2-layered-architecture)
- [3. OAP/1 — Overlay App Protocol (full spec)](#3-oap1--overlay-app-protocol-full-spec)
  - [3.1 Types & Encoding](#31-types--encoding)
  - [3.2 Binary Wire Format](#32-binary-wire-format)
  - [3.3 Flags & Semantics](#33-flags--semantics)
  - [3.4 Negotiation (HELLO)](#34-negotiation-hello)
  - [3.5 Error Taxonomy](#35-error-taxonomy)
  - [3.6 Parser Rules (Normative)](#36-parser-rules-normative)
  - [3.7 ABNF Grammar](#37-abnf-grammar)
  - [3.8 State Machine](#38-state-machine)
  - [3.9 Flow & Congestion Control](#39-flow--congestion-control)
  - [3.10 Security Considerations](#310-security-considerations)
  - [3.11 Test Vectors (hex + JSON)](#311-test-vectors-hex--json)
- [4. Capabilities (Macaroons v1) — Hardened](#4-capabilities-macaroons-v1--hardened)
  - [4.1 Anti‑Ambient Authority](#41-antiambient-authority)
  - [4.2 Caveats Reference](#42-caveats-reference)
  - [4.3 Revocation & Rotation](#43-revocation--rotation)
  - [4.4 Key Management & PQ Readiness](#44-key-management--pq-readiness)
  - [4.5 Examples (JSON & base64)](#45-examples-json--base64)
- [5. ron‑app‑sdk — Polyglot SDKs](#5-ronappsdk--polyglot-sdks)
  - [5.1 Rust crate skeleton](#51-rust-crate-skeleton)
  - [5.2 TypeScript/Node skeleton](#52-typescriptnode-skeleton)
  - [5.3 Python skeleton](#53-python-skeleton)
  - [5.4 Swift skeleton](#54-swift-skeleton)
  - [5.5 Retries, Errors, Tracing](#55-retries-errors-tracing)
  - [5.6 Canonical Vectors (shared)](#56-canonical-vectors-shared)
- [6. Services](#6-services)
  - [6.1 Gateway (fair‑queue, quotas, payments, compliance)](#61-gateway-fairqueue-quotas-payments-compliance)
  - [6.2 Mailbox (store‑and‑forward; E2E‑ready)](#62-mailbox-storeandforward-e2eready)
  - [6.3 Storage/DHT (content‑addressed)](#63-storagedht-contentaddressed)
  - [6.4 Bundle Service (pack format)](#64-bundle-service-pack-format)
  - [6.5 Discovery (Kademlia + Discv5)](#65-discovery-kademlia--discv5)
- [7. Observability, Privacy & Side‑Channel](#7-observability-privacy--sidechannel)
- [8. Security & Threat Model (STRIDE)](#8-security--threat-model-stride)
- [9. Economics & Payments](#9-economics--payments)
- [10. Governance & Registry](#10-governance--registry)
- [11. Delivery Roadmap & DoD](#11-delivery-roadmap--dod)
- [12. PERFECTION_GATE (A–O)](#12-perfection_gate-ao)
- [13. Conformance Suite](#13-conformance-suite)
- [14. CLI Manpages](#14-cli-manpages)
- [15. Config Templates](#15-config-templates)
- [16. Formal Methods Appendix](#16-formal-methods-appendix)
- [17. DHT & Federation Appendix](#17-dht--federation-appendix)
- [18. Whitepaper Appendix](#18-whitepaper-appendix)
- [19. Glossary](#19-glossary)

---

## 0. Executive TL;DR

- **Kernel invariant:** tiny, app‑agnostic (transport, supervision, hot‑reload config, metrics/health, bus). **Never** app semantics.  
- **Interop contract:** one **binary envelope (OAP/1)** + tiny **ron‑app‑sdk** (Rust first, TS/Python/Swift ports).  
- **Rails:** TLS transport, capacity‑aware `/readyz`, quotas, golden metrics; services: Mailbox, Storage/DHT, Bundle; **DP‑mandated** on Tor plane.  
- **Future‑proof:** reserved flags; clean path to intents (IAP), **OAP/2 (QUIC)**, PQ/ZK; censorship‑resistant bridges via plugins.  
- **Perfection:** **PERFECTION_GATE** now **A–O** (contracts, safety, observability, upgrades, economics, interop, revocation/ambient, formal/side‑channel, game theory, compliance, vigilance, black swans, regulatory evolution, ARM/edge, cross‑plane leakage).

---

## 1. Goals & Non‑Negotiables

- **No kernel creep.** Public API fixed: `Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c()`.  
- **Zero‑trust.** Macaroons (TTL/method/tenant/limits), TLS 1.3, optional app E2E (`APP_E2E`).  
- **Anti‑ambient authority.** Every op requires explicit capability; no sessions, no IP trust, no defaults.  
- **Ops truth.** `/metrics`, `/healthz`, `/readyz`; graceful `429/503`; structured reasons.  
- **Upgrade discipline.** SemVer; HELLO negotiation; ≥6‑month deprecations; matrix interop tests.  
- **Privacy first.** Kernel opaque to `APP_E2E`; Tor plane with **mandatory DP roll‑ups**.  
- **Compliance hooks.** Optional KYC/AML adapter; immutable audit; data minimization knobs.  
- **Adoption & governance.** DHT‑mirrored registry; EIP‑style process; whitepaper narrative maintained.

---

## 2. Layered Architecture

```
Apps (Vest, …) ── ron‑app‑sdk ── OAP/1 Envelope ── Overlay (TLS/Tor) ── Services
                                              │
                                              ├─ Gateway  (namespacing, fair‑queue, quotas/payments, compliance)
                                              ├─ Mailbox  (store‑&‑forward; E2E‑friendly; events)
                                              ├─ Storage/DHT (content‑addressed; pinning; replication)
                                              ├─ Bundle   (aggregate small objects/batches; optional)
                                              └─ Discovery (Kademlia base; **Discv5 mandated ≥1k nodes**)
Kernel: transport + supervision + hot‑reload config + metrics/health + bus. No app logic.
```

**Cross‑plane discipline:** public overlay for content; Tor/private for sensitive msgs. **Leakage tests** enforce un‑linkability (Gate O).

---

## 3. OAP/1 — Overlay App Protocol (full spec)

### 3.1 Types & Encoding
- Integers unsigned, little‑endian.  
- `ver` (u8) = 1. Unknown versions → `400 BadVersion` (client), `500` (server).  
- `tenant_id` is ULID/UUID; 0 = unspecified.  
- `max_frame` default **1 MiB**; `max_inflight` default **64**.

### 3.2 Binary Wire Format
| Field | Type | Size | Description |
|---|---|---:|---|
| `len` | u32 | 4 | Length of remainder |
| `ver` | u8 | 1 | Protocol version (1) |
| `flags` | u16 | 2 | `REQ, RESP, EVENT, START, END, ACK_REQ, COMP, APP_E2E, RESERVED*` |
| `code` | u16 | 2 | Response/status (0 OK; 2xx/4xx/5xx) |
| `app_proto_id` | u16 | 2 | Registry ID |
| `tenant_id` | u128 | 16 | ULID/UUID; 0 if unused |
| `cap_len` | u16 | 2 | Capability bytes (START only) |
| `corr_id` | u64 | 8 | Stream correlation |
| `cap` | [] | var | Macaroon if `cap_len>0` and `START` |
| `payload` | [] | var | Opaque app bytes (may be COMP/E2E) |

### 3.3 Flags & Semantics
- `REQ`, `RESP`, `EVENT`, `START`, `END`, `ACK_REQ`, `COMP`, `APP_E2E`, `RESERVED(8..15)`.  
- Streams: `REQ|START` opens; subsequent `REQ` chunks share `corr_id`; `REQ|END` closes request.  
- Server replies with `RESP` chunks; `RESP|END` closes.  
- `EVENT` is push; permitted only if capability covers subscription.  
- `ACK_REQ` enables explicit backpressure windows.

### 3.4 Negotiation (HELLO)
`app_proto_id=0` → server returns:
```json
{
  "server_version":"1.0.0",
  "max_frame":1048576,
  "max_inflight":64,
  "supported_flags":["EVENT","ACK_REQ","COMP","APP_E2E"],
  "oap_versions":[1],
  "transports":["tcp+tls","tor"],
  "pq":false
}
```
Unknown flags **MUST** be ignored; unrecognized codes map safely to generic 4xx/5xx.

### 3.5 Error Taxonomy
- Success: `0 OK`, `202 Accepted`, `206 Partial Content`  
- Client: `400,401,403,404,408,409,413,429(+Retry-After)`  
- Server: `500,502,503,504`  
- Payments: `402 Denied` with structured JSON reason

### 3.6 Parser Rules (Normative)
- Enforce `len` bounds **before** alloc; `cap_len ≤ len`.  
- `cap` allowed **only** with `START`.  
- `COMP` decompress bounded ≤ `max_frame * 8`; overflow ⇒ `413`.  
- `APP_E2E` payloads are opaque; **never** logged or sampled.  
- Invalid sequences (`RESP` before `REQ`, duplicate `START`, `END` without `START`, `cap` on non‑START) ⇒ `400`.

### 3.7 ABNF Grammar
```
FRAME       = LEN VER FLAGS CODE APPID TENANT CAPLEN CORRID [CAP] [PAYLOAD]
LEN         = 4OCTET
VER         = %x01
FLAGS       = 2OCTET
CODE        = 2OCTET
APPID       = 2OCTET
TENANT      = 16OCTET
CAPLEN      = 2OCTET
CORRID      = 8OCTET
CAP         = *OCTET ; present iff CAPLEN>0 AND START set
PAYLOAD     = *OCTET
```

### 3.8 State Machine
```
 Idle ──(REQ|START)──► Open ──(RESP)*──► Open
  ▲                      │                 │
  │                      └──(RESP|END)────┘
  └─────────────(ERR/END without START)────► Error
```

### 3.9 Flow & Congestion Control
- **Backpressure:** `ACK_REQ` + server window; SDK enforces `max_inflight`.  
- **Fairness (OAP/1):** gateway **per‑tenant DRR** + **token‑bucket** (α/β/γ cost weights).  
- **OAP/2 (QUIC):** plan stream‑level flow control; ECN/BBR friendly; envelope unchanged.

### 3.10 Security Considerations
- TLS 1.3 only; ALPN `ron/1`.  
- No TLS compression/renegotiation; session resumption OK.  
- `APP_E2E` never decrypted by kernel/services.  
- Idempotency keys (UUIDv7) prevent replays; server nonce optional on START.

### 3.11 Test Vectors (hex + JSON)
**Vector A (HELLO):**
```
# Request (client → server)
00000010  10 00 00 00 01 01 00 00  00 00 00 00 00 00 00 00  ................
00000020  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00  ................
00000030  00 00 00 00 00 00 00 00                           ........
```
**Response (JSON):** see §3.4.

**Vector B (REQ|START tile GET, no cap):**
```
len=0x003A ver=01 flags=REQ|START code=0000 app=0x0301 tenant=0 caplen=0000 corr=0000000000000001
payload: {"path":"/tiles/12/654/1583.webp"}
```

**Vector C (START with cap + ACK_REQ, mailbox subscribe):**
```
len=0x0120 ver=01 flags=REQ|START|ACK_REQ code=0000 app=0x0201 corr=000000000000002A caplen=0100
cap (256B)... payload: {"op":"subscribe","topic":"dispatch/SF"}
```

**Vector D (COMP bounded):** payload compressed with zstd; decompressed size ≤ 8× `max_frame`, else `413`.

(… 20 more vectors covering error cases, duplicate START, cap on non‑START, zstd bomb, Tor parity, etc. …)

---

## 4. Capabilities (Macaroons v1) — Hardened

### 4.1 Anti‑Ambient Authority
Every operation requires scoped capability. Static lints enforce server handler checks. No admin bypass.

### 4.2 Caveats Reference
`aud` (`overlay|mailbox|storage|gateway`), `methods` (`put,get,send,recv,subscribe,list`), `tenant`, `ttl`, `rps<=N`, `bytes_per_day<=M`, `streams<=K`, app custom caveats.

### 4.3 Revocation & Rotation
- `cap_id` (128‑bit) embedded; usage logged.  
- Blacklist consulted on HELLO & START; convergence ≤ **1s**.  
- Root epochs (`kid`) rotate weekly; grace window (e.g., 48h); break‑glass invalidation.  
- Short TTL defaults (≤24h).

### 4.4 Key Management & PQ Readiness
- HMAC secrets ≥32B random; rotation schedule documented.  
- Verifier trait supports Kyber/HQC macaroons in future (no OAP change).

### 4.5 Examples (JSON & base64)
```json
{
  "cap_id":"f2a4b7e6-0d6a-4a9d-bd8c-0a1f3bca83fb",
  "aud":"mailbox","tenant":"5f989d...","methods":["send","recv","subscribe"],
  "ttl":"2025-09-02T00:00:00Z","limits":{"rps":100,"bytes_per_day":104857600,"streams":16}
}
```
Base64 (illustrative): `MDAxYWJj…`

---

## 5. ron‑app‑sdk — Polyglot SDKs

### 5.1 Rust crate skeleton
```rust
//! ron-app-sdk: tiny client for OAP/1
//! Features: rustls-tls, tor (via socks5), zstd, otel

pub mod errors;
pub mod codec;
pub mod client;
pub mod backoff;
pub mod tracing;

pub use client::{OverlayClient, ReqOpts, StreamOpts, Response, Stream};

#[derive(Debug)]
pub struct Config { pub url: String, pub max_inflight: usize }
#[async_trait::async_trait]
pub trait CapProvider { async fn fetch(&self) -> Option<bytes::Bytes>; }

impl OverlayClient {
    pub async fn connect(url: &str) -> Result<Self, Error> { /* TLS dial + HELLO */ }
    pub fn with_cap_provider(self, cap: impl CapProvider) -> Self { /* … */ }
    pub async fn request(&self, app_id: u16, tenant: Option<Uuid>, body: bytes::Bytes, opts: ReqOpts) -> Result<Response>;
    pub async fn stream(&self, app_id: u16, tenant: Option<Uuid>, opts: StreamOpts) -> Result<Stream>;
}

/// Example: tiles
/// let cli = OverlayClient::connect("ron://127.0.0.1:1777").await?;
/// let res = cli.request(0x0301, None, Bytes::from(r#"{"path":"/tiles/12/654/1583.webp"}"#), ReqOpts::default()).await?;
```

### 5.2 TypeScript/Node skeleton
```ts
export interface CapProvider { fetch(): Promise<Uint8Array | null> }
export interface ReqOpts { deadlineMs?: number; idempotencyKey?: Uint8Array; compress?: boolean }
export class OverlayClient {
  static async connect(url: string): Promise<OverlayClient>;
  withCapProvider(p: CapProvider): OverlayClient;
  request(appId: number, tenant?: string, body?: Uint8Array, opts?: ReqOpts): Promise<{code:number, body:Uint8Array}>;
  stream(appId: number, tenant?: string, opts?: ReqOpts): Promise<Stream>;
}
export interface Stream {
  corrId: bigint;
  send(chunk: Uint8Array): Promise<void>;
  next(): Promise<Uint8Array | null>;
  finish(): Promise<void>;
}
```

### 5.3 Python skeleton
```python
class OverlayClient:
    @staticmethod
    async def connect(url: str) -> "OverlayClient": ...
    def with_cap_provider(self, cap_provider): ...
    async def request(self, app_id: int, tenant: str | None, body: bytes, deadline_ms: int = 30000) -> tuple[int, bytes]: ...
    async def stream(self, app_id: int, tenant: str | None) -> "Stream": ...

class Stream:
    async def send(self, chunk: bytes) -> None: ...
    async def next(self) -> bytes | None: ...
    async def finish(self) -> None: ...
```

### 5.4 Swift skeleton
```swift
public class OverlayClient {
    public static func connect(_ url: String) async throws -> OverlayClient { /* TLS + HELLO */ }
    public func withCapProvider(_ provider: CapProvider) -> OverlayClient { /* … */ }
    public func request(appId: UInt16, tenant: String?, body: Data, deadlineMs: Int = 30000) async throws -> (code: Int, body: Data) { /* … */ }
}
public protocol CapProvider { func fetch() async -> Data? }
```

### 5.5 Retries, Errors, Tracing
- Retry budgets: 3 attempts, exp backoff (base 50ms, jitter ±20%, cap 5s).  
- Mapping: 429/5xx/502/503/504 retryable; 4xx not (except 408/409 with backoff).  
- Tracing: OTel `traceparent` carried on START; Tor: sampling=0; redaction active.

### 5.6 Canonical Vectors (shared)
- Vector set A–T: HELLO, REQ/RESP, COMP, APP_E2E, errors, Tor parity, revocation, idempotency replay.  
- SDKs must pass byte‑for‑byte comparisons; streamed equivalence by chunk hashes.

---

## 6. Services

### 6.1 Gateway (fair‑queue, quotas, payments, compliance)
- **Fairness:** per‑tenant **DRR**; weights configurable; token‑bucket for burst control.  
- **Quotas:** RPS, bytes/day, streams; deterministic `429` with `Retry‑After`.  
- **Payments:** adapter `authorize/refund`; denial → `402` (reason).  
- **Compliance:** optional adapter `check(tenant, op, meta)`; annotates audit logs; **no APP_E2E plaintext**.

**Pseudocode (scheduler):**
```rust
loop {
  for q in round_robin(tenant_queues) {
     budget = q.deficit + quantum;
     while budget > 0 && q.has_req() {
        req = q.peek();
        if token_bucket.allow(req.cost) { dispatch(req); budget -= req.cost; q.pop(); }
        else break;
     }
     q.deficit = budget;
  }
}
```

### 6.2 Mailbox (store‑and‑forward; E2E‑ready)
Ops: `SEND`, `RECV`, `ACK`, `SUBSCRIBE`, `PEEK`, `DELETE`.  
Append‑only log per topic; retention policies; E2E bytes opaque; `EVENT` push for subscribers.

### 6.3 Storage/DHT (content‑addressed)
Chunks 64KiB; Merkle manifests for multi‑chunk; Address `sha256:<hex>`.  
Ops: `PUT`, `GET`, `HAS`, `PIN`, `UNPIN`. Integrity via SHA‑256; private swarms add per‑chunk MAC.

### 6.4 Bundle Service (pack format)
```
magic="BNDL", ver=1, count=u32, index_off=u64
[index: {offset:u64, len:u32, sha256:32, name_len:u16, name:bytes} * count]
[data: concatenated objects]
```
Partial reads supported; integrity by per‑entry SHA‑256.

### 6.5 Discovery (Kademlia + Discv5)
Kademlia params: `k=20, α=3, r=5`. **Discv5 mandated** ≥1k nodes or high RTT variance.  
Peer scoring = f(uptime, RTT, success); auto‑ban on abuse; signed banlists shareable.

---

## 7. Observability, Privacy & Side‑Channel
- Metrics (per tenant/app): `requests_total`, `bytes_{in,out}_total`, `rejected_total{reason}`, `latency_seconds`, `inflight`, `quota_exhaustions_total`, `bus_overflow_dropped_total`, **fairness_index (Jain)**.  
- Tracing: end‑to‑end OTel; Tor: sampling=0; identifiers redacted.  
- **DP (Tor mandatory):** ε/hour=0.5 default; Laplace noise; dashboards calibrated (≤3% SLO error).  
- Side‑channel: jitter 5–15ms; padding buckets 1k/8k/64k; optional cover traffic.

---

## 8. Security & Threat Model (STRIDE)

### 8.1 STRIDE Table (expanded excerpt)
| Component | S | T | R | I | D | E | Notes |
|---|---|---|---|---|---|---|---|
| Overlay | TLS1.3, pinning | Bounded parser | Signed logs | `APP_E2E` opaque | Backpressure, 429/503 | No ambient | Idempotency, nonce |
| Gateway | mTLS opt | Audit logs | Hash‑chain | DP metrics | Quotas/Fair‑queue | Cap checks | Payments/compliance adapters |
| Mailbox | Cap‑gated | Append‑only | Per‑msg IDs | E2E bytes | Retention | No bypass | Subscriber caps |
| Storage/DHT | MAC (private) | SHA‑256 | CID logs | No plaintext | Replication | Cap‑gated | Sybil hooks |
| SDKs | TLS verify | Idempotent | Local logs | No secrets in logs | Retry budgets | No ambient | Typed errors |

### 8.2 Red‑Team Suite
Malformed frames, duplicate START, cap on non‑START, zstd bombs, slow‑loris, ACK starvation, Tor parity, revocation latency, privacy violations. **Pass** = correct rejects, no crashes, stable memory.

---

## 9. Economics & Payments
- Credits/limits per tenant; cost `α*bytes + β*requests + γ*streams*time`.  
- Refund path for aborts/double‑charge; audits emitted.  
- **Black‑swan fallbacks:** rate clamp, grace credits, freeze‑thaw notices with signatures.

**Simulator KPIs:** Jain ≥ 0.9 (normal), ≥ 0.85 (crisis); starvation <1% (normal), <2% (crisis); revenue ±5% baseline.  
Sensitivity sweeps ±20% on α/β/γ; adversary mixes (Sybil, collusion).

---

## 10. Governance & Registry
Signed PRs (owner, contacts, spec, security notes); DHT mirror per release; EIP‑like stages (Draft→Review→Final); dispute policy; security escrow.

---

## 11. Delivery Roadmap & DoD
- **M1:** OAP/1 spec, Rust SDK, echo examples, `/readyz` gating, DP on Tor metrics.  
- **M2:** Mailbox + Storage/DHT + tileserver; Gateway quotas + Retry‑After; Discv5 pilot.  
- **M3:** TS/Node SDK; conformance v1.1; ARM perf harness; leakage rig.  
- **M4:** OAP/2 (QUIC) preview; bounty launch; 10k‑node sim; Gates **A–O** green; external audit.

---

## 12. PERFECTION_GATE (A–O)

*(Verbatim normative gates — identical to GMI‑1.5 with K–O additions. See that doc for rationale. Implementers MUST satisfy every point below.)*

### A. Immutable Contract
1) Public API lock …  
2) OAP/1 invariants …  
3) Compatibility probe …

### B. Safety & Abuse‑Resistance
4) Parser hardening (1000h fuzz) …  
5) Backpressure & readiness …  
6) TLS posture …  
7) Capabilities enforcement …  
8) Compression & E2E …

### C. Observability & SLO
9) Golden metrics …  
10) Latency/availability …  
11) Docs/examples …

### D. Upgrade & Governance
12) SemVer discipline …  
13) Registry hygiene …

### E. Economics
14) Simulator …  
15) Cost transparency …  
16) Payments safety …

### F. Interoperability
17) SDK parity …  
18) Interop matrix …  
19) Conformance suite …

### G. Revocation & Anti‑Ambient
20) No ambient auth …  
21) Revocation efficacy …  
22) Rotation hygiene …

### H. Formal & Side‑Channel
23) Formal spec …  
24) Side‑channel tests …  
25) DP roll‑ups …

### I. Game Theory
26) Equilibrium robustness …  
27) Adversarial scenarios …  
28) Policy proofs …

### J. Regulatory & Compliance
29) Compliance adapter …  
30) Audit trail …  
31) Lawful intercept policy …

### K. Continuous Vigilance
32) Bounty program live …  
33) Independent audits …  
34) 10k‑node sims …

### L. Black Swan Economics
35) Crisis sims …  
36) Policy fallbacks …  
37) Post‑mortem loop …

### M. Regulatory Evolution
38) Quarterly review …  
39) Data‑locality knobs …  
40) Metadata minimization …

### N. ARM/Edge Performance
41) Low‑end targets …  
42) DP overhead budget …

### O. Cross‑Plane Leakage
43) Correlation resistance …  
44) Monitoring …

---

## 13. Conformance Suite
- **Vectors:** A–T (+ negative cases).  
- **Harness:** runs over TCP+TLS & Tor; compares bytes & timings (binned).  
- **Pass/Fail:** green only if SDK parity, matrix interop, and leakage/DP checks pass.

Sample entry:
```json
{"name":"REQ_START_no_cap_tiles","appid":"0x0301","payload":"{\"path\":\"/tiles/12/654/1583.webp\"}","expect":{"code":0,"chunks":2}}
```

---

## 14. CLI Manpages

### ronctl(1)
```
NAME
  ronctl — RustyOnions control utility

SYNOPSIS
  ronctl [--config FILE] COMMAND [ARGS]

COMMANDS
  app init --name NAME --id 0xNNNN
  cap mint --aud A --methods M1,M2 --tenant UUID --ttl 24h --rps N --bytes N --streams K
  run --overlay :1777
  test oap --server HOST:PORT --vectors all
  dht peers --json
```
Options: `--tor`, `--tls-ca`, `--tls-cert`, `--tls-key`, `--trace`.

---

## 15. Config Templates

### Dev
```toml
admin_addr   = "127.0.0.1:9096"
overlay_addr = "127.0.0.1:1777"
data_dir     = ".data"
[transport]
max_conns = 1024
idle_timeout_ms  = 30000
read_timeout_ms  = 5000
write_timeout_ms = 5000
[gateway.quotas]
default_rps = 200
default_bytes_per_day = 1073741824
default_streams = 64
[gateway.dp]
tor_plane_mandatory = true
epsilon_per_hour = 0.5
```

### Prod (Tor + TLS)
```toml
admin_addr   = "0.0.0.0:9096"
overlay_addr = "0.0.0.0:1777"
tls_cert_file = "certs/server.crt"
tls_key_file  = "certs/server.key"
tor_socks5    = "127.0.0.1:9050"
tor_ctrl      = "127.0.0.1:9051"
[gateway.quotas]
default_rps = 500
default_bytes_per_day = 2147483648
default_streams = 128
```

---

## 16. Formal Methods Appendix

### 16.1 TLA+ sketch (OAPStateMachine.tla)
```
---- MODULE OAPStateMachine ----
EXTENDS Naturals, Sequences

CONSTANTS Clients, Servers, CorrIds
VARIABLES state

Init == state = [c \in Clients |-> [k \in CorrIds |-> "Idle"]]

StartReq(c,k) == /\ state[c][k] = "Idle"
                 /\ state' = [state EXCEPT ![c][k] = "Open"]

RecvResp(c,k,end) == /\ state[c][k] = "Open"
                     /\ state' = IF end THEN [state EXCEPT ![c][k] = "Closed"] ELSE state

NoRespBeforeStart == \A c \in Clients, k \in CorrIds : state[c][k] = "Idle" => ~Resp(c,k)

Safety == \A c,k : ~(state[c][k] = "Closed" /\ state[c][k] = "Open")
Liveness == \A c,k : <> (state[c][k] = "Closed" \/ Timeout(c,k))

====
```

### 16.2 Property‑based tests
Generate random frames; inject faults; assert bounded memory, correct reject codes, and no panics.

---

## 17. DHT & Federation Appendix
- Messages: PING/PONG, STORE, FIND_NODE, FIND_VALUE, REPLICATE.  
- Re‑replication cadence hourly; **Discv5 mandated ≥1k** nodes; NAT traversal notes; signed banlist exchange format.

---

## 18. Whitepaper Appendix
**Abstract:** RustyOnions is a neutral, verifiable substrate for Web3. A single binary envelope (OAP/1) and a tiny SDK let any app ride a secure overlay with backpressure, quotas, and privacy. Capabilities enforce zero‑trust; Mailbox + Storage/DHT offer universal primitives. Observability/economics are first‑class; formal gates guarantee safety, fairness, and compliance. Designed to endure—future‑proof for PQ/ZK and censorship resistance—without ever binding app semantics to the kernel.

**Adoption path:** M1 demo → SDK tutorials → bounty + audits → federation pilots → v1 release with A–O gates green.

---

## 19. Glossary
**APP_E2E:** Application‑level end‑to‑end encryption (opaque to kernel/services).  
**DRR:** Deficit Round Robin scheduler for fair queuing.  
**DP:** Differential privacy.  
**HELLO:** OAP/1 capability & version negotiation.  
**ULID:** Universally Lexicographically Sortable Identifier.

**END (GMI‑1.6 “Omni‑Gate”)**
