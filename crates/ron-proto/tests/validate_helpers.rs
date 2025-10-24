use ron_proto::{oap, ContentId, Limits, Validate};

#[test]
fn hello_and_start_validate() {
    let hello = oap::hello::Hello::default();
    hello.validate(Limits::default()).unwrap();

    let start = oap::start::Start { seq_start: 0, max_frame_bytes: 1_048_576, meta: None };
    start.validate(Limits::default()).unwrap();
}

#[test]
fn data_respects_negotiated_max() {
    let limits = Limits { max_frame_bytes: 8 };

    // Make the type explicit so .parse() knows the target type:
    let cid: ContentId = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        .parse()
        .unwrap();

    let ok = oap::data::Data { obj: cid.clone(), seq: 1, bytes: b"12345678".to_vec() };
    ok.validate(limits).unwrap();

    let too_big = oap::data::Data { obj: cid, seq: 2, bytes: b"abcdefghi".to_vec() };
    assert!(too_big.validate(limits).is_err());
}
