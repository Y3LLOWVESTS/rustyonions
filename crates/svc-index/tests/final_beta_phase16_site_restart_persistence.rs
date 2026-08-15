//! RO:WHAT — FINAL_BETA Phase 16 restart/recreate persistence proof for named-Site manifest pointers.
//! RO:WHY — Site acceptance requires durable named-Site resolution to survive a real svc-index sled store reopen.
//! RO:INTERACTS — svc_index::store::Store, SiteManifestPointer, RON_INDEX_DB.
//! RO:INVARIANTS — pointer survives store drop/reopen; recreate updates only the named pointer; no raw bytes or economic truth are created.
//! RO:METRICS — none.
//! RO:CONFIG — isolated temporary RON_INDEX_DB path; requires sled-store feature.
//! RO:SECURITY — reference metadata only; no wallet/ledger/receipt/ownership authority.
//! RO:TEST — cargo test -p svc-index --features sled-store --test final_beta_phase16_site_restart_persistence.

#![cfg(feature = "sled-store")]

use std::{
    env,
    fs,
    path::PathBuf,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use svc_index::{
    store::Store,
    types::SiteManifestPointer,
};

const MANIFEST_A: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const MANIFEST_B: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn isolated_db_path() -> PathBuf {
    let stamp =
        SystemTime::now()
            .duration_since(
                UNIX_EPOCH,
            )
            .expect(
                "system clock after epoch",
            )
            .as_nanos();

    env::temp_dir().join(
        format!(
            "svc-index-phase16-site-restart-{}-{stamp}",
            std::process::id(),
        ),
    )
}

fn pointer(
    manifest_cid:
        &str,

    updated_at_ms:
        u64,
) -> SiteManifestPointer {
    SiteManifestPointer {
        version:
            1,

        name:
            "phase16-restart-site"
                .to_owned(),

        manifest_cid:
            manifest_cid
                .to_owned(),

        owner_passport_subject:
            Some(
                "passport:main:phase16-owner"
                    .to_owned(),
            ),

        owner_wallet_account:
            Some(
                "acct_phase16_owner"
                    .to_owned(),
            ),

        updated_at_ms,
    }
}

#[test]
fn phase16_named_site_pointer_survives_restart_and_recreate() {
    let path =
        isolated_db_path();

    env::set_var(
        "RON_INDEX_DB",
        &path,
    );

    let first =
        pointer(
            MANIFEST_A,
            1_800_000_000_000,
        );

    {
        let store =
            Store::new(
                true,
            )
            .expect(
                "open first persistent svc-index store",
            );

        store
            .put_site_manifest_pointer(
                &first,
            )
            .expect(
                "write first Site pointer",
            );

        assert_eq!(
            store
                .get_site_manifest_pointer(
                    "phase16-restart-site",
                )
                .expect(
                    "read first Site pointer",
                ),
            first,
        );
    }

    {
        let reopened =
            Store::new(
                true,
            )
            .expect(
                "reopen persistent svc-index store",
            );

        assert_eq!(
            reopened
                .get_site_manifest_pointer(
                    "phase16-restart-site",
                )
                .expect(
                    "Site pointer survives restart",
                ),
            first,
        );

        let recreated =
            pointer(
                MANIFEST_B,
                1_800_000_000_001,
            );

        reopened
            .put_site_manifest_pointer(
                &recreated,
            )
            .expect(
                "recreate/update Site pointer",
            );

        assert_eq!(
            reopened
                .get_site_manifest_pointer(
                    "phase16-restart-site",
                )
                .expect(
                    "read recreated Site pointer",
                ),
            recreated,
        );
    }

    {
        let reopened_again =
            Store::new(
                true,
            )
            .expect(
                "reopen after recreate",
            );

        let final_pointer =
            reopened_again
                .get_site_manifest_pointer(
                    "phase16-restart-site",
                )
                .expect(
                    "recreated pointer survives second restart",
                );

        assert_eq!(
            final_pointer
                .manifest_cid,
            MANIFEST_B,
        );

        assert_eq!(
            final_pointer
                .updated_at_ms,
            1_800_000_000_001,
        );
    }

    env::remove_var(
        "RON_INDEX_DB",
    );

    let _ =
        fs::remove_dir_all(
            &path,
        );
}
