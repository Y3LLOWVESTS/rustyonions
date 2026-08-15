// RO:WHAT — Focused FINAL_BETA Phase 14 svc-index publication-relation tests.
// RO:WHY — Proves durable parent/thread discovery can exist generically before Omnigate Comment publish wiring.
// RO:INVARIANTS — typed Comment source, exact parent/Site context, private/unlisted omission, moderation placeholders, oldest-first bounded pagination.
// RO:SECURITY — no raw bytes, wallet, ledger, receipt, entitlement, moderation mutation, QuickChain, ROX, or Solana authority.

use svc_index::{
    publications::{
        PublicationReferencesV1,
    },
    relations::{
        PublicationRelationKind,
        PublicationRelationVisibility,
        PublicationRelationPageRequest,
        PublicationRelationSummaryV1,
        PublicationRelationV1,
        PUBLICATION_RELATION_PAGE_SCHEMA,
        PUBLICATION_RELATION_SCHEMA,
    },
    store::{
        keys,
        Store,
    },
};

const SITE: &str =
    "crab://picture-board";

const IMAGE_A: &str =
    "crab://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.image";

const IMAGE_B: &str =
    "crab://bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.image";

const MANIFEST_CID: &str =
    "b3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

const CONTENT_CID: &str =
    "b3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn relation(
    id:
        &str,

    hash:
        char,

    parent:
        &str,

    thread:
        Option<&str>,

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
                    PublicationRelationKind::Comment,

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
                        "Reply summary {id}",
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
                                Some(
                                    MANIFEST_CID
                                        .to_owned(),
                                ),

                            content_cid:
                                Some(
                                    CONTENT_CID
                                        .to_owned(),
                                ),

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
            thread
                .map(
                    str::to_owned,
                ),

        site_crab_url:
            SITE
                .to_owned(),
    }
}

#[test]
fn phase14a6a_relation_contract_is_strict_and_comment_owned() {
    let value =
        relation(
            "reply-001",
            'e',
            IMAGE_A,
            Some(
                IMAGE_A,
            ),
            1_000,
            PublicationRelationVisibility::Public,
        );

    value
        .validate()
        .expect(
            "valid Comment relation",
        );

    let mut wrong_kind_json =
        serde_json::to_value(
            &value,
        )
        .expect(
            "relation JSON",
        );

    wrong_kind_json[
        "publication"
    ][
        "kind"
    ] =
        serde_json::Value::String(
            "post"
                .to_owned(),
        );

    assert!(
        serde_json::from_value::<
            PublicationRelationV1,
        >(
            wrong_kind_json,
        )
        .is_err(),
    );

    let mut json =
        serde_json::to_value(
            &value,
        )
        .expect(
            "relation JSON",
        );

    json
        .as_object_mut()
        .expect(
            "relation object",
        )
        .insert(
            "walletAuthority"
                .to_owned(),
            serde_json::Value::Bool(
                true,
            ),
        );

    assert!(
        serde_json::from_value::<
            PublicationRelationV1,
        >(
            json,
        )
        .is_err(),
    );
}

#[test]
fn phase14a6a_relation_requires_exact_site_and_direct_image_thread() {
    let mut wrong_site =
        relation(
            "reply-site",
            'f',
            IMAGE_A,
            Some(
                IMAGE_A,
            ),
            1_001,
            PublicationRelationVisibility::Public,
        );

    wrong_site
        .site_crab_url =
        "crab://other-board"
            .to_owned();

    assert!(
        wrong_site
            .validate()
            .is_err(),
    );

    let wrong_thread =
        relation(
            "reply-thread",
            '1',
            IMAGE_A,
            Some(
                IMAGE_B,
            ),
            1_002,
            PublicationRelationVisibility::Public,
        );

    assert!(
        wrong_thread
            .validate()
            .is_err(),
    );
}

#[test]
fn phase14a6a_relation_keyspace_is_parent_isolated() {
    let first =
        keys::publication_relation_key(
            IMAGE_A,
            "reply-001",
        );

    let second =
        keys::publication_relation_key(
            IMAGE_B,
            "reply-001",
        );

    assert!(
        first.starts_with(
            &keys::publication_relation_prefix(
                IMAGE_A,
            ),
        ),
    );

    assert!(
        second.starts_with(
            &keys::publication_relation_prefix(
                IMAGE_B,
            ),
        ),
    );

    assert_ne!(
        first,
        second,
    );
}

#[test]
fn phase14a6a_store_lists_only_exact_parent_relations() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    for record in [
        relation(
            "reply-a",
            '2',
            IMAGE_A,
            Some(
                IMAGE_A,
            ),
            1_001,
            PublicationRelationVisibility::Public,
        ),
        relation(
            "reply-b",
            '3',
            IMAGE_B,
            Some(
                IMAGE_B,
            ),
            1_002,
            PublicationRelationVisibility::Public,
        ),
    ] {
        store
            .put_publication_relation(
                &record,
            )
            .expect(
                "store relation",
            );
    }

    let page =
        store
            .list_publication_relations(
                IMAGE_A,
                &PublicationRelationPageRequest::new(
                    None,
                    Some(
                        50,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "relation page",
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
            .parent_crab_url,
        IMAGE_A,
    );
}

#[test]
fn phase14a6a_private_and_unlisted_relations_do_not_enter_public_projection() {
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
            '4',
            PublicationRelationVisibility::Public,
        ),
        (
            "private",
            '5',
            PublicationRelationVisibility::Private,
        ),
        (
            "unlisted",
            '6',
            PublicationRelationVisibility::Unlisted,
        ),
        (
            "deleted",
            '7',
            PublicationRelationVisibility::Deleted,
        ),
        (
            "blocked",
            '8',
            PublicationRelationVisibility::Blocked,
        ),
        (
            "moderated",
            '9',
            PublicationRelationVisibility::Moderated,
        ),
    ] {
        store
            .put_publication_relation(
                &relation(
                    id,
                    hash,
                    IMAGE_A,
                    Some(
                        IMAGE_A,
                    ),
                    1_003,
                    visibility,
                ),
            )
            .expect(
                "store relation",
            );
    }

    let page =
        store
            .list_publication_relations(
                IMAGE_A,
                &PublicationRelationPageRequest::new(
                    None,
                    Some(
                        100,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "relation page",
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
fn phase14a6a_relation_page_is_oldest_first_and_bounded() {
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
        timestamp,
    ) in [
        (
            "third",
            'a',
            1_003,
        ),
        (
            "first",
            'b',
            1_001,
        ),
        (
            "second",
            'c',
            1_002,
        ),
    ] {
        store
            .put_publication_relation(
                &relation(
                    id,
                    hash,
                    IMAGE_A,
                    Some(
                        IMAGE_A,
                    ),
                    timestamp,
                    PublicationRelationVisibility::Public,
                ),
            )
            .expect(
                "store relation",
            );
    }

    let first_page =
        store
            .list_publication_relations(
                IMAGE_A,
                &PublicationRelationPageRequest::new(
                    None,
                    Some(
                        2,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "first page",
            );

    assert_eq!(
        first_page.schema,
        PUBLICATION_RELATION_PAGE_SCHEMA,
    );

    assert_eq!(
        first_page
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
            >(),
        vec![
            "first",
            "second",
        ],
    );

    assert_eq!(
        first_page.has_more,
        true,
    );

    let cursor =
        first_page
            .next_cursor
            .expect(
                "next cursor",
            );

    assert!(
        cursor.starts_with(
            "r_",
        ),
    );

    let second_page =
        store
            .list_publication_relations(
                IMAGE_A,
                &PublicationRelationPageRequest::new(
                    Some(
                        cursor,
                    ),
                    Some(
                        2,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "second page",
            );

    assert_eq!(
        second_page.items
            .len(),
        1,
    );

    assert_eq!(
        second_page.items[
            0
        ]
            .publication
            .publication_id,
        "third",
    );
}

#[test]
fn phase14a6a_corrupt_relation_record_fails_closed() {
    let store =
        Store::new(
            false,
        )
        .expect(
            "memory store",
        );

    let key =
        keys::publication_relation_key(
            IMAGE_A,
            "corrupt",
        );

    store.put_manifest(
        &key,
        "not-json",
    );

    let page =
        store
            .list_publication_relations(
                IMAGE_A,
                &PublicationRelationPageRequest::new(
                    None,
                    Some(
                        50,
                    ),
                )
                .expect(
                    "request",
                ),
            )
            .expect(
                "relation page",
            );

    assert!(
        page.items
            .is_empty(),
    );
}
