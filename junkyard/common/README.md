# common

## 1. Overview
- **What it is:** A shared utility library used across RustyOnions services. It provides foundational helpers such as hashing, config parsing, and small utilities.  
- **Role in RustyOnions:** Serves as the base crate for types and helpers that don’t belong to any single service but are required by many (e.g., `naming`, `overlay`, `gateway`, `svc-*`).

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Provides hashing utilities (e.g., blake3 wrappers).  
  - [x] Defines shared types and constants.  
  - [x] Provides environment/config helpers.  
  - [x] Offers general-purpose utility functions that are safe to reuse.  

- **What this crate does *not* do:**
  - [x] Does not expose IPC or networking (that’s `ron-bus` / `transport`).  
  - [x] Does not implement service-specific logic (keep it generic).  
  - [x] Does not depend on async runtimes or heavy external crates.  

---

## 3. APIs / Interfaces
- **Rust API:**
  - Hashing helpers (e.g., `hash_bytes`, `addr_from_hash`).  
  - Config/environment loaders (e.g., `get_env_var_with_default`).  
  - Shared constants for chunk sizes, protocol versions, etc.  

---

## 5. Configuration
- **Environment variables:** None directly.  
- Used only via helpers (other services define their own env vars).  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `naming` (uses hash helpers).  
  - `overlay` (legacy hashing and chunk size).  
  - `svc-*` services (for constants, env utils).  
  - `gateway`.  

- **Downstream (what it depends on):**  
  - `blake3` and other lightweight crates.  

- **Flow:**  
  ```text
  any service needing hashing/config → common
```