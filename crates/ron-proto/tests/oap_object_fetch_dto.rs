//! RO:WHAT — Wire-contract tests for OAP OBJ_GET DTOs.
//! RO:WHY — Lock strict b3 object requests and prevent IP-bearing request drift.
//! RO:INTERACTS — ron_proto::oap::object.
//! RO:INVARIANTS — strict serde; no unknown fields; canonical app protocol ID.
//! RO:TEST — cargo test -p ron-proto --test oap_object_fetch_dto.

use ron_proto::oap::object::{ObjGet, ObjStreamStart, OBJ_GET_APP_PROTO_ID};
use ron_proto::ContentId;

const CID: &str = "b3:6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

#[test]
fn obj_get_roundtrips_without_network_identity() {
    let request = ObjGet {
        obj: CID.parse::<ContentId>().expect("valid b3 CID"),
    };

    let json = serde_json::to_string(&request).expect("serialize OBJ_GET");

    assert_eq!(json, format!(r#"{{"obj":"{CID}"}}"#));
    assert!(!json.contains("ip"));
    assert!(!json.contains("socket"));
    assert!(!json.contains("route"));

    let decoded: ObjGet = serde_json::from_str(&json).expect("deserialize OBJ_GET");

    assert_eq!(decoded, request);
    assert_eq!(OBJ_GET_APP_PROTO_ID, 0x0101);
}

#[test]
fn obj_get_rejects_ip_and_unknown_fields() {
    let json = format!(r#"{{"obj":"{CID}","client_ip":"192.0.2.10"}}"#);

    assert!(
        serde_json::from_str::<ObjGet>(&json).is_err(),
        "OBJ_GET must reject client IP and every other unknown field"
    );
}

#[test]
fn object_stream_start_roundtrips_with_canonical_bounds() {
    let start = ObjStreamStart {
        obj: CID.parse::<ContentId>().expect("valid b3 CID"),
        total_bytes: 131_089,
        chunk_bytes: 65_536,
    };

    let json = serde_json::to_vec(&start).expect("serialize stream start");
    let decoded: ObjStreamStart = serde_json::from_slice(&json).expect("deserialize stream start");

    assert_eq!(decoded, start);
}
