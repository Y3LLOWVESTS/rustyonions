# 🔗 INTEROP.md — ron-policy

*Audience: developers, auditors, external SDK authors*
*msrv: 1.80.0*

---

## 0) Purpose

Define the **interop surface** of `ron-policy`:

* **Wire contracts** for policy bundles (canonical JSON; TOML for authoring).
* **DTOs & JSON Schema** (stable field names/types).
* **Bus topics & event payloads** surrounding policy rollout.
* **Canonical test vectors & behaviors** (round-trip, content addressing).
* **Platform invariants** this crate validates (e.g., OAP/1 body/ratio caps).

`ron-policy` is a **pure library**. It performs no network I/O. All transport, endpoints, and storage sit in **consuming services** (e.g., `svc-registry`, `svc-gateway`, `svc-storage`). This doc is the **contract** that binds those services and external SDKs to the library’s expectations.

---

## 1) Protocols & Endpoints

**Library:** *no* network endpoints.
**Relevant platform invariants (validated by `ron-policy`):**

* **OAP/1 max_frame/body:** `1 MiB` (policy cannot relax this upper bound).
* **Decompression ratio guard:** `≤ 10×` (tighten-only).
* **Canonical wire for bundles:** **JSON** (UTF-8, stable key names).
* **Authoring convenience:** **TOML** accepted by tooling/tests; converted to canonical JSON prior to hashing.

**Service touchpoints (informative):**

* **Ingress protocols (services):** HTTP/1.1 + TLS (tokio-rustls), OAP/1 framed, gRPC/QUIC (where applicable).
* **Policy distribution:** `svc-registry` publishes `PolicyChanged` events on the bus (see §3). Services fetch the referenced bundle (over mutually-auth TLS), validate with `ron-policy`, then **atomically swap** the snapshot used for request evaluation.

---

## 2) DTOs / Schemas

### 2.1 Canonical DTOs (Rust → JSON)

```rust
// ron_policy::model (summary)
pub struct PolicyBundle {
  pub version: u64,
  pub id_b3: String,                  // "b3:<hex>" over canonical JSON bytes
  pub issued_at_epoch_ms: u64,
  pub residency: Residency,
  pub quotas: Vec<QuotaRule>,
  pub routes: Vec<RouteRule>,
  pub features: Features,
  pub metadata: Meta,
}

pub struct Residency {
  pub allowed_regions: Vec<String>,
  pub required_regions: Vec<String>,
  pub deny_regions: Vec<String>,
  pub placement: PlacementPrefs,      // { prefer_rtt_ms, hedge_local, hedge_remote }
}

pub enum Scope { Global, PerCap, PerPeer, PerRoute }
pub enum AppliesTo { Ingress, Storage, Mailbox, Overlay, Index }

pub enum Limit { Rps(u32), BytesPerSec(u64), Inflight(u32), StorageBytes(u64) }

pub struct QuotaRule {
  pub scope: Scope,
  pub limit: Limit,
  pub applies_to: AppliesTo,
  pub when_anonymous: bool,
  pub burst: Option<u32>,
}

pub struct RouteRule {
  pub route: String,                  // e.g., "GET /o/*"
  pub max_body_bytes: Option<u64>,    // ≤ 1_048_576 (tighten-only)
  pub decompress_ratio_max: Option<f32>, // ≤ 10.0 (tighten-only)
  pub require_cap: bool,
  pub obligations: Vec<Obligation>,   // AuditTag, Tarpit(ms), RequireProof{...}, etc.
}

pub struct Features {
  pub amnesia_ok: bool,
  pub requires_persistence: bool,
  pub pq_required: bool,              // advisory; enforcement elsewhere
}

pub struct Meta {
  pub change_reason: String,
  pub runbook_url: Option<String>,
  pub dashboards: Vec<String>,
  pub break_change: bool,             // required for widening rollouts
}

// Evaluation input/output
pub struct RequestCtx<'a> {
  pub route: &'a str,
  pub body_len: u64,
  pub target_region: &'a str,
  pub has_cap: bool,
  pub peer_id: &'a str,
  pub cap_audience: Option<&'a str>,
}
pub struct Decision {
  pub allow: bool,
  pub reason: &'static str,           // stable reason code (see §4.3)
  pub obligations: Vec<Obligation>,
}
```

**Serde rules:** All DTOs use `#[serde(deny_unknown_fields)]`. Unknown keys are **rejected**, not ignored (prevents config smuggling/drift). See §6 for compatibility policy.

### 2.2 JSON Schema (canonical wire)

* Location: `crates/ron-policy/schema/policybundle.schema.json` (versioned with the crate).
* Canonical wire is **JSON**. TOML is permitted for human authoring, but must be converted to canonical JSON before hashing (`id_b3`) and distribution.

**Excerpt (abbrev.):**

```json
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "title":"PolicyBundle",
  "type":"object",
  "additionalProperties": false,
  "required":["version","id_b3","issued_at_epoch_ms","residency","quotas","routes","features","metadata"],
  "properties":{
    "version":{"type":"integer","minimum":0},
    "id_b3":{"type":"string","pattern":"^b3:[0-9a-f]+$"},
    "issued_at_epoch_ms":{"type":"integer","minimum":0},
    "residency":{"$ref":"#/definitions/Residency"},
    "quotas":{"type":"array","items":{"$ref":"#/definitions/QuotaRule"}},
    "routes":{"type":"array","items":{"$ref":"#/definitions/RouteRule"}},
    "features":{"$ref":"#/definitions/Features"},
    "metadata":{"$ref":"#/definitions/Meta"}
  }
}
```

### 2.3 Canonicalization & Content Address

* **Canonical JSON:** serialize with stable key order (serde_json default is stable for maps when using structs; avoid `BTreeMap` for order-sensitive hashing unless documented).
* **`id_b3`:** `BLAKE3(canonical_json_bytes)`, rendered as `"b3:<hex>"`.
* **Helper:** `ron_policy::recompute_b3(canonical_json: &str) -> String`.

---

## 3) Bus Topics (Event Contracts)

`ron-policy` doesn’t publish/subscribe itself; these are **interop contracts** for services using it.

### 3.1 Events **Published** (by services)

* **`policy.changed`** — emitted by `svc-registry` when a new bundle is available.
  **Payload (JSON):**

  ```json
  {
    "version": 43,
    "id_b3": "b3:<hex>",
    "issuer": "registry@zone-a",           // optional, informative
    "at_ms": 1764950400000
  }
  ```

* **`policy.applied`** — emitted by consumers after successful validation & swap.

  ```json
  {
    "service": "svc-gateway",
    "from_version": 42,
    "to_version": 43,
    "id_b3": "b3:<hex>",
    "at_ms": 1764950432100
  }
  ```

* **`policy.rejected`** — consumer failed to apply bundle.

  ```json
  {
    "service": "svc-gateway",
    "attempt_version": 43,
    "id_b3": "b3:<hex>",
    "cause": "tighten_only|churn|schema|perf|other",
    "at_ms": 1764950432100
  }
  ```

### 3.2 Events **Subscribed** (by services)

* **`policy.changed`** — triggers fetch → validate (`ron-policy`) → atomic snapshot swap.
* **`bus.shutdown`** — standard shutdown; retain **last good** snapshot.

**Correlation:** Include `corr_id` headers/fields per OBSERVABILITY contract for cross-service tracing.

---

## 4) Canonical Test Vectors

> All vectors live under `crates/ron-policy/tests/vectors/` and are validated in CI.

### 4.1 Minimal Policy (TOML authoring)

```toml
version = 1
id_b3   = "b3:<computed>"
issued_at_epoch_ms = 1700000000000

[residency]
allowed_regions  = ["us-east-1"]
required_regions = ["us-east-1"]
deny_regions     = []
[residency.placement]
prefer_rtt_ms = 50
hedge_local   = 2
hedge_remote  = 1

[[quotas]]
scope = "Global"
applies_to = "Ingress"
limit = { Rps = 500 }
when_anonymous = true
burst = 100

[[routes]]
route = "GET /o/*"
require_cap = false
max_body_bytes = 1048576
decompress_ratio_max = 10.0
obligations = [{ AuditTag = "get_object" }]

[features]
amnesia_ok = true
requires_persistence = false
pq_required = false

[metadata]
change_reason = "bootstrap"
runbook_url = "https://ops/policies/1"
dashboards = []
break_change = false
```

**Assertions:**

* Parsed → canonical JSON (normalized) → `recompute_b3(json) == id_b3`.
* `validate_bundle` passes (tighten-only satisfied; no widening vs baseline).

### 4.2 Deterministic Evaluation

```json
// request_ctx.json
{ "route":"GET /o/*", "body_len":512, "target_region":"us-east-1", "has_cap":true, "peer_id":"p1", "cap_audience":"u1" }
```

**Expected:**

```json
{ "allow": true, "reason": "ok", "obligations": ["AuditTag:policy.ok","DegradeWritesFirst"] }
```

> Exact obligation encoding is crate-internal; tests assert **presence** of `reason="ok"` and `DegradeWritesFirst` when `requires_persistence=false`.

### 4.3 Deny Reasons (stable set)

* `region.denied` — target region in denylist.
* `region.not_allowed` — allowed list present and does not include target.
* `cap.required` — route demands a capability.
* `body.too_large` — payload exceeds route cap.
* `decompress.guard` — projected decompression exceeds guard.

Vectors exist for each, pairing `(bundle, ctx)` with the expected reason.

---

## 5) Error Taxonomy & Mapping

**Library errors (returned by `ron-policy`):**

* `ValidationError::TightenOnly` — bundle attempts to raise hard limits.
* `ValidationError::Churn` — widens residency/limits without `break_change=true`.
* `ValidationError::UnknownField` — schema strictness violation.
* `ValidationError::PerfBudget` — violates stated perf budgets.
* `ValidationError::Schema(msg)` — structural/typing issues.

**Service mapping (informative):**

* **400 Bad Request** — schema/unknown field (reject at ingest).
* **409 Conflict** — churn protection failed (requires runbook/break_change).
* **422 Unprocessable Entity** — tighten-only violation or perf budget.
* **503 Not Ready** — no valid snapshot (readiness gate).

---

## 6) Interop Guarantees

* **Canonical wire = JSON.** TOML is authoring-only and must be normalized to JSON before hashing/distribution.
* **Unknown fields → reject.** DTOs use `deny_unknown_fields`; consumers must not silently drop fields.
* **Content addressing is binding.** `id_b3` MUST equal the BLAKE3 of canonical JSON bytes; mismatch ⇒ reject.
* **SemVer discipline.** Breaking schema changes → **major**; additive fields use optional semantics and corresponding validator updates.
* **Determinism.** Same `(bundle, ctx, now_ms)` ⇒ identical `Decision` across patch/minor versions.
* **Tighten-only & churn.** Policies cannot relax platform bounds; widening requires `break_change=true` + runbook URL.
* **Auditability.** Canonical vectors (inputs/expected outputs) stored under `/tests/vectors/` and exercised in CI.

---

## 7) References

* **Interop Blueprint (GMI-1.6 / Omni-Gate)** — platform invariants, transport, tracing.
* **IDB.md (ron-policy)** — invariants, acceptance gates.
* **CONFIG.md** — canonical wire, strictness flags, perf budgets.
* **CONCURRENCY.md** — snapshot swap pattern (atomic, no torn reads).
* **OBSERVABILITY.md** — reason codes, metric names/labels, tracing spans.

---

## Appendix A — Canonical JSON (example)

```json
{
  "version": 43,
  "id_b3": "b3:<hex>",
  "issued_at_epoch_ms": 1764950400000,
  "residency": {
    "allowed_regions": ["us-east-1","eu-central-1"],
    "required_regions": ["us-east-1"],
    "deny_regions": [],
    "placement": { "prefer_rtt_ms": 50, "hedge_local": 2, "hedge_remote": 1 }
  },
  "quotas": [
    { "scope":"Global", "limit":{"Rps":500}, "applies_to":"Ingress", "when_anonymous":true, "burst":100 }
  ],
  "routes": [
    { "route":"GET /o/*", "max_body_bytes":1048576, "decompress_ratio_max":10.0, "require_cap":false,
      "obligations":[ { "AuditTag":"get_object" }, { "RequireProof": { "proof_kind":"zk-capped-spend", "scope":"PerCap" } } ]
    }
  ],
  "features": { "amnesia_ok": true, "requires_persistence": false, "pq_required": false },
  "metadata": { "change_reason":"tighten anon GET", "runbook_url":"https://ops/runbooks/policy-43", "dashboards":[], "break_change": false }
}
```

**Normative behavior:** `recompute_b3(canonical_json) == id_b3`.

---

## Appendix B — SDK Guidance (polyglot)

* **Language-agnostic schema:** consume **JSON Schema**; generate DTOs via codegen where possible.
* **Stable reason codes:** treat `Decision.reason` as an **opaque, stable identifier** for metrics/branching; do **not** localize strings inside SDKs.
* **Strict mode:** enable reject-on-unknown to mirror `ron-policy` behavior (most JSON parsers support strict decode).
* **Time source:** always pass an explicit `now_ms` to maintain deterministic evaluation.

---

✅ With this interop contract, services and SDKs integrate with `ron-policy` **without drift**: bundles hash the same everywhere, unknown fields never slip through, reason codes remain stable, and platform invariants (frame/body/ratio) are enforced uniformly.
