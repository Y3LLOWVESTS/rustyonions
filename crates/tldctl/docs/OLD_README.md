# tldctl

## 1. Overview
- **What it is:** A developer/admin control tool for working with RustyOnions TLDs, manifests, and metadata.  
- **Role in RustyOnions:** Helps generate, validate, and inspect `Manifest.toml` files, and provides utilities for TLD-specific content management. It is primarily intended as a helper CLI for developers and operators.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Parses and validates `Manifest.toml` files.  
  - [x] Provides CLI commands for working with bundle metadata.  
  - [x] Ensures manifests follow RustyOnions schema (payment, attribution, etc.).  

- **What this crate does *not* do:**
  - [x] Does not resolve addresses (that’s `svc-index`).  
  - [x] Does not serve bundles (that’s `gateway` / `svc-overlay`).  
  - [x] Does not enforce payments (just inspects metadata).  

---

## 3. APIs / Interfaces
- **CLI commands:**
  - `tldctl validate <path>` — Validate a `Manifest.toml`.  
  - `tldctl gen` — Generate a manifest scaffold (planned).  
  - `tldctl show <path>` — Pretty-print manifest details.  

- **Rust API (optional):**
  - May expose manifest parsing helpers for use in other crates.  

---

## 5. Configuration
- **Environment variables:** None (operates on local files).  
- **Command-line flags:**  
  - `--pretty` — Pretty-print output.  
  - `--strict` — Enforce stricter schema rules.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - Developers and operators via CLI.  
- **Downstream (what it depends on):**  
  - `serde` + `toml` for manifest parsing.  
  - `common` for shared constants and types.  

- **Flow:**  
  ```text
  user (CLI) → tldctl → Manifest.toml validation / inspection
```