//! Aligns integer values with float checks.

use ron_kernel::Metrics;

#[test]
fn property_config_sanity_numbers_cast() {
    // this test just needs Metrics in scope; we don't actually use it.
    let _metrics = Metrics::new(false);

    // pretend these came from a config (i64s):
    let v0: i64 = 0;
    let v1: i64 = 1;
    let v2: i64 = 0;

    // cast to f64 before float math
    assert!((v0 as f64 - 0.0).abs() < f64::EPSILON);
    assert!((v1 as f64 - 1.0).abs() < f64::EPSILON);
    assert!((v2 as f64 - 0.0).abs() < f64::EPSILON);
}
