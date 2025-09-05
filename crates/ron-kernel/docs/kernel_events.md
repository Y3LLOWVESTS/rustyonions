# Kernel Events (stability contract)

These events are **stable**. Changing names/shapes/types requires a **major** version bump.

- `Health { service: String, ok: bool }`
- `ConfigUpdated { version: u64 }`
- `ServiceCrashed { service: String, reason: String }`
- `Shutdown`

See `tests/event_snapshot.rs` for the serde snapshot that prevents drift.

**Public API (freeze):** `Bus`, `KernelEvent`, `Metrics`, `HealthState`, `Config`, `wait_for_ctrl_c()`.
