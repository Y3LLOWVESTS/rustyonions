//! RO:WHAT   Omnigate configuration model + loaders (env/file) + defaults.
//! RO:INVARS  oap.max_frame_bytes ≤ 1MiB; body caps aligned with middleware guards.

use serde::Deserialize;
use std::{fs, net::SocketAddr, path::Path};

mod env;
mod file;
mod validate;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: Server,
    pub oap: Oap,
    pub admission: Admission,
    pub policy: Policy,
    pub readiness: Readiness,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    /// API listener bind, e.g. "127.0.0.1:5305"
    pub bind: SocketAddr,
    /// Admin/metrics bind, e.g. "127.0.0.1:9605"
    pub metrics_addr: SocketAddr,
    pub amnesia: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Oap {
    pub max_frame_bytes: u64,
    pub stream_chunk_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Admission {
    pub global_quota: GlobalQuota,
    pub ip_quota: IpQuota,
    pub fair_queue: FairQueue,
    pub body: BodyCaps,
    pub decompression: Decompress,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalQuota {
    pub qps: u64,
    pub burst: u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct IpQuota {
    pub enabled: bool,
    pub qps: u64,
    pub burst: u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct FairQueue {
    pub max_inflight: u64,
    pub weights: Weights,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Weights {
    pub anon: u32,
    pub auth: u32,
    pub admin: u32,
}
#[derive(Debug, Clone, Deserialize)]
pub struct BodyCaps {
    pub max_content_length: u64,
    pub reject_on_missing_length: bool,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Decompress {
    pub allow: Vec<String>,
    pub deny_stacked: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    pub enabled: bool,
    pub bundle_path: String,
    pub fail_mode: String,
}
impl Policy {
    pub fn fail_deny(&self) -> bool {
        self.fail_mode.eq_ignore_ascii_case("deny")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Readiness {
    pub max_inflight_threshold: u64,
    pub error_rate_429_503_pct: f64,
    pub window_secs: u64,
    pub hold_for_secs: u64,
}

impl Config {
    /// Load config with precedence: CLI `--config <path>` (handled in main) → env overrides → defaults/file.
    pub fn load() -> anyhow::Result<Self> {
        // Try file from default search paths.
        if let Some(cfg) = file::load_from_default_path()? {
            let mut cfg = cfg;
            env::apply_env_overrides(&mut cfg)?;
            validate::validate(&cfg)?;
            // OAP hard limit
            anyhow::ensure!(
                cfg.oap.max_frame_bytes <= 1_048_576,
                "oap.max_frame_bytes > 1MiB not allowed"
            );
            return Ok(cfg);
        }

        // Fallback minimal defaults (safe localhost).
        let mut cfg = Self {
            server: Server {
                bind: "127.0.0.1:5305".parse()?,
                metrics_addr: "127.0.0.1:9605".parse()?,
                amnesia: true,
            },
            oap: Oap {
                max_frame_bytes: 1_048_576,
                stream_chunk_bytes: 65_536,
            },
            admission: Admission {
                global_quota: GlobalQuota {
                    qps: 20_000,
                    burst: 40_000,
                },
                ip_quota: IpQuota {
                    enabled: true,
                    qps: 2_000,
                    burst: 4_000,
                },
                fair_queue: FairQueue {
                    max_inflight: 2_048,
                    weights: Weights {
                        anon: 1,
                        auth: 5,
                        admin: 10,
                    },
                },
                body: BodyCaps {
                    max_content_length: 1_048_576 * 10,
                    reject_on_missing_length: true,
                },
                decompression: Decompress {
                    allow: vec!["identity".into(), "gzip".into()],
                    deny_stacked: true,
                },
            },
            policy: Policy {
                enabled: false,
                bundle_path: "policy.bundle.json".into(),
                fail_mode: "deny".into(),
            },
            readiness: Readiness {
                max_inflight_threshold: 1_800,
                error_rate_429_503_pct: 2.0,
                window_secs: 10,
                hold_for_secs: 30,
            },
        };
        env::apply_env_overrides(&mut cfg)?;
        validate::validate(&cfg)?;
        Ok(cfg)
    }

    /// Explicit file load (used by main when `--config` is provided).
    pub fn from_toml_file<P: AsRef<Path>>(p: P) -> anyhow::Result<Self> {
        let s = fs::read_to_string(p)?;
        let mut cfg: Self = toml::from_str(&s)?;
        env::apply_env_overrides(&mut cfg)?;
        validate::validate(&cfg)?;
        anyhow::ensure!(
            cfg.oap.max_frame_bytes <= 1_048_576,
            "oap.max_frame_bytes > 1MiB not allowed"
        );
        Ok(cfg)
    }
}
