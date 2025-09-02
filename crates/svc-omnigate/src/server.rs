#![forbid(unsafe_code)]

use anyhow::Result;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use ron_app_sdk::{OapCodec, OapFlags, OapFrame, OAP_VERSION, DEFAULT_MAX_DECOMPRESSED};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tokio_util::codec::Framed;
use tracing::{error, info};

use crate::config::Config;
use crate::handlers::{handle_hello, handle_storage_get, handle_mailbox};
use crate::metrics::Metrics;
use crate::storage::{FsStorage, TILE_APP_PROTO_ID};
use crate::mailbox::{Mailbox, MAILBOX_APP_PROTO_ID};

pub async fn run(
    cfg: Config,
    acceptor: TlsAcceptor,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let listener = TcpListener::bind(cfg.addr).await?;
    info!("OAP listener on {}", cfg.addr);

    loop {
        let (tcp, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let storage = storage.clone();
        let mailbox = mailbox.clone();
        let metrics = metrics.clone();
        let cfg = cfg.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_conn(acceptor, tcp, cfg, storage, mailbox, metrics).await {
                let msg = e.to_string().to_lowercase();
                if msg.contains("close_notify") || msg.contains("unexpected eof") {
                    info!("conn {peer:?} disconnected: {e}");
                } else {
                    error!("conn {peer:?} error: {e}");
                }
            }
        });
    }
}

async fn handle_conn(
    acceptor: TlsAcceptor,
    tcp: TcpStream,
    cfg: Config,
    storage: Arc<FsStorage>,
    mailbox: Arc<Mailbox>,
    metrics: Arc<Metrics>,
) -> Result<()> {
    let tls = acceptor.accept(tcp).await?;
    let mut framed: Framed<TlsStream<TcpStream>, OapCodec> =
        Framed::new(tls, OapCodec::new(cfg.max_frame, DEFAULT_MAX_DECOMPRESSED));

    while let Some(next) = framed.next().await {
        let frame = next?;
        metrics.inc_requests();

        // capacity gating
        let inflight = metrics.inflight_inc();
        if inflight > cfg.max_inflight {
            metrics.inc_overload();
            metrics.inflight_dec();
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code: 503,
                app_proto_id: frame.app_proto_id,
                tenant_id: frame.tenant_id,
                cap: Bytes::new(),
                corr_id: frame.corr_id,
                payload: Bytes::from_static(br#"{"error":"overloaded","retry_after":"1-5s"}"#),
            };
            let _ = framed.send(resp).await;
            continue;
        }

        let result = match frame.app_proto_id {
            0 => handle_hello(&mut framed, &cfg, &frame).await,
            TILE_APP_PROTO_ID => handle_storage_get(&mut framed, &cfg, &storage, &frame, metrics.clone()).await,
            MAILBOX_APP_PROTO_ID => handle_mailbox(&mut framed, &mailbox, &frame, &metrics).await,
            _ => {
                let resp = OapFrame {
                    ver: OAP_VERSION,
                    flags: OapFlags::RESP | OapFlags::END,
                    code: 404,
                    app_proto_id: frame.app_proto_id,
                    tenant_id: frame.tenant_id,
                    cap: Bytes::new(),
                    corr_id: frame.corr_id,
                    payload: Bytes::from_static(br#"{"error":"unknown app_proto_id"}"#),
                };
                framed.send(resp).await?;
                Ok(())
            }
        };

        metrics.inflight_dec();

        if let Err(e) = result {
            let (code, body) = map_err(&e);
            let resp = OapFrame {
                ver: OAP_VERSION,
                flags: OapFlags::RESP | OapFlags::END,
                code,
                app_proto_id: frame.app_proto_id,
                tenant_id: frame.tenant_id,
                cap: Bytes::new(),
                corr_id: frame.corr_id,
                payload: Bytes::from(body),
            };
            let _ = framed.send(resp).await;
        }
    }

    Ok(())
}

fn map_err(e: &anyhow::Error) -> (u16, Vec<u8>) {
    let s = e.to_string();
    if s.contains("413") || s.contains("too large") {
        (413, br#"{"error":"too_large"}"#.to_vec())
    } else if s.contains("404")
        || s.contains("No such file")
        || s.contains("not a file")
        || s.contains("open")
        || s.contains("not_found")
    {
        (404, br#"{"error":"not_found"}"#.to_vec())
    } else if s.contains("invalid json") || s.contains("bad_request") || s.contains("bad op") {
        (400, br#"{"error":"bad_request"}"#.to_vec())
    } else {
        (500, br#"{"error":"internal"}"#.to_vec())
    }
}
