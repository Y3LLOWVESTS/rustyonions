//! RO:WHAT — Simple ranking heuristics placeholder.

use crate::types::ProviderEntry;

pub fn rank(mut v: Vec<ProviderEntry>) -> Vec<ProviderEntry> {
    v.sort_by(|a, b| b.score.total_cmp(&a.score));
    v
}
