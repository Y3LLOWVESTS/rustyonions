use bytes::Bytes;
use oap::{writer::OapWriter, writer::WriterConfig, prelude::*, codec::OapDecoder};
use tokio_util::codec::Decoder as _;

fn normalize(mut f: Frame) -> Frame {
    let cap_len = f.cap.as_ref().map(|b| b.len()).unwrap_or(0);
    let payload_len = f.payload.as_ref().map(|b| b.len()).unwrap_or(0);
    f.header.cap_len = cap_len as u16;
    f.header.len = (Header::WIRE_SIZE + cap_len + payload_len) as u32;
    f
}

#[test]
fn encode_to_buf_and_decode_back() {
    // Build a START frame with cap + payload.
    let f = FrameBuilder::request(42, 0xCAFE, 99)
        .start_with_cap(Bytes::from_static(b"scope=read"))
        .payload(Bytes::from_static(b"ping"))
        .want_ack()
        .build();

    // Encode using writer (buffered, no async I/O used).
    let mut w = OapWriter::new(WriterConfig::default());
    w.encode_to_buf(f.clone()).unwrap();
    let bytes = w.take_buf();

    // Decode back.
    let mut dec = OapDecoder::default();
    let mut buf: bytes::BytesMut = bytes.clone().into(); // <-- direct From<Bytes>
    let out = dec.decode(&mut buf).unwrap().unwrap();

    assert_eq!(out, normalize(f));
    assert!(buf.is_empty(), "buffer fully consumed");
}
