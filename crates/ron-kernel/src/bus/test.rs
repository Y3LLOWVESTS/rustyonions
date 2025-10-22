/*!
Unit tests for the Bus contract (lives next to the implementation so it runs with unit tests too).

Contract under test:
- Zero subscribers: publish returns Ok(0) and increments bus_no_receivers_total.
- One subscriber: publish returns Ok(1) and does NOT increment bus_no_receivers_total again.

These are intentionally minimal: they do not receive messages; they only validate publish semantics + metrics.
*/

#![cfg(test)]

use crate::events::KernelEvent;
use crate::Metrics;

#[test]
fn publish_with_zero_subscribers_returns_ok_zero_and_counts_metric() {
    let metrics = Metrics::new(false);
    let bus = metrics.make_bus(8);

    let before = metrics.bus_no_receivers_total.get();
    let delivered = bus.publish(KernelEvent::ConfigUpdated { version: 1 });
    assert_eq!(delivered, Ok(0), "zero subscribers must yield Ok(0)");
    let after = metrics.bus_no_receivers_total.get();
    assert_eq!(
        after,
        before + 1,
        "bus_no_receivers_total must increment on publish with zero subscribers"
    );
}

#[test]
fn publish_with_one_subscriber_returns_ok_one_and_metric_stays_flat() {
    let metrics = Metrics::new(false);
    let bus = metrics.make_bus(8);

    // establish baseline and add one subscriber
    let base = metrics.bus_no_receivers_total.get();
    let _rx = bus.subscribe();

    // publish and assert Ok(1)
    let delivered = bus.publish(KernelEvent::ConfigUpdated { version: 2 });
    assert_eq!(delivered, Ok(1), "one subscriber must yield Ok(1)");

    // metric must not increment in this case
    let now = metrics.bus_no_receivers_total.get();
    assert_eq!(
        now, base,
        "bus_no_receivers_total must NOT increment when at least one subscriber exists"
    );
}
