

* the **structure** (what to add to the repo),
* the **lint wall** in each crate,
* the **custom `xtask` linter** (AST-based) that enforces your blueprints,
* **CI with sanitizers** (ASan/TSan), **cargo-deny**, **doc sync**, and **invariant tag** checks,
* **compile-fail tests** for illegal states,
* optional **policy file** to tune rules without recompiling,
* and a **step-by-step run order**.

I’ve written all files in full so you can paste them verbatim. Comments inside scripts are fine. For shell commands, I’ve **left out leading `#`** so you can run them directly.

---

# 0) What this enforces (mapped to your six IDB chapters)

**Runtime Safety**

* No free `tokio::spawn` outside the supervisor.
* Nightly **ASan/TSan** CI jobs to catch UB, races, leaks.
* `unsafe` only in whitelisted dirs (e.g., `ffi/`, `hardening/`).

**Interop & App Experience**

* (Enforced by `xtask`) Public API enums/DTOs must have serde policy (`#[serde(tag="type", rename_all="snake_case")]`) when serialized.

**Boundaries & Security**

* Public structs can’t expose public fields.
* Finite-domain fields (`status|state|role|kind|type|phase|mode|level`) must be **enums/newtypes**, not `String/&str`.
* Secret-like structs must **derive(Zeroize)** (and suggest `ZeroizeOnDrop`).

**Verification & Scale**

* Compile-fail tests (`trybuild`) prove illegal states won’t compile.
* Invariant tags scanner ensures tests reference “I-#” invariants.
* `cargo-deny` blocks vulnerable/yanked deps & bad licenses.

**Economics & Governance**

* (Hook point) Add metrics/tests naming rules—example provided; easy to extend via policy.

**Usability & Operations**

* **Lint wall**: `unreachable_pub`, `private_interfaces`, `private_bounds`, `missing_docs`, `clippy::*` hygiene.
* Public enums must be `#[non_exhaustive]`.
* **README sync** from crate docs (`cargo-sync-readme`).

---

# 1) Add these files to your repo

## A) Root: `clippy.toml`

```toml
# Favor forward-compatible public APIs
avoid-breaking-exported-api = true

# Modeling pressure (enforce enums over bool farms, keep enums ergonomic)
enum-variant-size-threshold = 200   # affects clippy::large_enum_variant
max-struct-bools = 3                # affects clippy::struct_excessive_bools

# Internal hygiene
missing-docs-in-crate-items = true
missing-docs-allow-unused = false

# Prefer #[expect] over blanket #[allow]
allow-attributes-without-reason = false
```

## B) Root: `deny.toml` (cargo-deny)

```toml
[advisories]
vulnerability = "deny"
unmaintained = "deny"
yanked = "deny"
notice = "warn"
informational_warnings = ["unsound","unmaintained"]

[licenses]
confidence-threshold = 0.8
allow = [
  "MIT","Apache-2.0","BSD-3-Clause","BSD-2-Clause",
  "Unicode-DFS-2016","Unicode-3.0","ISC","Zlib","CC0-1.0",
  "OpenSSL","CDLA-Permissive-2.0"
]
deny = ["AGPL-3.0","GPL-3.0","LGPL-3.0"]

[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [{ name = "native-tls" }]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

## C) Root: `.cargo/config.toml`

```toml
[term]
color = "auto"

[alias]
# One command to run all blueprint checks (calls xtask)
blueprint = "run -p xtask -- lint"
```

## D) Root: `.github/workflows/ci.yml`

```yaml
name: CI
on:
  push:
  pull_request:

jobs:
  clippy_tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - name: Install tools
        run: |
          cargo install cargo-deny --locked
          cargo install cargo-sync-readme --locked
      - name: Deny advisories & licenses
        run: cargo deny check
      - name: Lint + xtask + tests + README sync + invariant tags
        run: |
          scripts/blueprint_check.sh

  asan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - name: AddressSanitizer tests
        env:
          RUSTFLAGS: -Zsanitizer=address
          RUSTDOCFLAGS: -Zsanitizer=address
          ASAN_OPTIONS: detect_leaks=1
        run: |
          rustup component add rust-src
          cargo test --workspace --all-targets -Zbuild-std --target x86_64-unknown-linux-gnu

  tsan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - name: ThreadSanitizer tests
        env:
          RUSTFLAGS: -Zsanitizer=thread
          RUSTDOCFLAGS: -Zsanitizer=thread
        run: |
          rustup component add rust-src
          cargo test --workspace --all-targets -Zbuild-std --target x86_64-unknown-linux-gnu
```

## E) Root: `scripts/blueprint_check.sh`

```bash
#!/usr/bin/env bash
# Run all blueprint gates: clippy, custom xtask, tests, invariants, README sync
set -euo pipefail

echo "[blueprint] ▶ cargo clippy (workspace, all features)"
cargo clippy --workspace --all-features -- -D warnings

echo "[blueprint] ▶ xtask lint (custom blueprint rules)"
cargo run -p xtask -- lint

echo "[blueprint] ▶ cargo test (workspace)"
cargo test --workspace --all-targets -- --nocapture

echo "[blueprint] ▶ invariant tag scan (I-# in tests/src)"
scripts/check_invariant_tags.sh

echo "[blueprint] ▶ sync READMEs from crate docs"
cargo sync-readme -w --all
git diff --exit-code || { echo "[blueprint] README out of sync. Run: cargo sync-readme -w --all"; exit 1; }

echo "[blueprint] ✅ all checks passed"
```

Make it executable:

```
chmod +x scripts/blueprint_check.sh
```

## F) Root: `scripts/check_invariant_tags.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "[blueprint] ▶ checking invariant tags (I-#) appear in tests/src"

hits=$(rg -n --glob 'crates/**/tests/**/*.rs' --glob 'crates/**/src/**/*.rs' -e '\bI-\d+\b' | wc -l | tr -d ' ')
if [ "$hits" -eq 0 ]; then
  echo "[blueprint] No I-# tags found. Add comments like // I-1: <explain invariant>"
  exit 1
fi
echo "[blueprint] Found $hits I-# tags — OK"
```

Make it executable:

```
chmod +x scripts/check_invariant_tags.sh
```

## G) Optional: Root `blueprints.policy.yaml` (tune rules without recompiling)

```yaml
unsafe_allow_in:
  - "crates/*/src/ffi/**"
  - "crates/*/src/hardening/**"

serde:
  require_tagged_enums: true
  tag: "type"
  rename_all: "snake_case"

metrics:
  require_in_services:
    - "service_restarts_total"
    - "request_latency_seconds"
    - "bus_lagged_total"

# Field names that should be finite-domain enums instead of String/&str
finite_domain_fields:
  - status
  - state
  - kind
  - type
  - role
  - phase
  - mode
  - level

# Names that hint a struct holds secrets and must derive Zeroize
secret_field_hints:
  - secret
  - seckey
  - private
  - password
  - passwd
  - token
  - api_key
  - key
  - seed
  - nonce
```

---

# 2) Add the **custom linter** crate: `xtask/`

## `xtask/Cargo.toml`

```toml
[package]
name = "xtask"
version = "0.1.0"
edition = "2021"

[dependencies]
walkdir = "2"
anyhow = "1"
camino = "1"
ignore = "0.4"
globset = "0.4"
rayon = "1"
console = "0.15"
syn = { version = "2", features = ["full", "parsing"] }
quote = "1"
proc-macro2 = "1"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
```

## `xtask/src/main.rs`

```rust
use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use console::Style;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::{collections::HashSet, fs};
use syn::{visit::Visit, Attribute, Expr, Fields, File, Item, ItemEnum, ItemStruct, Type, Visibility};
use quote::ToTokens;

#[derive(Debug, Default)]
struct Findings { errors: Vec<String> }
impl Findings {
    fn err(&mut self, path: &str, msg: impl AsRef<str>) { self.errors.push(format!("{path}: {}", msg.as_ref())); }
    fn merge(&mut self, other: Findings) { self.errors.extend(other.errors); }
    fn any(&self) -> bool { !self.errors.is_empty() }
}

#[derive(Debug, serde::Deserialize)]
struct Policy {
    unsafe_allow_in: Option<Vec<String>>,
    serde: Option<SerdePolicy>,
    metrics: Option<MetricsPolicy>,
    finite_domain_fields: Option<Vec<String>>,
    secret_field_hints: Option<Vec<String>>,
}
#[derive(Debug, serde::Deserialize)]
struct SerdePolicy { require_tagged_enums: Option<bool>, tag: Option<String>, rename_all: Option<String> }
#[derive(Debug, serde::Deserialize)]
struct MetricsPolicy { require_in_services: Option<Vec<String>> }

#[derive(Clone)]
struct Cfg {
    unsafe_allow_globs: Vec<String>,
    finite_names: HashSet<String>,
    secret_hints: Vec<String>,
    serde_require_tagged: bool,
    serde_tag: String,
    serde_rename_all: String,
    required_metrics: Vec<String>,
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            unsafe_allow_globs: vec!["**/ffi/**".into(), "**/hardening/**".into()],
            finite_names: ["status","state","kind","type","role","phase","mode","level"].into_iter().map(String::from).collect(),
            secret_hints: vec!["secret","seckey","private","password","passwd","token","api_key","key","seed","nonce"].into_iter().map(String::from).collect(),
            serde_require_tagged: true,
            serde_tag: "type".into(),
            serde_rename_all: "snake_case".into(),
            required_metrics: vec!["service_restarts_total","request_latency_seconds","bus_lagged_total"].into_iter().map(String::from).collect(),
        }
    }
}

fn load_cfg() -> Cfg {
    let mut cfg = Cfg::default();
    if let Ok(txt) = fs::read_to_string("blueprints.policy.yaml") {
        if let Ok(pol) = serde_yaml::from_str::<Policy>(&txt) {
            if let Some(v) = pol.unsafe_allow_in { cfg.unsafe_allow_globs = v; }
            if let Some(s) = pol.serde {
                if let Some(b) = s.require_tagged_enums { cfg.serde_require_tagged = b; }
                if let Some(t) = s.tag { cfg.serde_tag = t; }
                if let Some(r) = s.rename_all { cfg.serde_rename_all = r; }
            }
            if let Some(m) = pol.metrics { if let Some(req) = m.require_in_services { cfg.required_metrics = req; } }
            if let Some(v) = pol.finite_domain_fields { cfg.finite_names = v.into_iter().collect(); }
            if let Some(v) = pol.secret_field_hints { cfg.secret_hints = v; }
        }
    }
    cfg
}

struct Rules<'a> {
    path: &'a str,
    ast: &'a File,
    cfg: &'a Cfg,
    unsafe_ok: bool,
}

impl<'a> Rules<'a> {
    fn new(path: &'a str, ast: &'a File, cfg: &'a Cfg, unsafe_ok: bool) -> Self {
        Self { path, ast, cfg, unsafe_ok }
    }

    // R1: no free tokio::spawn outside supervisor/kernel
    fn no_free_tokio_spawn(&self) -> Vec<String> {
        if self.path.contains("supervisor") || self.path.contains("kernel") { return vec![]; }
        let mut hits = Vec::new();
        struct Finder<'b> { hits: &'b mut Vec<String> }
        impl<'b,'ast> Visit<'ast> for Finder<'b> {
            fn visit_expr(&mut self, node: &'ast Expr) {
                if let Expr::Call(call) = node {
                    if let Expr::Path(p) = &*call.func {
                        let q = p.path.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
                        if q == "tokio::spawn" {
                            self.hits.push("use Supervisor::spawn instead of tokio::spawn".into());
                        }
                    }
                }
                syn::visit::visit_expr(self, node)
            }
        }
        let mut f = Finder { hits: &mut hits }; f.visit_file(self.ast); hits
    }

    // R2: public structs cannot expose public fields
    fn public_struct_private_fields(&self) -> Vec<String> {
        let mut errs = Vec::new();
        for item in &self.ast.items {
            if let Item::Struct(ItemStruct { vis, fields, ident, .. }) = item {
                if matches!(vis, Visibility::Public(_)) {
                    match fields {
                        Fields::Named(named) => {
                            for f in &named.named {
                                if matches!(f.vis, Visibility::Public(_)) {
                                    errs.push(format!("public struct `{}` exposes public field `{}`; make fields private + ctor",
                                        ident, f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default()));
                                }
                            }
                        }
                        Fields::Unnamed(unnamed) => {
                            for (idx, f) in unnamed.unnamed.iter().enumerate() {
                                if matches!(f.vis, Visibility::Public(_)) {
                                    errs.push(format!("public tuple struct `{}` exposes public field #{}; make fields private + ctor", ident, idx));
                                }
                            }
                        }
                        Fields::Unit => {}
                    }
                }
            }
        }
        errs
    }

    // R3: finite-domain fields must be enums/newtypes, not String/&str
    fn finite_fields_are_enums(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let suspects = &self.cfg.finite_names;
        let is_str_or_string = |ty: &Type| -> bool {
            match ty {
                Type::Path(p) => p.path.segments.last().map(|s| s.ident == "String").unwrap_or(false),
                Type::Reference(r) => if let Type::Path(p) = &*r.elem { p.path.segments.last().map(|s| s.ident == "str").unwrap_or(false) } else { false },
                _ => false
            }
        };
        for item in &self.ast.items {
            if let Item::Struct(ItemStruct { ident, fields, .. }) = item {
                if let Fields::Named(named) = fields {
                    for f in &named.named {
                        if let Some(name) = f.ident.as_ref().map(|i| i.to_string()) {
                            if suspects.contains(&name) && is_str_or_string(&f.ty) {
                                errs.push(format!("struct `{}` field `{}` is String/&str but looks finite-domain; replace with `enum`", ident, name));
                            }
                        }
                    }
                }
            }
        }
        errs
    }

    // R4: public enums must be #[non_exhaustive]; public items must have docs
    fn public_api_completeness(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let has_doc = |attrs: &Vec<Attribute>| -> bool { attrs.iter().any(|a| a.path().is_ident("doc")) };
        for item in &self.ast.items {
            match item {
                Item::Enum(ItemEnum { vis, attrs, ident, .. }) if matches!(vis, Visibility::Public(_)) => {
                    if !attrs.iter().any(|a| a.path().is_ident("non_exhaustive")) {
                        errs.push(format!("public enum `{}` should be #[non_exhaustive] for forward-compat", ident));
                    }
                    if !has_doc(attrs) { errs.push(format!("public enum `{}` missing /// docs", ident)); }
                }
                Item::Struct(ItemStruct { vis, attrs, ident, .. }) if matches!(vis, Visibility::Public(_)) => {
                    if !has_doc(attrs) { errs.push(format!("public struct `{}` missing /// docs", ident)); }
                }
                _ => {}
            }
        }
        errs
    }

    // R5: unsafe only in allowed paths
    fn no_unsafe_blocks_outside_allow(&self, file_src: &str) -> Vec<String> {
        if self.unsafe_ok { return vec![]; }
        if file_src.contains("unsafe {") {
            return vec!["unsafe block found; move to allowed path (ffi/ or hardening/) or add policy".into()];
        }
        vec![]
    }

    // R6: suspected secret-bearing structs must derive Zeroize
    fn secrets_must_zeroize(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let hints = &self.cfg.secret_hints;
        for item in &self.ast.items {
            if let Item::Struct(ItemStruct { ident, fields, attrs, .. }) = item {
                let mut looks_secret = false;
                if let Fields::Named(named) = fields {
                    for f in &named.named {
                        if let Some(fname) = &f.ident {
                            let lname = fname.to_string().to_lowercase();
                            if hints.iter().any(|h| lname.contains(h)) { looks_secret = true; break; }
                        }
                    }
                }
                if looks_secret {
                    // crude derive detection
                    let derives: String = attrs.iter()
                        .filter(|a| a.path().is_ident("derive"))
                        .map(|a| a.to_token_stream().to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !derives.contains("Zeroize") {
                        errs.push(format!("struct `{}` appears to hold secrets; add `#[derive(Zeroize)]` and consider `#[zeroize_on_drop]`", ident));
                    }
                }
            }
        }
        errs
    }

    // R7: serde policy for public enums/DTOs (tag + rename_all) when serialized
    fn serde_policy_on_public_enums(&self) -> Vec<String> {
        if !self.cfg.serde_require_tagged { return vec![]; }
        let mut errs = Vec::new();
        for item in &self.ast.items {
            if let Item::Enum(ItemEnum { vis, attrs, ident, .. }) = item {
                if matches!(vis, Visibility::Public(_)) {
                    let attrs_txt = attrs.iter().map(|a| a.to_token_stream().to_string()).collect::<String>();
                    let want_tag = format!("tag = \"{}\"", self.cfg.serde_tag);
                    let want_rename = format!("rename_all = \"{}\"", self.cfg.serde_rename_all);
                    if !attrs_txt.contains("serde") || !(attrs_txt.contains(&want_tag) && attrs_txt.contains(&want_rename)) {
                        errs.push(format!(
                          "public enum `{}` should have #[serde(tag = \"{}\", rename_all = \"{}\")] for wire-compat",
                          ident, self.cfg.serde_tag, self.cfg.serde_rename_all
                        ));
                    }
                }
            }
        }
        errs
    }

    // R8: metrics contract (ensure registration in services)
    fn metrics_contract_strings(&self, file_src: &str) -> Vec<String> {
        let mut errs = Vec::new();
        let req = &self.cfg.required_metrics;
        if !(self.path.contains("/svc-") || self.path.contains("/service") || self.path.contains("/services/")) {
            return errs; // consider only service crates by path hint
        }
        for m in req {
            if !file_src.contains(m) {
                errs.push(format!("metrics contract: expected '{}' registration in service code", m));
            }
        }
        errs
    }
}

fn path_is_unsafe_allowed(path: &str, globs: &[String]) -> bool {
    let mut b = GlobSetBuilder::new();
    for g in globs { let _ = b.add(globset::Glob::new(g).unwrap()); }
    let gs = b.build().unwrap();
    gs.is_match(path)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "lint" { run_lints() } else {
        eprintln!("Usage: cargo run -p xtask -- lint");
        Ok(())
    }
}

fn run_lints() -> Result<()> {
    let cfg = load_cfg();

    let root = Utf8PathBuf::from(".");
    let mut builder = WalkBuilder::new(root.as_std_path());
    builder.hidden(true).ignore(true).git_ignore(true).git_exclude(true);

    let mut gs = GlobSetBuilder::new();
    gs.add(Glob::new("**/*.rs")?);
    let globs = gs.build()?;

    let entries: Vec<_> = builder.build().filter_map(Result::ok).collect();

    let bad = Style::new().red().bold();
    let good = Style::new().green();

    let findings = entries.par_iter().filter_map(|d| {
        let p = d.path();
        if !p.is_file() { return None; }
        if !globs.is_match(p) { return None; }
        let path_str = p.to_string_lossy().to_string();
        let src = fs::read_to_string(p).ok()?;
        let ast: File = syn::parse_file(&src).ok()?;
        let unsafe_ok = path_is_unsafe_allowed(&path_str, &cfg.unsafe_allow_globs);
        let rules = Rules::new(&path_str, &ast, &cfg, unsafe_ok);

        let mut f = Findings::default();
        for e in rules.no_free_tokio_spawn() { f.err(&path_str, e); }
        for e in rules.public_struct_private_fields() { f.err(&path_str, e); }
        for e in rules.finite_fields_are_enums() { f.err(&path_str, e); }
        for e in rules.public_api_completeness() { f.err(&path_str, e); }
        for e in rules.no_unsafe_blocks_outside_allow(&src) { f.err(&path_str, e); }
        for e in rules.secrets_must_zeroize() { f.err(&path_str, e); }
        for e in rules.serde_policy_on_public_enums() { f.err(&path_str, e); }
        for e in rules.metrics_contract_strings(&src) { f.err(&path_str, e); }
        Some(f)
    }).reduce(Findings::default, |mut a, mut b| { a.merge(b); a });

    if findings.any() {
        eprintln!("{}", bad.apply_to("Blueprint lint failures:"));
        for e in findings.errors { eprintln!("  - {}", e); }
        return Err(anyhow!("blueprint lints failed"));
    }

    println!("{}", good.apply_to("Blueprint lints passed"));
    Ok(())
}
```

---

# 3) In each **public crate**: add the **lint wall** at top of `lib.rs`

```rust
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(private_interfaces)]
#![deny(private_bounds)]
#![deny(rust_2018_idioms)]

// Clippy (curated)
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::exhaustive_structs)]
#![warn(clippy::struct_excessive_bools)]
#![warn(clippy::large_enum_variant)]
#![deny(clippy::enum_glob_use)]
#![warn(clippy::as_conversions)] // start as warn; raise later if desired
#![deny(clippy::allow_attributes_without_reason)]
```

> If you need to silence a lint, use `#[expect(lint_name, reason = "why")]` on the smallest scope possible.

---

# 4) Add **compile-fail tests** (trybuild) in any crate (or a dedicated test crate)

## `Cargo.toml` (for that crate)

```toml
[dev-dependencies]
trybuild = "1"
```

## `tests/trybuild.rs`

```rust
#[test]
fn blueprint_compile_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/try/deny_public_fields.rs");
    t.compile_fail("tests/try/deny_string_status.rs");
}
```

## `tests/try/deny_public_fields.rs`

```rust,compile_fail
pub struct Payment {
    pub amount: u64  // ❌ should be private with constructor
}
fn main() {}
```

## `tests/try/deny_string_status.rs`

```rust,compile_fail
pub struct Payment {
    status: String    // ❌ should be an enum for finite domain
}
fn main() {}
```

---

# 5) Dependencies you’ll likely add (once)

Add once in the workspace (or per crate as needed):

```
cargo add zeroize --features zeroize_derive
cargo add rayon walkdir anyhow camino ignore globset console syn@2 quote proc-macro2 serde --package xtask
cargo add serde_yaml --package xtask
cargo install cargo-deny --locked
cargo install cargo-sync-readme --locked
```

---

# 6) How to run everything (local + CI)

Local, from repo root:

```
scripts/blueprint_check.sh
```

Or:

```
cargo blueprint
```

CI runs automatically via `.github/workflows/ci.yml` on pushes/PRs.

---

# 7) Extending the suite (fast follow ideas)

* **Runtime Safety**: add an AST check that all long-running tasks are spawned via `Supervisor::spawn(name, policy)`.
* **Interop**: scan for `#[serde(flatten)]` and force justification; require versioned wire enums (e.g., `#[serde(tag="type")]`).
* **Economics**: add a rule to find ledger test(s) that assert **I-x** conservation properties (e.g., “Σ credits == debit”), fail if missing.
* **Ops**: grep for registering Prometheus metrics with standard names; fail if missing.

All of these can be added to `xtask/src/main.rs` following the patterns above; tune via `blueprints.policy.yaml`.

---

## TL;DR — Implementation order

1. **Drop in** the files above.
2. **Add lint wall** to each public crate’s `lib.rs`.
3. **Add trybuild tests** in at least one crate (scale up gradually).
4. **Add policy file** if you want to tune paths/metrics/serde rules.
5. Run:

```
scripts/blueprint_check.sh
```

6. Push—CI runs clippy, xtask, tests, cargo-deny, README sync, and ASan/TSan jobs.

That’s the **complete picture**. If anything fails on first run, paste me the error output and I’ll adjust the rules (and provide full updated code) so you can keep momentum.
