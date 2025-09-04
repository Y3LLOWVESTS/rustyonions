#![forbid(unsafe_code)]

use oap::*;
use serde_json::json;
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn hello_roundtrip() {
    let (mut a, mut b) = duplex(4096);

    let send = async {
        let f = hello_frame("oap/1");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await.unwrap();
        Ok::<_, OapError>(())
    };

    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert_eq!(fr.ver, OAP_VERSION);
        assert_eq!(fr.typ as u8, FrameType::Hello as u8);
        let m: serde_json::Value = serde_json::from_slice(&fr.payload).unwrap();
        assert_eq!(m["hello"], "oap/1");
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await.unwrap();
}

#[tokio::test]
async fn start_data_end_with_body() {
    let (mut a, mut b) = duplex(1 << 16);

    // sender
    let send = async {
        write_frame(&mut a, &start_frame("tiles/v1"), DEFAULT_MAX_FRAME).await.unwrap();

        // DATA with real bytes; header gets obj:"b3:<hex>"
        let body = b"hello world";
        let payload = encode_data_payload(json!({ "mime": "application/octet-stream" }), body).unwrap();
        let df = OapFrame::new(FrameType::Data, payload);
        write_frame(&mut a, &df, DEFAULT_MAX_FRAME).await.unwrap();

        write_frame(&mut a, &end_frame(), DEFAULT_MAX_FRAME).await.unwrap();
        Ok::<_, OapError>(())
    };

    // receiver
    let recv = async {
        // START
        let st = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert!(matches!(st.typ, FrameType::Start));

        // DATA
        let df = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert!(matches!(df.typ, FrameType::Data));

        let (hdr, body) = decode_data_payload(&df.payload).unwrap();
        assert_eq!(hdr["mime"], "application/octet-stream");
        let obj = hdr["obj"].as_str().unwrap();
        assert!(obj.starts_with("b3:"));
        assert_eq!(&body[..], b"hello world");

        // END
        let en = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert!(matches!(en.typ, FrameType::End));
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await.unwrap();
}

#[tokio::test]
async fn ack_roundtrip() {
    let (mut a, mut b) = duplex(4096);
    let send = async {
        write_frame(&mut a, &ack_frame(65536), DEFAULT_MAX_FRAME).await.unwrap();
        Ok::<_, OapError>(())
    };
    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap();
        assert!(matches!(fr.typ, FrameType::Ack));
        let j: serde_json::Value = serde_json::from_slice(&fr.payload).unwrap();
        assert_eq!(j["credit"].as_u64(), Some(65536));
        Ok::<_, OapError>(())
    };
    futures::future::try_join(send, recv).await.unwrap();
}

#[tokio::test]
async fn invalid_type_errors() {
    // Build a manual bad frame: ver=1, typ=0xFF, len=0
    let mut buf = Vec::new();
    buf.extend_from_slice(&[OAP_VERSION, 0xFF, 0, 0, 0, 0]); // len=0
    let (mut a, mut b) = duplex(64);
    a.write_all(&buf).await.unwrap();

    let err = read_frame(&mut b, DEFAULT_MAX_FRAME).await.unwrap_err();
    match err {
        OapError::UnknownType(0xFF) => {}
        other => panic!("unexpected: {other:?}"),
    }
}
