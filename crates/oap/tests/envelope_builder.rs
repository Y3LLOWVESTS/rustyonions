use bytes::Bytes;
use oap::{prelude::*, Flags, StatusCode};

#[test]
fn builder_sets_flags_and_fields() {
    let f = FrameBuilder::request(9, 0xDEAD, 777)
        .start_with_cap(Bytes::from_static(b"macaroon"))
        .payload(Bytes::from_static(b"body"))
        .want_ack()
        .end()
        .build();

    let flags = f.header.flags;
    assert!(flags.contains(Flags::REQ));
    assert!(flags.contains(Flags::START));
    assert!(flags.contains(Flags::ACK_REQ));
    assert!(flags.contains(Flags::END));

    // Response builder sanity.
    let r = FrameBuilder::response(9, 0xDEAD, 777, StatusCode::Ok)
        .payload(Bytes::from_static(b"ok"))
        .build();
    assert!(r.header.flags.contains(Flags::RESP));
    assert_eq!(r.header.code, StatusCode::Ok as u16);
}
