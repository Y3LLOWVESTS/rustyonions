#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

pub mod backends;
pub mod error;
pub mod ops;
pub mod traits;
pub mod types;
pub mod util;

#[cfg(feature = "with-metrics")]
pub mod metrics;
#[cfg(feature = "with-metrics")]
mod telemetry;

pub mod prelude;

pub use crate::error::KmsError;
pub use crate::traits::{Keystore, Signer, Verifier};
pub use crate::types::{Alg, KeyId};

#[must_use]
pub fn memory_keystore() -> backends::memory::MemoryKeystore {
    backends::memory::MemoryKeystore::default()
}
