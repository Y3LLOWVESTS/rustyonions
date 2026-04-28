use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use svc_rewarder::config::Config;
use svc_rewarder::core::{compute_manifest, AmountMinor, ComputeInput};
use svc_rewarder::inputs::{AccountContribution, AccountingSnapshot, ContentCid, RewardPolicy};
use svc_rewarder::outputs::artifacts::maybe_write_manifest;
use svc_rewarder::outputs::{IntentResult, RewardManifest};

fn cid() -> ContentCid {
    ContentCid::parse(format!("b3:{}", "a".repeat(64))).unwrap()
}

fn policy() -> RewardPolicy {
    RewardPolicy {
        id: "policy:v1".into(),
        hash: format!("b3:{}", "b".repeat(64)),
        signed: true,
        max_payout_minor_units: AmountMinor(1_000),
        min_payout_minor_units: AmountMinor(1),
        weight_bps: 10_000,
        rounding: "floor".into(),
    }
}

fn snapshot() -> AccountingSnapshot {
    AccountingSnapshot {
        produced_at_millis: 1,
        pool_minor_units: AmountMinor(1_000),
        contributions: vec![
            AccountContribution {
                account: "acct_a".into(),
                bytes_stored: 100,
                bytes_served: 0,
                uptime_seconds: 0,
            },
            AccountContribution {
                account: "acct_b".into(),
                bytes_stored: 200,
                bytes_served: 50,
                uptime_seconds: 10,
            },
        ],
    }
}

fn manifest_for_epoch(epoch_id: &str) -> RewardManifest {
    compute_manifest(
        ComputeInput {
            epoch_id: epoch_id.into(),
            inputs_cid: cid(),
            policy: policy(),
            snapshot: snapshot(),
            dry_run: false,
            idempotency_salt: "artifact-test".into(),
        },
        IntentResult::Accepted,
    )
    .unwrap()
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "svc_rewarder_{label}_{}_{}",
        std::process::id(),
        nanos
    ))
}

#[test]
fn artifact_write_is_suppressed_in_amnesia_mode() {
    let dir = unique_temp_dir("amnesia_on");
    let manifest = manifest_for_epoch("epoch-artifact-amnesia-on");

    let mut cfg = Config::default();
    cfg.amnesia.enabled = true;
    cfg.rewarder.artifact_dir = dir.to_string_lossy().to_string();

    let written = maybe_write_manifest(&cfg, &manifest).unwrap();

    assert!(written.is_none());
    assert!(!dir.exists());
}

#[test]
fn artifact_write_persists_manifest_when_amnesia_disabled() {
    let dir = unique_temp_dir("amnesia_off");
    let manifest = manifest_for_epoch("epoch-artifact-amnesia-off");

    let mut cfg = Config::default();
    cfg.amnesia.enabled = false;
    cfg.rewarder.artifact_dir = dir.to_string_lossy().to_string();

    let written = maybe_write_manifest(&cfg, &manifest).unwrap().unwrap();

    assert!(written.exists());

    let decoded = std::fs::read_to_string(&written).unwrap();
    let loaded = serde_json::from_str::<RewardManifest>(&decoded).unwrap();

    assert_eq!(loaded, manifest);
    assert_eq!(loaded.commitment, manifest.commitment);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn artifact_writer_sanitizes_epoch_id_for_filename() {
    let dir = unique_temp_dir("sanitize");
    let manifest = manifest_for_epoch("epoch:artifact/unsafe.name");

    let mut cfg = Config::default();
    cfg.amnesia.enabled = false;
    cfg.rewarder.artifact_dir = dir.to_string_lossy().to_string();

    let written = maybe_write_manifest(&cfg, &manifest).unwrap().unwrap();
    let file_name = written.file_name().unwrap().to_string_lossy();

    assert_eq!(file_name, "epoch_artifact_unsafe_name.run.json");

    let _ = std::fs::remove_dir_all(&dir);
}
