//! RO:WHAT — Phase 3 Round 1 passport-gated validator boundary tests for svc-storage.
//! RO:WHY — svc-storage may retain opaque validator/readiness artifact bytes by b3, but it must not become validator identity, passport registry, capability, staking, slashing, paid-unlock, wallet, ledger, or settlement authority.
//! RO:INTERACTS — storage::MemoryStorage, accounting usage/export DTOs, paid/policy/accounting source boundary, Cargo manifest.
//! RO:INVARIANTS — b3 proves bytes only; wallet/ledger receipts prove payment; cache/storage artifacts are not validator membership or paid-access authority.
//! RO:METRICS — none.
//! RO:CONFIG — in-process memory store only.
//! RO:SECURITY — blocks validator/passport/registry/capability/bond/staking/slashing authority creep.
//! RO:TEST — cargo test -p svc-storage --test quickchain_phase3_validator_boundary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::body::Bytes;
use svc_storage::{
    accounting::{exporter::AccountingExportRequest, UsageEventDto},
    storage::{MemoryStorage, Storage},
};

const PHASE3_VALIDATOR_AUTHORITY_KEYS: &[&str] = &[
    "validator_passport_subject",
    "validator_capability",
    "validator_capability_scope",
    "validator_capability_id",
    "validator_set_hash",
    "validator_set_version",
    "validator_registry_epoch",
    "validator_lifecycle_status",
    "validator_admission_rule",
    "validator_revocation_rule",
    "validator_rotation_epoch",
    "passport_registry_proof",
    "passport_admission_proof",
    "passport_revocation_proof",
    "registry_membership_proof",
    "registry_admission_proof",
    "capability_not_before_ms",
    "capability_expires_at_ms",
    "capability_rotation_proof",
    "passport_required",
    "bond_required",
    "bonded_economics",
    "validator_bond",
    "staking_power",
    "slash_evidence",
    "slashing",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", path.display());
    })
}

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).unwrap_or_else(|err| {
        panic!("failed to read directory {}: {err}", root.display());
    });

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "target")
        {
            continue;
        }

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "rs")
        {
            files.push(path);
        }
    }
}

fn strip_line_comments(input: &str) -> String {
    input
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

fn cid_for(bytes: &[u8]) -> String {
    format!("b3:{}", blake3::hash(bytes).to_hex())
}

fn assert_canonical_b3(cid: &str) {
    assert_eq!(cid.len(), 67, "b3 cid must be b3:<64 lowercase hex>");
    assert!(cid.starts_with("b3:"), "b3 cid must have b3: prefix");
    assert!(
        cid[3..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
        "b3 cid must use lowercase hex"
    );
}

#[tokio::test]
async fn storage_can_retain_phase3_validator_readiness_artifacts_as_opaque_b3_bytes_only() {
    let store = MemoryStorage::new();

    let artifacts = [
        Bytes::from_static(
            br#"{"schema":"quickchain.validator-set.v1","purpose":"readiness_display_only","validator_set_hash":"b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.validator-capability.v1","purpose":"opaque_bytes_only","validator_capability_scope":"quickchain.verify.v1"}"#,
        ),
        Bytes::from_static(
            br#"{"schema":"quickchain.passport-registry-proof.v1","purpose":"not_storage_authority","passport_registry_proof":"opaque"}"#,
        ),
    ];

    for body in artifacts {
        let cid = cid_for(body.as_ref());
        assert_canonical_b3(&cid);

        store
            .put(&cid, body.clone())
            .await
            .expect("opaque Phase 3 readiness artifact bytes should store by b3 cid");

        assert!(
            store.exists(&cid).await.expect("exists should succeed"),
            "stored opaque artifact should be discoverable by exact b3 cid"
        );

        let head = store.head(&cid).await.expect("head should succeed");
        assert_eq!(head.len, body.len() as u64);
        assert_eq!(
            head.etag,
            format!("\"{}\"", blake3::hash(body.as_ref()).to_hex())
        );

        let full = store
            .get_full(&cid)
            .await
            .expect("opaque artifact read should succeed");
        assert_eq!(full, body);

        let (prefix, total) = store
            .get_range(&cid, 0, 3)
            .await
            .expect("range read should succeed");
        assert_eq!(total, body.len() as u64);
        assert_eq!(&prefix[..], &body[..4]);
    }
}

#[test]
fn storage_accounting_usage_dtos_reject_phase3_validator_authority_fields() {
    let usage_with_validator_set_hash: &'static str = r#"{
        "timestamp_ms": 1,
        "tenant": 7,
        "subject": "svc-storage",
        "metric_kind": "bytes_stored",
        "value": 4096,
        "source_service": "svc-storage",
        "region": "dev",
        "route": "/paid/o",
        "validator_set_hash": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }"#;

    assert!(
        serde_json::from_str::<UsageEventDto>(usage_with_validator_set_hash).is_err(),
        "UsageEventDto must reject validator_set_hash authority smuggling"
    );

    let export_with_top_level_passport: &'static str = r#"{
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [],
        "passport_registry_proof": "client-supplied-passport-authority"
    }"#;

    assert!(
        serde_json::from_str::<AccountingExportRequest>(export_with_top_level_passport).is_err(),
        "AccountingExportRequest must reject top-level passport_registry_proof authority smuggling"
    );

    let export_with_nested_capability: &'static str = r#"{
        "schema": "svc-storage.usage-events.v1",
        "cid": "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "wallet_txid": "wallet_txid_display_only",
        "source_service": "svc-storage",
        "events": [
            {
                "timestamp_ms": 1,
                "tenant": 7,
                "subject": "svc-storage",
                "metric_kind": "bytes_stored",
                "value": 4096,
                "source_service": "svc-storage",
                "region": "dev",
                "route": "/paid/o",
                "validator_capability": "client-supplied-capability"
            }
        ]
    }"#;

    assert!(
        serde_json::from_str::<AccountingExportRequest>(export_with_nested_capability).is_err(),
        "nested UsageEventDto must reject validator_capability authority smuggling"
    );
}

#[test]
fn storage_manifest_keeps_passport_registry_auth_wallet_and_ledger_authority_out_of_runtime_deps() {
    let manifest = read(crate_dir().join("Cargo.toml"));

    for forbidden in [
        "svc-passport",
        "svc_passport",
        "svc-registry",
        "svc_registry",
        "ron-auth",
        "ron_auth",
        "ron-ledger",
        "ron_ledger",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "svc-storage must not link Phase 3 passport/registry/auth/ledger authority crate in this round: {forbidden}"
        );
    }
}

#[test]
fn storage_source_does_not_implement_phase3_validator_or_passport_authority() {
    let mut files = Vec::new();
    collect_rs_files(&crate_dir().join("src"), &mut files);

    assert!(
        !files.is_empty(),
        "source scanner should find svc-storage Rust files"
    );

    for path in files {
        let code = strip_line_comments(&read(&path));

        for forbidden in [
            "QuickChainValidator",
            "ValidatorCapability",
            "ValidatorSet",
            "ValidatorAdmission",
            "ValidatorRevocation",
            "validator_set_hash",
            "validator_passport_subject",
            "validator_capability",
            "validator_capability_scope",
            "validator_registry_epoch",
            "passport_admission_proof",
            "passport_revocation_proof",
            "registry_membership_proof",
            "registry_admission_proof",
            "passport_required",
            "bond_required",
            "bonded_economics",
            "validator_bond",
            "staking_power",
            "admit_validator",
            "revoke_validator",
            "rotate_validator",
            "slash_validator",
            "mint_from_storage",
            "storage_validator_runtime",
            "cache_validator_authority",
            "cache_unlock_authority",
            "ron_ledger::",
            "svc_passport",
            "svc_registry",
            "ron_auth::",
        ] {
            assert!(
                !code.contains(forbidden),
                "svc-storage source must not implement Phase 3 validator/passport/economic authority via `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn storage_paid_policy_and_accounting_sources_have_no_phase3_authority_keys() {
    let selected = strip_line_comments(
        &[
            "src/http/routes/paid_object.rs",
            "src/policy/paid_write.rs",
            "src/policy/settlement.rs",
            "src/accounting/mod.rs",
            "src/accounting/exporter.rs",
            "src/storage/mod.rs",
        ]
        .iter()
        .map(|relative| read(crate_dir().join(relative)))
        .collect::<Vec<_>>()
        .join("\n"),
    );

    for forbidden in PHASE3_VALIDATOR_AUTHORITY_KEYS {
        assert!(
            !selected.contains(forbidden),
            "svc-storage selected paid/policy/accounting/storage source must not expose Phase 3 authority marker `{forbidden}`"
        );
    }
}
