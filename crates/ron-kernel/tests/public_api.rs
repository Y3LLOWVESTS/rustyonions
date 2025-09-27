#[test]
fn public_api_reexports_exist() {
    // Stubs ensure these items exist; behavior comes later.
    use ron_kernel2::{Bus, KernelEvent, Metrics, HealthState, Config, wait_for_ctrl_c};
    let _ = (std::any::TypeId::of::<Bus>(),
             std::any::TypeId::of::<KernelEvent>(),
             std::any::TypeId::of::<Metrics>(),
             std::any::TypeId::of::<HealthState>(),
             std::any::TypeId::of::<Config>());
    let _ = wait_for_ctrl_c; // name check
}
