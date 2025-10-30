//! RO:WHAT — Obligation handling (logical; no side effects here).
//! RO:WHY  — Services will interpret obligations; engine just aggregates.

use crate::model::Obligation;

#[derive(Debug, Clone, Default)]
pub struct ObligationSet {
    pub items: Vec<Obligation>,
}

impl ObligationSet {
    pub fn extend(&mut self, more: &[Obligation]) {
        self.items.extend_from_slice(more);
    }
}
