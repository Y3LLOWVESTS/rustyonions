//! Optional lib surface for testing/public-api snapshots.
//! Keep this minimal; do not leak service internals here.

#[cfg(feature = "libapi")]
pub struct ServiceOptions;

#[cfg(feature = "libapi")]
impl ServiceOptions {
    pub fn new() -> Self { Self }
}
