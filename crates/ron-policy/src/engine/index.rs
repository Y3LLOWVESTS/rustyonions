//! RO:WHAT — Simple indices to speed up matching (by method/tenant/region).
//! RO:WHY  — Keep eval O(#matching rules), not O(all rules).

use crate::model::{PolicyBundle, Rule};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct RuleIndex<'a> {
    // Maps normalized key ("GET","*"/tenant/region) → slice of rules that might match
    by_method: BTreeMap<String, Vec<&'a Rule>>,
}

impl<'a> RuleIndex<'a> {
    pub fn build(bundle: &'a PolicyBundle) -> Self {
        let mut idx = Self::default();
        for r in &bundle.rules {
            let m = r.when.method.as_deref().unwrap_or("*").to_ascii_uppercase();
            idx.by_method.entry(m).or_default().push(r);
        }
        idx
    }

    pub fn candidates<'b>(&'b self, method: &str) -> impl Iterator<Item = &'a Rule> + 'b {
        self.by_method
            .get(method)
            .into_iter()
            .flatten()
            .chain(self.by_method.get("*").into_iter().flatten())
            .copied()
    }
}
