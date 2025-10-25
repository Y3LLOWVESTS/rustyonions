use std::collections::HashSet;
use oap::Seq;

#[test]
fn seq_is_monotonic_and_unique_across_sample() {
    let s = Seq::new();
    let mut seen = HashSet::new();
    let n = 10_000;
    for _ in 0..n {
        let id = s.next();
        assert!(seen.insert(id), "duplicate id detected");
    }
}
