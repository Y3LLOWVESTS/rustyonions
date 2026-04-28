//! RO:WHAT — Minimal shutdown state model until full loom interleavings are wired.
//! RO:WHY — Pillar 12; Concerns: RES. Keeps shutdown expectations executable.
//! RO:INTERACTS — exporter::worker and rollover ticker in later batch.
//! RO:INVARIANTS — shutdown moves from running to draining to stopped without dropping state.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — no secrets.
//! RO:TEST — cargo clippy -p ron-accounting --all-targets -- -D warnings.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShutdownState {
    Running,
    Draining,
    Stopped,
}

#[test]
fn shutdown_model_placeholder() {
    let mut state = ShutdownState::Running;
    assert_eq!(state, ShutdownState::Running);

    state = ShutdownState::Draining;
    assert_eq!(state, ShutdownState::Draining);

    state = ShutdownState::Stopped;
    assert_eq!(state, ShutdownState::Stopped);
}
