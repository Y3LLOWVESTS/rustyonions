# naming

## 1. Overview
- **What it is:** A library crate that defines and validates RustyOnions content addresses (e.g., `b3:<hash>.<ext>`).  
- **Role in RustyOnions:** Ensures that all services (`gateway`, `svc-index`, `svc-overlay`, etc.) use a canonical, well-formed representation of bundle addresses.

---

## 2. Responsibilities
- **What this crate does:**
  - [x] Parses string addresses into structured types.  
  - [x] Validates address format (hash algorithm, extension).  
  - [x] Normalizes addresses to a canonical form.  

- **What this crate does *not* do:**
  - [x] Does not resolve addresses to directories (that’s `svc-index`).  
  - [x] Does not fetch or serve content (that’s `svc-overlay` / `gateway`).  
  - [x] Does not perform hashing itself (delegates to `common`).  

---

## 3. APIs / Interfaces
- **Rust API:**
  - `parse_addr(addr: &str) -> Result<Address>` — Parse a string into an address type.  
  - `Address` struct with fields for hash type, hash bytes, and extension.  
  - `to_string(&self) -> String` — Convert back to canonical string form.  

---

## 5. Configuration
- **Environment variables:** None.  
- **Command-line flags:** None.  

---

## 8. Integration
- **Upstream (who calls this):**  
  - `svc-index` (to validate addresses before storing mappings).  
  - `gateway` and `svc-overlay` (to normalize user input addresses).  

- **Downstream (what it depends on):**  
  - `common` (for hash validation helpers).  

- **Flow:**  
  ```text
  client → gateway (HTTP addr) → naming::parse_addr() → normalized address
```