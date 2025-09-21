Short answer: **Yes, folding `tldctl` into `ron-naming` is sensible**, and doing so is the cleanest way to **re-introduce `svc-edge`** while keeping the canonical count at **33** (drop `tldctl` crate → add `svc-edge` crate).

Here’s the plan, why it’s okay, and exactly how to do it fast.

---

# ✅ Decision

* **Fold `tldctl` → into `ron-naming`** as a **binary target** (`bin/tldctl.rs`) that uses the `ron-naming` library API.
* **Add `svc-edge`** back as its own service (Edge & Integration pillar).
* **Net crate count stays 33** (remove `tldctl` crate; add `svc-edge` crate).

---

# 🧠 Why this is the right move

### Fold `tldctl` into `ron-naming`

**Pros**

* Keeps **domain logic + CLI** in one place → less drift.
* Eliminates a tiny “crate just for a CLI” (lower CI overhead, version pinning, docs duplication).
* Lets you ship **one versioned artifact**: `ron-naming` crate with the `tldctl` binary.
* Easier to expose shared invariants (address parsing, validation, normalization) without cross-crate churn.

**Cons (mitigations)**

* If you wanted separate release cadence for the CLI: mitigate by **binary feature flags** and separate **GitHub Release** assets from the same crate.

### Bring back `svc-edge` as separate

**Pros**

* Clear operational boundary: **public asset plane** (tiles/fonts/pkg mirrors), **HTTP caching**, **CORS**, **rate-limits**, **content hashing**, and **edge transforms** (e.g., shrinking images, brotli/gzip) are **not** the same as gateway auth/routing.
* Avoids stuffing gateway with CDN-like concerns (keeps `svc-gateway` focused on ingress & policy).
* Natural home for **Tor ↔ clearnet bridging** of *public, cacheable* assets w/ SRI-style hashing.

**Cons (mitigations)**

* Another service to run: mitigate with **shared Ops bundle** (RUNBOOK/HARDENING/OBSERVABILITY) and reuse gateway infra modules.

---

# 🧱 Interfaces & scope (quick guardrails)

* **`ron-naming` (now with `tldctl` bin)**

  * Responsibilities: address/tld parsing, normalization, validation, resolution rules, reserved names, deterministic casing; **CLI** for pack/get name ops, lint, and debugging.
  * Anti-scope: no network servers; no caching tiers; no business policy (that’s `ron-policy`).

* **`svc-edge` (new/returned)**

  * Responsibilities: public asset fetch/proxy, caching, CORS, response normalization, SRI/hash validation, rate-limits, time-to-live policy, optional asset minify/transform, optional Tor egress control for asset fetches.
  * Anti-scope: **no** auth/capabilities (gateway), **no** internal overlay/DAG (overlay/storage), **no** identity issuance (passport), **no** payment logic (wallet/ledger).

---

# 🛠️ Implementation steps (quick and safe)

## 1) Merge `tldctl` into `ron-naming`

* In `crates/ron-naming/Cargo.toml`, add a binary:

```toml
[package]
name = "ron-naming"
# …

[[bin]]
name = "tldctl"
path = "src/bin/tldctl.rs"
```

* Create `crates/ron-naming/src/bin/tldctl.rs`:

```rust
fn main() {
    // Minimal scaffold; wire to your existing lib once moved.
    // Example: parse args, call ron_naming::cli::run(...)
    eprintln!("tldctl: CLI moved into ron-naming (use --help)");
}
```

* Move any CLI code from the old `tldctl` crate into `ron-naming` under `src/bin/` or `src/cli.rs` with a small adapter.

* Update workspace dependencies that imported the old `tldctl` crate to stop referencing it (they shouldn’t depend on a CLI anyway).

* Remove the old crate from the workspace:

```bash
git rm -r crates/tldctl
```

* Remove it from the workspace members in root `Cargo.toml`.

## 2) Add `svc-edge` crate

```bash
cargo new crates/svc-edge --lib
```

* In `crates/svc-edge/Cargo.toml`, set it up as a service (axum 0.7, tower-http, prometheus, rustls via tokio-rustls per your standards).

* Add a `[[bin]]` section if you want `svc-edge` to build a server binary (recommended).

* Skeleton `main.rs` (paste-ready):

```rust
use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: load config (addr, cache dirs, CORS, limits)
    let app = Router::new().route("/healthz", get(|| async { "ok" }));
    let addr: SocketAddr = "0.0.0.0:8088".parse()?;
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

* Add to workspace `Cargo.toml` members and to any CI matrices.

## 3) Keep the total at 33

* Update **COMPLETECRATELIST.MD** to: remove `tldctl`, add `svc-edge`.
* (If you want me to generate the exact diff block for that file text, say the word.)

## 4) Docs & Ops (fast follow)

* In `ron-naming/docs/README.md`, add “Includes `tldctl` CLI”.
* In `svc-edge/docs/` add: `README.md`, `RUNBOOK.md`, `HARDENING.md`, `OBSERVABILITY.md`, `PERFORMANCE.md`, `QUANTUM.md` (use our God-tier templates).
* In `svc-gateway/docs/README.md`, explicitly mark **edge/CDN concerns as out-of-scope** and link to `svc-edge`.

---

# 🔍 Acceptance gates (bite-sized)

**ron-naming (with tldctl)**

* Bronze: binary builds, `tldctl --help` works, basic name parsing demo.
* Silver: commands for validate/normalize; unit tests for tricky TLDs.
* Gold: address rules + reserved list; integration tests w/ overlay/index.

**svc-edge**

* Bronze: `/healthz`, static proxy to a whitelisted origin, CORS headers, cache dir.
* Silver: SRI/hash validation, rate-limits, cache eviction policy, Prom metrics.
* Gold: Tor-egress mode (via `svc-arti-transport`), configurable allowlists, perf budget met.

---

# 🧭 Build order tweak (still 33)

Keep the same pillar sequence; just **replace `tldctl` with `svc-edge` in Pillar 8**:

* Pillar 8 (Edge & Integration):
  32\. `svc-arti-transport`
  33\. `svc-registry`
  — move `svc-edge` here; it can ship alongside/after `svc-arti-transport`.

*(If you prefer `svc-edge` before `svc-registry` that’s fine; it has low coupling.)*
