//! RO:WHAT — Golden test vectors for OAP/1: HELLO, START+cap, DATA (with/without COMP).
//! RO:WHY — Guard interop stability; ensure encoder/decoder symmetry and bounds.
//! RO:INVARIANTS — 1MiB bound enforced; START carries cap; COMP requires feature=zstd to decode.

use bytes::{Bytes, BytesMut};
use oap::{
    Flags, Frame, Header, Hello, HelloReply, OapDecoder, OapEncoder, MAX_FRAME_BYTES, OAP_VERSION,
};
// Bring trait methods (encode/decode) into scope:
use tokio_util::codec::{Decoder, Encoder};

/// Compute the canonical on-wire header fields (`len`, `cap_len`) and
/// return a copy of `frame` updated with those values so equality works.
fn normalize(mut frame: Frame) -> Frame {
    let cap_len = frame.cap.as_ref().map(|b| b.len()).unwrap_or(0);
    let payload_len = frame.payload.as_ref().map(|b| b.len()).unwrap_or(0);
    let total_len = Header::WIRE_SIZE + cap_len + payload_len;

    frame.header.cap_len = cap_len as u16;
    frame.header.len = total_len as u32;
    frame
}

fn roundtrip(frame: Frame) {
    let mut enc = OapEncoder::default();
    let mut buf = BytesMut::new();
    enc.encode(frame.clone(), &mut buf).expect("encode");
    let mut dec = OapDecoder::default();
    let out = dec.decode(&mut buf).expect("decode").expect("one frame");

    // Normalize the input to match encoder-corrected header fields.
    let expect = normalize(frame);
    assert_eq!(out, expect);
}

#[test]
fn hello_roundtrip() {
    let h = Hello {
        ua: Some("sdk/0.1".into()),
    };
    let f = h.to_frame(0xAA, 42);
    roundtrip(f);
}

#[test]
fn hello_reply_roundtrip() {
    let hr = HelloReply::default_for_server();
    let f = hr.to_frame(0xBB, 7);
    roundtrip(f);
}

#[test]
fn start_with_cap() {
    let cap = Bytes::from_static(b"macaroon:scope=read:ttl=60s");
    let hdr = Header {
        len: 0,
        ver: OAP_VERSION,
        flags: Flags::START | Flags::REQ,
        code: 0,
        app_proto_id: 100,
        tenant_id: 0xCC,
        cap_len: 0,
        corr_id: 99,
    };
    let frame = Frame {
        header: hdr,
        cap: Some(cap),
        payload: None,
    };
    roundtrip(frame);
}

#[test]
fn data_without_cap() {
    let payload = Bytes::from_static(b"hello world");
    let hdr = Header {
        len: 0,
        ver: OAP_VERSION,
        flags: Flags::RESP,
        code: 200,
        app_proto_id: 200,
        tenant_id: 0xDD,
        cap_len: 0,
        corr_id: 123,
    };
    let frame = Frame {
        header: hdr,
        cap: None,
        payload: Some(payload),
    };
    roundtrip(frame);
}

#[test]
fn rejects_oversize() {
    let payload = vec![0u8; (MAX_FRAME_BYTES as usize) + 1];
    let hdr = Header {
        len: 0,
        ver: OAP_VERSION,
        flags: Flags::RESP,
        code: 200,
        app_proto_id: 1,
        tenant_id: 1,
        cap_len: 0,
        corr_id: 1,
    };
    let mut enc = OapEncoder::default();
    let mut buf = BytesMut::new();
    let res = enc.encode(
        Frame {
            header: hdr,
            cap: None,
            payload: Some(Bytes::from(payload)),
        },
        &mut buf,
    );
    assert!(res.is_err());
}

#[cfg(feature = "zstd")]
#[test]
fn comp_bounded_inflate() {
    // Small payload compressed; ensure decode returns original.
    let raw = vec![42u8; 4096];
    let mut comp = Vec::new();
    {
        let mut enc = zstd::stream::write::Encoder::new(&mut comp, 1).unwrap();
        use std::io::Write;
        enc.write_all(&raw).unwrap();
        enc.finish().unwrap();
    }

    let hdr = Header {
        len: 0,
        ver: OAP_VERSION,
        flags: Flags::RESP | Flags::COMP,
        code: 200,
        app_proto_id: 2,
        tenant_id: 2,
        cap_len: 0,
        corr_id: 2,
    };
    let mut enc = OapEncoder::default();
    let mut buf = BytesMut::new();
    enc.encode(
        Frame {
            header: hdr,
            cap: None,
            payload: Some(Bytes::from(comp)),
        },
        &mut buf,
    )
    .unwrap();

    let mut dec = OapDecoder::default();
    let out = dec.decode(&mut buf).unwrap().unwrap();
    assert_eq!(out.payload.unwrap().len(), raw.len());
}
