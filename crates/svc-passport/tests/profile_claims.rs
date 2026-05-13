use svc_passport::profile::{
    normalize_handle, normalize_username, PassportKind, ProfileClaimError, UsernameClaimRequest,
    UsernameClaimStatus, UsernameClaimStore, PUBLIC_PROFILE_SCHEMA,
};

fn request(passport_subject: &str, username: &str) -> UsernameClaimRequest {
    UsernameClaimRequest {
        passport_subject: passport_subject.to_owned(),
        requested_username: username.to_owned(),
        display_name: Some("Skinny Crabby".to_owned()),
        bio: Some("Building the content-addressed creator web.".to_owned()),
        avatar_image: Some(
            "crab://2222222222222222222222222222222222222222222222222222222222222222.image"
                .to_owned(),
        ),
    }
}

#[test]
fn normalizes_username_and_handle() {
    assert_eq!(
        normalize_username("@Skinny.Crabby").unwrap(),
        "skinny.crabby"
    );
    assert_eq!(normalize_handle("Skinny-Crabby").unwrap(), "@skinny-crabby");
    assert_eq!(normalize_handle("skinny_crabby").unwrap(), "@skinny_crabby");
}

#[test]
fn rejects_reserved_and_confusing_usernames() {
    assert!(matches!(
        normalize_username("@site"),
        Err(ProfileClaimError::ReservedUsername { username }) if username == "site"
    ));
    assert!(matches!(
        normalize_username("ab"),
        Err(ProfileClaimError::UsernameTooShort { .. })
    ));
    assert!(matches!(
        normalize_username("-alice"),
        Err(ProfileClaimError::InvalidUsernameStart)
    ));
    assert!(matches!(
        normalize_username("alice_"),
        Err(ProfileClaimError::InvalidUsernameTrailingPunctuation)
    ));
    assert!(matches!(
        normalize_username("alice..bob"),
        Err(ProfileClaimError::ConsecutiveDots)
    ));
    assert!(matches!(
        normalize_username("alice bob"),
        Err(ProfileClaimError::InvalidUsernameCharacter)
    ));
}

#[test]
fn claim_main_username_returns_confirmed_public_profile() {
    let store = UsernameClaimStore::new();

    let record = store
        .claim_main_username(
            request("passport:main:skinnycrabby", "@SkinnyCrabby"),
            1_776_000_000_000,
        )
        .expect("claim succeeds");

    assert_eq!(record.passport_kind, PassportKind::Main);
    assert_eq!(record.username, "skinnycrabby");
    assert_eq!(record.handle, "@skinnycrabby");
    assert_eq!(record.username_status, UsernameClaimStatus::Confirmed);
    assert_eq!(record.profile_crab_url, "crab://@skinnycrabby");
    assert!(record.public_profile_cid.is_none());

    let profile = store
        .public_profile("@skinnycrabby")
        .expect("lookup succeeds")
        .expect("profile exists");

    assert_eq!(profile.schema, PUBLIC_PROFILE_SCHEMA);
    assert_eq!(profile.passport_subject, "passport:main:skinnycrabby");
    assert_eq!(profile.passport_kind, PassportKind::Main);
    assert_eq!(profile.username, "skinnycrabby");
    assert_eq!(profile.handle, "@skinnycrabby");
    assert_eq!(profile.username_status, UsernameClaimStatus::Confirmed);
    assert_eq!(profile.profile_crab_url, "crab://@skinnycrabby");
    assert!(profile.reputation_score.is_none());
    assert!(profile.moderator_score.is_none());
    assert!(
        profile.warnings.iter().any(
            |warning| warning.contains("reputation and moderation scores are not computed yet")
        ),
        "profile should be honest about missing rep/mod computation"
    );
}

#[test]
fn repeated_same_passport_same_username_is_idempotent() {
    let store = UsernameClaimStore::new();

    let first = store
        .claim_main_username(
            request("passport:main:skinnycrabby", "skinnycrabby"),
            1_776_000_000_000,
        )
        .expect("first claim succeeds");

    let second = store
        .claim_main_username(
            request("passport:main:skinnycrabby", "@skinnycrabby"),
            1_776_000_000_500,
        )
        .expect("repeat claim succeeds");

    assert_eq!(first, second);
}

#[test]
fn duplicate_username_for_different_passport_rejects() {
    let store = UsernameClaimStore::new();

    store
        .claim_main_username(request("passport:main:alice", "alice"), 1)
        .expect("first claim succeeds");

    let err = store
        .claim_main_username(request("passport:main:alice2", "alice"), 2)
        .expect_err("duplicate username rejects");

    assert_eq!(err.code(), "username_unavailable");
    assert!(matches!(
        &err,
        ProfileClaimError::UsernameUnavailable { username } if username == "alice"
    ));
}

#[test]
fn same_passport_cannot_claim_second_username() {
    let store = UsernameClaimStore::new();

    store
        .claim_main_username(request("passport:main:alice", "alice"), 1)
        .expect("first claim succeeds");

    let err = store
        .claim_main_username(request("passport:main:alice", "alice2"), 2)
        .expect_err("second username rejects");

    assert_eq!(err.code(), "passport_already_has_username");
    assert!(matches!(
        &err,
        ProfileClaimError::PassportAlreadyHasUsername {
            passport_subject,
            username
        } if passport_subject == "passport:main:alice" && username == "alice"
    ));
}

#[test]
fn public_profile_unknown_username_returns_none() {
    let store = UsernameClaimStore::new();

    let profile = store.public_profile("@missing").expect("lookup succeeds");

    assert!(profile.is_none());
}

#[test]
fn public_profile_json_does_not_leak_private_fields() {
    let store = UsernameClaimStore::new();

    store
        .claim_main_username(request("passport:main:skinnycrabby", "skinnycrabby"), 1)
        .expect("claim succeeds");

    let profile = store
        .public_profile("skinnycrabby")
        .expect("lookup succeeds")
        .expect("profile exists");

    let json = serde_json::to_string_pretty(&profile).expect("serialize profile");

    assert!(json.contains("\"schema\""));
    assert!(json.contains("\"username_status\""));
    assert!(!json.contains("private_key"));
    assert!(!json.contains("seed_phrase"));
    assert!(!json.contains("spend_authority"));
    assert!(!json.contains("wallet_spend_authority"));
    assert!(!json.contains("private_alt_mapping"));
    assert!(!json.contains("parent_passport"));
    assert!(!json.contains("main_passport_subject"));
}
