//! RO:WHAT — Phase 0A source authority inventory for Native Passport v0.3.
//! RO:WHY — P3 Identity & Keys; Concerns: SEC/GOV. Reconciles current svc-passport routes before new root/device authority.
//! RO:INTERACTS — http::router current routes, future proof/capability routes, ron-auth, ron-naming, svc-index.
//! RO:INVARIANTS — no new runtime authority; current dev/compat routes are not Native Passport V1 proof issuance.
//! RO:METRICS — none; static source inventory only.
//! RO:CONFIG — none.
//! RO:SECURITY — marks arbitrary-subject/dev/admin surfaces explicitly non-production-native.
//! RO:TEST — tests/native_passport_phase0_authority.rs.

/// Current Native Passport build-plan line being encoded in source.
///
/// Phase 0A is intentionally a reconciliation guard. It must not create root
/// keys, device keys, recovery phrases, vaults, or proof-gated capabilities.
pub const NATIVE_PASSPORT_PHASE0A_LABEL: &str =
    "NATIVE_PASSPORT_PHASE0A_SOURCE_AUTHORITY_RECONCILIATION";

/// Stable service/package owner for Passport behavior.
pub const CANONICAL_PASSPORT_PACKAGE_OWNER: &str = "svc-passport";

/// Compatibility-only development subject used by older local flows.
pub const LEGACY_DEV_PASSPORT_SUBJECT: &str = "passport:main:dev";

/// Compatibility re-export of the canonical ron-proto Passport ID prefix/domain.
///
/// ron-proto owns protocol-level Native Passport ID shapes and derivation
/// domains; svc-passport retains these names for existing native call sites.
pub use ron_proto::{PASSPORT_ID_V1_HASH_DOMAIN, PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX};

/// Frozen Phase 0C HKDF salt for Native Passport root-seed derivation.
pub const ROOT_IDENTITY_V1_HKDF_SALT: &str = "rustyonions.native-passport.root-seed.v1";

/// Frozen Phase 0C HKDF info for the Ed25519 root signing key.
pub const ROOT_IDENTITY_V1_HKDF_INFO: &str = "rustyonions.native-passport.root-signing-key.v1";

/// Native Passport V1 BIP-39 PBKDF2 iteration count.
pub const BIP39_SEED_V1_PBKDF2_ROUNDS: u32 = 2_048;

/// BIP-39 seed salt prefix. The current Native Passport V1 passphrase is empty.
pub const BIP39_SEED_V1_SALT_PREFIX: &str = "mnemonic";

/// Frozen Phase 0C Native Passport BIP-39 passphrase policy.
pub const BIP39_SEED_V1_PASSPHRASE_PROFILE: &str = "empty_string_v1";

/// BIP-39 seeds are 512 bits, or 64 bytes.
pub const ROOT_IDENTITY_V1_BIP39_SEED_BYTES: usize = 64;

/// Ed25519 signing seeds are 256 bits.
pub const ROOT_IDENTITY_V1_SIGNING_SEED_BYTES: usize = 32;

/// Required lowercase hex digest length for `PassportIdV1` BLAKE3 suffixes.
pub const PASSPORT_ID_V1_B3_DIGEST_HEX_LEN: usize = 64;

/// Compatibility re-export of the canonical ron-proto Device ID prefix/domain.
pub use ron_proto::{DEVICE_ID_V1_ED25519_B3_PREFIX, DEVICE_ID_V1_HASH_DOMAIN};

/// Required lowercase hex digest length for `DeviceIdV1` BLAKE3 suffixes.
pub const DEVICE_ID_V1_B3_DIGEST_HEX_LEN: usize = 64;

/// Validate the intended DeviceIdV1 string shape.
///
/// This is format validation only. It does not generate a device key, derive a
/// digest, sign an authorization, issue a challenge, or create a capability.
pub fn is_device_id_v1_ed25519_b3(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(DEVICE_ID_V1_ED25519_B3_PREFIX) else {
        return false;
    };

    digest.len() == DEVICE_ID_V1_B3_DIGEST_HEX_LEN
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Validate the locked Phase 0D PassportIdV1 string shape.
///
/// This is format validation only. It does not derive keys, hash inputs, sign
/// proofs, issue capabilities, or claim runtime authority.
pub fn is_passport_id_v1_main_ed25519_b3(value: &str) -> bool {
    let Some(digest) = value.strip_prefix(PASSPORT_ID_V1_MAIN_ED25519_B3_PREFIX) else {
        return false;
    };

    digest.len() == PASSPORT_ID_V1_B3_DIGEST_HEX_LEN
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Source-level classification for existing and planned Passport routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassportRoutePosture {
    /// Existing service route remains available for compatibility or current
    /// token behavior, but it is not the Native Passport V1 proof path.
    CompatibilityOnly,
    /// Route is public read/projection behavior and does not prove ownership.
    PublicReadProjection,
    /// Route is a local-development/debug surface and must not be treated as
    /// production Native Passport authority.
    DevelopmentOnly,
    /// Route is an admin/service operation and not a public client-auth path.
    AdminOnly,
    /// Route is required by Native Passport V1 but is not implemented in this
    /// Phase 0A inventory patch.
    PlannedNativeV1,
}

/// Minimal current/planned route inventory for source reconciliation tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassportRouteAuthority {
    /// HTTP method.
    pub method: &'static str,
    /// Stable route path.
    pub path: &'static str,
    /// Authority posture for this phase.
    pub posture: PassportRoutePosture,
    /// Whether this route can issue or prove Native Passport V1 authority.
    pub native_v1_authority: bool,
    /// Human-readable boundary note kept stable for tests and reviewers.
    pub note: &'static str,
}

/// Existing `svc-passport` routes that Phase 0A recognizes.
///
/// These entries document current source posture only. They do not add new
/// authorization behavior and do not change the router.
pub const CURRENT_ROUTE_AUTHORITIES: &[PassportRouteAuthority] = &[
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/issue",
        posture: PassportRoutePosture::CompatibilityOnly,
        native_v1_authority: false,
        note: "legacy token issue path; arbitrary public Native Passport subjects must not be treated as proof",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/verify",
        posture: PassportRoutePosture::CompatibilityOnly,
        native_v1_authority: false,
        note: "legacy token verification path; not a device-bound proof challenge verifier",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/verify_batch",
        posture: PassportRoutePosture::CompatibilityOnly,
        native_v1_authority: false,
        note: "legacy batch verification path; not a Native Passport challenge replay store",
    },
    PassportRouteAuthority {
        method: "GET",
        path: "/v1/keys",
        posture: PassportRoutePosture::PublicReadProjection,
        native_v1_authority: false,
        note: "service public-key discovery only",
    },
    PassportRouteAuthority {
        method: "GET",
        path: "/v1/passport/profile/_debug",
        posture: PassportRoutePosture::DevelopmentOnly,
        native_v1_authority: false,
        note: "debug posture route; no secret, wallet, ledger, or Passport ownership authority",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/profile/claim",
        posture: PassportRoutePosture::CompatibilityOnly,
        native_v1_authority: false,
        note: "current local profile claim route; production username claims must become proof-gated",
    },
    PassportRouteAuthority {
        method: "GET",
        path: "/v1/passport/profile/:username",
        posture: PassportRoutePosture::PublicReadProjection,
        native_v1_authority: false,
        note: "public profile read projection only",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/admin/rotate",
        posture: PassportRoutePosture::AdminOnly,
        native_v1_authority: false,
        note: "service/admin KMS operation; not public client Passport authority",
    },
    PassportRouteAuthority {
        method: "GET",
        path: "/admin/attest",
        posture: PassportRoutePosture::AdminOnly,
        native_v1_authority: false,
        note: "service/admin attestation operation; not public client Passport authority",
    },
];

/// Required Native Passport V1 routes from the build plan.
///
/// These are intentionally marked planned until later phases implement real
/// challenge issue, device proof, request proof, and capability lifecycle.
pub const PLANNED_NATIVE_V1_ROUTE_AUTHORITIES: &[PassportRouteAuthority] = &[
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/challenge",
        posture: PassportRoutePosture::PlannedNativeV1,
        native_v1_authority: true,
        note: "planned one-time purpose-bound challenge issue route",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/prove",
        posture: PassportRoutePosture::PlannedNativeV1,
        native_v1_authority: true,
        note: "planned root/device proof review and challenge consumption route",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/capability/refresh",
        posture: PassportRoutePosture::PlannedNativeV1,
        native_v1_authority: true,
        note: "planned device-bound capability refresh route",
    },
    PassportRouteAuthority {
        method: "POST",
        path: "/v1/passport/capability/revoke",
        posture: PassportRoutePosture::PlannedNativeV1,
        native_v1_authority: true,
        note: "planned capability self-revocation route",
    },
    PassportRouteAuthority {
        method: "GET",
        path: "/v1/passport/status/:passport_id",
        posture: PassportRoutePosture::PlannedNativeV1,
        native_v1_authority: true,
        note: "planned redacted Passport/device status route",
    },
];

/// Phase 0A owner/boundary facts that must remain true before implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePassportOwnerFact {
    /// Owning crate/service/package.
    pub owner: &'static str,
    /// Stable responsibility statement.
    pub responsibility: &'static str,
    /// Explicitly forbidden drift for this owner.
    pub forbidden: &'static str,
}

/// Canonical owner inventory for Native Passport v0.3.
pub const OWNER_FACTS: &[NativePassportOwnerFact] = &[
    NativePassportOwnerFact {
        owner: "svc-passport",
        responsibility:
            "single canonical Passport package owner for native-client and server surfaces",
        forbidden: "new Passport crate or duplicate identity state machine",
    },
    NativePassportOwnerFact {
        owner: "ron-auth",
        responsibility: "pure proof and capability verification rules",
        forbidden: "HTTP routing, challenge persistence, or local vault custody",
    },
    NativePassportOwnerFact {
        owner: "ron-proto",
        responsibility: "strict public DTOs and identifiers",
        forbidden: "mnemonic, PIN, private key, vault key, or filesystem custody",
    },
    NativePassportOwnerFact {
        owner: "ron-naming",
        responsibility: "canonical @username and site-name syntax",
        forbidden: "runtime uniqueness, ledger mutation, or profile storage",
    },
    NativePassportOwnerFact {
        owner: "svc-index",
        responsibility: "read-optimized handle/site projections",
        forbidden: "authoritative username ownership finality",
    },
    NativePassportOwnerFact {
        owner: "svc-storage",
        responsibility: "content-addressed profile and site manifests",
        forbidden: "Passport key custody or namespace finality",
    },
    NativePassportOwnerFact {
        owner: "ron-ledger",
        responsibility: "durable economic truth only",
        forbidden: "general Passport, @username, or site registry ownership",
    },
];

/// Find an existing route classification by method and path.
pub fn current_route(method: &str, path: &str) -> Option<&'static PassportRouteAuthority> {
    CURRENT_ROUTE_AUTHORITIES
        .iter()
        .find(|route| route.method == method && route.path == path)
}

/// Find a planned Native Passport V1 route by method and path.
pub fn planned_native_v1_route(
    method: &str,
    path: &str,
) -> Option<&'static PassportRouteAuthority> {
    PLANNED_NATIVE_V1_ROUTE_AUTHORITIES
        .iter()
        .find(|route| route.method == method && route.path == path)
}
