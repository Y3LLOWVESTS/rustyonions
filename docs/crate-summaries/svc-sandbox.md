Short answer: yes—it’s clever, and a great fit for RustyOnions. What you’re describing is **deception** (honeypot/honeynet). Done right, it buys time, soaks attacker effort, and gives you TTPs/IOCs—without risking your real plane.

Below is a tight, actionable blueprint for a new crate that does exactly this, plus the guardrails to make it safe.

# QRD: Why a sandbox crate is worth it

* **Soaks & signals:** Diverts bad actors into a realistic decoy, gathers indicators, and triggers auto-defenses.
* **Buys safety:** Keeps them away from real data/services while you watch them poke at fake ones.
* **Low coupling:** Perfect for our microkernel: it looks like the real ingress/overlay/storage, but is *physically isolated* and **never** touches the real bus or DB.

# New crate proposal: `svc-sandbox` (deception service)

---

crate: svc-sandbox
path: crates/svc-sandbox
role: service
owner: Stevan White
maturity: draft
last-reviewed: 2025-09-14
-------------------------

## 1) One-liner

A deception/decoy service that mimics RON endpoints and assets to safely trap, study, and throttle attackers.

## 2) Primary Responsibilities

* Present **realistic, instrumented** versions of ingress/index/storage APIs with fake data.
* **Steer suspicious sessions** from ingress into the sandbox (sticky diversion) and **tarpit** as needed.
* Emit **high-fidelity telemetry** (TTPs/IOCs) to block/learn without exposing production systems.

## 3) Non-Goals

* No access to real index/storage/bus; no keys or secrets; no outbound network beyond telemetry.
* No “hack back.” Purely defensive deception.

## 4) Public API Surface

* HTTP: mimic a subset of `svc-omnigate` (e.g., `/oap/v1/*`, `/assets/:id`), and health/metrics endpoints.
* Optional OAP/1 handshake **stub** that enforces our frame/size limits and streams fake chunks.
* Events: publish `sandbox.session_started/ended`, `sandbox.ioc` to the **telemetry bus only** (one-way).

## 5) Dependencies & Coupling

* Internal: none (no production bus/DB deps). Optional `ron-proto` for DTOs (loose; replaceable: yes).
* External: `axum`, `tokio`, `prometheus`, `rand`, `serde`. Low risk.
* Runtime: **No DB**, **no KMS**. Local ephemeral store only (in-mem or tempdir).

## 6) Config & Feature Flags

* `SANDBOX_ADDR` (bind), `SANDBOX_MODE=redirect|mirror|tarpit`, `SANDBOX_MAX_BODY=1MiB`, `SANDBOX_TARPIT_MS=250..2000`.
* `mirror-mode` (copy requests to recorder but don’t answer), `redirect-mode` (we answer, prod doesn’t).
* `decoy-seed` to deterministically generate assets/honeytokens.

## 7) Observability

* Metrics:

  * `sandbox_sessions_total{mode}`, `sandbox_requests_total{route,method,status}`
  * `sandbox_latency_seconds{route}`, `tarpit_ms_histogram`, `honeytoken_trips_total{token}`
* Logs: request/response summaries, decoded OAP headers (never secrets).
* Health: `/healthz`, `/readyz`; `/metrics`.

## 8) Concurrency Model

* Async axum server; **hardening layer** (timeouts, rate-limit, body limit).
* Optional **tarpit** mode introduces jittered sleeps; backpressure via concurrency limit.

## 9) Persistence & Data Model

* **Ephemeral** in-memory catalog of decoy assets: manifests, chunks, indexes.
* **Honeytokens** embedded in manifests/URLs (unique per session) for exfil detection.
* No durable writes; rotate tempdir on start.

## 10) Errors & Security

* Reject oversize frames/bodies; enforce chunking; decompression ratio cap.
* Socket/HTTP permissions as in DoH v1.1; **no UDS exposure to prod**.
* Egress policy: deny all except telemetry sink (if used).

## 11) Performance Notes

* Realistic latency shaping (p50/p95 distributions) to avoid easy detection.
* Tarpit adds bounded delay (25–2000 ms) to slow scans.

## 12) Tests

* Unit: token trip logic, tarpit bounds, frame-limit rejections.
* Integration: diversion stickiness, metrics presence, honeytoken exfil alerts.
* Fuzz: OAP parser stubs (oversize/malformed frames).

## 13) Improvement Opportunities

* Session replayer for blue-team drills.
* Canary bundles with rotating decoy “secrets.”
* Automatic IOC export to `ron-policy` to block at ingress.

## 14) Change Log (recent)

* 2025-09-14 — Draft spec.

## 15) Readiness Score (0–5 each)

* API clarity: 4
* Test coverage: 1
* Observability: 4
* Config hygiene: 3
* Security posture: 4
* Performance confidence: 3
* Coupling (lower is better): 5

---

# How we route traffic into the sandbox

**Ingress (svc-omnigate) adds a “deception router”:**

* **Triggers** (any → sandbox):

  * Honeytoken hit, invalid auth + suspicious UA, rapid-fire probes, malformed OAP frames, decoy hostnames, ASN/IP denylist, or ML anomaly flag.
* **Sticky mapping**: once flagged, keep 5-tuple/device-fingerprint in sandbox for session duration.
* **Modes**:

  * `redirect`: only sandbox answers (preferred).
  * `mirror`: copy traffic to sandbox recorder while prod still answers (useful for tuning).
  * `tarpit`: add jittered delays to slow automated probing.

**Safety valve:** If any sandbox policy would allow egress or internal reach, **drop** instead. The sandbox must be a *dead-end*.

---

# Isolation guardrails (non-negotiable)

* Run `svc-sandbox` in its own **network namespace**; **no egress** except to the telemetry collector.
* **Read-only root**, **seccomp default**, **no caps**, **non-root** user.
* No mounting of prod volumes; no prod UDS sockets; no access to `ron-bus`.
* Rotate/seed decoy assets at boot; never reuse real keys/certs.

---

# Deception content (credible but fake)

* Generate decoy bundles that look like real ones (correct BLAKE3 formatting, plausible sizes).
* Embed **honeytokens** (unique URLs/IDs). If they ever appear outside, you’ve positively identified exfil.
* Shape latencies & error codes to match prod distributions (avoid giveaway patterns).

---

# Telemetry & auto-defense

* Emit `sandbox.ioc` events (source IP, UA, path, token id, signatures) to a **telemetry sink** or a separate, append-only bus.
* Feed IOCs into `ron-policy` to **auto-rate-limit/deny** at ingress for future requests.
* Dashboard: sessions, requests, token trips, tarpit time, geos/ASNs.

---

# Risks & how we neutralize them

* **Detection by skilled attackers** → Use realistic catalogs, latency/error shaping, rotate tokens.
* **Pivot risk** → No egress, no prod sockets, seccomp + RO root.
* **Resource drain** → Concurrency/rate caps, tarpit jitter, and hard timeouts (part of our hardening layer).
* **Legal/ethics** → Pure defense; no counter-attacks; log terms in internal policy.

---

# Minimal skeleton (so you can drop it in later if you want)

```
crates/svc-sandbox/
├─ Cargo.toml
└─ src/
   ├─ main.rs        // axum server + hardening layer + routes
   ├─ decoy.rs       // decoy asset catalog + honeytoken generator
   ├─ router.rs      // diversion logic (headers, IP, UA, anomalies)
   ├─ oap_stub.rs    // strict OAP/1 frame/size checks + streaming stub
   ├─ metrics.rs     // prometheus registries + helpers
   └─ tarpit.rs      // jittered sleeps & rate shaping
```
