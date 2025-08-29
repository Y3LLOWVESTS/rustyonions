#![forbid(unsafe_code)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Minimal view of Manifest v2 `[payment]`.
#[derive(Debug, Deserialize, Default)]
pub struct Payment {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub currency: String, // e.g., "RON", "USD", "SAT"
    #[serde(default)]
    pub price_model: String, // "per_request" | "per_mib" | "flat"
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub wallet: String, // LNURL or address
}

#[derive(Debug, Deserialize, Default)]
struct ManifestV2 {
    #[serde(default)]
    payment: Payment,
}

/// Legacy filesystem check: read Manifest.toml from the bundle dir to decide 402.
pub fn guard(bundle_dir: &Path, _addr: &str) -> Result<(), (StatusCode, Response)> {
    let path = bundle_dir.join("Manifest.toml");
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(_) => return Ok(()), // no manifest -> free
    };
    guard_bytes(&bytes)
}

/// Decide via in-memory Manifest.toml bytes (used when bundle is fetched over overlay/storage).
pub fn guard_bytes(manifest_toml: &[u8]) -> Result<(), (StatusCode, Response)> {
    // `toml` doesn't provide from_slice in all versions; decode as UTF-8 then parse.
    let s = match std::str::from_utf8(manifest_toml) {
        Ok(x) => x,
        Err(_) => return Ok(()), // treat non-utf8 as free (best-effort)
    };

    let manifest: ManifestV2 = match toml::from_str(s) {
        Ok(m) => m,
        Err(_) => return Ok(()), // malformed -> treat as free for now
    };

    if manifest.payment.required {
        let msg = format!(
            "Payment required: {} {} ({})",
            manifest.payment.price, manifest.payment.currency, manifest.payment.price_model
        );
        let rsp = (StatusCode::PAYMENT_REQUIRED, msg).into_response();
        Err((StatusCode::PAYMENT_REQUIRED, rsp))
    } else {
        Ok(())
    }
}

#[cfg(feature = "legacy-pay")]
pub struct Enforcer {
    enabled: bool,
}

#[cfg(feature = "legacy-pay")]
impl Enforcer {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    pub fn guard(&self, bundle_dir: &Path, addr: &str) -> Result<(), (StatusCode, Response)> {
        if !self.enabled {
            return Ok(());
        }
        crate::pay_enforce::guard(bundle_dir, addr)
    }
}
