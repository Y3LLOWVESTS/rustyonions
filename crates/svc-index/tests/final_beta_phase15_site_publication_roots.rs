//! RO:WHAT — FINAL_BETA Phase 15 generic Site-publication root projection tests.
//! RO:WHY — Forum thread lists need durable Site-keyed Post roots while Comment relations continue to own reply chains.
//! RO:INVARIANTS — Post/Image/Article roots only; exact Site reference; bounded taxonomy; newest-first bounded pagination; private/unlisted omission.
//! RO:SECURITY — creatorDisplay is display-only; no verified username/profile/Passport claim and no economic or moderation authority.

use serde_json::{
    json,
};

use svc_index::{
    http::routes::site_publications::{
        list_site_publications_from_store,
        put_site_publication_into_store,
        SitePublicationQuery,
    },

    publications::PublicationReferencesV1,

    relations::PublicationRelationVisibility,

    site_publications::{
        SitePublicationKind,
        SitePublicationPageRequest,
        SitePublicationV1,
        SITE_PUBLICATION_PAGE_SCHEMA,
        SITE_PUBLICATION_SCHEMA,
    },

    store::{
        keys,
        Store,
    },
};

const SITE_A:
    &str =
    "crab://forum-one";

const SITE_B:
    &str =
    "crab://forum-two";

const MANIFEST:
    &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const CONTENT:
    &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn typed_url(
    hash:
        char,

    suffix:
        &str,
) -> String {
    format!(
        "crab://{}.{}",
        hash
            .to_string()
            .repeat(
                64,
            ),
        suffix,
    )
}

fn root(
    id:
        &str,

    hash:
        char,

    kind:
        SitePublicationKind,

    site:
        &str,

    created_at_ms:
        u64,

    visibility:
        PublicationRelationVisibility,
) -> SitePublicationV1 {
    SitePublicationV1 {
        schema:
            SITE_PUBLICATION_SCHEMA
                .to_owned(),

        publication_id:
            id.to_owned(),

        kind,

        crab_url:
            typed_url(
                hash,
                kind.suffix(),
            ),

        title:
            format!(
                "Root {id}",
            ),

        summary:
            format!(
                "Summary {id}",
            ),

        creator_display:
            Some(
                "Forum Author"
                    .to_owned(),
            ),

        created_at_ms,

        visibility,

        references:
            PublicationReferencesV1 {
                manifest_cid:
                    Some(
                        MANIFEST
                            .to_owned(),
                    ),

                content_cid:
                    Some(
                        CONTENT
                            .to_owned(),
                    ),

                site_url:
                    Some(
                        site
                            .to_owned(),
                    ),
            },

        tags:
            vec![
                "forum"
                    .to_owned(),

                "forum-category:general"
                    .to_owned(),
            ],

        site_crab_url:
            site.to_owned(),
    }
}

#[test]
fn phase15a4a2b1_forum_post_root_preserves_bounded_category_tags_and_display_only_creator() {
    let value =
        root(
            "thread-a",
            'c',
            SitePublicationKind::Post,
            SITE_A,
            100,
            PublicationRelationVisibility::Public,
        );

    value
        .validate()
        .expect(
            "Forum Post root should validate",
        );

    assert_eq!(
        value.tags,
        vec![
            "forum",
            "forum-category:general",
        ],
    );

    assert_eq!(
        value.creator_display
            .as_deref(),
        Some(
            "Forum Author",
        ),
    );
}

#[test]
fn phase15a4a2b1_shared_root_contract_accepts_post_image_and_article_but_not_comment() {
    for (
        kind,
        hash,
    ) in [
        (
            SitePublicationKind::Post,
            'c',
        ),

        (
            SitePublicationKind::Image,
            'd',
        ),

        (
            SitePublicationKind::Article,
            'e',
        ),
    ] {
        root(
            "shared-root",
            hash,
            kind,
            SITE_A,
            100,
            PublicationRelationVisibility::Public,
        )
        .validate()
        .expect(
            "shared Site root kind should validate",
        );
    }

    let invalid =
        serde_json::from_value::<
            SitePublicationV1
        >(
            json!({
                "schema":
                    SITE_PUBLICATION_SCHEMA,

                "publicationId":
                    "comment-root",

                "kind":
                    "comment",

                "crabUrl":
                    typed_url(
                        'f',
                        "comment",
                    ),

                "title":
                    "Comment",

                "summary":
                    "Comment must remain in relation index.",

                "creatorDisplay":
                    "Forum Author",

                "createdAtMs":
                    100,

                "visibility":
                    "public",

                "references": {
                    "manifestCid":
                        MANIFEST,

                    "contentCid":
                        CONTENT,

                    "siteUrl":
                        SITE_A,
                },

                "tags": [
                    "forum"
                ],

                "siteCrabUrl":
                    SITE_A,
            }),
        );

    assert!(
        invalid.is_err(),
        "Comment must not deserialize as a Site root publication",
    );
}

#[test]
fn phase15a4a2b1_exact_site_reference_and_bounded_tags_fail_closed() {
    let mut drift =
        root(
            "drift",
            'c',
            SitePublicationKind::Post,
            SITE_A,
            100,
            PublicationRelationVisibility::Public,
        );

    drift
        .references
        .site_url =
        Some(
            SITE_B
                .to_owned(),
        );

    assert!(
        drift
            .validate()
            .is_err(),
        "Site reference drift must reject",
    );

    let mut too_many_tags =
        root(
            "tags",
            'd',
            SitePublicationKind::Post,
            SITE_A,
            100,
            PublicationRelationVisibility::Public,
        );

    too_many_tags.tags =
        (
            0..33
        )
            .map(
                |index| {
                    format!(
                        "tag-{index}",
                    )
                },
            )
            .collect();

    assert!(
        too_many_tags
            .validate()
            .is_err(),
        "more than 32 tags must reject",
    );
}

#[test]
fn phase15a4a2b1_store_isolates_exact_named_sites() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let first =
        root(
            "same-id",
            'c',
            SitePublicationKind::Post,
            SITE_A,
            100,
            PublicationRelationVisibility::Public,
        );

    let second =
        root(
            "same-id",
            'd',
            SitePublicationKind::Post,
            SITE_B,
            200,
            PublicationRelationVisibility::Public,
        );

    store
        .put_site_publication(
            &first,
        )
        .expect(
            "Site A write",
        );

    store
        .put_site_publication(
            &second,
        )
        .expect(
            "Site B write",
        );

    let request =
        SitePublicationPageRequest::new(
            None,
            Some(
                20,
            ),
        )
        .expect(
            "page request",
        );

    let page_a =
        store
            .list_site_publications(
                SITE_A,
                &request,
            )
            .expect(
                "Site A page",
            );

    let page_b =
        store
            .list_site_publications(
                SITE_B,
                &request,
            )
            .expect(
                "Site B page",
            );

    assert_eq!(
        page_a.items.len(),
        1,
    );

    assert_eq!(
        page_b.items.len(),
        1,
    );

    assert_eq!(
        page_a.items[0]
            .crab_url,
        first.crab_url,
    );

    assert_eq!(
        page_b.items[0]
            .crab_url,
        second.crab_url,
    );

    assert_ne!(
        keys::site_publication_key(
            SITE_A,
            "same-id",
        ),

        keys::site_publication_key(
            SITE_B,
            "same-id",
        ),
    );
}

#[test]
fn phase15a4a2b1_public_page_omits_private_unlisted_and_keeps_moderation_placeholders() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let records = [
        (
            "public",
            'a',
            PublicationRelationVisibility::Public,
        ),

        (
            "preview",
            'b',
            PublicationRelationVisibility::PublicPreview,
        ),

        (
            "private",
            'c',
            PublicationRelationVisibility::Private,
        ),

        (
            "unlisted",
            'd',
            PublicationRelationVisibility::Unlisted,
        ),

        (
            "deleted",
            'e',
            PublicationRelationVisibility::Deleted,
        ),

        (
            "blocked",
            'f',
            PublicationRelationVisibility::Blocked,
        ),

        (
            "moderated",
            '1',
            PublicationRelationVisibility::Moderated,
        ),
    ];

    for (
        index,
        (
            id,
            hash,
            visibility,
        ),
    ) in records
        .into_iter()
        .enumerate()
    {
        store
            .put_site_publication(
                &root(
                    id,
                    hash,
                    SitePublicationKind::Post,
                    SITE_A,
                    100
                        +
                        index as u64,
                    visibility,
                ),
            )
            .expect(
                "root write",
            );
    }

    let page =
        store
            .list_site_publications(
                SITE_A,
                &SitePublicationPageRequest::new(
                    None,
                    Some(
                        20,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "public page",
            );

    let ids =
        page
            .items
            .iter()
            .map(
                |item| {
                    item
                        .publication_id
                        .as_str()
                },
            )
            .collect::<
                Vec<_>
            >();

    assert!(
        ids.contains(
            &"public",
        ),
    );

    assert!(
        ids.contains(
            &"preview",
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

    assert!(
        !ids.contains(
            &"private",
        ),
    );

    assert!(
        !ids.contains(
            &"unlisted",
        ),
    );
}

#[test]
fn phase15a4a2b1_site_page_is_newest_first_and_uses_opaque_bounded_pagination() {
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
        created_at_ms,
    ) in [
        (
            "old",
            'a',
            100,
        ),

        (
            "new",
            'b',
            300,
        ),

        (
            "middle",
            'c',
            200,
        ),
    ] {
        store
            .put_site_publication(
                &root(
                    id,
                    hash,
                    SitePublicationKind::Post,
                    SITE_A,
                    created_at_ms,
                    PublicationRelationVisibility::Public,
                ),
            )
            .expect(
                "root write",
            );
    }

    let first =
        store
            .list_site_publications(
                SITE_A,
                &SitePublicationPageRequest::new(
                    None,
                    Some(
                        2,
                    ),
                )
                .expect(
                    "first request",
                ),
            )
            .expect(
                "first page",
            );

    assert_eq!(
        first.schema,
        SITE_PUBLICATION_PAGE_SCHEMA,
    );

    assert_eq!(
        first
            .items
            .iter()
            .map(
                |item| {
                    item
                        .publication_id
                        .as_str()
                },
            )
            .collect::<Vec<_>>(),
        vec![
            "new",
            "middle",
        ],
    );

    assert!(
        first.has_more,
    );

    let cursor =
        first
            .next_cursor
            .clone()
            .expect(
                "next cursor",
            );

    assert!(
        cursor
            .starts_with(
                "s_",
            ),
    );

    let second =
        store
            .list_site_publications(
                SITE_A,
                &SitePublicationPageRequest::new(
                    Some(
                        cursor,
                    ),
                    Some(
                        2,
                    ),
                )
                .expect(
                    "second request",
                ),
            )
            .expect(
                "second page",
            );

    assert_eq!(
        second.items.len(),
        1,
    );

    assert_eq!(
        second.items[0]
            .publication_id,
        "old",
    );

    assert!(
        !second.has_more,
    );
}

#[test]
fn phase15a4a2b1_http_helpers_reuse_same_store_projection() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let value =
        root(
            "http-root",
            'c',
            SitePublicationKind::Post,
            SITE_A,
            500,
            PublicationRelationVisibility::Public,
        );

    let written =
        put_site_publication_into_store(
            &store,
            value.clone(),
        )
        .expect(
            "HTTP helper write",
        );

    assert_eq!(
        written,
        value,
    );

    let page =
        list_site_publications_from_store(
            &store,
            SitePublicationQuery {
                site_crab_url:
                    SITE_A
                        .to_owned(),

                cursor:
                    None,

                limit:
                    Some(
                        20,
                    ),
            },
        )
        .expect(
            "HTTP helper read",
        );

    assert_eq!(
        page.items,
        vec![
            value,
        ],
    );
}

#[test]
fn phase15a4a2b1_source_boundary_is_generic_site_index_not_creator_or_forum_authority() {
    let model_source =
        include_str!(
            "../src/site_publications.rs",
        );

    let route_source =
        include_str!(
            "../src/http/routes/site_publications.rs",
        );

    let router_source =
        include_str!(
            "../src/router.rs",
        );

    for required in [
        "crablink.site-publication.v1",
        "SitePublicationKind",
        "Post",
        "Image",
        "Article",
        "creator_display",
        "tags",
        "site_crab_url",
        "SITE_PUBLICATION_PAGE_MAX_LIMIT",
    ] {
        assert!(
            model_source.contains(
                required,
            ),
            "missing generic Site-publication marker: {required}",
        );
    }

    for required in [
        "/v1/index/site-publications",
        "put_site_publication",
        "list_site_publications",
    ] {
        assert!(
            route_source.contains(
                required,
            )
                ||
            router_source.contains(
                required,
            ),
            "missing Site-publication HTTP marker: {required}",
        );
    }

    for forbidden in [
        "creator.username",
        "profile_url",
        "profileUrl",
        "passport_subject",
        "passportSubject",
        "verifiedCreator",
        "walletMutation",
        "ledgerMutation",
        "receiptAuthority",
        "entitlementAuthority",
        "moderationMutation",
        "quickchain_submit",
        "rox_mint",
        "solana_submit",
    ] {
        assert!(
            !model_source.contains(
                forbidden,
            ),
            "Site-publication projection crossed authority boundary: {forbidden}",
        );
    }

    assert!(
        !model_source.contains(
            "Comment,"
        ),
        "Comment must remain owned by publication relations",
    );
}
