//! Config validation + normalization.
use super::model::Config;

pub fn validate_config(c: &Config) -> anyhow::Result<()> {
    // existing checks
    if c.max_conns == 0 {
        anyhow::bail!("max_conns must be > 0");
    }
    if !c.bind_addr.contains(':') {
        anyhow::bail!("bind_addr must be host:port");
    }
    if !c.metrics_addr.contains(':') {
        anyhow::bail!("metrics_addr must be host:port");
    }

    // new structured guards (simple sanity; we avoid parsing times here)
    if c.limits.max_request_bytes == 0 {
        anyhow::bail!("limits.max_request_bytes must be > 0");
    }
    if c.timeouts.request_ms == 0 {
        anyhow::bail!("timeouts.request_ms must be > 0");
    }
    if c.sse.heartbeat_ms == 0 {
        anyhow::bail!("sse.heartbeat_ms must be > 0");
    }
    if c.sse.max_clients == 0 {
        anyhow::bail!("sse.max_clients must be > 0");
    }

    Ok(())
}
