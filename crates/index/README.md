# index

## 1. Overview
- **What it is:** A library crate that implements the core logic for mapping RustyOnions addresses to bundle directories.  
- **Role in RustyOnions:** Provides the pure functions and types behind `svc-index`. This crate holds the business logic for resolution and storage of mappings, while `svc-index` wraps it with a `ron-bus` interface.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Defines how to resolve an address string (e.g., `b3:<hash>.ext`) into a canonical bundle directory.  
  - [x] Provides functions to insert or update mappings (addr → dir).  
  - [x] Performs basic normalization and validation of paths.  

- **What this crate does *not* do:**
  - [x] Does not expose any sockets or IPC (that’s `svc-index`).  
  - [x] Does not read or write actual file bytes (that’s `svc-storage`).  
  - [x] Does not handle overlay logic (that’s `svc-overlay`).  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `resolve(addr: &str) -> Option<PathBuf>` — Return directory path if known.  
  - `put(addr: &str, dir: &str) -> Result<()>` — Insert/overwrite mapping.  
  - Types for address normalization and validation.  

---

## 5. Configuration
- **Environment variables:** None.  
- **Command-line flags:** None (library only).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `svc-index` (all bus RPC calls are implemented using this library).  

- **Downstream (what it depends on):**  
  - May use `sled` or other lightweight storage backend under the hood (depending on configuration).  

- **Flow:**  
  ```text
  ronctl → svc-index (bus) → index (library functions)
```