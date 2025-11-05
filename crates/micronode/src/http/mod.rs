//! RO:WHAT — HTTP handlers (admin + basic routes + dev).
//! RO:WHY  — Keep app.rs readable.

pub mod admin;
pub mod kv;
pub mod routes;

pub mod dev {
    pub use super::routes::dev::echo;
}
