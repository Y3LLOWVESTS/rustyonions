//! RO:WHAT — Minimal lib API surface (feature `libapi`)
#![allow(dead_code)]
use anyhow::Result;
use crate::{config::Config, readiness::HealthGate, bootstrap::start_runtime};

#[cfg(feature = "libapi")]
pub struct OverlayHandle(pub(crate) crate::supervisor::OverlayRuntime);

#[cfg(feature = "libapi")]
pub async fn spawn(cfg: Config) -> Result<OverlayHandle> {
    let hg = HealthGate::new();
    let rt = start_runtime(cfg, hg).await?;
    Ok(OverlayHandle(rt))
}

#[cfg(feature = "libapi")]
impl OverlayHandle {
    pub async fn shutdown(self) -> Result<()> { self.0.shutdown().await }
}
