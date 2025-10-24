#![cfg(feature = "bus")]
use ron_metrics::{BaseLabels, HealthState, Metrics};
use ron_metrics::build_info::build_version;
use ron_metrics::bus_watcher::start_bus_watcher;
use ron_bus::{Bus, BusConfig, Event};

#[tokio::test]
async fn watcher_maps_health_events() {
    let base = BaseLabels {
        service: "svc".into(),
        instance: "t".into(),
        build_version: build_version(),
        amnesia: "off".into(),
    };
    let bus = Bus::new(BusConfig::default().with_capacity(64)).expect("bus");
    let health = HealthState::new();
    let metrics = Metrics::new(base, health).unwrap();
    let _h = start_bus_watcher(metrics.clone(), &bus, "test");

    let tx = bus.sender();
    tx.send(Event::Health { service: "db".into(), ok: false }).unwrap();
    tx.send(Event::Health { service: "cache".into(), ok: true }).unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let snap = metrics.health().snapshot();
    assert_eq!(snap.get("db"), Some(&false));
    assert_eq!(snap.get("cache"), Some(&true));

    tx.send(Event::Shutdown).unwrap();
}
