// Property/invariant test scaffold: attenuation must never widen authority.

use std::collections::BTreeSet;

#[test]
fn attenuation_scaffold_never_widens_scope_set() {
    let original: BTreeSet<&str> = ["profile:read", "profile:write"].into_iter().collect();
    let attenuated: BTreeSet<&str> = ["profile:read"].into_iter().collect();

    assert!(attenuated.is_subset(&original));
    assert!(!original.is_subset(&attenuated));
}
