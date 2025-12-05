// crates/svc-admin/src/metrics/prometheus_bridge.rs

//! Prometheus bridge for svc-admin.
//!
//! This module is intentionally small for now: we expose a `/metrics`
//! handler and register a couple of node inventory gauges. Later
//! slices will add per-facet metrics, fan-out stats, etc.

use crate::config::Config;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, IntGauge, IntGaugeVec, Opts, TextEncoder};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Total number of nodes known to svc-admin.
static NODES_TOTAL: OnceLock<IntGauge> = OnceLock::new();

/// Node counts grouped by environment.
static NODES_BY_ENV: OnceLock<IntGaugeVec> = OnceLock::new();

fn nodes_total() -> &'static IntGauge {
    NODES_TOTAL.get_or_init(|| {
        let gauge = IntGauge::with_opts(
            Opts::new(
                "svc_admin_nodes_total",
                "Total number of nodes registered in svc-admin.",
            )
            .namespace("ron"),
        )
        .expect("svc_admin_nodes_total gauge must be constructible");

        prometheus::default_registry()
            .register(Box::new(gauge.clone()))
            .expect("svc_admin_nodes_total must register successfully");

        gauge
    })
}

fn nodes_by_env() -> &'static IntGaugeVec {
    NODES_BY_ENV.get_or_init(|| {
        let vec = IntGaugeVec::new(
            Opts::new(
                "svc_admin_nodes_by_env",
                "Number of nodes by environment.",
            )
            .namespace("ron"),
            &["environment"],
        )
        .expect("svc_admin_nodes_by_env gauge must be constructible");

        prometheus::default_registry()
            .register(Box::new(vec.clone()))
            .expect("svc_admin_nodes_by_env must register successfully");

        vec
    })
}

/// Initialize static node inventory metrics from the config.
///
/// This should be called once at startup; the metrics are effectively
/// static until we add dynamic config reloads.
pub fn init_node_inventory_metrics(config: &Config) {
    let total = config.nodes.len() as i64;
    nodes_total().set(total);

    let mut by_env: HashMap<&str, i64> = HashMap::new();
    for node in config.nodes.values() {
        let env = node.environment.as_str();
        *by_env.entry(env).or_insert(0) += 1;
    }

    let gauge_vec = nodes_by_env();
    // Clear any existing values before setting.
    gauge_vec.reset();

    for (env, count) in by_env {
        gauge_vec.with_label_values(&[env]).set(count);
    }
}

/// Axum handler for `/metrics`.
pub async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    if let Err(err) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!(error = ?err, "failed to encode Prometheus metrics");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let body = match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(error = ?err, "metrics output was not valid UTF-8");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, encoder.format_type().to_string())],
        body,
    )
        .into_response()
}
