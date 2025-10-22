use ron_kernel::Metrics;

#[test]
fn amnesia_gauge_flips_between_0_and_1() {
    let metrics = Metrics::new(false);

    // starts at 0
    assert_eq!(metrics.amnesia_mode.get(), 0);

    // flip on -> 1
    metrics.set_amnesia(true);
    assert_eq!(metrics.amnesia_mode.get(), 1);

    // flip off -> 0
    metrics.set_amnesia(false);
    assert_eq!(metrics.amnesia_mode.get(), 0);
}
