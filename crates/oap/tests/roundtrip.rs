#![forbid(unsafe_code)]

use oap::*;
use serde_json::{json, Value};
use tokio::io::duplex;

#[tokio::test]
async fn hello_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);

    let send = async {
        let f = hello_frame("oap/1");
        write_frame(&mut a, &f, DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };

    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert_eq!(fr.ver, OAP_VERSION);
        assert!(matches!(fr.typ, FrameType::Hello));
        let m: Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(m["hello"], "oap/1");
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn start_data_end_with_body() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(1 << 16);

    // sender
    let send = async {
        write_frame(&mut a, &start_frame("tiles/v1"), DEFAULT_MAX_FRAME).await?;

        // DATA with real bytes; header gets obj:"b3:<hex>"
        let body = b"hello world";
        let payload = encode_data_payload(json!({ "mime": "application/octet-stream" }), body)?;
        let df = OapFrame::new(FrameType::Data, payload);
        write_frame(&mut a, &df, DEFAULT_MAX_FRAME).await?;

        write_frame(&mut a, &end_frame(), DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };

    // receiver
    let recv = async {
        // START
        let st = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(st.typ, FrameType::Start));

        // DATA
        let df = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(df.typ, FrameType::Data));

        let (hdr, body) = decode_data_payload(&df.payload)?;
        assert_eq!(hdr["mime"], "application/octet-stream");
        let obj = hdr["obj"].as_str().unwrap_or("");
        assert!(obj.starts_with("b3:"));
        assert_eq!(&body[..], b"hello world");

        // END
        let en = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(en.typ, FrameType::End));
        Ok::<_, OapError>(())
    };

    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn ack_roundtrip() -> Result<(), OapError> {
    let (mut a, mut b) = duplex(4096);
    let send = async {
        write_frame(&mut a, &ack_frame(65536), DEFAULT_MAX_FRAME).await?;
        Ok::<_, OapError>(())
    };
    let recv = async {
        let fr = read_frame(&mut b, DEFAULT_MAX_FRAME).await?;
        assert!(matches!(fr.typ, FrameType::Ack));
        let j: Value = serde_json::from_slice(&fr.payload)?;
        assert_eq!(j["credit"].as_u64(), Some(65536));
        Ok::<_, OapError>(())
    };
    futures::future::try_join(send, recv).await?;
    Ok(())
}

#[tokio::test]
async fn invalid_type_errors() -> Result<(), OapError> {
    // Build a manual bad frame: ver=1, typ=0xFF, len=0
    let mut buf = Vec::new();
    buf.extend_from_slice(&[OAP_VERSION, 0xFF, 0, 0, 0, 0]); // len=0
    let (mut a, mut b) = duplex(64);
    use tokio::io::AsyncWriteExt;
    a.write_all(&buf).await?;

    match read_frame(&mut b, DEFAULT_MAX_FRAME).await {
        Err(OapError::UnknownType(0xFF)) => Ok(()),
        other => panic!("unexpected: {other:?}"),
    }
}
