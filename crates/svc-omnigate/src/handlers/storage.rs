#![forbid(unsafe_code)]

use anyhow::{anyhow, Context, Result};
use bytes::{Bytes, BytesMut};
use futures_util::SinkExt;
use ron_app_sdk::{OapCodec, OapFlags, OapFrame, OAP_VERSION};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_util::codec::Framed;

use crate::config::Config;
use crate::metrics::Metrics;
use crate::storage::{FsStorage, TILE_APP_PROTO_ID};

#[derive(Deserialize)]
struct GetReq { op: String, path: String }

pub async fn handle_storage_get(
    framed: &mut Framed<TlsStream<TcpStream>, OapCodec>,
    cfg: &Config,
    storage: &FsStorage,
    req: &OapFrame,
    metrics: std::sync::Arc<Metrics>,
) -> Result<()> {
    // Parse JSON body
    let gr: GetReq = serde_json::from_slice(&req.payload).with_context(|| "invalid JSON")?;
    if gr.op.as_str() != "get" {
        return Err(anyhow!("bad op"));
    }

    // Open file (FsStorage enforces max size, safe path)
    let (mut file, _size) = storage.open(&gr.path).await.map_err(|e| {
        if e.to_string().contains("too large") {
            metrics.inc_too_large();
            anyhow!("413 {}", e)
        } else if e.to_string().contains("not a file")
            || e.to_string().contains("No such file")
            || e.to_string().contains("open")
        {
            metrics.inc_not_found();
            anyhow!("404 {}", e)
        } else { e }
    })?;

    // Stream chunks
    let mut sent_any = false;
    let mut chunk = BytesMut::with_capacity(cfg.chunk_bytes);

    loop {
        chunk.clear();
        chunk.reserve(cfg.chunk_bytes);
        use tokio::io::AsyncReadExt;
        let n = file.read_buf(&mut chunk).await?;
        if n == 0 { break; }
        metrics.add_bytes_out(n as u64);

        let mut flags = OapFlags::RESP;
        if !sent_any { flags |= OapFlags::START; sent_any = true; }

        let resp = OapFrame {
            ver: OAP_VERSION,
            flags,
            code: 0,
            app_proto_id: TILE_APP_PROTO_ID,
            tenant_id: req.tenant_id,
            cap: Bytes::new(),
            corr_id: req.corr_id,
            payload: chunk.clone().freeze(),
        };
        framed.send(resp).await?;
        tokio::task::yield_now().await;
    }

    // END (or empty START|END if zero bytes)
    let end_flags = if sent_any { OapFlags::RESP | OapFlags::END } else { OapFlags::RESP | OapFlags::START | OapFlags::END };
    let resp_end = OapFrame {
        ver: OAP_VERSION,
        flags: end_flags,
        code: 0,
        app_proto_id: TILE_APP_PROTO_ID,
        tenant_id: req.tenant_id,
        cap: Bytes::new(),
        corr_id: req.corr_id,
        payload: Bytes::new(),
    };
    framed.send(resp_end).await?;
    Ok(())
}
