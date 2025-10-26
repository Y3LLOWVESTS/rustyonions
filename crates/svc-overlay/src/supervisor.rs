//! RO:WHAT — Runtime supervisor for overlay loops
use crate::{admin::ReadyProbe, config::Config, listener};
use anyhow::Result;
use tokio::task::JoinHandle;
use tracing::{info, warn};

pub struct OverlayRuntime {
    join: JoinHandle<()>,
    stop: tokio::sync::oneshot::Sender<()>,
    listener: Option<listener::ListenerHandle>,
}

impl OverlayRuntime {
    pub async fn start(cfg: Config, probe: ReadyProbe) -> Result<Self> {
        // Bind listener first; flips /readyz to green when successful.
        let lh = listener::spawn_listener(&cfg, &probe).await?;

        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let join = tokio::spawn(async move {
            info!("overlay supervisor running");
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("overlay supervisor stopping");
                        break;
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(250)) => {
                        // place periodic tasks here (samplers, house-keeping)
                    }
                }
            }
        });

        Ok(Self {
            join,
            stop: stop_tx,
            listener: Some(lh),
        })
    }

    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(lh) = self.listener.take() {
            lh.shutdown().await?;
        }
        let _ = self.stop.send(());
        if let Err(e) = self.join.await {
            warn!(error=?e, "overlay supervisor join error");
        }
        Ok(())
    }
}
