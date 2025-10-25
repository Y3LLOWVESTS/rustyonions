//! Ensure decoder returns `None` until a full frame is buffered.

use bytes::{Bytes, BytesMut};
use oap::{codec::OapEncoder, prelude::*};
use tokio_util::codec::{Decoder as _, Encoder as _};

fn encode_frame(f: Frame) -> bytes::Bytes {
    let mut enc = OapEncoder::default();
    let mut buf = BytesMut::new();
    enc.encode(f, &mut buf).unwrap();
    buf.freeze()
}

#[test]
fn need_more_before_full_header() {
    let f = FrameBuilder::request(7, 0xAA, 1)
        .payload(Bytes::from_static(b"hello"))
        .build();
    let bytes = encode_frame(f);

    let mut dec = OapDecoder::default();
    let mut buf = BytesMut::new();

    // Feed fewer than the header size — decode must return None.
    buf.extend_from_slice(&bytes[..Header::WIRE_SIZE - 1]);
    assert!(dec.decode(&mut buf).unwrap().is_none());

    // Feed rest — now a frame should be produced.
    buf.extend_from_slice(&bytes[Header::WIRE_SIZE - 1..]);
    let out = dec.decode(&mut buf).unwrap();
    assert!(out.is_some());
}
