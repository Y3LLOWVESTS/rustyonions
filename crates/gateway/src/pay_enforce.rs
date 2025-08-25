// crates/gateway/src/pay_enforce.rs
#![forbid(unsafe_code)]

use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Minimal view of Manifest v2 `[payment]`.
#[derive(Debug, Deserialize)]
pub struct Payment {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub price_model: String, // "per_request" | "per_mib" | "flat"
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub wallet: String, // LNURL or address
}

#[derive(Debug, Deserialize)]
pub struct ManifestV2 {
    pub schema_version: u32,
    #[serde(default)]
    pub payment: Option<Payment>,
}

/// Switchable enforcer. Construct with `Enforcer::new(true)` when `--enforce-payments` is set.
#[derive(Clone, Debug)]
pub struct Enforcer {
    enabled: bool,
}

impl Enforcer {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Guard access to a resolved bundle directory (contains Manifest.toml).
    /// If `[payment].required = true` and enforcement is enabled, returns
    /// `Err((402, Response))` that you should return from your handler.
    pub fn guard(&self, bundle_dir: &Path, addr: &str) -> Result<(), (StatusCode, Response)> {
        if !self.enabled {
            return Ok(());
        }

        let manifest_path = bundle_dir.join("Manifest.toml");
        if !manifest_path.exists() {
            // Fail-open: no manifest visible; allow access.
            return Ok(());
        }

        let txt = match fs::read_to_string(&manifest_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "[gateway] payment: failed to read {}: {e}",
                    manifest_path.display()
                );
                return Ok(());
            }
        };

        let m: ManifestV2 = match toml::from_str(&txt) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "[gateway] payment: TOML parse error in {}: {e}",
                    manifest_path.display()
                );
                return Ok(());
            }
        };

        if m.schema_version != 2 {
            return Ok(());
        }

        let Some(p) = m.payment else {
            return Ok(());
        };
        if !p.required {
            return Ok(());
        }

        // Build advisory headers + RFC 7807-ish JSON problem body.
        let mut builder = Response::builder().status(StatusCode::PAYMENT_REQUIRED);

        if !p.currency.is_empty() {
            builder = builder.header("X-Payment-Currency", p.currency.as_str());
        }
        if !p.price_model.is_empty() {
            builder = builder.header("X-Payment-Price-Model", p.price_model.as_str());
        }
        if p.price > 0.0 {
            builder = builder.header("X-Payment-Price", format!("{}", p.price));
        }
        if !p.wallet.is_empty() {
            builder = builder.header("X-Payment-Wallet", p.wallet.as_str());
        }

        let body = serde_json::json!({
            "type": "about:blank",
            "title": "Payment Required",
            "status": 402,
            "error": "payment_required",
            "addr": addr,
            "currency": (!p.currency.is_empty()).then_some(p.currency),
            "price_model": (!p.price_model.is_empty()).then_some(p.price_model),
            "price": (p.price > 0.0).then_some(p.price),
            "wallet": (!p.wallet.is_empty()).then_some(p.wallet),
        })
        .to_string();

        let resp = builder
            .header("Content-Type", "application/problem+json; charset=utf-8")
            .body(axum::body::Body::from(body))
            .unwrap();

        Err((StatusCode::PAYMENT_REQUIRED, resp))
    }
}
