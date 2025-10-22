//! RO:WHAT — Integration smoke for SoA backend under the `bus_soa` feature.
//! RO:WHY  — Ensure crate-level feature compiles & basic flows hold.

#![cfg(feature = "bus_soa")]

use ron_kernel::bus::bounded::Bus; // bounded is re-exported to SoA when feature=bus_soa
use tokio::runtime::Runtime;

#[test]
fn feature_compiles_and_basic_flow_ok() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let bus: Bus<u64> = Bus::with_capacity(16);
        let mut rx = bus.subscribe();
        let _rc = bus.publish(42);
        // bounded-style: recv returns Result<T, Lagged>, use handle_recv to map to Option
        let got = Bus::handle_recv(rx.recv().await, None);
        assert_eq!(got, Some(42));
    });
}
