````markdown
---
title: API Surface & SemVer Reference — svc-dht
status: draft
msrv: 1.80.0
last-updated: 2025-10-11
audience: contributors, auditors, API consumers
---

# API.md — svc-dht

## 0. Purpose

This document captures the **public API surface** of `svc-dht`:

- Snapshot of exported Rust symbols (functions, types, traits, modules) and **service endpoints**.
- **SemVer discipline:** what changes are additive vs. breaking.
- Alignment with `CHANGELOG.md` (behavioral vs. surface changes).
- CI-enforceable via `cargo public-api` and optional `cargo semver-checks`.
- Serves as the “spec” for external consumers: SDK authors, operators, auditors.

> `svc-dht` is a **service crate**. Its API includes:
> - Minimal Rust exports intended for wiring/testing.
> - **CLI** flags & exit codes.
> - **HTTP** endpoints (`/metrics`, `/healthz`, `/readyz`, `/version`).
> - **DHT RPCs** (wire-level) over `ron-transport`.

---

## 1. Public API Surface (Rust)

Generated via:

```bash
cargo public-api -p svc-dht --simplified --deny-changes
````

> The list below is a maintained snapshot. Update on any symbol change.
> Rust surface is intentionally **small**; internals remain crate-private.

### 1.1 Current Surface Snapshot (Rust)

```text
# Keep in sync with actual `cargo public-api --simplified` output

pub mod api            // wire/HTTP schema types (#[non_exhaustive])
pub mod config         // Config (public for integration/tests)
pub mod error          // DhtError (#[non_exhaustive])
pub mod metrics        // Metrics handle + constructors
pub mod version        // Build info for /version endpoint

pub use crate::config::Config;
pub use crate::error::DhtError;

pub struct Metrics;                              // opaque handle
pub fn run(cfg: Config) -> Result<(), DhtError>; // main runner
pub fn build_info() -> &'static version::BuildInfo;
```

### 1.2 API History Snapshots

* Stored at: `docs/api-history/svc-dht/<crate-version>.txt` (exact output of `cargo public-api --simplified`).
* CI posts a **diff** vs. previous snapshot on each PR (see §6).

---

## 2. Service API Surface (non-Rust)

### 2.1 CLI (stable surface)

```text
svc-dht [FLAGS] [--config <path>]

  --bind <ip:port>                     # HTTP/ingress bind
  --metrics <ip:port>                  # Prometheus bind
  --max-conns <num>
  --read-timeout <dur>                 # e.g., 5s, 250ms
  --write-timeout <dur>
  --idle-timeout <dur>

  --alpha <n> --beta <n> --k <n> --hop-budget <n>
  --rpc-timeout <dur> --hedge-after <dur>

  --bootstrap-required <n>
  --ready-bucket-fill <pct>
  --bootstrap-seed <host:port>         # repeatable

  --asn-max-pct <pct> --asn-entropy-min <bits>
  --he-stagger <dur>

  --pq-dual-sign | --no-pq-dual-sign
  --pq-require-pq-on <YYYY-MM-DD>
  --pq-alg <name>                      # repeatable

  --amnesia
  --profile <micronode|macronode>
  --log-format <json|text>
  --log-level <trace|debug|info|warn|error>
  --version
  --help
```

**CLI SemVer rules**

* **Additive**: new flags with safe defaults → **minor**.
* **Breaking**: rename/remove flags, or change default **semantics** → **major**.
* **Patch**: help text, aliases (keep old flag), non-behavioral defaults → **patch**.

**Exit codes**

* `0` success; `2` bad config/validation; `3` bind error; `4` bootstrap timeout (if `--strict-ready` enabled); `>10` reserved.

### 2.2 HTTP (operational endpoints)

| Method | Path       | Purpose            | Code(s) | Notes                                                    |
| -----: | ---------- | ------------------ | ------- | -------------------------------------------------------- |
|    GET | `/metrics` | Prometheus metrics | 200     | Bind on `metrics_addr`; **localhost in prod**            |
|    GET | `/healthz` | Liveness           | 200     | Handler always returns quickly                           |
|    GET | `/readyz`  | Readiness          | 200/503 | On 503 returns JSON with `missing[]`, `retry_after`      |
|    GET | `/version` | Build info         | 200     | `{ service, version, git, build_ts, rustc, features[] }` |

**HTTP SemVer rules**

* Adding **fields** to `/version` → **minor**.
* Changing field names/types → **major**.
* `/metrics` names in §5.1 are contractual (see **metrics-as-API**).

### 2.3 DHT RPCs (wire-level)

Kademlia-style RPCs over `ron-transport` (TLS/QUIC/TCP, optional Tor). **Protocol envelopes include** `proto_ver: u16` (current: `1`).

| Op           | Dir          | Request (sketch)                           | Response (sketch)                                       | Notes                           |
| ------------ | ------------ | ------------------------------------------ | ------------------------------------------------------- | ------------------------------- |
| `FIND_NODE`  | bi-dir       | `{ proto_ver:1, target_key:b3, corr_id }`  | `{ proto_ver:1, closest:[NodeInfo;≤k], corr_id }`       | Hop budget ≤ `hop_budget`       |
| `FIND_VALUE` | bi-dir       | `{ proto_ver:1, content_key:b3, corr_id }` | `{ proto_ver:1, value?:ProviderRecord, closest?:[..] }` | Value or closest nodes          |
| `PROVIDE`    | inbound      | `{ proto_ver:1, record:ProviderRecord }`   | `{ proto_ver:1, accepted:bool, reason? }`               | Verify signatures; TTL enforced |
| `STORE`      | inbound(opt) | `{ proto_ver:1, key, value, sigs[] }`      | `{ proto_ver:1, accepted:bool, reason? }`               | Feature-gated                   |

**ProviderRecord (stable sketch, non-normative)**

```json
{
  "proto_ver": 1,
  "key": "b3:<hex-32>",
  "publisher": "peer:<id>",
  "addrs": ["tcp://host:port","quic://host:port"],
  "ttl": 86400,
  "ts": 1731264000,
  "sigs": [
    {"alg":"ed25519","sig":"<base64>"},
    {"alg":"mldsa87","sig":"<base64>"}  // optional PQ during migration
  ]
}
```

**Wire SemVer rules**

* **Additive**: new **optional** fields → **minor**; older peers must ignore them.
* **Breaking**: incompatible wire changes bump **proto_ver**; must be **flag-gated** prior to defaulting.
* **Behavioral** tightening (e.g., require PQ after `require_pq_on`): **minor** if gated with compatibility mode; **major** if ungated.

**Protocol versioning policy**

* `proto_ver = 1` initial line. Additive changes keep major; incompatible changes bump to `2`, announced with a deprecation window (§3.4).

---

## 3. SemVer Discipline

### 3.1 Additive (Minor / Non-Breaking)

* New public functions/types/modules **behind non-default features**.
* Extending `#[non_exhaustive]` enums/structs with new variants/fields.
* Adding `/version` fields, **new metrics**, or **optional** RPC fields.

### 3.2 Breaking (Major)

* Remove/rename exports.
* Change function signatures, trait bounds, or error types.
* Make `#[non_exhaustive]` types exhaustive.
* Remove CLI flags or change their semantics non-compatibly.
* Change **required** wire fields or accept/reject logic **without** a gate.

### 3.3 Patch-Level

* Doc/test-only changes.
* Perf improvements with identical API/behavior.
* Metrics **bucket** tuning that does not break alert contracts (§5.2).

### 3.4 Behavioral Deprecation Window

For **behavioral tightenings** (e.g., stricter signature acceptance, PQ requirement):

* Announce in CHANGELOG and API.md.
* Emit WARN telemetry for **≥ 1 minor release** (or **≥ 90 days**) with counters/samples.
* Provide a config/feature flag for **compat mode** during the window.
* After the window, switch default to strict mode (may be **minor** if gated; **major** if ungated).

---

## 4. API Invariants

* **Service-first:** minimal Rust surface; internals (runtime/transport/buckets) are private.
* **No internal leakage:** do not expose `tokio`/transport concrete types unintentionally.
* **Non-exhaustive errors:** public error enums are `#[non_exhaustive]`.
* **Stable op names:** `FIND_NODE`, `FIND_VALUE`, `PROVIDE`, `STORE?` additions → minor; removals → major.
* **Metrics as API:** alert-coupled metric names/labels are **contractual** (§5).

---

## 5. Observability API (Contract)

### 5.1 Canonical Names (do not rename without deprecation)

* `dht_lookup_hops_histogram_bucket`
* `dht_lookup_latency_seconds`
* `rejected_total{reason}`
* `pq_dual_sign_ratio`
* `dht_bucket_asn_entropy_bits`
* `dht_ready_bucket_fill_pct`
* `tls_handshake_failures_total`

### 5.2 Alert Coupling (contractual)

* **Hop SLO:** p99 hops ≤ 5 (from `dht_lookup_hops_histogram_bucket`).
* **PQ Ratio:** `pq_dual_sign_ratio ≥ 0.95` during migration.
* **Diversity:** `avg_over_time(dht_bucket_asn_entropy_bits[15m]) ≥ 1.5`.
* **Quota:** `sum(rate(rejected_total{reason="quota"}[5m]))`.

**Metric deprecation policy**

* Provide a duplicate metric/label for **≥ 1 minor** (or **90 days**) with deprecation notice; migrate dashboards/alerts in the same PR.

---

## 6. Tooling & CI Gates

**Minimum tool versions**

* `cargo-public-api` **≥ 0.44.0**
* `cargo-semver-checks` **≥ 0.44.0**
* `rustc` **= 1.80.0** (MSRV)

**Commands**

```bash
cargo public-api -p svc-dht --simplified --deny-changes
cargo semver-checks check-release -p svc-dht
RUSTDOCFLAGS="--deny warnings" cargo doc -p svc-dht
```

**API history snapshots**

* On release: write `docs/api-history/svc-dht/<version>.txt` from `cargo public-api --simplified`.
* CI diff (PR comment):

```bash
git diff --no-index \
  docs/api-history/svc-dht/<prev>.txt \
  docs/api-history/svc-dht/<next>.txt || true
```

**Fail policy**

* Surface diff without matching CHANGELOG entry → **fail**.
* `--deny-changes` failure without `semver-ack` label → **fail**.

---

## 7. Behavior vs. Surface (CHANGELOG Mapping)

* **Surface**: list added/removed Rust symbols, flags, endpoints.
* **Behavior**: stricter policy/validation, SLO adjustments; include migration steps and deprecation windows (§3.4).
* **Observability**: metric/label changes + alert migration notes.

---

## 8. Acceptance Checklist (Definition of Done)

* [ ] Current `cargo public-api` snapshot stored at `docs/api-history/svc-dht/<version>.txt`.
* [ ] SemVer impact reviewed (minor/major/patch justified).
* [ ] CI gates pass (`cargo public-api`; optional `cargo semver-checks`).
* [ ] `CHANGELOG.md` updated (surface + behavior).
* [ ] Docs/tests updated for new/changed APIs.
* [ ] Service endpoints documented; payload examples updated.
* [ ] Metrics/alerts coupling reviewed; deprecations (if any) staged.

---

## 9. Appendix

### 9.1 References

* Rust SemVer: [https://doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
* cargo-public-api: [https://github.com/Enselic/cargo-public-api](https://github.com/Enselic/cargo-public-api)
* cargo-semver-checks: [https://github.com/obi1kenobi/cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)

### 9.2 Example `/version` payload

```json
{
  "service": "svc-dht",
  "version": "0.2.0",
  "git": "a1b2c3d (dirty: false)",
  "build_ts": "2025-10-11T14:23:11Z",
  "rustc": "1.80.0 (stable)",
  "features": ["pq","tls"]
}
```

### 9.3 Example `/readyz` 503 payload

```json
{
  "service": "svc-dht",
  "degraded": true,
  "missing": ["bootstrap_min_seeds","bucket_fill_pct"],
  "retry_after": 10
}
```

---

## 10. API Test Matrix (smoke/contract)

| Area    | Test                                           | Expectation                                                   |
| ------- | ---------------------------------------------- | ------------------------------------------------------------- |
| CLI     | `svc-dht --version`                            | JSON matches `/version` fields; exit 0                        |
| CLI     | unknown flag                                   | Non-zero exit; helpful usage on stderr                        |
| HTTP    | `/readyz` pre-bootstrap                        | `503` with `missing[]`; schema matches example                |
| HTTP    | `/version`                                     | Contains `service, version, git, build_ts, rustc, features[]` |
| Metrics | Registry                                       | Presence of contractual metrics (§5.1), correct types         |
| Wire    | `proto_ver=1` + unknown optional field         | Ignored by receiver without error                             |
| Wire    | Signature policy tightening in **compat mode** | WARN + accept during window; REJECT after window toggled      |
| SemVer  | `cargo public-api --deny-changes`              | No diffs unless CHANGELOG + `semver-ack` label present        |

---

✅ With this reference:

* The Rust surface stays **tight** and intentional.
* **CLI/HTTP/metrics/wire** are treated as API with SemVer expectations.
* CI enforces drift control, and the changelog tells consumers what changed and how to migrate.

```
```
