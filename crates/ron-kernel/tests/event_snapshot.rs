// FILE: crates/ron-kernel/tests/event_snapshot.rs
#![forbid(unsafe_code)]

use ron_kernel::KernelEvent;
use serde_json::json;

#[test]
fn kernel_event_serde_snapshot() {
    let cases = vec![
        KernelEvent::Health { service: "svc".into(), ok: true },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed { service: "svc".into(), reason: "boom".into() },
        KernelEvent::Shutdown,
    ];

    let got: Vec<serde_json::Value> = cases
        .iter()
        .map(|ev| serde_json::to_value(ev).expect("serialize"))
        .collect();

    // Externally-tagged serde enum representation is intentional and stable.
    let expected = vec![
        json!({ "Health":        { "service": "svc", "ok": true } }),
        json!({ "ConfigUpdated": { "version": 42 } }),
        json!({ "ServiceCrashed":{ "service": "svc", "reason": "boom" } }),
        json!("Shutdown"),
    ];

    assert_eq!(got, expected, "KernelEvent serde snapshot changed");
}

#[test]
fn kernel_event_json_roundtrip() {
    let cases = vec![
        KernelEvent::Health { service: "svc".into(), ok: true },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed { service: "svc".into(), reason: "boom".into() },
        KernelEvent::Shutdown,
    ];

    for ev in cases {
        // Serialize to JSON text…
        let s = serde_json::to_string(&ev).expect("serialize");
        // …and back to the enum.
        let back: KernelEvent = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(ev, back, "roundtrip changed the value");
    }
}
