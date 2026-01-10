//! RO:WHAT — HTTP handlers (admin + admin API + kv + basic routes + dev).
//! RO:WHY  — Keep app.rs readable.

pub mod admin;
pub mod admin_api;
pub mod kv;
pub mod routes;

pub mod dev {
    pub use super::routes::dev::echo;
}
