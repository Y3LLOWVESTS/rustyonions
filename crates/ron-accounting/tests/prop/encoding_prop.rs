//! RO:WHAT — Property tests for canonical encoding/digest stability.
//! RO:WHY — Pillar 12; Concerns: ECON/DX. Sealed slice commitments must be deterministic.
//! RO:INTERACTS — SealedSlice, utils::encode, utils::hashing.
//! RO:INVARIANTS — identical input produces identical digest.
//! RO:METRICS — none.
//! RO:CONFIG — no config.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --test encoding_prop.

use proptest::prelude::*;
use ron_accounting::{
    accounting::{SliceMeta, SliceRow},
    Dimension, LabelSet, SealedSlice, SliceId, Window,
};

proptest! {
    #[test]
    fn same_slice_input_has_same_digest(value in 0_u64..1_000_000) {
        let row = SliceRow {
            labels: LabelSet::new(1, "svc-storage", "local", "GET", "/objects/123"),
            dimension: Dimension::Bytes,
            value,
        };
        let id = SliceId { tenant: 1, dimension: Dimension::Bytes, seq: 1 };
        let meta = SliceMeta::new(Window::for_timestamp_ms(300_000, 300).expect("window"), 300_001, None, true);
        let a = SealedSlice::new(id.clone(), meta.clone(), vec![row.clone()]).expect("slice a");
        let b = SealedSlice::new(id, meta, vec![row]).expect("slice b");
        prop_assert_eq!(a.digest(), b.digest());
    }
}
