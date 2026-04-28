//! RO:WHAT — Node-local network + request accounting with rollups and chart-ready series.
//! RO:WHY  — Pillar Observability; Concerns: PERF/RES/DX. svc-admin needs truthful bytes+req windows.
//! RO:INTERACTS — http_admin::middleware::request_accounting, http_admin::handlers::system_net_accounting
//! RO:INVARIANTS — no lock across .await; bounded memory (fixed ring buffers); monotonic totals best-effort
//! RO:METRICS/LOGS — exposes JSON DTO for dashboard; (separate from Prometheus facet counters)
//! RO:CONFIG — tick interval fixed at 1s in this slice
//! RO:SECURITY — no PII; aggregates only
//! RO:TEST — exercised via curl against /api/v1/system/net/accounting

use std::{
    collections::BTreeMap,
    sync::OnceLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use serde::Serialize;
use sysinfo::Networks;
use tokio::time;

use crate::supervisor::ShutdownToken;

const TICK: Duration = Duration::from_secs(1);

const SEC_RING: usize = 60; // last 60 seconds (rollup "minute")
const MIN_RING: usize = 60; // last 60 minutes (seriesMinute + rollup "hour")
const HOUR_RING: usize = 24; // last 24 hours (seriesHour + rollup "day")
const DAY_RING: usize = 30; // last 30 days (seriesDay + rollup "month")
const MONTH_RING: usize = 12; // last 12 "months" (seriesMonth), where month = 30 days

const DAY_SECS: u64 = 86_400;
const MONTH_SECS: u64 = 30 * DAY_SECS;

#[derive(Clone, Copy, Debug)]
#[repr(usize)]
enum FacetId {
    Healthz = 0,
    Readyz = 1,
    Metrics = 2,
    Version = 3,
    Status = 4,

    SystemSummary = 5,
    SystemNetAccounting = 6,

    StorageSummary = 7,
    StorageDatabases = 8,
    StorageDatabaseDetail = 9,

    BenchRun = 10,
    BenchStatus = 11,
    BenchResult = 12,

    DebugCrash = 13,

    Other = 14,
}
const FACET_COUNT: usize = 15;

impl FacetId {
    fn as_str(self) -> &'static str {
        match self {
            FacetId::Healthz => "admin.healthz",
            FacetId::Readyz => "admin.readyz",
            FacetId::Metrics => "admin.metrics",
            FacetId::Version => "admin.version",
            FacetId::Status => "admin.status",
            FacetId::SystemSummary => "admin.system.summary",
            FacetId::SystemNetAccounting => "admin.system.net_accounting",
            FacetId::StorageSummary => "admin.storage.summary",
            FacetId::StorageDatabases => "admin.storage.databases",
            FacetId::StorageDatabaseDetail => "admin.storage.database_detail",
            FacetId::BenchRun => "admin.bench.run",
            FacetId::BenchStatus => "admin.bench.status",
            FacetId::BenchResult => "admin.bench.result",
            FacetId::DebugCrash => "admin.debug_crash",
            FacetId::Other => "admin.other",
        }
    }
}

fn facet_from_path(path: &str) -> FacetId {
    // Keep cardinality LOW. Group by stable prefixes; do NOT explode by IDs/names.
    match path {
        "/healthz" => FacetId::Healthz,
        "/readyz" => FacetId::Readyz,
        "/metrics" => FacetId::Metrics,
        "/version" => FacetId::Version,
        "/api/v1/status" => FacetId::Status,

        "/api/v1/system/summary" => FacetId::SystemSummary,
        "/api/v1/system/net/accounting" => FacetId::SystemNetAccounting,

        "/api/v1/storage/summary" => FacetId::StorageSummary,
        "/api/v1/storage/databases" => FacetId::StorageDatabases,

        "/api/v1/bench/run" => FacetId::BenchRun,

        "/api/v1/debug/crash" => FacetId::DebugCrash,

        _ if path.starts_with("/api/v1/storage/databases/") => FacetId::StorageDatabaseDetail,
        _ if path.starts_with("/api/v1/bench/runs/") && path.ends_with("/result") => {
            FacetId::BenchResult
        }
        _ if path.starts_with("/api/v1/bench/runs/") => FacetId::BenchStatus,
        _ => FacetId::Other,
    }
}

#[derive(Clone, Copy)]
struct Bucket {
    epoch: u64,
    rx: u64,
    tx: u64,
    req_by_facet: [u64; FACET_COUNT],
}
impl Bucket {
    fn new(epoch: u64) -> Self {
        Self {
            epoch,
            rx: 0,
            tx: 0,
            req_by_facet: [0; FACET_COUNT],
        }
    }
    fn reset(&mut self, epoch: u64) {
        *self = Bucket::new(epoch);
    }
    fn add_bytes(&mut self, rx: u64, tx: u64) {
        self.rx = self.rx.saturating_add(rx);
        self.tx = self.tx.saturating_add(tx);
    }
    fn add_reqs(&mut self, reqs: &[u64; FACET_COUNT]) {
        for i in 0..FACET_COUNT {
            self.req_by_facet[i] = self.req_by_facet[i].saturating_add(reqs[i]);
        }
    }
    fn req_total(&self) -> u64 {
        self.req_by_facet.iter().copied().sum()
    }
    fn total_bytes(&self) -> u64 {
        self.rx.saturating_add(self.tx)
    }
}

struct Ring {
    unit_secs: u64,
    cursor: usize,
    last_epoch: u64,
    buckets: Vec<Bucket>,
}
impl Ring {
    fn new(len: usize, unit_secs: u64, now_epoch: u64) -> Self {
        let mut buckets = vec![Bucket::new(0); len];
        // Pre-seed epochs so series timestamps are stable from the first request.
        // Values are zero until we observe traffic.
        // Oldest bucket has epoch = now_epoch - (len-1).
        for i in 0..len {
            let e = now_epoch.saturating_sub((len - 1 - i) as u64);
            buckets[i] = Bucket::new(e);
        }
        Self {
            unit_secs,
            cursor: len - 1, // last element corresponds to now_epoch
            last_epoch: now_epoch,
            buckets,
        }
    }

    fn advance_to(&mut self, now_epoch: u64) {
        if now_epoch <= self.last_epoch {
            return;
        }
        let delta = now_epoch - self.last_epoch;

        // If time jumped beyond the ring window, just reset cleanly.
        if delta as usize >= self.buckets.len() {
            let len = self.buckets.len();
            for i in 0..len {
                let e = now_epoch.saturating_sub((len - 1 - i) as u64);
                self.buckets[i].reset(e);
            }
            self.cursor = len - 1;
            self.last_epoch = now_epoch;
            return;
        }

        for step in 0..delta {
            let e = self.last_epoch + 1 + step;
            self.cursor = (self.cursor + 1) % self.buckets.len();
            self.buckets[self.cursor].reset(e);
        }
        self.last_epoch = now_epoch;
    }

    fn current_mut(&mut self) -> &mut Bucket {
        &mut self.buckets[self.cursor]
    }

    fn sum_all(&self) -> Bucket {
        let mut out = Bucket::new(self.last_epoch);
        for b in &self.buckets {
            out.rx = out.rx.saturating_add(b.rx);
            out.tx = out.tx.saturating_add(b.tx);
            for i in 0..FACET_COUNT {
                out.req_by_facet[i] = out.req_by_facet[i].saturating_add(b.req_by_facet[i]);
            }
        }
        out
    }

    fn series(&self) -> Vec<Bucket> {
        // Oldest -> newest
        let len = self.buckets.len();
        let mut out = Vec::with_capacity(len);
        // Cursor is newest. Oldest is cursor+1.
        for i in 0..len {
            let idx = (self.cursor + 1 + i) % len;
            out.push(self.buckets[idx]);
        }
        out
    }
}

struct NetAccountingState {
    started_at_unix: u64,
    tick_count: u64,

    rx_total: u64,
    tx_total: u64,
    rx_bps: Option<u64>,
    tx_bps: Option<u64>,

    pending_req_by_facet: [u64; FACET_COUNT],

    ring_sec: Ring,
    ring_min: Ring,
    ring_hour: Ring,
    ring_day: Ring,
    ring_month: Ring,
}

impl NetAccountingState {
    fn new(now_secs: u64) -> Self {
        Self {
            started_at_unix: now_secs,
            tick_count: 0,
            rx_total: 0,
            tx_total: 0,
            rx_bps: None,
            tx_bps: None,
            pending_req_by_facet: [0; FACET_COUNT],
            ring_sec: Ring::new(SEC_RING, 1, now_secs),
            ring_min: Ring::new(MIN_RING, 60, now_secs / 60),
            ring_hour: Ring::new(HOUR_RING, 3600, now_secs / 3600),
            ring_day: Ring::new(DAY_RING, DAY_SECS, now_secs / DAY_SECS),
            ring_month: Ring::new(MONTH_RING, MONTH_SECS, now_secs / MONTH_SECS),
        }
    }

    fn record_request(&mut self, facet: FacetId) {
        let idx = facet as usize;
        self.pending_req_by_facet[idx] = self.pending_req_by_facet[idx].saturating_add(1);
    }

    fn on_sample(
        &mut self,
        now_secs: u64,
        rx_total: u64,
        tx_total: u64,
        rx_delta: u64,
        tx_delta: u64,
        rx_bps: Option<u64>,
        tx_bps: Option<u64>,
    ) {
        self.tick_count = self.tick_count.saturating_add(1);

        self.rx_total = rx_total;
        self.tx_total = tx_total;
        self.rx_bps = rx_bps;
        self.tx_bps = tx_bps;

        // Rotate rings to "now".
        self.ring_sec.advance_to(now_secs);
        self.ring_min.advance_to(now_secs / 60);
        self.ring_hour.advance_to(now_secs / 3600);
        self.ring_day.advance_to(now_secs / DAY_SECS);
        self.ring_month.advance_to(now_secs / MONTH_SECS);

        // Drain pending request counters into THIS tick's buckets.
        let reqs = self.pending_req_by_facet;
        self.pending_req_by_facet = [0; FACET_COUNT];

        // Attribute the tick deltas to all current buckets.
        self.ring_sec.current_mut().add_bytes(rx_delta, tx_delta);
        self.ring_sec.current_mut().add_reqs(&reqs);

        self.ring_min.current_mut().add_bytes(rx_delta, tx_delta);
        self.ring_min.current_mut().add_reqs(&reqs);

        self.ring_hour.current_mut().add_bytes(rx_delta, tx_delta);
        self.ring_hour.current_mut().add_reqs(&reqs);

        self.ring_day.current_mut().add_bytes(rx_delta, tx_delta);
        self.ring_day.current_mut().add_reqs(&reqs);

        self.ring_month.current_mut().add_bytes(rx_delta, tx_delta);
        self.ring_month.current_mut().add_reqs(&reqs);
    }

    fn observed_seconds(&self, now_secs: u64) -> u64 {
        now_secs.saturating_sub(self.started_at_unix)
    }
}

static STATE: OnceLock<Mutex<NetAccountingState>> = OnceLock::new();
static STARTED: OnceLock<()> = OnceLock::new();

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn state() -> &'static Mutex<NetAccountingState> {
    STATE.get_or_init(|| Mutex::new(NetAccountingState::new(now_unix_secs())))
}

/// Start the background sampler exactly once.
pub fn ensure_started(shutdown: ShutdownToken) {
    // Always ensure state exists so handlers can return "warming" snapshots even if not started.
    let _ = state();

    if STARTED.set(()).is_err() {
        return;
    }

    tokio::spawn(async move {
        let mut nets = Networks::new_with_refreshed_list();

        let mut prev_rx_total: Option<u64> = None;
        let mut prev_tx_total: Option<u64> = None;
        let mut prev_at: Option<std::time::Instant> = None;

        let mut ticker = time::interval(TICK);

        loop {
            if shutdown.is_triggered() {
                break;
            }

            ticker.tick().await;

            nets.refresh();

            // Sum across interfaces; node-wide view is enough for v1.
            let mut rx_total = 0u64;
            let mut tx_total = 0u64;
            for (_name, data) in nets.iter() {
                rx_total = rx_total.saturating_add(data.total_received());
                tx_total = tx_total.saturating_add(data.total_transmitted());
            }

            let now_secs = now_unix_secs();
            let now_inst = std::time::Instant::now();

            let (rx_delta, tx_delta, rx_bps, tx_bps) = match (prev_rx_total, prev_tx_total, prev_at)
            {
                (Some(prx), Some(ptx), Some(pat)) => {
                    let dt = now_inst.duration_since(pat).as_secs_f64();
                    let rxd = rx_total.saturating_sub(prx);
                    let txd = tx_total.saturating_sub(ptx);

                    if dt > 0.0 {
                        let rx_rate = (rxd as f64 / dt).round() as u64;
                        let tx_rate = (txd as f64 / dt).round() as u64;
                        (rxd, txd, Some(rx_rate), Some(tx_rate))
                    } else {
                        (rxd, txd, None, None)
                    }
                }
                _ => (0, 0, None, None),
            };

            prev_rx_total = Some(rx_total);
            prev_tx_total = Some(tx_total);
            prev_at = Some(now_inst);

            // Update shared state quickly; no awaits while locked.
            let mut st = state().lock();
            st.on_sample(
                now_secs, rx_total, tx_total, rx_delta, tx_delta, rx_bps, tx_bps,
            );
        }
    });
}

/// Record a single admin-plane request into the current tick's pending counters.
pub fn record_request(path: &str) {
    let facet = facet_from_path(path);
    let mut st = state().lock();
    st.record_request(facet);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowRollupDto {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub total_bytes: u64,
    pub requests: u64,
    pub requests_by_facet: BTreeMap<String, u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesPointDto {
    pub ts: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub total_bytes: u64,
    pub requests: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetAccountingDto {
    pub updated_at: String,
    pub observed_seconds: u64,
    pub tick_count: u64,

    pub rx_bytes_total: u64,
    pub tx_bytes_total: u64,

    /// Bytes/sec (best-effort). None until we have at least 2 samples.
    pub rx_bps: Option<u64>,
    pub tx_bps: Option<u64>,

    pub minute: WindowRollupDto, // last 60 seconds
    pub hour: WindowRollupDto,   // last 60 minutes
    pub day: WindowRollupDto,    // last 24 hours
    pub month: WindowRollupDto,  // last 30 days (rolling)

    pub series_minute: Vec<SeriesPointDto>, // 60 points (per-minute), last 60 minutes
    pub series_hour: Vec<SeriesPointDto>,   // 24 points (per-hour), last 24 hours
    pub series_day: Vec<SeriesPointDto>,    // 30 points (per-day), last 30 days
    pub series_month: Vec<SeriesPointDto>,  // 12 points (per-30d "month"), last 12 months
}

fn facet_map_from_counts(counts: &[u64; FACET_COUNT]) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for i in 0..FACET_COUNT {
        let c = counts[i];
        if c == 0 {
            continue;
        }
        let name = match i {
            0 => FacetId::Healthz,
            1 => FacetId::Readyz,
            2 => FacetId::Metrics,
            3 => FacetId::Version,
            4 => FacetId::Status,
            5 => FacetId::SystemSummary,
            6 => FacetId::SystemNetAccounting,
            7 => FacetId::StorageSummary,
            8 => FacetId::StorageDatabases,
            9 => FacetId::StorageDatabaseDetail,
            10 => FacetId::BenchRun,
            11 => FacetId::BenchStatus,
            12 => FacetId::BenchResult,
            13 => FacetId::DebugCrash,
            _ => FacetId::Other,
        }
        .as_str()
        .to_string();
        out.insert(name, c);
    }
    out
}

fn fmt_ts_from_epoch(unit_secs: u64, epoch: u64) -> String {
    // Epoch is in "bucket units". Convert to seconds since UNIX_EPOCH.
    let secs = epoch.saturating_mul(unit_secs);
    let t = UNIX_EPOCH + Duration::from_secs(secs);
    humantime::format_rfc3339_seconds(t).to_string()
}

fn series_from_ring(r: &Ring) -> Vec<SeriesPointDto> {
    r.series()
        .into_iter()
        .map(|b| SeriesPointDto {
            ts: fmt_ts_from_epoch(r.unit_secs, b.epoch),
            rx_bytes: b.rx,
            tx_bytes: b.tx,
            total_bytes: b.total_bytes(),
            requests: b.req_total(),
        })
        .collect()
}

/// Snapshot the full accounting DTO for dashboards.
pub fn snapshot() -> NetAccountingDto {
    let now_secs = now_unix_secs();
    let st = state().lock();

    let updated_at = humantime::format_rfc3339_seconds(SystemTime::now()).to_string();

    // Rollups (sum full rings)
    let sec_sum = st.ring_sec.sum_all();
    let min_sum = st.ring_min.sum_all();
    let hour_sum = st.ring_hour.sum_all();
    let day_sum = st.ring_day.sum_all();

    let minute = WindowRollupDto {
        rx_bytes: sec_sum.rx,
        tx_bytes: sec_sum.tx,
        total_bytes: sec_sum.total_bytes(),
        requests: sec_sum.req_total(),
        requests_by_facet: facet_map_from_counts(&sec_sum.req_by_facet),
    };

    let hour = WindowRollupDto {
        rx_bytes: min_sum.rx,
        tx_bytes: min_sum.tx,
        total_bytes: min_sum.total_bytes(),
        requests: min_sum.req_total(),
        requests_by_facet: facet_map_from_counts(&min_sum.req_by_facet),
    };

    let day = WindowRollupDto {
        rx_bytes: hour_sum.rx,
        tx_bytes: hour_sum.tx,
        total_bytes: hour_sum.total_bytes(),
        requests: hour_sum.req_total(),
        requests_by_facet: facet_map_from_counts(&hour_sum.req_by_facet),
    };

    let month = WindowRollupDto {
        rx_bytes: day_sum.rx,
        tx_bytes: day_sum.tx,
        total_bytes: day_sum.total_bytes(),
        requests: day_sum.req_total(),
        requests_by_facet: facet_map_from_counts(&day_sum.req_by_facet),
    };

    NetAccountingDto {
        updated_at,
        observed_seconds: st.observed_seconds(now_secs),
        tick_count: st.tick_count,
        rx_bytes_total: st.rx_total,
        tx_bytes_total: st.tx_total,
        rx_bps: st.rx_bps,
        tx_bps: st.tx_bps,
        minute,
        hour,
        day,
        month,
        series_minute: series_from_ring(&st.ring_min),
        series_hour: series_from_ring(&st.ring_hour),
        series_day: series_from_ring(&st.ring_day),
        series_month: series_from_ring(&st.ring_month),
    }
}
