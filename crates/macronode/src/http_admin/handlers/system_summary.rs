// crates/macronode/src/http_admin/handlers/system_summary.rs

//! RO:WHAT — `/api/v1/system/summary` handler (CPU/RAM + optional network rate).
//! RO:WHY  — Let dashboards render truthful node “preview tiles” without scraping.
//! RO:INVARIANTS —
//!   - No unsafe code (macronode forbids unsafe).
//!   - Bytes are real bytes (u64).
//!   - Network rates require two samples; first call returns null rates.
//!   - No lock held across .await (we do no awaits while locked).

use std::{
    sync::OnceLock,
    time::{Instant, SystemTime},
};

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use parking_lot::Mutex;
use serde::Serialize;
use sysinfo::{Networks, System};

use crate::{
    observability::metrics::{observe_facet_error, observe_facet_ok},
    types::AppState,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSummaryDto {
    pub updated_at: String,

    /// 0..=100 (best-effort). Some platforms need two samples before this stabilizes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<f32>,

    pub ram_total_bytes: u64,
    pub ram_used_bytes: u64,

    /// Network receive rate in bytes/sec (best-effort; None until we have 2 samples).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_rx_bps: Option<u64>,

    /// Network transmit rate in bytes/sec (best-effort; None until we have 2 samples).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_tx_bps: Option<u64>,
}

#[derive(Debug)]
struct NetPrev {
    rx_total: u64,
    tx_total: u64,
    at: Instant,
}

#[derive(Debug)]
struct Sampler {
    sys: System,
    nets: Networks,
    prev_net: Option<NetPrev>,
}

impl Sampler {
    fn new() -> Self {
        // sysinfo 0.30: System is used for CPU/memory; Networks is separate.
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();

        let mut nets = Networks::new_with_refreshed_list();
        nets.refresh();

        Self {
            sys,
            nets,
            prev_net: None,
        }
    }

    fn sample(&mut self) -> SystemSummaryDto {
        // Refresh CPU/mem
        self.sys.refresh_cpu();
        self.sys.refresh_memory();

        // Refresh networks
        self.nets.refresh();

        // CPU usage (best-effort)
        let cpu = self.sys.global_cpu_info().cpu_usage();
        let cpu_percent = if cpu.is_finite() && cpu >= 0.0 {
            Some(cpu.min(100.0))
        } else {
            None
        };

        // Memory (sysinfo 0.30 reports KiB)
        let ram_total_bytes = self.sys.total_memory().saturating_mul(1024);
        let ram_used_bytes = self.sys.used_memory().saturating_mul(1024);

        // Network totals (bytes since boot per iface)
        let mut rx_total: u64 = 0;
        let mut tx_total: u64 = 0;

        for (_name, data) in self.nets.iter() {
            rx_total = rx_total.saturating_add(data.received());
            tx_total = tx_total.saturating_add(data.transmitted());
        }

        // Convert totals -> rate based on delta.
        let now = Instant::now();
        let (net_rx_bps, net_tx_bps) = match self.prev_net.take() {
            None => {
                self.prev_net = Some(NetPrev {
                    rx_total,
                    tx_total,
                    at: now,
                });
                (None, None)
            }
            Some(prev) => {
                let dt = now.duration_since(prev.at);
                let dt_secs = dt.as_secs_f64();

                // Require a minimum dt so we don't show nonsense from ultra-fast polling.
                if dt_secs < 0.2 {
                    self.prev_net = Some(NetPrev {
                        rx_total,
                        tx_total,
                        at: now,
                    });
                    (None, None)
                } else {
                    let rx_bps = {
                        let dx = rx_total.saturating_sub(prev.rx_total) as f64;
                        Some((dx / dt_secs).round() as u64)
                    };
                    let tx_bps = {
                        let dx = tx_total.saturating_sub(prev.tx_total) as f64;
                        Some((dx / dt_secs).round() as u64)
                    };

                    self.prev_net = Some(NetPrev {
                        rx_total,
                        tx_total,
                        at: now,
                    });

                    (rx_bps, tx_bps)
                }
            }
        };

        let updated_at = humantime::format_rfc3339(SystemTime::now()).to_string();

        SystemSummaryDto {
            updated_at,
            cpu_percent,
            ram_total_bytes,
            ram_used_bytes,
            net_rx_bps,
            net_tx_bps,
        }
    }
}

static SAMPLER: OnceLock<Mutex<Sampler>> = OnceLock::new();

pub async fn handler(State(_state): State<AppState>) -> impl IntoResponse {
    const FACET: &str = "admin.system.summary";

    let sampler = SAMPLER.get_or_init(|| Mutex::new(Sampler::new()));

    // No awaits while locked.
    let dto = {
        let mut g = sampler.lock();
        g.sample()
    };

    if dto.ram_total_bytes == 0 {
        observe_facet_error(FACET);
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "system_stats_unavailable" })),
        )
            .into_response();
    }

    observe_facet_ok(FACET);
    (StatusCode::OK, Json(dto)).into_response()
}
