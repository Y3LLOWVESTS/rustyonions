use omnigate::types::dto::{PingResponse, VersionResponse};

#[test]
fn dto_roundtrips() {
    // VersionResponse now: { version, git }
    let v = VersionResponse {
        version: "0.0.0".to_string(),
        git: Some("deadbeef".to_string()),
    };
    let s = serde_json::to_string(&v).unwrap();
    let _: VersionResponse = serde_json::from_str(&s).unwrap();

    // PingResponse now: { ok }
    let p = PingResponse { ok: true };
    let s = serde_json::to_string(&p).unwrap();
    let _: PingResponse = serde_json::from_str(&s).unwrap();
}
