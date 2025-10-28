//! RO:WHAT — Provider record facade (RAM default; TTL pruning worker)
//! RO:WHY — Enables local provide/find_providers without network
//! RO:INVARIANTS — TTL respected; amnesia-friendly (no disk by default)

pub mod record;
pub mod republish;
pub mod store;
pub mod ttl;

pub use store::Store;
