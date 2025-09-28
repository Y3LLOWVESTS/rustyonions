/*! ryker — embedded actor & bounded mailbox runtime (library-only).
    Public facade only. See README for invariants and acceptance gates. */
pub mod prelude;
pub mod errors;
pub mod config;
pub mod runtime;
pub mod supervisor;
pub mod mailbox;
pub mod observe;

