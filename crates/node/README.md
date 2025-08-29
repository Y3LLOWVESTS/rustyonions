# node

## 1. Overview
- **What it is:** A developer-oriented CLI for running a simple RustyOnions node.  
- **Role in RustyOnions:** Originally provided PUT/GET commands and a `serve` mode before the service-split architecture. Today it’s mostly used for local testing, demos, and developer workflows.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Provides `serve` command to run a local HTTP gateway + overlay in one process.  
  - [x] Provides `put` and `get` commands for working with local bundles.  
  - [x] Useful for quick tests without starting `ron-kernel` + multiple services.  

- **What this crate does *not* do:**
  - [x] Does not participate in the microkernel service supervision model.  
  - [x] Does not replace the real `gateway` + `svc-*` services for production.  
  - [x] Does not implement advanced orchestration.  

---

## 3. APIs / Interfaces
- **CLI commands:**
  - `node serve` → Start a simple HTTP server serving bundles from local storage.  
  - `node put <path>` → Insert a bundle into local storage.  
  - `node get <addr>` → Fetch a bundle from local storage.  

- **Rust API:** Internal only (CLI-focused).  

---

## 5. Configuration
- **Environment variables:**  
  - Minimal; primarily uses local paths.  
- **Command-line flags:**  
  - `--bind <addr:port>` — Set bind address for `serve`.  
  - `--db <path>` — Location of local index DB.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - Developers and testers.  

- **Downstream (what it depends on):**  
  - `overlay` (legacy crate) for protocol and storage.  
  - `common` and `naming` for address handling.  

- **Flow:**  
  ```text
  user (CLI) → node serve/put/get → overlay (legacy) → local sled DB
```