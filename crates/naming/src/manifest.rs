// crates/naming/src/manifest.rs
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Canonical manifest schema for RustyOnions.
/// v2 core fields stay stable. Optional blocks below are safe for old readers to ignore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV2 {
    // ---- Core (required) ----
    pub schema_version: u32, // 2
    pub tld: String,
    pub address: String,     // e.g., b3:<hex>.<tld>
    pub hash_algo: String,   // "b3"
    pub hash_hex: String,    // 64 hex chars
    pub bytes: u64,
    pub created_utc: String, // RFC3339
    pub mime: String,        // best guess (e.g., text/plain; application/json)
    pub stored_filename: String,   // usually "payload.bin"
    pub original_filename: String, // original source file name

    // Precompressed encodings (zstd/br). Hidden in TOML if empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub encodings: Vec<Encoding>,

    // ---- Optional blocks (hidden if absent/empty) ----
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<Payment>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relations: Option<Relations>,

    /// Namespaced, TLD-specific extras: [ext.image], [ext.video], [ext.<yourkind>]
    /// Values are arbitrary TOML trees so TLDs can evolve independently.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ext: BTreeMap<String, toml::Value>,
}

/// Description of a precompressed file variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    pub coding: String,   // "zstd" | "br"
    pub level: i32,       // compression level/quality used
    pub bytes: u64,       // size on disk
    pub filename: String, // e.g., "payload.bin.zst"
    pub hash_hex: String, // BLAKE3 of the compressed bytes
}

/// Optional micropayments / wallet info.
/// Gateways can later enforce `required=true` before serving bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    #[serde(default)]
    pub required: bool,      // default false

    #[serde(default)]
    pub currency: String,    // e.g., "USD", "sats", "ETH", "SOL"

    #[serde(default)]
    pub price_model: String, // "per_mib" | "flat" | "per_request"

    #[serde(default)]
    pub price: f64,          // unit depends on price_model

    #[serde(default)]
    pub wallet: String,      // LNURL, onchain addr, etc.

    #[serde(default)]
    pub settlement: String,  // "onchain" | "offchain" | "custodial"

    #[serde(default)]
    pub splits: Vec<RevenueSplit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSplit {
    pub account: String, // wallet/account id
    pub pct: f32,        // 0..100
}

/// Optional relations/metadata for threading, licensing, provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relations {
    #[serde(default)]
    pub parent: Option<String>,  // b3:<hex>.<tld>

    #[serde(default)]
    pub thread: Option<String>,  // root addr

    #[serde(default)]
    pub license: Option<String>, // SPDX or human-readable

    #[serde(default)]
    pub source: Option<String>,  // freeform (e.g., "camera:sony-a7c")
}

/// Helper to write Manifest.toml to a bundle directory.
pub fn write_manifest(bundle_dir: &Path, manifest: &ManifestV2) -> Result<PathBuf> {
    let toml = toml::to_string_pretty(manifest).context("serialize manifest v2")?;
    let path = bundle_dir.join("Manifest.toml");
    fs::write(&path, toml).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

