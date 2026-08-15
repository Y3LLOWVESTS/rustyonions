use std::error::Error;

use svc_index::{
    discovery::{
        build_explore_discovery,
        ExploreDiscoveryError,
        ExploreDiscoveryRequest,
        EXPLORE_CREATOR_DEFAULT_LIMIT,
        EXPLORE_CREATOR_MAX_LIMIT,
        EXPLORE_DISCOVERY_SCHEMA,
        EXPLORE_PUBLICATION_DEFAULT_LIMIT,
        EXPLORE_PUBLICATION_MAX_LIMIT,
        EXPLORE_SITE_DEFAULT_LIMIT,
        EXPLORE_SITE_MAX_LIMIT,
    },
    http::routes::
        explore_discovery::
            ExploreDiscoveryQuery,
    publications::{
        PublicationAccess,
        PublicationCreatorV1,
        PublicationKind,
        PublicationSummaryV1,
        PublicationVisibility,
        PUBLICATION_SUMMARY_SCHEMA,
    },
    store::Store,
};

type TestResult =
    Result<
        (),
        Box<dyn Error>,
    >;

fn ensure(
    condition: bool,
    message: &str,
) -> TestResult {
    if condition {
        return Ok(());
    }

    Err(
        std::io::Error::new(
            std::io::ErrorKind::Other,
            message,
        )
        .into(),
    )
}

fn creator(
    username: &str,
    display_name: &str,
) -> PublicationCreatorV1 {
    PublicationCreatorV1 {
        username:
            username.to_owned(),

        display_name:
            display_name.to_owned(),

        profile_url:
            [
                "crab://@",
                username,
            ]
            .concat(),

        avatar_cid:
            None,
    }
}

fn publication(
    publication_id: &str,
    creator:
        PublicationCreatorV1,
    published_at: &str,
    visibility:
        PublicationVisibility,
    pinned: bool,
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
            [
                "crab://",
                publication_id,
                ".post",
            ]
            .concat(),

        title:
            publication_id
                .to_owned(),

        summary:
            "Explore discovery test publication."
                .to_owned(),

        creator,

        published_at:
            published_at
                .to_owned(),

        updated_at:
            published_at
                .to_owned(),

        visibility,

        access:
            PublicationAccess::Free,

        thumbnail:
            None,

        references:
            None,

        pinned,
    }
}

#[test]
fn phase10a3b_request_defaults_and_bounds_are_locked(
) -> TestResult {
    let request =
        ExploreDiscoveryRequest::new(
            None,
            None,
            None,
        )?;

    ensure(
        request.publication_limit
            == EXPLORE_PUBLICATION_DEFAULT_LIMIT,
        "publication default limit mismatch",
    )?;

    ensure(
        request.creator_limit
            == EXPLORE_CREATOR_DEFAULT_LIMIT,
        "creator default limit mismatch",
    )?;

    ensure(
        request.site_limit
            == EXPLORE_SITE_DEFAULT_LIMIT,
        "site default limit mismatch",
    )?;

    ensure(
        ExploreDiscoveryRequest::new(
            Some(
                EXPLORE_PUBLICATION_MAX_LIMIT
                    + 1,
            ),
            None,
            None,
        )
        .is_err(),
        "publication limit above maximum must reject",
    )?;

    ensure(
        ExploreDiscoveryRequest::new(
            None,
            Some(
                EXPLORE_CREATOR_MAX_LIMIT
                    + 1,
            ),
            None,
        )
        .is_err(),
        "creator limit above maximum must reject",
    )?;

    ensure(
        ExploreDiscoveryRequest::new(
            None,
            None,
            Some(
                EXPLORE_SITE_MAX_LIMIT
                    + 1,
            ),
        )
        .is_err(),
        "site limit above maximum must reject",
    )
}

#[test]
fn phase10a3b_recent_publications_are_newest_first_without_pinned_override(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    store
        .put_creator_publication(
            &publication(
                "older-pinned",
                creator(
                    "alice",
                    "Alice",
                ),
                "2026-08-09T20:00:00.000Z",
                PublicationVisibility::Public,
                true,
            ),
        )?;

    store
        .put_creator_publication(
            &publication(
                "newer",
                creator(
                    "bob",
                    "Bob",
                ),
                "2026-08-09T22:00:00.000Z",
                PublicationVisibility::Public,
                false,
            ),
        )?;

    let discovery =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                Some(12),
                Some(12),
                Some(8),
            )?,
        )?;

    ensure(
        discovery.schema
            == EXPLORE_DISCOVERY_SCHEMA,
        "Explore schema mismatch",
    )?;

    ensure(
        discovery
            .recent_publications
            .len()
            == 2,
        "expected two recent publications",
    )?;

    ensure(
        discovery
            .recent_publications[0]
            .publication_id
            == "newer",
        "newest publication must appear first",
    )?;

    ensure(
        discovery
            .recent_publications[1]
            .publication_id
            == "older-pinned",
        "pinned state must not override Explore chronology",
    )
}

#[test]
fn phase10a3b_nonpublic_records_never_enter_explore(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    for (
        publication_id,
        visibility,
    ) in [
        (
            "public",
            PublicationVisibility::Public,
        ),
        (
            "private",
            PublicationVisibility::Private,
        ),
        (
            "deleted",
            PublicationVisibility::Deleted,
        ),
        (
            "blocked",
            PublicationVisibility::Blocked,
        ),
        (
            "moderated",
            PublicationVisibility::Moderated,
        ),
    ] {
        store
            .put_creator_publication(
                &publication(
                    publication_id,
                    creator(
                        "alice",
                        "Alice",
                    ),
                    "2026-08-09T21:00:00.000Z",
                    visibility,
                    false,
                ),
            )?;
    }

    let discovery =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                None,
                None,
                None,
            )?,
        )?;

    ensure(
        discovery
            .recent_publications
            .len()
            == 1,
        "Explore must expose public records only",
    )?;

    ensure(
        discovery
            .recent_publications[0]
            .publication_id
            == "public",
        "unexpected nonpublic publication survived",
    )
}

#[test]
fn phase10a3b_public_creators_are_deduplicated_and_username_ordered(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    for record in [
        publication(
            "zoe-post",
            creator(
                "zoe",
                "Zoe",
            ),
            "2026-08-09T20:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        publication(
            "alice-new",
            creator(
                "alice",
                "Alice",
            ),
            "2026-08-09T22:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        publication(
            "alice-old",
            creator(
                "alice",
                "Alice",
            ),
            "2026-08-09T19:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
        publication(
            "bob-post",
            creator(
                "bob",
                "Bob",
            ),
            "2026-08-09T21:00:00.000Z",
            PublicationVisibility::Public,
            false,
        ),
    ] {
        store
            .put_creator_publication(
                &record,
            )?;
    }

    let discovery =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                None,
                None,
                None,
            )?,
        )?;

    let usernames =
        discovery
            .public_creators
            .iter()
            .map(
                |value| {
                    value
                        .username
                        .as_str()
                },
            )
            .collect::<Vec<_>>();

    ensure(
        usernames
            == Vec::from([
                "alice",
                "bob",
                "zoe",
            ]),
        "public creators must be deduplicated and username ordered",
    )
}

#[test]
fn phase10a3b_conflicting_creator_projection_fails_closed(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    store
        .put_creator_publication(
            &publication(
                "alice-one",
                creator(
                    "alice",
                    "Alice",
                ),
                "2026-08-09T20:00:00.000Z",
                PublicationVisibility::Public,
                false,
            ),
        )?;

    store
        .put_creator_publication(
            &publication(
                "alice-two",
                creator(
                    "alice",
                    "Alice Changed",
                ),
                "2026-08-09T21:00:00.000Z",
                PublicationVisibility::Public,
                false,
            ),
        )?;

    let result =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                None,
                None,
                None,
            )?,
        );

    ensure(
        matches_conflicting_creator(
            result,
        ),
        "conflicting creator projection must fail closed",
    )
}

fn matches_conflicting_creator(
    result:
        Result<
            svc_index::discovery::
                ExploreDiscoveryV1,
            ExploreDiscoveryError,
        >,
) -> bool {
    match result {
        Err(
            ExploreDiscoveryError::
                ConflictingCreator {
                    username,
                },
        ) => {
            username
                == "alice"
        }

        _ => false,
    }
}

#[test]
fn phase10a3b_template_sites_remain_empty_until_truthful_display_metadata_exists(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    store
        .put_creator_publication(
            &publication(
                "public-post",
                creator(
                    "alice",
                    "Alice",
                ),
                "2026-08-09T22:00:00.000Z",
                PublicationVisibility::Public,
                false,
            ),
        )?;

    let discovery =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                None,
                None,
                None,
            )?,
        )?;

    ensure(
        discovery
            .template_sites
            .is_empty(),
        "template site cards must not be fabricated from incomplete site pointers",
    )
}

#[test]
fn phase10a3b_wire_shape_matches_shared_explore_contract(
) -> TestResult {
    let store =
        Store::new(
            false,
        )?;

    let discovery =
        build_explore_discovery(
            &store,
            &ExploreDiscoveryRequest::new(
                None,
                None,
                None,
            )?,
        )?;

    let json =
        serde_json::to_value(
            discovery,
        )?;

    ensure(
        json.get(
            "schema",
        )
        .is_some(),
        "schema field missing",
    )?;

    ensure(
        json.get(
            "recentPublications",
        )
        .is_some(),
        "recentPublications field missing",
    )?;

    ensure(
        json.get(
            "publicCreators",
        )
        .is_some(),
        "publicCreators field missing",
    )?;

    ensure(
        json.get(
            "templateSites",
        )
        .is_some(),
        "templateSites field missing",
    )?;

    ensure(
        json.as_object()
            .map(
                |value| {
                    value.len()
                },
            )
            == Some(4),
        "wire response must contain exactly four top-level fields",
    )
}

#[test]
fn phase10a3b_http_query_is_strict_and_route_is_registered(
) -> TestResult {
    let query =
        serde_json::from_value::<
            ExploreDiscoveryQuery,
        >(
            serde_json::Value::Object(
                serde_json::Map::from_iter([
                    (
                        "publicationLimit"
                            .to_owned(),
                        serde_json::Value::
                            Number(
                                12_u64
                                    .into(),
                            ),
                    ),
                    (
                        "creatorLimit"
                            .to_owned(),
                        serde_json::Value::
                            Number(
                                12_u64
                                    .into(),
                            ),
                    ),
                    (
                        "siteLimit"
                            .to_owned(),
                        serde_json::Value::
                            Number(
                                8_u64
                                    .into(),
                            ),
                    ),
                ]),
            ),
        )?;

    ensure(
        query.publication_limit
            == Some(12),
        "publication query limit mismatch",
    )?;

    let rejected =
        serde_json::from_value::<
            ExploreDiscoveryQuery,
        >(
            serde_json::Value::Object(
                serde_json::Map::from_iter([
                    (
                        "ranking"
                            .to_owned(),
                        serde_json::Value::
                            String(
                                "popular"
                                    .to_owned(),
                            ),
                    ),
                ]),
            ),
        );

    ensure(
        rejected.is_err(),
        "unknown ranking query must reject",
    )?;

    let router_source =
        std::fs::read_to_string(
            "src/router.rs",
        )?;

    ensure(
        router_source.contains(
            "/v1/index/explore",
        ),
        "svc-index Explore route missing",
    )?;

    ensure(
        router_source.contains(
            "routes::explore_discovery",
        ),
        "svc-index Explore handler registration missing",
    )
}
