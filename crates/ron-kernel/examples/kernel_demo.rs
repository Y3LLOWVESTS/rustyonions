// crates/ron-kernel/examples/kernel_demo.rs
//
// Minimal runnable demo for ron-kernel surfaces.
// - Exposes /metrics, /healthz, /readyz
// - Reads RON_CONFIG (default: /tmp/ron-kernel.toml) and toggles amnesia on real content change
// - Publishes KernelEvent::ConfigUpdated { version } on each change
//
// ENV:
//   RON_CONFIG=/tmp/ron-kernel.toml   # optional; default shown
//   RON_AMNESIA=1                     # optional; force amnesia=1 at startup

use ron_kernel::{Bus, KernelEvent, Metrics, HealthState, wait_for_ctrl_c};
use ron_kernel::metrics::readiness::Readiness;
use std::{env, fs, net::SocketAddr, time::{Duration, SystemTime}};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() {
    // Core kernel surfaces
    let metrics = Metrics::new(false);
    let health = HealthState::new();
    let ready = Readiness::new(health.clone());

    // HTTP exporter (metrics / health / ready)
    let bind: SocketAddr = "127.0.0.1:9600".parse().unwrap();
    let (_handle, local) = metrics
        .clone()
        .serve(bind, health.clone(), ready.clone())
        .await
        .expect("metrics/health/ready server to bind");
    println!("metrics:  http://{}/metrics", local);
    println!("healthz:  http://{}/healthz", local);
    println!("readyz :  http://{}/readyz", local);

    // Config source
    let cfg_path = env::var("RON_CONFIG").unwrap_or_else(|_| "/tmp/ron-kernel.toml".to_string());
    println!("edit {} or set RON_AMNESIA=1 to see updates", cfg_path);

    // Seed readiness + amnesia
    // Mark kernel service healthy for demo visibility; /readyz still waits for config_loaded=true.
    health.set("kernel", true);

    // Apply env override immediately for a quick sanity check; else seed from file.
    let mut config_loaded = false;
    if let Ok(v) = env::var("RON_AMNESIA") {
        if v == "1" || v.eq_ignore_ascii_case("true") {
            metrics.set_amnesia(true);
            config_loaded = true;
        }
    } else if let Some(a) = read_amnesia_flag(&cfg_path) {
        metrics.set_amnesia(a);
        config_loaded = true;
    }
    if config_loaded {
        ready.set_config_loaded(true);
    }

    // Bus for demo events
    let bus: Bus<KernelEvent> = Bus::new().with_metrics(metrics.clone());

    // --- A3: Capacity Autotune (feature-gated) --------------------------------
    // For demo purposes we assume ~4 global subscribers. This computes a cache-friendly
    // capacity and exports it via the `bus_cap_selected` gauge. When the feature is OFF,
    // this block is not compiled and no behavior changes.
    #[cfg(feature = "bus_autotune_cap")]
    {
        // If you later expose a Bus::with_capacity(cap) builder path, you can pass `cap` there.
        // For now, we record and print the selection for observability.
        let expected_subs = 4usize;
        let cap = ron_kernel::bus::capacity::autotune_capacity(expected_subs, None);
        println!("autotune: expected_subs={} → selected bus cap = {}", expected_subs, cap);
    }
    // ---------------------------------------------------------------------------

    // Poller: detect real content changes, apply amnesia, publish ConfigUpdated
    let poller: JoinHandle<()> = tokio::spawn({
        let metrics = metrics.clone();
        let ready = ready.clone();
        let bus = bus.clone();
        let cfg_path = cfg_path.clone();
        async move {
            let mut last_hash: Option<u64> = None;
            let mut version: u64 = 1;

            loop {
                let (hash, amnesia) = match read_file_and_hash(&cfg_path) {
                    Some((h, a)) => (Some(h), Some(a)),
                    None => (None, None),
                };

                if hash.is_some() && hash != last_hash {
                    // We have a real change; mark config loaded and flip amnesia.
                    ready.set_config_loaded(true);
                    if let Some(a) = amnesia {
                        metrics.set_amnesia(a);
                    }
                    bus.publish(KernelEvent::ConfigUpdated { version });
                    println!(
                        "kernel event: ConfigUpdated {{ version: {} }} → amnesia={:?}",
                        version, amnesia
                    );
                    version = version.saturating_add(1);
                    last_hash = hash;
                }

                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    });

    println!("press Ctrl-C to stop …");
    wait_for_ctrl_c().await;
    poller.abort(); // best-effort cleanup
}

// --- helpers ---------------------------------------------------------------

// Parse `amnesia = true|false` from a TOML-ish line.
fn read_amnesia_flag(path: &str) -> Option<bool> {
    let s = fs::read_to_string(path).ok()?;
    for line in s.lines() {
        let t = line.trim();
        if t.starts_with("amnesia") && t.contains('=') {
            let val = t.splitn(2, '=').nth(1)?.trim();
            let val = val.trim_matches(|c: char| c == '"' || c.is_ascii_whitespace());
            return Some(val.eq_ignore_ascii_case("true"));
        }
    }
    None
}

// Read file and return (content_hash, amnesia_flag) using a tiny FNV-1a 64-bit hash.
fn read_file_and_hash(path: &str) -> Option<(u64, bool)> {
    let s = fs::read_to_string(path).ok()?;
    let mut hasher = Fnv1a64::new();
    hasher.update(s.as_bytes());
    // Fold in mtime to ensure delta on edits even if content normalizes
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            if let Ok(dur) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                hasher.update(&dur.as_nanos().to_le_bytes());
            }
        }
    }
    let amnesia = s.lines().any(|line| {
        let t = line.trim();
        t.starts_with("amnesia")
            && t.contains('=')
            && t.splitn(2, '=')
                .nth(1)
                .map(|v| {
                    v.trim()
                        .trim_matches(|c: char| c == '"' || c.is_ascii_whitespace())
                        .eq_ignore_ascii_case("true")
                })
                .unwrap_or(false)
    });
    Some((hasher.finish(), amnesia))
}

// Tiny FNV-1a (64-bit) hasher (self-contained).
struct Fnv1a64(u64);
impl Fnv1a64 {
    fn new() -> Self { Self(0xcbf29ce484222325) } // offset basis
    fn update(&mut self, bytes: &[u8]) {
        const PRIME: u64 = 0x100000001b3;
        let mut h = self.0;
        for b in bytes {
            h ^= *b as u8 as u64;
            h = h.wrapping_mul(PRIME);
        }
        self.0 = h;
    }
    fn finish(&self) -> u64 { self.0 }
}
