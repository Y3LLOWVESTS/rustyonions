//! RO:WHAT — Request orchestration (lookup/provide/hedging/limits)
//! RO:WHY — Keep policies out of handlers; Concerns: PERF/RES
pub mod deadlines;
pub mod hedging;
pub mod lookup;
pub mod rate_limit;
// (left for later slices)
pub mod provide { /* TODO: networked replication in next slice */
}
pub mod asn_guard { /* TODO: ASN diversity guard in next slice */
}
