//! RO:WHAT — HTTP layer: router, middleware, and handlers.
//! RO:WHY  — Surface area kept small/boring; DTO hygiene.
//! RO:INVARIANTS — body caps, timeouts, deterministic errors.

pub mod handlers;
pub mod middleware;
pub mod router;
