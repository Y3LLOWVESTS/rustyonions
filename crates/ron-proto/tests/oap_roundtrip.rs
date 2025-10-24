use ron_proto::ContentId;
use serde_json as json;

#[test]
fn hello_default_roundtrip() {
    let h = ron_proto::oap::hello::Hello::default();
    let s = json::to_string(&h).unwrap();
    let back: ron_proto::oap::hello::Hello = json::from_str(&s).unwrap();
    assert_eq!(h.protocol, back.protocol);
    assert_eq!(h.version, back.version);
}

#[test]
fn data_frame_roundtrip() {
    let cid: ContentId = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        .parse()
        .unwrap();
    let d = ron_proto::oap::data::Data { obj: cid, seq: 42, bytes: b"hello world".to_vec() };
    let s = json::to_string(&d).unwrap();
    let back: ron_proto::oap::data::Data = json::from_str(&s).unwrap();
    assert_eq!(back.seq, 42);
    assert_eq!(back.bytes, b"hello world");
}

#[test]
fn end_with_error_roundtrip() {
    let e = ron_proto::oap::error::Error {
        code: ron_proto::error::Kind::TooLarge,
        message: "frame exceeds limit".into(),
        detail: Some("max 1MiB".into()),
    };
    let end = ron_proto::oap::end::End { seq_end: 99, ok: false, error: Some(e) };
    let s = json::to_string(&end).unwrap();
    let back: ron_proto::oap::end::End = json::from_str(&s).unwrap();
    assert!(!back.ok);
    assert!(matches!(back.error.as_ref().unwrap().code, ron_proto::error::Kind::TooLarge));
}

#[test]
fn kind_enum_ser_names_match() {
    use ron_proto::oap::OapKind;
    let kinds = [OapKind::Hello, OapKind::Start, OapKind::Data, OapKind::End, OapKind::Error];
    let names: Vec<String> = kinds.iter().map(|k| json::to_string(k).unwrap()).collect();
    assert_eq!(names, vec![r#""HELLO""#, r#""START""#, r#""DATA""#, r#""END""#, r#""ERROR""#]);
}
