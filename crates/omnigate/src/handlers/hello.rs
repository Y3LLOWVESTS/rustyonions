#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use futures_util::SinkExt;
use ron_app_sdk::{Hello, OapCodec, OapFlags, OapFrame, OAP_VERSION};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::config::Config;

pub async fn handle_hello(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    cfg: &Config,
    req: &OapFrame,
) -> Result<()> {
    let hello = Hello {
        server_version: "svc-omnigate-dev-1.0.0".into(),
        max_frame: cfg.max_frame as u64,
        max_inflight: cfg.max_inflight,
        supported_flags: vec![
            "EVENT".into(),
            "ACK_REQ".into(),
            "COMP".into(),
            "APP_E2E".into(),
        ],
        oap_versions: vec![OAP_VERSION],
        transports: vec!["tcp+tls".into()],
    };
    let body = serde_json::to_vec(&hello)?;
    let resp = OapFrame {
        ver: OAP_VERSION,
        flags: OapFlags::RESP | OapFlags::END,
        code: 0,
        app_proto_id: 0,
        tenant_id: req.tenant_id,
        cap: Bytes::new(),
        corr_id: req.corr_id,
        payload: Bytes::from(body),
    };
    framed.send(resp).await?;
    Ok(())
}
