// Feature switch for the overlay listener substrate.
// Default = plain TCP listener; when `transport_ron` is enabled we’ll use ron-transport.
//
// We keep `plain` always compiled so the `ron` variant can delegate during early bring-up.
mod plain;

#[cfg(feature = "transport_ron")]
mod ron;

#[cfg(feature = "transport_ron")]
pub use ron::*;

#[cfg(not(feature = "transport_ron"))]
pub use plain::*;
