//! RO:WHAT — Property tests for label normalization idempotence.
//! RO:WHY — Pillar 12; Concerns: PERF/SEC. Labels must stay bounded and PII-safe-ish.
//! RO:INTERACTS — normalize, LabelSet.
//! RO:INVARIANTS — normalization is idempotent and length-capped.
//! RO:METRICS — none.
//! RO:CONFIG — normalize constants.
//! RO:SECURITY — obvious dynamic IDs are templated.
//! RO:TEST — cargo test -p ron-accounting --test prop.

use proptest::prelude::*;
use ron_accounting::normalize::{normalize_component, normalize_route, MAX_LABEL_LEN};

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn component_normalization_is_idempotent(input in ".{0,128}") {
        let once = normalize_component(&input);
        let twice = normalize_component(&once);

        prop_assert_eq!(&once, &twice);
        prop_assert!(twice.len() <= MAX_LABEL_LEN);
    }

    #[test]
    fn route_normalization_is_idempotent(input in ".{0,128}") {
        let once = normalize_route(&input);
        let twice = normalize_route(&once);

        prop_assert_eq!(&once, &twice);
        prop_assert!(twice.starts_with('/'));
    }
}
