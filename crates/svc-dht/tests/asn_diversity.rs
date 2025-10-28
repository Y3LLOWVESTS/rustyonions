//! RO:WHAT — Local, self-contained test for "ASN diversity" selection logic.
//! RO:WHY  — We don’t have a real ASN guard yet; this models the policy in-test
//!           so we can lock behavior now and swap to the real guard later.
//! RO:NOTES — Pure test logic; no crate changes needed to pass.

use std::collections::HashSet;

/// Minimal stand-in for an "ASN diversity" filter:
/// Keep candidates while ensuring at least `min_unique_asn` distinct ASNs stay present.
/// Returns Err if impossible.
fn select_with_asn_diversity(
    mut candidates: Vec<(String, u32)>,
    min_unique_asn: usize,
    limit: usize,
) -> Result<Vec<(String, u32)>, &'static str> {
    // Greedy: first ensure we include one per ASN to hit the floor, then fill up to limit.
    candidates.sort_by_key(|(_, asn)| *asn);

    let mut seen = HashSet::new();
    let mut out = Vec::new();

    // Phase A: one per ASN until we hit the floor (or run out)
    for (node, asn) in candidates.iter().cloned() {
        if seen.insert(asn) {
            out.push((node, asn));
            if seen.len() >= min_unique_asn {
                break;
            }
        }
    }
    if seen.len() < min_unique_asn {
        return Err("asn_floor_unmet");
    }

    // Phase B: fill remainder by round-robin (here just linear pass) without ASN constraint
    for (node, asn) in candidates.into_iter() {
        if out.len() >= limit {
            break;
        }
        // allow duplicates of ASNs now
        if !out.iter().any(|(n, _)| *n == node) {
            out.push((node, asn));
        }
    }

    if out.len() > limit {
        out.truncate(limit);
    }
    Ok(out)
}

#[test]
fn rejects_all_same_asn_when_floor_gt1() {
    let candidates =
        vec![("n1".to_string(), 64512), ("n2".to_string(), 64512), ("n3".to_string(), 64512)];
    let res = select_with_asn_diversity(candidates, /*min_unique_asn*/ 2, /*limit*/ 2);
    assert!(res.is_err(), "should reject when all candidates share the same ASN");
}

#[test]
fn accepts_mix_and_meets_floor() {
    let candidates = vec![
        ("a".to_string(), 64512),
        ("b".to_string(), 64513),
        ("c".to_string(), 64512),
        ("d".to_string(), 64514),
    ];
    let out = select_with_asn_diversity(candidates, /*min_unique_asn*/ 2, /*limit*/ 3).unwrap();
    let unique_asn: HashSet<_> = out.iter().map(|(_, a)| *a).collect();
    assert!(unique_asn.len() >= 2, "expected ASN diversity floor met");
    assert!(out.len() <= 3);
}
