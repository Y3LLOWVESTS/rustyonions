//! RO:WHAT — Peer ID, Kademlia k-buckets, routing table, selectors
//! RO:WHY — Core routing structures; Concerns: RES/PERF
pub mod bucket;
pub mod id;
pub mod selector;
pub mod table;

pub use id::NodeId;
pub use table::RoutingTable;
