// FILE: crates/ron-kernel/tests/event_snapshot.rs
#![forbid(unsafe_code)]

use std::error::Error;

use ron_kernel::KernelEvent;
use serde_json::{json, Value};

#[test]
fn kernel_event_serde_snapshot() -> Result<(), Box<dyn Error>> {
    let cases = [
        KernelEvent::Health { service: "svc".into(), ok: true },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed { service: "svc".into(), reason: "boom".into() },
        KernelEvent::Shutdown,
    ];

    // Produce the JSON values without expect/unwrap.
    let got: Vec<Value> = cases
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;

    // Externally-tagged serde enum representation is intentional and stable.
    let expected = [
        json!({ "Health":        { "service": "svc", "ok": true } }),
        json!({ "ConfigUpdated": { "version": 42 } }),
        json!({ "ServiceCrashed":{ "service": "svc", "reason": "boom" } }),
        json!("Shutdown"),
    ];

    assert_eq!(got.as_slice(), &expected, "KernelEvent serde snapshot changed");
    Ok(())
}

#[test]
fn kernel_event_json_roundtrip() -> Result<(), Box<dyn Error>> {
    let cases = [
        KernelEvent::Health { service: "svc".into(), ok: true },
        KernelEvent::ConfigUpdated { version: 42 },
        KernelEvent::ServiceCrashed { service: "svc".into(), reason: "boom".into() },
        KernelEvent::Shutdown,
    ];

    for ev in cases {
        // Serialize to JSON text…
        let s = serde_json::to_string(&ev)?;
        // …and back to the enum.
        let back: KernelEvent = serde_json::from_str(&s)?;
        assert_eq!(ev, back, "roundtrip changed the value");
    }

    Ok(())
}
