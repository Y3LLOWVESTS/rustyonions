//! RO:WHAT — FINAL_BETA Phase 15A4A1 generic creator-publication projection write tests.
//! RO:WHY — Successful publishers need one validated write boundary feeding the existing public creator-publication reader.
//! RO:INTERACTS — svc-index creator_publications route helper, Store, PublicationSummaryV1.
//! RO:INVARIANTS — body validation and exact route identity precede mutation; existing GET semantics remain unchanged.
//! RO:SECURITY — projection metadata only; no wallet, ledger, receipt, entitlement, settlement, key, PIN, recovery, or capability authority.

use svc_index::{
    error::SvcError,

    http::routes::creator_publications::{
        get_creator_publication_from_store,
        put_creator_publication_into_store,
    },

    publications::{
        PublicationAccess,
        PublicationCreatorV1,
        PublicationKind,
        PublicationReferencesV1,
        PublicationSummaryV1,
        PublicationVisibility,
        PUBLICATION_SUMMARY_SCHEMA,
    },

    store::Store,
};

const HASH_A: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const HASH_B: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const POST_HASH: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

fn publication(
    publication_id: &str,
) -> PublicationSummaryV1 {
    PublicationSummaryV1 {
        schema:
            PUBLICATION_SUMMARY_SCHEMA
                .to_owned(),

        publication_id:
            publication_id
                .to_owned(),

        kind:
            PublicationKind::Post,

        crab_url:
            format!(
                "crab://{POST_HASH}.post",
            ),

        title:
            "Forum-compatible Post"
                .to_owned(),

        summary:
            "Backend-derived creator publication projection."
                .to_owned(),

        creator:
            PublicationCreatorV1 {
                username:
                    "rusty_crab"
                        .to_owned(),

                display_name:
                    "Rusty Crab"
                        .to_owned(),

                profile_url:
                    "crab://@rusty_crab"
                        .to_owned(),

                avatar_cid:
                    None,
            },

        published_at:
            "2026-08-13T02:00:00.000Z"
                .to_owned(),

        updated_at:
            "2026-08-13T02:00:00.000Z"
                .to_owned(),

        visibility:
            PublicationVisibility::Public,

        access:
            PublicationAccess::Free,

        thumbnail:
            None,

        references:
            Some(
                PublicationReferencesV1 {
                    manifest_cid:
                        Some(
                            HASH_A.to_owned(),
                        ),

                    content_cid:
                        Some(
                            HASH_B.to_owned(),
                        ),

                    site_url:
                        Some(
                            "crab://rusty-forum"
                                .to_owned(),
                        ),
                },
            ),

        pinned:
            false,
    }
}

#[test]
fn phase15a4a1_valid_projection_write_persists_and_existing_public_detail_reads_it() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let value =
        publication(
            "thread-001",
        );

    put_creator_publication_into_store(
        &store,
        "rusty_crab",
        "thread-001",
        &value,
    )
    .expect(
        "validated publication write",
    );

    let fetched =
        get_creator_publication_from_store(
            &store,
            "rusty_crab",
            "thread-001",
        )
        .expect(
            "existing public detail read",
        );

    assert_eq!(
        fetched,
        value,
    );
}

#[test]
fn phase15a4a1_creator_path_mismatch_rejects_before_store_mutation() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let value =
        publication(
            "thread-creator-mismatch",
        );

    let result =
        put_creator_publication_into_store(
            &store,
            "other_crab",
            "thread-creator-mismatch",
            &value,
        );

    assert!(
        matches!(
            result,
            Err(
                SvcError::BadRequest(_),
            ),
        ),
    );

    assert!(
        store
            .get_creator_publication(
                "rusty_crab",
                "thread-creator-mismatch",
            )
            .expect(
                "store read",
            )
            .is_none(),
    );
}

#[test]
fn phase15a4a1_publication_id_path_mismatch_rejects_before_store_mutation() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let value =
        publication(
            "thread-body-id",
        );

    let result =
        put_creator_publication_into_store(
            &store,
            "rusty_crab",
            "thread-path-id",
            &value,
        );

    assert!(
        matches!(
            result,
            Err(
                SvcError::BadRequest(_),
            ),
        ),
    );

    assert!(
        store
            .get_creator_publication(
                "rusty_crab",
                "thread-body-id",
            )
            .expect(
                "store read",
            )
            .is_none(),
    );
}

#[test]
fn phase15a4a1_invalid_publication_body_rejects_before_store_mutation() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let mut value =
        publication(
            "thread-invalid",
        );

    value.title =
        String::new();

    value.summary =
        String::new();

    let result =
        put_creator_publication_into_store(
            &store,
            "rusty_crab",
            "thread-invalid",
            &value,
        );

    assert!(
        matches!(
            result,
            Err(
                SvcError::BadRequest(_),
            ),
        ),
    );

    assert!(
        store
            .get_creator_publication(
                "rusty_crab",
                "thread-invalid",
            )
            .expect(
                "store read",
            )
            .is_none(),
    );
}

#[test]
fn phase15a4a1_router_owns_put_plus_existing_get_on_same_detail_path() {
    let router_source =
        include_str!(
            "../src/router.rs",
        );

    let compact:
        String =
        router_source
            .split_whitespace()
            .collect();

    for required in [
        "/v1/index/creators/:username/publications/:publication_id",
        "put(routes::creator_publications::put_creator_publication,)",
        ".get(routes::creator_publications::get_creator_publication,)",
    ] {
        assert!(
            compact.contains(
                required,
            ),
            "missing creator-publication router marker: {required}",
        );
    }

    let route_source =
        include_str!(
            "../src/http/routes/creator_publications.rs",
        );

    for required in [
        "pub async fn put_creator_publication",
        "put_creator_publication_into_store",
        "publication creator username does not match route",
        "publication id does not match route",
        "StatusCode::NO_CONTENT",
    ] {
        assert!(
            route_source.contains(
                required,
            ),
            "missing publication-write boundary marker: {required}",
        );
    }
}
