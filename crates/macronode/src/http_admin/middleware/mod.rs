//! RO:WHAT — Admin HTTP middleware for Macronode.
//! RO:WHY  — Cross-cutting behaviors around the admin router.
//!
//! RO:INVARIANTS —
//!   - Middlewares are pure functions over `Request<Body>` and `Next`.
//!   - No panics on malformed headers; we fail closed where appropriate.
//!   - Admin auth is opt-in via `RON_ADMIN_TOKEN` but loudly logs when unset.

pub mod auth;
pub mod rate_limit;
pub mod request_id;
pub mod timeout;
