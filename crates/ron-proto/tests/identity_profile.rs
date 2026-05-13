use ron_proto::{
    canonical_crab_asset_url, AssetKind, AttributionStatus, ContentId, IdentityValidationError,
    ModeratorSummaryV1, PassportKindV1, PassportPublicManifestV1, PassportPublicProfileV1,
    PublicAssetReferenceV1, ReputationSummaryV1, SiteModeratorRoleAssignmentV1,
    SiteModeratorRoleV1, UsernameStatusV1, PASSPORT_PUBLIC_MANIFEST_VERSION,
    PASSPORT_PUBLIC_PROFILE_VERSION,
};

const PROFILE_CID: &str = "b3:1111111111111111111111111111111111111111111111111111111111111111";
const IMAGE_CID: &str = "b3:2222222222222222222222222222222222222222222222222222222222222222";

fn cid(value: &str) -> ContentId {
    value.parse().expect("valid ContentId")
}

fn public_asset() -> PublicAssetReferenceV1 {
    let cid = cid(IMAGE_CID);
    PublicAssetReferenceV1 {
        asset_cid: Some(cid.clone()),
        asset_kind: AssetKind::Image,
        crab_url: canonical_crab_asset_url(&cid, AssetKind::Image),
        title: Some("Profile avatar".to_owned()),
    }
}

fn profile() -> PassportPublicProfileV1 {
    PassportPublicProfileV1 {
        version: PASSPORT_PUBLIC_PROFILE_VERSION,
        passport_subject: "passport:main:skinnycrabby".to_owned(),
        passport_kind: PassportKindV1::Main,
        username: "skinnycrabby".to_owned(),
        handle: "@skinnycrabby".to_owned(),
        username_status: UsernameStatusV1::Confirmed,
        display_name: Some("Skinny Crabby".to_owned()),
        bio: Some("Building the content-addressed creator web.".to_owned()),
        avatar_image: Some(canonical_crab_asset_url(&cid(IMAGE_CID), AssetKind::Image)),
        public_profile_cid: Some(cid(PROFILE_CID)),
        public_payout_account: Some("acct_creator_skinnycrabby".to_owned()),
        public_sites: vec!["crab://thedustyonion6".to_owned()],
        public_assets: vec![public_asset()],
        reputation_summary: Some(ReputationSummaryV1 {
            global_reputation_score: Some(1776),
            source: "svc-passport.future-read-model".to_owned(),
            updated_at_ms: Some(1_776_000_000_100),
            warnings: Vec::new(),
        }),
        moderator_summary: Some(ModeratorSummaryV1 {
            global_moderator_score: Some(100),
            site_roles: vec![SiteModeratorRoleAssignmentV1 {
                site: "crab://thedustyonion6".to_owned(),
                role: SiteModeratorRoleV1::SiteOwner,
                status: AttributionStatus::BackendConfirmed,
            }],
            source: "svc-passport.future-read-model".to_owned(),
            updated_at_ms: Some(1_776_000_000_100),
            warnings: Vec::new(),
        }),
        attribution_status: AttributionStatus::BackendConfirmed,
        warnings: Vec::new(),
    }
}

fn manifest() -> PassportPublicManifestV1 {
    PassportPublicManifestV1 {
        version: PASSPORT_PUBLIC_MANIFEST_VERSION,
        manifest_cid: Some(cid(PROFILE_CID)),
        profile: profile(),
        public_proof_refs: vec!["proof:profile:backend-confirmed".to_owned()],
        public_asset_catalogue_cid: None,
        created_at_ms: 1_776_000_000_000,
        updated_at_ms: 1_776_000_000_100,
        warnings: Vec::new(),
    }
}

#[test]
fn public_profile_validates_and_roundtrips_json() {
    let profile = profile();

    profile.validate().expect("profile validates");
    assert_eq!(profile.canonical_profile_crab_url(), "crab://@skinnycrabby");

    let json = serde_json::to_string_pretty(&profile).expect("serialize profile");
    assert!(json.contains("\"passport_subject\""));
    assert!(json.contains("\"handle\""));
    assert!(json.contains("\"username_status\""));
    assert!(json.contains("\"reputation_summary\""));
    assert!(json.contains("\"moderator_summary\""));

    let decoded: PassportPublicProfileV1 =
        serde_json::from_str(&json).expect("deserialize profile");

    assert_eq!(decoded, profile);
    decoded.validate().expect("decoded profile validates");
}

#[test]
fn public_manifest_validates_and_roundtrips_json() {
    let manifest = manifest();

    manifest.validate().expect("manifest validates");

    let json = serde_json::to_string_pretty(&manifest).expect("serialize manifest");
    assert!(json.contains("\"profile\""));
    assert!(json.contains("\"public_proof_refs\""));
    assert!(!json.contains("private_key"));
    assert!(!json.contains("spend_authority"));
    assert!(!json.contains("recovery"));

    let decoded: PassportPublicManifestV1 =
        serde_json::from_str(&json).expect("deserialize manifest");

    assert_eq!(decoded, manifest);
    decoded.validate().expect("decoded manifest validates");
}

#[test]
fn profile_rejects_unknown_private_fields() {
    let mut value = serde_json::to_value(profile()).expect("profile to json value");
    value
        .as_object_mut()
        .expect("profile object")
        .insert("private_alt_mapping".to_owned(), serde_json::json!("nope"));

    let err = serde_json::from_value::<PassportPublicProfileV1>(value)
        .expect_err("unknown private field rejects");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn handle_must_match_username() {
    let mut profile = profile();
    profile.handle = "@someoneelse".to_owned();

    let err = profile.validate().expect_err("mismatched handle rejects");

    assert!(matches!(err, IdentityValidationError::HandleMismatch));
}

#[test]
fn avatar_must_be_crab_url_when_present() {
    let mut profile = profile();
    profile.avatar_image = Some("https://example.com/avatar.png".to_owned());

    let err = profile.validate().expect_err("non-crab avatar rejects");

    assert!(matches!(
        err,
        IdentityValidationError::InvalidCrabUrl {
            field: "avatar_image"
        }
    ));
}

#[test]
fn manifest_cid_and_profile_cid_must_not_conflict() {
    let mut manifest = manifest();
    manifest.manifest_cid = Some(cid(IMAGE_CID));

    let err = manifest.validate().expect_err("cid mismatch rejects");

    assert!(matches!(
        err,
        IdentityValidationError::CidMismatch {
            field: "profile.public_profile_cid"
        }
    ));
}

#[test]
fn alt_profile_does_not_serialize_public_parent_linkage() {
    let mut profile = profile();
    profile.passport_kind = PassportKindV1::Alt;
    profile.passport_subject = "passport:alt:crabby-shadow".to_owned();
    profile.username = "crabby-shadow".to_owned();
    profile.handle = "@crabby-shadow".to_owned();
    profile.public_payout_account = None;

    profile.validate().expect("alt profile validates");

    let json = serde_json::to_string_pretty(&profile).expect("serialize alt profile");

    assert!(!json.contains("main_passport_subject"));
    assert!(!json.contains("parent_passport"));
    assert!(!json.contains("funding_source"));
    assert!(!json.contains("private_alt_mapping"));
    assert!(!json.contains("wallet_spend_authority"));
}

#[test]
fn asset_kind_identity_roots_have_stable_suffixes() {
    assert_eq!(AssetKind::Profile.suffix(), "profile");
    assert_eq!(AssetKind::Passport.suffix(), "passport");
    assert_eq!(AssetKind::Alt.suffix(), "alt");
    assert_eq!(AssetKind::Podcast.suffix(), "podcast");
    assert_eq!(AssetKind::Stream.suffix(), "stream");

    let profile_url = canonical_crab_asset_url(&cid(PROFILE_CID), AssetKind::Profile);
    assert_eq!(
        profile_url,
        "crab://1111111111111111111111111111111111111111111111111111111111111111.profile"
    );
}
