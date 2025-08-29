# ronctl

## 1. Overview
- **What it is:** A CLI tool for interacting with `svc-index` over the RustyOnions bus (`ron-bus`).  
- **Role in RustyOnions:** Provides developers and operators with a simple way to check the health of `svc-index`, resolve addresses, and insert new address → directory mappings.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Sends bus requests to `svc-index`.  
  - [x] Provides human-readable CLI commands for health, resolve, and put.  
  - [x] Prints results or errors to stdout/stderr.  

- **What this crate does *not* do:**
  - [x] Does not serve HTTP (that’s `gateway`).  
  - [x] Does not directly read/write bundle files (that’s `svc-storage`).  
  - [x] Does not manage multiple services (that’s `ron-kernel`).  

---

## 3. APIs / Interfaces
- **CLI commands:**
  - `ronctl health` → Check if `svc-index` is alive.  
  - `ronctl resolve <addr>` → Resolve an address to a directory.  
  - `ronctl put-address <addr> <dir>` → Insert or overwrite a mapping.  

- **Rust API:** Internal only (no exported library interface).  

---

## 5. Configuration
- **Environment variables:**  
  - `RON_INDEX_SOCK` — Path to `svc-index` Unix Domain Socket (default: `/tmp/ron/svc-index.sock`).  

- **Command-line flags:**  
  - `--sock <path>` — Override socket path.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - Developers / operators via CLI.  

- **Downstream (what it depends on):**  
  - `svc-index` (must be running to respond).  
  - `ron-bus` crate (for Envelope + UDS helpers).  

- **Flow:**  
  ```text
  user (CLI) → ronctl (bus client) → svc-index (bus RPC)
```