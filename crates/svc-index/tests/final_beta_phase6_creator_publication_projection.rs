// RO:WHAT — Focused FINAL_BETA Phase 6 svc-index creator-publication projection tests.
// RO:WHY — Proves strict wire shape, public filtering, deterministic ordering, bounded pagination, persistence abstraction reuse, and authority non-expansion.

use svc_index::{
    publications::{
        PublicationAccess,
        PublicationCreatorV1,
        PublicationKind,
        PublicationPageRequest,
        PublicationProjectionError,
        PublicationReferencesV1,
        PublicationSummaryV1,
        PublicationThumbnailKind,
        PublicationThumbnailV1,
        PublicationVisibility,
        PUBLICATION_PAGE_SCHEMA,
        PUBLICATION_SUMMARY_SCHEMA,
    },
    store::{
        keys,
        Store,
    },
};

const HASH_A: &str =
    "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

const HASH_B: &str =
    "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn summary(
    publication_id: &str,
    published_at: &str,
    visibility: PublicationVisibility,
    pinned: bool,
) -> PublicationSummaryV1 {
    PublicationSummaryV1 {
        schema:
            PUBLICATION_SUMMARY_SCHEMA.to_owned(),
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
            "Backend-derived creator publication projection."
                .to_owned(),
        creator:
            PublicationCreatorV1 {
                username:
                    "rusty_crab".to_owned(),
                display_name:
                    "Rusty Crab".to_owned(),
                profile_url:
                    "crab://@rusty_crab".to_owned(),
                avatar_cid:
                    Some(HASH_B.to_owned()),
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
                        PublicationThumbnailKind::Image,
                    cid:
                        HASH_A.to_owned(),
                    alt:
                        "Publication thumbnail".to_owned(),
                },
            ),
        references:
            Some(
                PublicationReferencesV1 {
                    manifest_cid:
                        Some(HASH_A.to_owned()),
                    content_cid:
                        Some(HASH_B.to_owned()),
                    site_url:
                        None,
                },
            ),
        pinned,
    }
}

#[test]
fn phase6b1_strict_wire_shape_rejects_unknown_and_economic_fields() {
    let valid =
        serde_json::to_value(
            summary(
                "publication-001",
                "2026-08-05T12:00:00.000Z",
                PublicationVisibility::Public,
                false,
            ),
        )
        .expect(
            "serialize valid summary",
        );

    let decoded:
        PublicationSummaryV1 =
            serde_json::from_value(
                valid.clone(),
            )
            .expect(
                "decode strict summary",
            );

    assert_eq!(
        decoded.publication_id,
        "publication-001",
    );

    for field in [
        "priceMinor",
        "balance",
        "receipt",
        "entitlement",
        "privateKey",
    ] {
        let mut rejected =
            valid.clone();

        rejected
            .as_object_mut()
            .expect(
                "summary object",
            )
            .insert(
                field.to_owned(),
                serde_json::Value::Bool(true),
            );

        let result =
            serde_json::from_value::<
                PublicationSummaryV1,
            >(rejected);

        assert!(
            result.is_err(),
            "unknown field accepted: {field}",
        );
    }
}

#[test]
fn phase6b1_validation_rejects_identity_timestamp_and_reference_mismatch() {
    let mut invalid_profile =
        summary(
            "publication-identity",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        );

    invalid_profile.creator.profile_url =
        "crab://@different_user".to_owned();

    assert!(
        invalid_profile.validate().is_err(),
    );

    let mut invalid_time =
        summary(
            "publication-time",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        );

    invalid_time.updated_at =
        "2026-08-05T11:59:59.999Z".to_owned();

    assert!(
        invalid_time.validate().is_err(),
    );

    let mut invalid_cid =
        summary(
            "publication-cid",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        );

    invalid_cid
        .thumbnail
        .as_mut()
        .expect("thumbnail")
        .cid =
            "b3:invalid".to_owned();

    assert!(
        invalid_cid.validate().is_err(),
    );
}

#[test]
fn phase6b1_memory_projection_is_public_bounded_and_deterministic() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    let records = [
        summary(
            "older-public",
            "2026-08-05T10:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        summary(
            "newer-public",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        summary(
            "pinned-public",
            "2026-08-05T09:00:00.000Z",
            PublicationVisibility::Public,
            true,
        ),
        summary(
            "private-record",
            "2026-08-05T13:00:00.000Z",
            PublicationVisibility::Private,
            false,
        ),
        summary(
            "unlisted-record",
            "2026-08-05T14:00:00.000Z",
            PublicationVisibility::Unlisted,
            false,
        ),
        summary(
            "deleted-record",
            "2026-08-05T15:00:00.000Z",
            PublicationVisibility::Deleted,
            false,
        ),
    ];

    for record in &records {
        store
            .put_creator_publication(record)
            .expect(
                "store publication",
            );
    }

    let request =
        PublicationPageRequest::new(
            None,
            Some(10),
        )
        .expect(
            "request",
        );

    let page =
        store
            .list_creator_publications(
                "rusty_crab",
                &request,
            )
            .expect(
                "creator page",
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
        "pinned-public",
    );

    assert_eq!(
        page.items[1].publication_id,
        "newer-public",
    );

    assert_eq!(
        page.items[2].publication_id,
        "older-public",
    );

    assert_eq!(
        page.has_more,
        false,
    );

    assert_eq!(
        page.next_cursor,
        None,
    );
}

#[test]
fn phase6b1_pagination_is_bounded_and_uses_opaque_cursor_tokens() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    for index in 0..5 {
        let record =
            summary(
                &format!(
                    "publication-{index}",
                ),
                &format!(
                    "2026-08-05T12:00:0{index}.000Z",
                ),
                PublicationVisibility::Public,
                false,
            );

        store
            .put_creator_publication(
                &record,
            )
            .expect(
                "store publication",
            );
    }

    let first_request =
        PublicationPageRequest::new(
            None,
            Some(2),
        )
        .expect(
            "first request",
        );

    let first =
        store
            .list_creator_publications(
                "rusty_crab",
                &first_request,
            )
            .expect(
                "first page",
            );

    assert_eq!(
        first.items.len(),
        2,
    );

    assert_eq!(
        first.has_more,
        true,
    );

    let cursor =
        first
            .next_cursor
            .clone()
            .expect(
                "next cursor",
            );

    assert!(
        cursor.starts_with("p_"),
    );

    assert_eq!(
        cursor.len(),
        10,
    );

    let second_request =
        PublicationPageRequest::new(
            Some(cursor),
            Some(2),
        )
        .expect(
            "second request",
        );

    let second =
        store
            .list_creator_publications(
                "rusty_crab",
                &second_request,
            )
            .expect(
                "second page",
            );

    assert_eq!(
        second.items.len(),
        2,
    );

    assert_eq!(
        second.has_more,
        true,
    );

    for invalid_limit in [0, 51] {
        assert!(
            PublicationPageRequest::new(
                None,
                Some(invalid_limit),
            )
            .is_err(),
        );
    }

    assert_eq!(
        PublicationPageRequest::new(
            Some(
                "plain-offset".to_owned(),
            ),
            Some(2),
        ),
        Err(
            PublicationProjectionError::InvalidCursor,
        ),
    );
}

#[test]
fn phase6b1_creator_and_publication_keys_are_isolated() {
    let first =
        keys::creator_publication_key(
            "rusty_crab",
            "publication-001",
        );

    let second =
        keys::creator_publication_key(
            "other_crab",
            "publication-001",
        );

    assert_eq!(
        first,
        "creator_publication:rusty_crab:publication-001",
    );

    assert!(
        first.starts_with(
            &keys::creator_publication_prefix(
                "rusty_crab",
            ),
        ),
    );

    assert!(
        second.starts_with(
            &keys::creator_publication_prefix(
                "other_crab",
            ),
        ),
    );

    assert_ne!(
        first,
        second,
    );
}

#[test]
fn phase6b1_get_projection_rejects_cross_creator_records() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    let record =
        summary(
            "publication-lookup",
            "2026-08-05T12:00:00.000Z",
            PublicationVisibility::Public,
            false,
        );

    store
        .put_creator_publication(
            &record,
        )
        .expect(
            "store publication",
        );

    assert_eq!(
        store
            .get_creator_publication(
                "rusty_crab",
                "publication-lookup",
            )
            .expect(
                "lookup",
            ),
        Some(record),
    );

    assert_eq!(
        store
            .get_creator_publication(
                "other_crab",
                "publication-lookup",
            )
            .expect(
                "other lookup",
            ),
        None,
    );
}

#[test]
fn phase6b1_corrupt_stored_projection_fails_closed() {
    let store =
        Store::new(false)
            .expect(
                "memory store",
            );

    let key =
        keys::creator_publication_key(
            "rusty_crab",
            "corrupt-record",
        );

    store.put_manifest(
        &key,
        "not-json",
    );

    let request =
        PublicationPageRequest::new(
            None,
            Some(20),
        )
        .expect(
            "request",
        );

    let page =
        store
            .list_creator_publications(
                "rusty_crab",
                &request,
            )
            .expect(
                "creator page",
            );

    assert!(
        page.items.is_empty(),
    );
}

#[test]
fn phase6b1_projection_adds_no_economic_or_relationship_authority() {
    let source =
        include_str!(
            "../src/publications.rs",
        );

    for forbidden in [
        "walletMutation: true",
        "ledgerMutation: true",
        "receiptAuthority: true",
        "paidEntitlementAuthority: true",
        "followMutation: true",
        "settlementAuthority: true",
        "private_key",
        "recovery_phrase",
        "pin_value",
        "capability_token",
    ] {
        assert!(
            source.contains(forbidden) == false,
            "forbidden authority: {forbidden}",
        );
    }

    assert!(
        source.contains(
            "public read projection only",
        ),
    );
}
