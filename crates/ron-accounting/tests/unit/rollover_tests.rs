//! RO:WHAT — Unit tests for UTC fixed-window rollover behavior.
//! RO:WHY — Pillar 12; Concerns: ECON/RES. Prevents duplicate or drifting rollover seals.
//! RO:INTERACTS — Window, RolloverHandle, Recorder.
//! RO:INVARIANTS — start inclusive/end exclusive; one rollover per boundary crossing.
//! RO:METRICS — none.
//! RO:CONFIG — window_len_s=300 in tests.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo test -p ron-accounting --test rollover_tests.

use ron_accounting::accounting::{RolloverDecision, RolloverHandle};
use ron_accounting::Window;

#[test]
fn window_aligns_to_utc_boundary() {
    let window = Window::for_timestamp_ms(599_999, 300).expect("valid window");
    assert_eq!(window.start_ms, 300_000);
    assert_eq!(window.end_ms, 600_000);
    assert!(window.contains(599_999));
    assert!(!window.contains(600_000));
}

#[test]
fn rollover_happens_once_when_boundary_crosses() {
    let mut handle = RolloverHandle::new(599_000, 300).expect("handle");
    assert!(matches!(
        handle.observe(599_500).expect("stay"),
        RolloverDecision::Stay(_)
    ));

    let decision = handle.observe(600_000).expect("rollover");
    match decision {
        RolloverDecision::Rollover { previous, next } => {
            assert_eq!(previous.start_ms, 300_000);
            assert_eq!(next.start_ms, 600_000);
        }
        RolloverDecision::Stay(_) => panic!("expected rollover"),
    }

    assert!(matches!(
        handle.observe(600_001).expect("stay after rollover"),
        RolloverDecision::Stay(_)
    ));
}
