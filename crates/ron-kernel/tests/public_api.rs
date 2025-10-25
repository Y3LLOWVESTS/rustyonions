//! Verifies the frozen public API is re-exported at the crate root.
//! Fails to compile if any item disappears or moves.

use ron_kernel::{wait_for_ctrl_c, Bus, Config, HealthState, KernelEvent, Metrics};

#[test]
fn api_compiles_and_names_resolve() {
    // Type names resolve? good enough for compile-time surface guard.
    let _ = std::any::type_name::<Bus<KernelEvent>>();
    let _ = std::any::type_name::<Metrics>();
    let _ = std::any::type_name::<HealthState>();
    let _ = std::any::type_name::<Config>();
    let _ = wait_for_ctrl_c as fn() -> _;
}
