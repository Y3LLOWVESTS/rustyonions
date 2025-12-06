// crates/svc-admin/src/interop.rs
//
// Interop helper module for tests and SDK harnesses.
// See docs/INTEROP.MD for the invariants and supported flows.

pub struct InteropHarness;

impl Default for InteropHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl InteropHarness {
    pub fn new() -> Self {
        Self
    }
}
