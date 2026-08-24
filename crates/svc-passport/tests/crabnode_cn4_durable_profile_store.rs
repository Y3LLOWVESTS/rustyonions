//! RO:WHAT — CN-4 durability/restart/corruption tests for svc-passport username/profile state.
//! RO:WHY — Proves username ownership survives store reopen and persistence failures never create RAM-only success.
//! RO:INTERACTS — UsernameClaimStore::open_durable and immutable profile snapshot backing.
//! RO:INVARIANTS — same-subject idempotency; one subject→one username; one username→one subject; corrupt latest state fails closed.
//! RO:METRICS — none.
//! RO:CONFIG — isolated temporary state directories only.
//! RO:SECURITY — no DevKms, wallet, ledger, private key, or network authority.
//! RO:TEST — this file is the focused CN-4A gate.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use svc_passport::profile::{ProfileClaimError, UsernameClaimRequest, UsernameClaimStore};

fn isolated_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "svc-passport-cn4-{label}-{}-{stamp}",
        std::process::id(),
    ))
}

fn request(subject: &str, username: &str) -> UsernameClaimRequest {
    UsernameClaimRequest {
        passport_subject: subject.to_owned(),
        requested_username: username.to_owned(),
        display_name: Some(username.to_owned()),
        bio: Some("CN-4 durable profile".to_owned()),
        avatar_image: None,
    }
}

fn latest_snapshot(root: &Path) -> PathBuf {
    let mut snapshots = fs::read_dir(root)
        .expect("read profile state dir")
        .map(|entry| entry.expect("read profile state entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    snapshots.sort();

    snapshots.pop().expect("at least one durable snapshot")
}

#[test]
fn durable_claims_survive_reopen_and_preserve_uniqueness_and_idempotency() {
    let root = isolated_root("restart");

    let first = {
        let store = UsernameClaimStore::open_durable(&root).expect("open empty durable store");

        let first = store
            .claim_main_username(
                request("passport:main:testmac", "testmac"),
                1_800_000_000_000,
            )
            .expect("claim testmac");

        let repeated = store
            .claim_main_username(
                request("passport:main:testmac", "testmac"),
                1_800_000_000_999,
            )
            .expect("same-subject retry is idempotent");

        assert_eq!(repeated, first,);

        first
    };

    {
        let reopened = UsernameClaimStore::open_durable(&root).expect("reopen durable store");

        let profile = reopened
            .public_profile("testmac")
            .expect("lookup testmac")
            .expect("testmac survives reopen");

        assert_eq!(profile.passport_subject, "passport:main:testmac",);

        assert_eq!(profile.username, "testmac",);

        let same_subject_other_username = reopened
            .claim_main_username(
                request("passport:main:testmac", "othermac"),
                1_800_000_001_000,
            )
            .expect_err("one main subject cannot claim a second username");

        assert!(matches!(
            same_subject_other_username,
            ProfileClaimError::PassportAlreadyHasUsername { .. }
        ),);

        let same_username_other_subject = reopened
            .claim_main_username(request("passport:main:other", "testmac"), 1_800_000_001_001)
            .expect_err("username cannot move to another subject");

        assert!(matches!(
            same_username_other_subject,
            ProfileClaimError::UsernameUnavailable { .. }
        ),);

        reopened
            .claim_main_username(request("passport:main:testpc", "testpc"), 1_800_000_001_002)
            .expect("independent testpc claim");
    }

    {
        let reopened = UsernameClaimStore::open_durable(&root).expect("second reopen");

        assert_eq!(
            reopened
                .public_profile("testmac",)
                .expect("testmac lookup",)
                .expect("testmac exists",)
                .passport_subject,
            first.passport_subject,
        );

        assert_eq!(
            reopened
                .public_profile("testpc",)
                .expect("testpc lookup",)
                .expect("testpc exists",)
                .passport_subject,
            "passport:main:testpc",
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn corrupt_latest_snapshot_fails_closed_instead_of_falling_back() {
    let root = isolated_root("corrupt");

    {
        let store = UsernameClaimStore::open_durable(&root).expect("open durable store");

        store
            .claim_main_username(
                request("passport:main:testmac", "testmac"),
                1_800_000_010_000,
            )
            .expect("first generation");

        store
            .claim_main_username(request("passport:main:testpc", "testpc"), 1_800_000_010_001)
            .expect("second generation");
    }

    fs::write(latest_snapshot(&root), b"{corrupt").expect("corrupt latest snapshot");

    let error = UsernameClaimStore::open_durable(&root)
        .expect_err("corrupt authoritative generation must fail closed");

    assert!(matches!(error, ProfileClaimError::StoreCorrupt { .. }),);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn snapshot_schema_is_strict_and_versioned() {
    let root = isolated_root("version");

    {
        let store = UsernameClaimStore::open_durable(&root).expect("open durable store");

        store
            .claim_main_username(
                request("passport:main:testmac", "testmac"),
                1_800_000_020_000,
            )
            .expect("create snapshot");
    }

    let snapshot = latest_snapshot(&root);

    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&snapshot).expect("read snapshot"))
            .expect("parse fixture snapshot");

    value["version"] = serde_json::json!(2);

    fs::write(
        &snapshot,
        serde_json::to_vec_pretty(&value).expect("serialize modified snapshot"),
    )
    .expect("write unsupported version");

    assert!(matches!(
        UsernameClaimStore::open_durable(&root,),
        Err(ProfileClaimError::StoreCorrupt { .. })
    ),);

    value["version"] = serde_json::json!(1);

    value["unknown_field"] = serde_json::json!(true);

    fs::write(
        &snapshot,
        serde_json::to_vec_pretty(&value).expect("serialize unknown field snapshot"),
    )
    .expect("write unknown field");

    assert!(matches!(
        UsernameClaimStore::open_durable(&root,),
        Err(ProfileClaimError::StoreCorrupt { .. })
    ),);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn persistence_failure_never_commits_ram_only_username_success() {
    let root = isolated_root("transaction");

    let store = UsernameClaimStore::open_durable(&root).expect("open durable store");

    fs::remove_dir(&root).expect("remove empty durable root");

    fs::write(&root, b"not-a-directory").expect("replace root with regular file");

    let error = store
        .claim_main_username(
            request("passport:main:testmac", "testmac"),
            1_800_000_030_000,
        )
        .expect_err("persistence failure must reject claim");

    assert!(matches!(
        error,
        ProfileClaimError::StoreUnavailable { .. } | ProfileClaimError::StoreCorrupt { .. }
    ),);

    assert!(
        store
            .public_profile("testmac",)
            .expect("RAM lookup after failed claim",)
            .is_none(),
        "failed durable write must not leave RAM-only identity truth",
    );

    let _ = fs::remove_file(&root);
}
