//! RO:WHAT — Fail-closed validation and normalization for ron-accounting Config.
//! RO:WHY — Pillar 12; Concerns: RES/GOV/SEC. Bad config must not start unsafe metering.
//! RO:INTERACTS — config::schema, recorder, exporter, WAL, future HTTP adapter.
//! RO:INVARIANTS — window 60s..3600s; shards power-of-two; amnesia forces WAL disabled.
//! RO:METRICS — future adapters increment config_reload_fail_total on failures.
//! RO:CONFIG — all Config fields.
//! RO:SECURITY — validates body cap and TLS path presence when TLS is enabled.
//! RO:TEST — config unit tests in later batch.

use std::net::SocketAddr;

use crate::{
    config::schema::Config,
    errors::{Error, Result},
    utils::{encode::MAX_CANONICAL_BYTES, time::validate_window_len_s},
};

/// Return a normalized copy of config; amnesia always disables WAL persistence.
pub fn normalize_config(mut cfg: Config) -> Config {
    if cfg.accounting.amnesia {
        cfg.wal.enabled = false;
    }
    cfg
}

/// Validate a config snapshot.
pub fn validate(cfg: &Config) -> Result<()> {
    validate_window_len_s(cfg.accounting.window_len_s)?;

    if cfg.accounting.shards == 0
        || !cfg.accounting.shards.is_power_of_two()
        || cfg.accounting.shards > 4096
    {
        return Err(Error::schema(
            "accounting.shards must be a power of two in [1,4096]",
        ));
    }
    if cfg.accounting.capacity_rows < 1_024 {
        return Err(Error::schema(
            "accounting.capacity_rows must be at least 1024",
        ));
    }
    if cfg.accounting.pending_slices_cap < 64 {
        return Err(Error::schema(
            "accounting.pending_slices_cap must be at least 64",
        ));
    }
    if cfg.accounting.amnesia && cfg.wal.enabled {
        return Err(Error::schema(
            "accounting.amnesia=true requires wal.enabled=false after normalization",
        ));
    }

    if cfg.exporter.ordered_buffer_cap < 1 {
        return Err(Error::schema(
            "exporter.ordered_buffer_cap must be positive",
        ));
    }
    if cfg.exporter.backoff_base_ms == 0
        || cfg.exporter.backoff_cap_ms < cfg.exporter.backoff_base_ms
    {
        return Err(Error::schema(
            "exporter backoff cap must be greater than or equal to positive base",
        ));
    }

    if cfg.wal.enabled {
        if cfg.wal.dir.as_os_str().is_empty() {
            return Err(Error::schema("wal.dir must be set when wal.enabled=true"));
        }
        if cfg.wal.max_bytes < 1_048_576 {
            return Err(Error::schema("wal.max_bytes must be at least 1MiB"));
        }
        if cfg.wal.max_entries < u64::from(cfg.accounting.pending_slices_cap) {
            return Err(Error::schema(
                "wal.max_entries must be >= accounting.pending_slices_cap",
            ));
        }
        if cfg.wal.max_age_s < cfg.accounting.window_len_s {
            return Err(Error::schema(
                "wal.max_age_s must be >= accounting.window_len_s",
            ));
        }
    }

    if cfg.export_http.enabled {
        parse_socket("export_http.bind_addr", &cfg.export_http.bind_addr)?;
        parse_socket("export_http.metrics_addr", &cfg.export_http.metrics_addr)?;
        if cfg.export_http.read_timeout_ms == 0 || cfg.export_http.write_timeout_ms == 0 {
            return Err(Error::schema("HTTP timeouts must be positive"));
        }
        if cfg.export_http.limits.max_body_bytes > MAX_CANONICAL_BYTES as u64 {
            return Err(Error::schema(
                "export_http.limits.max_body_bytes must be <= 1MiB",
            ));
        }
        if cfg.export_http.limits.decompress_ratio_cap == 0 {
            return Err(Error::schema("decompress_ratio_cap must be at least 1"));
        }
        if cfg.export_http.tls.enabled
            && (cfg.export_http.tls.cert_path.as_os_str().is_empty()
                || cfg.export_http.tls.key_path.as_os_str().is_empty())
        {
            return Err(Error::schema(
                "TLS cert_path and key_path are required when TLS is enabled",
            ));
        }
    }

    Ok(())
}

fn parse_socket(name: &str, value: &str) -> Result<SocketAddr> {
    value
        .parse::<SocketAddr>()
        .map_err(|err| Error::schema(format!("{name} is not a socket address: {err}")))
}
