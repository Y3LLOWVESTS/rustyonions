//! STUB: decode guard (decompression ratio / absolute cap checks) temporarily disabled.
//! Returns an identity layer to unblock compilation; real logic will be restored later.

#![allow(clippy::unused_async)]

use tower::layer::util::Identity;

/// Stub that produces a `Layer` satisfying axum's `route_layer` bounds.
#[must_use]
pub fn layer(_ratio_max: usize, _abs_cap: usize) -> Identity {
    Identity::new()
}
