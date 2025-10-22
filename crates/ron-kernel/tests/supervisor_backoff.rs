use ron_kernel::Metrics;

/// Minimal invariant: the supervisor restart counter should be usable and
/// monotonically increasing under a service label. This stands in until the
/// real supervisor backoff is wired.
#[test]
fn service_restart_counter_increments_monotonically() {
    let metrics = Metrics::new(false);

    // Simulate a supervisor incrementing a labeled counter.
    let ctr = metrics.service_restarts_total.with_label_values(&["demo"]);

    let before = ctr.get();
    ctr.inc();
    let after1 = ctr.get();
    assert_eq!(after1, before + 1, "inc() should add exactly 1");

    ctr.inc_by(5);
    let after2 = ctr.get();
    assert_eq!(after2, after1 + 5, "inc_by(5) should add exactly 5");
}
