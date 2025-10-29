//! svc-storage library entry — exposes modules to the bin target.
//! RO:WHAT  — Crate root and module exposes.
//! RO:WHY   — Keep bin thin; organize HTTP and storage layers cleanly.

#![forbid(unsafe_code)]

pub mod errors;
pub mod readiness;
pub mod storage;
pub mod types;
pub mod version;

pub mod http {
    pub mod error;
    pub mod extractors;
    pub mod middleware;
    pub mod routes {
        pub mod get_object;
        pub mod head_object;
        pub mod health;
        pub mod metrics;
        pub mod post_object; // present for completeness; not mounted by default
        pub mod put_object;
        pub mod ready;
        pub mod version;
    }
    pub mod server;
}
