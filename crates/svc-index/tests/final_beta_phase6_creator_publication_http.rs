//! RO:WHAT — Focused tests for svc-index creator-publication HTTP projection helpers.
//! RO:WHY — Proves public filtering, strict query decoding, bounded pagination, detail isolation, and non-authority posture.

use serde_json::json;
use svc_index::{
    error::SvcError,
    http::routes::creator_publications::{
        get_creator_publication_from_store,
        list_creator_publications_from_store,
        CreatorPublicationQuery,
    },
    publications::{
        PublicationAccess,
        PublicationCreatorV1,
        PublicationKind,
        PublicationReferencesV1,
        PublicationSummaryV1,
        PublicationThumbnailKind,
        PublicationThumbnailV1,
        PublicationVisibility,
        PUBLICATION_PAGE_SCHEMA,
        PUBLICATION_SUMMARY_SCHEMA,
    },
    store::Store,
};

const HASH_A: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const HASH_B: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn publication(
    publication_id: &str,
    published_at: &str,
    visibility: PublicationVisibility,
    pinned: bool,
) -> PublicationSummaryV1 {
    PublicationSummaryV1 {
        schema:
            PUBLICATION_SUMMARY_SCHEMA
                .to_owned(),
        publication_id:
            publication_id.to_owned(),
        kind:
            PublicationKind::Post,
        crab_url:
            format!(
                "crab://{publication_id}.post",
            ),
        title:
            format!(
                "Publication {publication_id}",
            ),
        summary:
            "Canonical creator publication projection."
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
                    Some(
                        HASH_B.to_owned(),
                    ),
            },
        published_at:
            published_at.to_owned(),
        updated_at:
            published_at.to_owned(),
        visibility,
        access:
            PublicationAccess::Free,
        thumbnail:
            Some(
                PublicationThumbnailV1 {
                    kind:
                        PublicationThumbnailKind::
                            Image,
                    cid:
                        HASH_A.to_owned(),
                    alt:
                        "Publication thumbnail"
                            .to_owned(),
                },
            ),
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
                        None,
                },
            ),
        pinned,
    }
}

#[test]
fn phase6b2_list_route_projection_is_public_bounded_and_deterministic() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    let records = [
        publication(
            "older",
            "2026-08-05T10:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        publication(
            "newer",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        publication(
            "pinned",
            "2026-08-05T09:00:00.000Z",
            PublicationVisibility::Public,
            true,
        ),
        publication(
            "private",
            "2026-08-05T13:00:00.000Z",
            PublicationVisibility::Private,
            false,
        ),
        publication(
            "moderated",
            "2026-08-05T14:00:00.000Z",
            PublicationVisibility::Moderated,
            false,
        ),
    ];

    for record in &records {
        store
            .put_creator_publication(
                record,
            )
            .expect(
                "store publication",
            );
    }

    let page =
        list_creator_publications_from_store(
            &store,
            "rusty_crab",
            CreatorPublicationQuery {
                cursor: None,
                limit: Some(20),
            },
        )
        .expect(
            "publication page",
        );

    assert_eq!(
        page.schema,
        PUBLICATION_PAGE_SCHEMA,
    );

    assert_eq!(
        page.items.len(),
        3,
    );

    assert_eq!(
        page.items[0].publication_id,
        "pinned",
    );

    assert_eq!(
        page.items[1].publication_id,
        "newer",
    );

    assert_eq!(
        page.items[2].publication_id,
        "older",
    );
}

#[test]
fn phase6b2_list_route_supports_opaque_bounded_pagination() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    for index in 0..5 {
        store
            .put_creator_publication(
                &publication(
                    &format!(
                        "publication-{index}",
                    ),
                    &format!(
                        "2026-08-05T12:00:0{index}.000Z",
                    ),
                    PublicationVisibility::Public,
                    false,
                ),
            )
            .expect(
                "store publication",
            );
    }

    let first =
        list_creator_publications_from_store(
            &store,
            "rusty_crab",
            CreatorPublicationQuery {
                cursor: None,
                limit: Some(2),
            },
        )
        .expect(
            "first page",
        );

    assert_eq!(
        first.items.len(),
        2,
    );

    assert!(
        first.has_more,
    );

    let cursor =
        first
            .next_cursor
            .expect(
                "opaque cursor",
            );

    assert!(
        cursor.starts_with("p_"),
    );

    let second =
        list_creator_publications_from_store(
            &store,
            "rusty_crab",
            CreatorPublicationQuery {
                cursor:
                    Some(cursor),
                limit:
                    Some(2),
            },
        )
        .expect(
            "second page",
        );

    assert_eq!(
        second.items.len(),
        2,
    );

    assert!(
        second.has_more,
    );
}

#[test]
fn phase6b2_detail_route_hides_non_public_and_cross_creator_records() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    let public =
        publication(
            "public-detail",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        );

    let private =
        publication(
            "private-detail",
            "2026-08-05T12:01:00.000Z",
            PublicationVisibility::Private,
            false,
        );

    store
        .put_creator_publication(
            &public,
        )
        .expect(
            "store public",
        );

    store
        .put_creator_publication(
            &private,
        )
        .expect(
            "store private",
        );

    let fetched =
        get_creator_publication_from_store(
            &store,
            "rusty_crab",
            "public-detail",
        )
        .expect(
            "public detail",
        );

    assert_eq!(
        fetched,
        public,
    );

    assert!(
        matches!(
            get_creator_publication_from_store(
                &store,
                "rusty_crab",
                "private-detail",
            ),
            Err(SvcError::NotFound),
        ),
    );

    assert!(
        matches!(
            get_creator_publication_from_store(
                &store,
                "other_crab",
                "public-detail",
            ),
            Err(SvcError::NotFound),
        ),
    );
}

#[test]
fn phase6b2_query_contract_rejects_unknown_fields() {
    let decoded:
        CreatorPublicationQuery =
            serde_json::from_value(
                json!({
                    "cursor": null,
                    "limit": 20
                }),
            )
            .expect(
                "strict query",
            );

    assert_eq!(
        decoded.limit,
        Some(20),
    );

    let rejected =
        serde_json::from_value::<
            CreatorPublicationQuery,
        >(
            json!({
                "cursor": null,
                "limit": 20,
                "walletBalance": "100"
            }),
        );

    assert!(
        rejected.is_err(),
    );
}

#[test]
fn phase6b2_invalid_username_limit_and_cursor_fail_closed() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    for query in [
        CreatorPublicationQuery {
            cursor: None,
            limit: Some(0),
        },
        CreatorPublicationQuery {
            cursor: None,
            limit: Some(51),
        },
        CreatorPublicationQuery {
            cursor:
                Some(
                    "plain-offset"
                        .to_owned(),
                ),
            limit:
                Some(20),
        },
    ] {
        assert!(
            matches!(
                list_creator_publications_from_store(
                    &store,
                    "rusty_crab",
                    query,
                ),
                Err(SvcError::BadRequest(_)),
            ),
        );
    }

    assert!(
        matches!(
            list_creator_publications_from_store(
                &store,
                "../rusty_crab",
                CreatorPublicationQuery::default(),
            ),
            Err(SvcError::BadRequest(_)),
        ),
    );
}

#[test]
fn phase6b2_http_projection_adds_no_economic_or_relationship_authority() {
    let route_source =
        include_str!(
            "../src/http/routes/creator_publications.rs",
        );

    let router_source =
        include_str!(
            "../src/router.rs",
        );

    let compact_router: String =
        router_source
            .split_whitespace()
            .collect();

    for required in [
        "/v1/index/creators/:username/publications",
        "/v1/index/creators/:username/publications/:publication_id",
        "routes::creator_publications::list_creator_publications",
        "routes::creator_publications::get_creator_publication",
    ] {
        assert!(
            compact_router.contains(
                required,
            ),
            "missing route fragment: {required}",
        );
    }

    for forbidden in [
        "wallet_mutation",
        "ledger_mutation",
        "receipt_authority",
        "paid_entitlement_authority",
        "follow_mutation",
        "settlement_authority",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
    ] {
        assert!(
            route_source.contains(
                forbidden,
            ) == false,
            "forbidden authority: {forbidden}",
        );
    }

    assert!(
        route_source.contains(
            "public read projection only",
        ),
    );
}
