//! RO:WHAT    Verification module split into pipeline + evaluators.
//! RO:LAYOUT  pipeline (API) | streaming (small sets) | soa (columns) | soa_eval | parse tests.

pub mod parse; // tests/utilities (kept)
pub mod soa; // CaveatsSoA columnar representation
pub mod soa_eval;
pub mod streaming; // eval for small caveat sets (early short-circuit) // evaluator over SoA columns

mod pipeline; // main API (private module)

pub use pipeline::{verify_many, verify_many_into, verify_token};

#[allow(unused_imports)] // bench-only toggles may be unused in normal builds
pub use pipeline::{
    verify_many_soa_only, verify_many_streaming_only, verify_token_soa_only,
    verify_token_streaming_only,
};
