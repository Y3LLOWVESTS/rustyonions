//! RO:WHAT — Focused HTTP contract tests for FINAL_BETA Phase 14 publication relations.
//! RO:WHY — Proves strict query/body decoding, bounded exact-parent reads, validated writes, and authority non-expansion.
//! RO:SECURITY — relation metadata only; no raw bytes, wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.

use serde_json::json;

use svc_index::{
    error::SvcError,

    http::routes::
        publication_relations::{
            list_publication_relations_from_store,
            put_publication_relation_into_store,
            PublicationRelationQuery,
        },

    publications::{
        PublicationReferencesV1,
    },

    relations::{
        PublicationRelationKind,
        PublicationRelationVisibility,
        PublicationRelationSummaryV1,
        PublicationRelationV1,
        PUBLICATION_RELATION_PAGE_SCHEMA,
        PUBLICATION_RELATION_SCHEMA,
    },

    store::Store,
};

const SITE: &str =
    "crab://picture-board";

const IMAGE_A: &str =
    "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.image";

const IMAGE_B: &str =
    "crab://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.image";

fn relation(
    id:
        &str,

    hash:
        char,

    parent:
        &str,

    created_at_ms:
        u64,

    visibility:
        PublicationRelationVisibility,
) -> PublicationRelationV1 {
    let hash =
        hash
            .to_string()
            .repeat(
                64,
            );

    PublicationRelationV1 {
        schema:
            PUBLICATION_RELATION_SCHEMA
                .to_owned(),

        publication:
            PublicationRelationSummaryV1 {
                publication_id:
                    id.to_owned(),

                kind:
                    PublicationRelationKind::
                        Comment,

                crab_url:
                    format!(
                        "crab://{hash}.comment",
                    ),

                title:
                    format!(
                        "Reply {id}",
                    ),

                summary:
                    format!(
                        "Reply body {id}",
                    ),

                creator_display:
                    Some(
                        "Alice"
                            .to_owned(),
                    ),

                created_at_ms,

                visibility,

                references:
                    Some(
                        PublicationReferencesV1 {
                            manifest_cid:
                                None,

                            content_cid:
                                None,

                            site_url:
                                Some(
                                    SITE
                                        .to_owned(),
                                ),
                        },
                    ),
            },

        parent_crab_url:
            parent
                .to_owned(),

        thread_crab_url:
            Some(
                parent
                    .to_owned(),
            ),

        site_crab_url:
            SITE
                .to_owned(),
    }
}

#[test]
fn phase14a6b_query_contract_is_strict_and_camel_case() {
    let query =
        serde_json::from_value::<
            PublicationRelationQuery,
        >(
            json!({
                "parentCrabUrl": IMAGE_A,
                "cursor": null,
                "limit": 50
            }),
        )
        .expect(
            "strict relation query",
        );

    assert_eq!(
        query.parent_crab_url,
        IMAGE_A,
    );

    assert_eq!(
        query.limit,
        Some(
            50,
        ),
    );

    let rejected =
        serde_json::from_value::<
            PublicationRelationQuery,
        >(
            json!({
                "parentCrabUrl": IMAGE_A,
                "limit": 50,
                "walletBalance": "100"
            }),
        );

    assert!(
        rejected.is_err(),
    );
}

#[test]
fn phase14a6b_put_validates_then_persists_relation() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let record =
        relation(
            "reply-one",
            'c',
            IMAGE_A,
            1_000,
            PublicationRelationVisibility::PublicPreview,
        );

    let stored =
        put_publication_relation_into_store(
            &store,
            record.clone(),
        )
        .expect(
            "validated relation write",
        );

    assert_eq!(
        stored,
        record,
    );

    let page =
        list_publication_relations_from_store(
            &store,
            PublicationRelationQuery {
                parent_crab_url:
                    IMAGE_A
                        .to_owned(),

                cursor:
                    None,

                limit:
                    Some(
                        50,
                    ),
            },
        )
        .expect(
            "relation read",
        );

    assert_eq!(
        page.schema,
        PUBLICATION_RELATION_PAGE_SCHEMA,
    );

    assert_eq!(
        page.items,
        vec![
            record,
        ],
    );
}

#[test]
fn phase14a6b_read_is_exact_parent_oldest_first_and_bounded() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    for record in [
        relation(
            "second",
            'd',
            IMAGE_A,
            1_002,
            PublicationRelationVisibility::Public,
        ),
        relation(
            "first",
            'e',
            IMAGE_A,
            1_001,
            PublicationRelationVisibility::Public,
        ),
        relation(
            "other-parent",
            'f',
            IMAGE_B,
            1_000,
            PublicationRelationVisibility::Public,
        ),
    ] {
        put_publication_relation_into_store(
            &store,
            record,
        )
        .expect(
            "relation write",
        );
    }

    let page =
        list_publication_relations_from_store(
            &store,
            PublicationRelationQuery {
                parent_crab_url:
                    IMAGE_A
                        .to_owned(),

                cursor:
                    None,

                limit:
                    Some(
                        1,
                    ),
            },
        )
        .expect(
            "bounded page",
        );

    assert_eq!(
        page.items
            .len(),
        1,
    );

    assert_eq!(
        page.items[
            0
        ]
            .publication
            .publication_id,
        "first",
    );

    assert_eq!(
        page.has_more,
        true,
    );

    assert!(
        page.next_cursor
            .as_deref()
            .is_some_and(
                |cursor| {
                    cursor.starts_with(
                        "r_",
                    )
                },
            ),
    );
}

#[test]
fn phase14a6b_public_read_filters_private_and_unlisted_but_preserves_moderation_placeholders() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    for (
        id,
        hash,
        visibility,
    ) in [
        (
            "public",
            '1',
            PublicationRelationVisibility::Public,
        ),
        (
            "private",
            '2',
            PublicationRelationVisibility::Private,
        ),
        (
            "unlisted",
            '3',
            PublicationRelationVisibility::Unlisted,
        ),
        (
            "deleted",
            '4',
            PublicationRelationVisibility::Deleted,
        ),
        (
            "blocked",
            '5',
            PublicationRelationVisibility::Blocked,
        ),
        (
            "moderated",
            '6',
            PublicationRelationVisibility::Moderated,
        ),
    ] {
        put_publication_relation_into_store(
            &store,
            relation(
                id,
                hash,
                IMAGE_A,
                1_003,
                visibility,
            ),
        )
        .expect(
            "relation write",
        );
    }

    let page =
        list_publication_relations_from_store(
            &store,
            PublicationRelationQuery {
                parent_crab_url:
                    IMAGE_A
                        .to_owned(),

                cursor:
                    None,

                limit:
                    Some(
                        100,
                    ),
            },
        )
        .expect(
            "public relation page",
        );

    let ids =
        page
            .items
            .iter()
            .map(
                |item| {
                    item
                        .publication
                        .publication_id
                        .as_str()
                },
            )
            .collect::<
                Vec<_>,
            >();

    assert!(
        ids.contains(
            &"public",
        ),
    );

    assert!(
        ids.contains(
            &"deleted",
        ),
    );

    assert!(
        ids.contains(
            &"blocked",
        ),
    );

    assert!(
        ids.contains(
            &"moderated",
        ),
    );

    assert_eq!(
        ids.contains(
            &"private",
        ),
        false,
    );

    assert_eq!(
        ids.contains(
            &"unlisted",
        ),
        false,
    );
}

#[test]
fn phase14a6b_invalid_query_and_invalid_body_fail_closed() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    for query in [
        PublicationRelationQuery {
            parent_crab_url:
                "https://example.com/image"
                    .to_owned(),

            cursor:
                None,

            limit:
                Some(
                    50,
                ),
        },

        PublicationRelationQuery {
            parent_crab_url:
                IMAGE_A
                    .to_owned(),

            cursor:
                Some(
                    "plain-offset"
                        .to_owned(),
                ),

            limit:
                Some(
                    50,
                ),
        },

        PublicationRelationQuery {
            parent_crab_url:
                IMAGE_A
                    .to_owned(),

            cursor:
                None,

            limit:
                Some(
                    101,
                ),
        },
    ] {
        assert!(
            matches!(
                list_publication_relations_from_store(
                    &store,
                    query,
                ),
                Err(
                    SvcError::BadRequest(
                        _,
                    ),
                ),
            ),
        );
    }

    let mut invalid =
        relation(
            "bad-reply",
            '7',
            IMAGE_A,
            1_004,
            PublicationRelationVisibility::Public,
        );

    invalid.site_crab_url =
        "https://invalid.example"
            .to_owned();

    assert!(
        matches!(
            put_publication_relation_into_store(
                &store,
                invalid,
            ),
            Err(
                SvcError::BadRequest(
                    _,
                ),
            ),
        ),
    );

    let page =
        list_publication_relations_from_store(
            &store,
            PublicationRelationQuery {
                parent_crab_url:
                    IMAGE_A
                        .to_owned(),

                cursor:
                    None,

                limit:
                    Some(
                        50,
                    ),
            },
        )
        .expect(
            "empty relation page",
        );

    assert!(
        page.items
            .is_empty(),
    );
}

#[test]
fn phase14a6b_router_registers_generic_relation_boundary_without_new_authority() {
    let route_source =
        include_str!(
            "../src/http/routes/publication_relations.rs",
        );

    let router_source =
        include_str!(
            "../src/router.rs",
        );

    let compact_router =
        router_source
            .split_whitespace()
            .collect::<
                String,
            >();

    for required in [
        "/v1/index/publication-relations",
        "routes::publication_relations::put_publication_relation",
        "routes::publication_relations::list_publication_relations",
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
        "settlement_authority",
        "follow_mutation",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
        "quickchain_finality",
        "rox_settlement",
        "solana_mutation",
    ] {
        assert_eq!(
            route_source.contains(
                forbidden,
            ),
            false,
            "forbidden authority: {forbidden}",
        );
    }

    assert!(
        route_source.contains(
            "relation metadata only",
        ),
    );

    assert_eq!(
        route_source.contains(
            "imageboard",
        ),
        false,
    );
}
