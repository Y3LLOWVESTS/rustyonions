

# Storage Patterns Plan — DX-First Integration of External DBs with CAS

**Goal:**
Let developers keep their favorite DB (Postgres/Mongo/S3/Redis/…) *without* friction, while RON-CORE still treats CAS (`svc-storage`) as the canonical fabric for integrity, replication, and capabilities.

**DX Principle:**

> *“Don’t make me abandon my DB. Don’t make me read a manual to wire it up.”*
> Integration should be: **copy/paste, run one command, set one DSN/env, done.**

We achieve this with:

* A clear **Storage Principle** in docs,
* A **simple CAS surface** in SDKs,
* A small **set of facet examples** (Patterns A/B/C) that:

  * live under `examples/`,
  * ship with mini-IDBs,
  * are configurable via env vars,
  * and provide copy-paste recipes for devs.

No new core crates or risky refactors are needed right now.

---

## 0) TL;DR — What We’re Actually Doing

1. **Write down the Storage Principle** in RON-CORE docs so it’s official.
2. **Expose CAS in SDKs** (Rust now, TS later) via a tiny, obvious API.
3. **Add 2–3 facet examples** under `examples/`:

   * Pattern A: CAS primary → Postgres view.
   * Pattern B: DB primary → CAS anchors.
   * Pattern C (later): Hybrid snapshots (CAS history + DB hot state).
4. **Make the examples DX-first:**

   * One command to run,
   * A few env vars for DSN/URL,
   * 10–20 lines of “hello DB + RON-CORE” snippets.
5. Add **light tests** (idempotency, replay) and notes for future chaos/perf work.

---

## 1) Storage Principle (Core Doctrine)

This belongs in `RON_CORE.MD` or `INTEROP_BLUEPRINT.MD` as the canonical stance.

### 1.1 CAS as canonical fabric

* `svc-storage` (CAS) is the **canonical fabric** for data that RON-CORE:

  * replicates,
  * verifies (BLAKE3 hashes),
  * guards with capabilities,
  * and respects in **amnesia** mode.
* CAS is immutable: blobs are write-once, identified by hash; no in-place mutation.

### 1.2 External DBs are *integrated*, not forbidden

Developers **do not have to abandon their DB**. Instead, external DBs are integrated via three *blessed* patterns:

* **Pattern A – CAS primary; DB as view**

  * Writes go to CAS.
  * A facet subscribes to CAS/index events and **projects** into Postgres/Mongo/whatever.
  * DB is a **read-friendly projection**: tables, indexes, caches.
* **Pattern B – DB primary; CAS as anchor**

  * Writes go to an external DB.
  * A facet reads/streams changes and writes **anchors** to CAS:

    * URIs like `db://cluster/schema/table/id`,
    * plus checksums or snapshots.
  * CAS becomes a **capability+integrity layer** around an existing DB.
* **Pattern C – Hybrid**

  * DB holds hot/mutable state.
  * CAS holds history/snapshots/event logs.
  * Facets handle snapshotting and replay to rebuild DB if needed.

### 1.3 Micronode vs Macronode behavior

* **Micronode + amnesia mode:**

  * No persistent external DBs **by default**.
  * Suggested patterns:

    * CAS only, or
    * in-memory/**ephemeral** views (e.g., SQLite `:memory:`, Redis running in RAM).
  * If someone insists on persistent DBs in micronode:

    * requires explicit opt-in,
    * loud warnings (“you are weakening amnesia”).
* **Macronode:**

  * External DB connectors via facets are **encouraged** for:

    * reporting,
    * analytics,
    * search,
    * hot caches.
  * For “RON-CORE guarantees” (integrity/replication), data should still flow through CAS (Pattern A or C).

### 1.4 Capability and audit rules

* All access to external DBs **must go through facets** that:

  * validate a **capability** (macaroon) from RON-CORE (via `ron-auth`),
  * fetch DB credentials from `ron-kms`,
  * never log secrets,
  * and emit audits via `ron-audit`.
* There is **no ambient DB root credential** hidden inside the platform.

---

## 2) DX Principles for Storage Integration

To keep DX God-tier, we adopt a few hard rules:

1. **Keep My DB**

   * Every example and doc should assume the dev already has:

     * a Postgres / Mongo / S3 / etc. instance,
     * and we’re wiring RON-CORE *into* that, not replacing it.
2. **No Manual Required to Get Started**

   * Each pattern gets:

     * a small “3-snippet” doc section:

       1. “Start RON node + facet.”
       2. “Write data in your DB / CAS.”
       3. “See it appear on the other side.”
   * Every facet example’s README has copy-paste commands.
3. **One Env File, One Command**

   * DSN, RON endpoint, token, etc. all come from env vars with sane defaults.
   * Running the facet should be as easy as:

     * `cp .env.example .env` → edit DSN → `cargo run -p facet-storage-postgres`.
4. **Idiomatic SDKs**

   * For Rust/TS SDKs, interacting with CAS or anchors should look completely normal for those ecosystems, no weird RON-isms leaking through.
   * Example: `client.storage.put_blob(bytes)` and `client.storage.get_blob(addr)`.

---

## 3) CAS Surface in SDKs (DX-Focused)

We don’t reinvent storage at the SDK layer; we simply make CAS **trivially easy** to call.

### 3.1 ron-app-sdk (Rust) — near-term

Add a small `storage` surface to the Rust SDK:

* Conceptual API (we’ll align with actual types when editing code):

```rust
impl Client {
    /// Store a blob in CAS; returns its address (hash).
    pub async fn put_blob(&self, data: bytes::Bytes) -> Result<Addr, Error>;

    /// Fetch a blob by its CAS address.
    pub async fn get_blob(&self, addr: Addr) -> Result<bytes::Bytes, Error>;
}
```

**DX requirements:**

* Works with a few env vars set (gateway URL, token).
* Example snippet in README:

```rust
let client = Client::from_env()?; // picks up RON_SDK_* vars

let addr = client.put_blob(Bytes::from("hello world")).await?;
let data = client.get_blob(addr).await?;
assert_eq!(&data[..], b"hello world");
```

### 3.2 Future: `ron-app-sdk-ts` (Tier 1)

Once TS SDK is in play, mirror a similar surface:

```ts
const client = RonClient.fromEnv(); // uses RON_SDK_GATEWAY, etc.

const addr = await client.storage.putBlob(new TextEncoder().encode("hello world"));
const data = await client.storage.getBlob(addr);
console.log(new TextDecoder().decode(data)); // "hello world"
```

This becomes the **canonical mental model** for CAS operations in all SDKs.

---

## 4) Examples: DX-First Facets for Patterns A/B/C

We’ll implement facets as **examples** first (under `examples/`), not full crates. That keeps the surface small, copy-paste friendly, and lets us graduate the ones that prove valuable.

### 4.1 Pattern A: CAS → Postgres View (`facet-storage-postgres`)

**Goal:**
Demonstrate “CAS primary, Postgres as view” with minimal friction.

**Location:**

```text
examples/
  facet-storage-postgres/
    Cargo.toml
    src/main.rs
    IDB.MD
    README.md
    .env.example
```

**What it does:**

* Uses `ron-app-sdk` to:

  * connect to a RON-CORE node,
  * subscribe to a CAS/index topic (e.g., “document created/updated”).
* Uses a Postgres client to:

  * insert/update rows in a simple table reflecting those documents/events.

**DX behavior:**

* `README.md` includes three snippets:

  1. Start node + Postgres (docker-compose snippet if helpful).
  2. Run facet:
     `cargo run --example facet-storage-postgres` (or via `cargo run -p` depending on structure).
  3. Use `ron-app-sdk` or `curl` to write to CAS → see rows appear in Postgres.

* Configuration:

  * `.env.example`:

    ```env
    RON_SDK_GATEWAY_URL=http://127.0.0.1:8080
    RON_SDK_TOKEN=...
    PG_DSN=postgres://user:pass@localhost:5432/ron_demo
    ```

  * `main.rs` reads these via `dotenvy`/`std::env`.

**Mini-IDB highlights (IDB.MD inside the example):**

* RO:WHAT – Facet that projects CAS events to Postgres tables.
* RO:WHY – DX: “I keep SQL and get CAS for free”; RES: replayable from CAS.
* RO:INTERACTS – `ron-app-sdk` (subscribe), Postgres client (`tokio-postgres` or `sqlx`), `ron-bus`/svc-index topics.
* INVARIANTS:

  * Idempotent projection:

    * same CAS event processed twice doesn’t duplicate rows.
  * Capability check:

    * facet uses a token with only “read CAS events” and “write to its specific view.”
  * Micronode: can run with SQLite in-memory instead of Postgres.
* TEST:

  * Unit/integration: synthetic CAS event → row in a test DB.
  * Future (post-core): property test for idempotent replay.

### 4.2 Pattern B: DB → CAS Anchors (`facet-anchors-postgres`)

**Goal:**
Demonstrate “DB primary, CAS anchors” so devs can keep an existing DB and *still* use CAS for integrity/capabilities.

**Location:**

```text
examples/
  facet-anchors-postgres/
    Cargo.toml
    src/main.rs
    IDB.MD
    README.md
    .env.example
```

**What it does:**

* Uses a Postgres client to:

  * periodically scan a table (or tail via a simple CDC mechanism).
* Uses `ron-app-sdk` to:

  * write a **CAS anchor** record per row:

    * anchor payload might include:

      * `db_uri: "db://cluster/schema/table/id"`
      * `checksum: "..."` (BLAKE3 or similar)
      * small metadata (tenant, type).

**DX behavior:**

* `README.md` snippets:

  1. “Point this facet at your existing DB via PG_DSN.”
  2. “Run the facet.”
  3. “Use CAS client or admin GUI to inspect anchors for your rows.”

* `.env.example` similar:

  ```env
  RON_SDK_GATEWAY_URL=http://127.0.0.1:8080
  RON_SDK_TOKEN=...
  PG_DSN=postgres://user:pass@localhost:5432/existing_app
  ANCHOR_NAMESPACE=my_app
  ```

**Mini-IDB highlights:**

* RO:WHAT – Facet that creates CAS anchors pointing at existing DB rows.
* RO:WHY – DX: you keep your DB; SEC: you get capabilities + audit around access/anchors.
* INVARIANTS:

  * CAS anchor never includes full sensitive row data (just ref + checksum).
  * Checksums are verifiable back against DB if needed.
  * In amnesia/micronode: facet can be disabled or run in a no-op mode.
* TEST:

  * Write one row → expect one anchor in CAS with correct checksum.
  * Future: property tests around integrity (row modifications vs anchors).

### 4.3 Pattern C: Hybrid Snapshots (`facet-hybrid-postgres`) — *post-M milestone*

This is **explicitly later** (post-Mx), but we bake it into the plan.

**Idea:**

* DB holds hot state; CAS holds:

  * periodic snapshots or event log.
* Facet:

  * periodically snapshot DB → CAS,
  * can rebuild DB (or a copy) from CAS solely.

This becomes your go-to example for “forensic truth + rebuild from CAS.” You tag it in the notes as:

* `post-M5: facet-hybrid-postgres`.

---

## 5) SDK Documentation: “3-Snippet” Recipes for Storage

To reinforce DX, SDK docs should include **copy-paste recipes** for each pattern. For example:

### 5.1 Rust SDK (ron-app-sdk) docs

* Snippet 1: Put/get CAS blobs (hello world).
* Snippet 2: Pattern A-like: consume CAS updates, write to local DB (in-memory example).
* Snippet 3: Pattern B-like: write anchors to CAS for existing data.

### 5.2 TS SDK docs (when built)

* Same three ideas, but in TypeScript:

  * Browser and Node variants where appropriate.

These snippets should not mention “Pattern A/B/C” in big theoretical terms; they should just show:

* “Here’s how to mirror data from CAS into your DB,” and
* “Here’s how to mirror data from your DB into CAS.”

The *names* “Pattern A/B/C” can live in RON-CORE docs and NOTES.

---

## 6) Micronode & Macronode Config/Docs

We add **small but explicit** notes in node docs:

### 6.1 Micronode

* Config:

  * `MICRONODE_ALLOW_PERSISTENT_DB=false` by default.
* Docs:

  * “Micronode is optimized for amnesia / RAM-only. If you connect it to a persistent DB, you partially defeat that. Do this only if you know what you’re doing.”
* Facets:

  * Pattern A example can default to an in-memory DB (e.g., SQLite `:memory:`) when it detects micronode profile.

### 6.2 Macronode

* Config:

  * facet/connector section where you can list:

    * `facet-storage-postgres`,
    * `facet-anchors-postgres`, etc.
* Docs:

  * Recommend Pattern A or C when you want RON-CORE to own the truth.
  * Pattern B when you are wrapping an existing monolith / DB.

---

## 7) Security & Testing: Keep It Light but Real

We don’t block the examples on full-blown chaos rigs, but we **do**:

### 7.1 Security hygiene (minimum)

* Every facet example:

  * reads DB creds from env (or better, from `ron-kms` in the future),
  * never logs credentials,
  * uses a limited-scope RON token (capabilities) for CAS operations.
* IDB for each facet calls this out in RO:SEC/INVARIANTS.

### 7.2 Testing (minimum)

For each facet:

* At least one integration test:

  * bring up a test DB,
  * send a fake CAS event (Pattern A) or fake row (Pattern B),
  * assert it gets mirrored correctly.
* Note in IDB:

  * “Future: property tests for idempotent replay.”
  * “Future: chaos tests with DB flakiness (toxiproxy).”

No need to overdo it now — just leave hooks so future-you knows where to add the heavy tests.

---

## 8) Future Work: svc-storage Backends (Post-Core)

After core, SDK, and facet examples are in a good place:

* Revisit `svc-storage` and consider a **pluggable backend trait**:

  * default: sled/disk,
  * optional: S3, GCS, etc. via feature flags.
* Keep invariants:

  * immutable blobs,
  * BLAKE3 verification,
  * amnesia-aware backends (e.g., memory-only for micronode),
  * no Web3/value-plane creep.

This is **firmly a later milestone**, and the plan keeps it that way.

---

## 9) Summary (DX-centric)

**For devs:**

* You don’t have to abandon Postgres/Mongo/etc.
* You don’t have to read a giant manual.
* You get:

  * copy-paste examples,
  * CAS APIs that feel natural in your language,
  * and the option to bring your DB in **as a view, as primary with anchors, or hybrid**.

**For RON-CORE:**

* CAS remains the canonical fabric.
* External DBs are integrated via facets and SDKs that honor:

  * capabilities,
  * amnesia,
  * and replay correctness.
* Nothing in this plan disrupts current crates; it just adds:

  * docs,
  * SDK APIs,
  * and example facets that live close to the dev experience.

