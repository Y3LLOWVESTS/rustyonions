use bytes::Bytes;
use oap::{parser::ParserState, parser::ParserConfig, prelude::*, codec::OapEncoder};
use tokio_util::codec::Encoder as _;

fn normalize(mut f: Frame) -> Frame {
    let cap_len = f.cap.as_ref().map(|b| b.len()).unwrap_or(0);
    let payload_len = f.payload.as_ref().map(|b| b.len()).unwrap_or(0);
    f.header.cap_len = cap_len as u16;
    f.header.len = (Header::WIRE_SIZE + cap_len + payload_len) as u32;
    f
}

#[test]
fn parses_frames_across_chunks() {
    // Build two frames and encode into one buffer.
    let f1 = FrameBuilder::request(7, 0xA, 1)
        .payload(Bytes::from_static(b"hello"))
        .end()
        .build();
    let f2 = FrameBuilder::response(7, 0xA, 1, StatusCode::Ok)
        .payload(Bytes::from_static(b"world"))
        .build();

    let mut enc = OapEncoder::default();
    let mut buf = bytes::BytesMut::new();
    enc.encode(f1.clone(), &mut buf).unwrap();
    enc.encode(f2.clone(), &mut buf).unwrap();
    let bytes = buf.freeze();

    // Feed in small chunks to the parser.
    let mut p = ParserState::new(ParserConfig::default());
    for chunk in bytes.chunks(3) {
        p.push(chunk).unwrap();
    }

    let frames = p.drain().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], normalize(f1));
    assert_eq!(frames[1], normalize(f2));
}
