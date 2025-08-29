# <Crate Name>

## 1. Overview
- **What it is:** (one-paragraph summary of the crate’s purpose)
- **Role in RustyOnions:** (how it fits into the microkernel and which crates/services use it)

## 2. Responsibilities
- **What this crate does:**
  - [ ] <responsibility 1>
  - [ ] <responsibility 2>
- **What this crate does *not* do (boundaries):**
  - [ ] <explicit non-responsibility 1>
  - [ ] <explicit non-responsibility 2>

## 3. APIs / Interfaces
- **Rust API (if library crate):**  
  - [ ] <list public functions/types>
- **Bus RPC methods (if service crate):**  
  - [ ] `<method>` → `<response>`
- **CLI commands (if binary/tool crate):**  
  - [ ] `<command>` — `<description>`

## 5. Configuration
- **Environment variables:**  
  - [ ] `<VAR>` — `<description>` (default: `<value>`)
- **Command-line flags (if applicable):**  
  - [ ] `--flag` — `<description>` (default: `<value>`)

## 8. Integration
- **Upstream crates/services that call this:**  
  - [ ] <crate A>
  - [ ] <crate B>
- **Downstream crates/services this depends on:**  
  - [ ] <crate C>
  - [ ] <crate D>
- **Typical flow / sequence diagram:**  
  ```text
  <caller> → <this crate> → <dependency>
