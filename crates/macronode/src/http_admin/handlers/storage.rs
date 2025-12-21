// crates/macronode/src/http_admin/handlers/storage.rs
//
// RO:WHAT — `/api/v1/storage/*` handlers for macronode admin plane.
// RO:WHY  — Provide a curated, read-only storage/DB inventory that svc-admin can proxy.
// RO:SECURITY —
//   - No arbitrary path browsing.
//   - Only a small, known set of DB roots.
//   - No raw absolute path leakage in responses (alias-only).
// RO:INVARIANTS —
//   - camelCase JSON keys to match svc-admin SPA types.
//   - Health is coarse: "ok" | "degraded" | "error".
//   - No unsafe code (macronode forbids unsafe).
//
// NOTE:
//   UI expects DB names like "svc-index.sled". We expose that shape and accept
//   both "svc-index" and "svc-index.sled" for compatibility.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::observability::metrics::{observe_facet_error, observe_facet_ok};
use crate::types::AppState;

#[derive(Debug, Clone)]
struct KnownDb {
    /// Public name used by UI / svc-admin
    name: &'static str,
    engine: &'static str,
    env_key: &'static str,
    default_path: &'static str,
    path_alias: &'static str,
}

fn known_databases() -> &'static [KnownDb] {
    // v1 wedge: start with the known on-disk DB that drives the earlier 404 fix
    // (index resolve path). Expand later (storage, overlay, naming, etc.)
    &[
        KnownDb {
            name: "svc-index.sled",
            engine: "sled",
            env_key: "RON_INDEX_DB",
            default_path: "svc-index.db",
            path_alias: "index-db",
        },
    ]
}

fn normalize_db_name(name: &str) -> &str {
    name.strip_suffix(".sled").unwrap_or(name)
}

fn find_known_db(name: &str) -> Option<&'static KnownDb> {
    let want = normalize_db_name(name);

    known_databases().iter().find(|db| {
        // Compare normalized variants, so "svc-index" and "svc-index.sled" both match.
        normalize_db_name(db.name) == want
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummaryDto {
    pub fs_type: String,
    pub mount: String,

    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_read_bps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_write_bps: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseEntryDto {
    pub name: String,
    pub engine: String,
    pub size_bytes: u64,

    pub mode: String,
    pub owner: String,

    /// "ok" | "degraded" | "error"
    pub health: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_readable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_writable: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseDetailDto {
    pub name: String,
    pub engine: String,
    pub size_bytes: u64,

    pub mode: String,
    pub owner: String,

    /// "ok" | "degraded" | "error"
    pub health: String,

    /// Alias only; avoid leaking raw absolute path.
    pub path_alias: String,

    pub file_count: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_compaction: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub approx_keys: Option<u64>,

    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScanResult {
    bytes: u64,
    files: u64,
    truncated: bool,
}

fn scan_path_best_effort(root: &Path, max_files: u64) -> ScanResult {
    fn walk(
        p: &Path,
        max_files: u64,
        acc_bytes: &mut u64,
        acc_files: &mut u64,
        truncated: &mut bool,
    ) {
        if *truncated {
            return;
        }

        let md = match fs::symlink_metadata(p) {
            Ok(m) => m,
            Err(_) => return,
        };

        // Don’t follow symlinks (no loops / no surprises).
        if md.file_type().is_symlink() {
            return;
        }

        if md.is_file() {
            *acc_bytes = acc_bytes.saturating_add(md.len());
            *acc_files = acc_files.saturating_add(1);
            if *acc_files >= max_files {
                *truncated = true;
            }
            return;
        }

        if md.is_dir() {
            let rd = match fs::read_dir(p) {
                Ok(r) => r,
                Err(_) => return,
            };

            for ent in rd.flatten() {
                walk(&ent.path(), max_files, acc_bytes, acc_files, truncated);
                if *truncated {
                    return;
                }
            }
        }
    }

    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut truncated = false;
    walk(root, max_files, &mut bytes, &mut files, &mut truncated);

    ScanResult {
        bytes,
        files,
        truncated,
    }
}

#[cfg(unix)]
fn file_mode_owner(path: &Path) -> (String, String, Option<bool>, Option<bool>) {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let md = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return ("????".to_string(), "unknown".to_string(), None, None),
    };

    let mode_raw = md.permissions().mode() & 0o7777;
    let mode = format!("{mode_raw:04o}");
    let uid = md.uid();
    let owner = format!("uid:{uid}");

    let world_readable = (mode_raw & 0o0004) != 0;
    let world_writable = (mode_raw & 0o0002) != 0;

    (mode, owner, Some(world_readable), Some(world_writable))
}

#[cfg(not(unix))]
fn file_mode_owner(_path: &Path) -> (String, String, Option<bool>, Option<bool>) {
    ("????".to_string(), "unknown".to_string(), None, None)
}

fn resolve_db_path(db: &KnownDb) -> PathBuf {
    std::env::var(db.env_key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(db.default_path))
}

fn coarse_health_for_path(p: &Path) -> String {
    if p.exists() {
        "ok".to_string()
    } else {
        "error".to_string()
    }
}

/// Try multiple paths for disk stats to avoid platform/CWD quirks.
/// Returns (total, free) on success.
fn disk_stats_best_effort(rep_for_stats: &Path) -> Option<(u64, u64)> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1) Prefer the representative directory we computed.
    candidates.push(rep_for_stats.to_path_buf());

    // 2) Try current dir (robust fallback).
    if let Ok(cd) = std::env::current_dir() {
        candidates.push(cd);
    }

    // 3) Try filesystem root.
    candidates.push(PathBuf::from("/"));

    // 4) macOS: Data volume is sometimes the better stat target.
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/System/Volumes/Data"));
    }

    for p in candidates {
        // Must exist, or fs2 can error.
        if !p.exists() {
            continue;
        }
        let total = fs2::total_space(&p).ok()?;
        let free = fs2::available_space(&p).ok()?;
        if total > 0 {
            return Some((total, free));
        }
    }

    None
}

pub async fn storage_summary(State(_state): State<AppState>) -> impl IntoResponse {
    const FACET: &str = "admin.storage.summary";

    // Representative path:
    // Use first known DB’s parent dir if possible, else ".".
    let rep_path = known_databases()
        .first()
        .map(resolve_db_path)
        .unwrap_or_else(|| PathBuf::from("."));

    // For "svc-index.db" (single component), parent() can be None. Use "." then.
    let rep_for_stats = rep_path.parent().unwrap_or_else(|| Path::new("."));

    let Some((total, free)) = disk_stats_best_effort(rep_for_stats) else {
        observe_facet_error(FACET);
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({ "error": "disk_stats_unavailable" })),
        )
            .into_response();
    };

    let used = total.saturating_sub(free);

    let dto = StorageSummaryDto {
        fs_type: "unknown".to_string(),
        mount: "local".to_string(),
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        io_read_bps: None,
        io_write_bps: None,
    };

    observe_facet_ok(FACET);
    (StatusCode::OK, Json(dto)).into_response()
}

pub async fn storage_databases(State(_state): State<AppState>) -> impl IntoResponse {
    const FACET: &str = "admin.storage.databases";

    let mut out: Vec<DatabaseEntryDto> = Vec::new();

    for db in known_databases() {
        let path = resolve_db_path(db);
        let mut health = coarse_health_for_path(&path);

        let (mode, owner, world_readable, world_writable) = file_mode_owner(&path);

        // Size scan can be expensive; cap to keep admin plane snappy.
        let scan = if path.exists() {
            scan_path_best_effort(&path, 25_000)
        } else {
            ScanResult {
                bytes: 0,
                files: 0,
                truncated: false,
            }
        };

        let mut notes: Option<String> = None;
        if !path.exists() {
            notes = Some(format!(
                "Path not found. Set {} (default {}).",
                db.env_key, db.default_path
            ));
        } else if scan.truncated {
            notes = Some("Size/file count truncated (cap reached).".to_string());
            if health == "ok" {
                health = "degraded".to_string();
            }
        }

        out.push(DatabaseEntryDto {
            name: db.name.to_string(),
            engine: db.engine.to_string(),
            size_bytes: scan.bytes,
            mode,
            owner,
            health,
            notes,
            world_readable,
            world_writable,
        });
    }

    observe_facet_ok(FACET);
    (StatusCode::OK, Json(out)).into_response()
}

pub async fn storage_database_detail(
    AxumPath(name): AxumPath<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    const FACET: &str = "admin.storage.database_detail";

    let Some(db) = find_known_db(&name) else {
        observe_facet_error(FACET);
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "not_found" })),
        )
            .into_response();
    };

    let path = resolve_db_path(db);
    let (mode, owner, world_readable, world_writable) = file_mode_owner(&path);

    let scan = if path.exists() {
        scan_path_best_effort(&path, 50_000)
    } else {
        ScanResult {
            bytes: 0,
            files: 0,
            truncated: false,
        }
    };

    let mut warnings: Vec<String> = Vec::new();
    if !path.exists() {
        warnings.push(format!(
            "Database path not found. Set {} or create default path {}.",
            db.env_key, db.default_path
        ));
    }
    if world_readable.unwrap_or(false) {
        warnings.push(format!("World-readable permissions detected (mode {mode})."));
    }
    if world_writable.unwrap_or(false) {
        warnings.push(format!("World-writable permissions detected (mode {mode})."));
    }
    if scan.truncated {
        warnings.push("File scan truncated (cap reached); counts are partial.".to_string());
    }

    let mut health = coarse_health_for_path(&path);
    if scan.truncated && health == "ok" {
        health = "degraded".to_string();
    }

    let dto = DatabaseDetailDto {
        name: db.name.to_string(),
        engine: db.engine.to_string(),
        size_bytes: scan.bytes,
        mode,
        owner,
        health,
        path_alias: db.path_alias.to_string(),
        file_count: scan.files,
        last_compaction: None,
        approx_keys: None,
        warnings,
    };

    observe_facet_ok(FACET);
    (StatusCode::OK, Json(dto)).into_response()
}
