//! Parser config soft-cap behavior and minimal builder sanity.

use bytes::Bytes;
use oap::{parser::ParserConfig, parser::ParserState, prelude::*};

#[test]
fn parser_soft_cap_trips() {
    // Tiny soft cap to trigger error on push.
    let mut p = ParserState::new(ParserConfig { max_buffer_bytes: Some(8) });
    // Push more than 8 bytes; we expect a decode error (soft-cap signal).
    let err = p.push(&[0u8; 16]).expect_err("should hit soft-cap");
    // We reuse PayloadOutOfBounds to signal backpressure.
    match err {
        oap::OapDecodeError::PayloadOutOfBounds => {}
        e => panic!("unexpected error: {e:?}"),
    }
}

#[test]
fn builder_minimal() {
    // Basic request/response builders compile and set flags.
    let r = FrameBuilder::request(7, 0xCAFE, 1)
        .payload(Bytes::from_static(b"ping"))
        .build();
    assert!(r.header.flags.contains(Flags::REQ));

    let s = FrameBuilder::response(7, 0xCAFE, 1, StatusCode::Ok)
        .payload(Bytes::from_static(b"pong"))
        .build();
    assert!(s.header.flags.contains(Flags::RESP));
}
