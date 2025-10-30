//! RO:WHAT — In-memory index to accelerate rule lookup by HTTP method.
//!
//! RO:WHY  — Avoid scanning all rules; common case is method-restricted rules.
//!
//! RO:INVARIANTS — Keys are uppercased method names or "*".

use crate::model::{PolicyBundle, Rule};
use std::collections::BTreeMap;

pub struct RuleIndex<'a> {
    by_method: BTreeMap<String, Vec<&'a Rule>>,
}

impl<'a> RuleIndex<'a> {
    /// Build an index from a validated `PolicyBundle`.
    ///
    /// # Errors
    ///
    /// Currently infallible; reserved for future index-build errors.
    #[must_use]
    pub fn build(bundle: &'a PolicyBundle) -> Self {
        let mut by_method: BTreeMap<String, Vec<&'a Rule>> = BTreeMap::new();
        for r in &bundle.rules {
            // clippy(map_unwrap_or): use map_or_else
            let key = r
                .when
                .method
                .as_ref()
                .map_or_else(|| "*".to_string(), |s| s.to_ascii_uppercase());
            by_method.entry(key).or_default().push(r);
        }
        Self { by_method }
    }

    /// Return candidates for a given (already UPPERCASED) method,
    /// falling back to "*" rules as well.
    pub fn candidates(&'a self, method: &str) -> impl Iterator<Item = &'a Rule> + 'a {
        // Avoid map/unwrap and redundant closures; iterate directly.
        self.by_method
            .get(method)
            .into_iter()
            .flat_map(|v| v.iter().copied())
            .chain(
                self.by_method
                    .get("*")
                    .into_iter()
                    .flat_map(|v| v.iter().copied()),
            )
    }
}
