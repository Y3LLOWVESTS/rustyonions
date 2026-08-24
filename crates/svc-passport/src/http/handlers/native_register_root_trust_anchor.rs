//! RO:WHAT — Exposes the fixed CN-4 Native Passport RegisterRoot challenge-verification trust anchor on the constrained internal svc-passport router.
//! RO:WHY — Physical CrabLink must pin the durable svc-passport service public key/KID and trusted context before any recovery-root proof is signed.
//! RO:INTERACTS — injected KmsClient active public identity, NativePassportServerRuntimeMountConfigV1, ron-proto trusted context/KID/public-key DTOs, and the constrained CrabNode svc-passport router.
//! RO:INVARIANTS — response contains public verification material only; it does not sign, rotate keys, issue challenges, consume replay state, register roots, issue capabilities, mutate usernames, wallets, or ledgers.
//! RO:METRICS — none.
//! RO:CONFIG — network/environment/audience/service identity, challenge TTL, and initial root epoch come only from reviewed server runtime configuration.
//! RO:SECURITY — no secret seed, private key, recovery factor, phrase, PIN, VMK, vault bytes, KMS administration, or ambient authority is exposed.
//! RO:TEST — crabnode_cn4_register_root_challenge_route.rs plus gateway negative-route regression.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{Extension, Json};
use ron_proto::{Ed25519PublicKeyHex, NativePassportContextLabelV1, ServiceKeyIdV1};
use serde::{Deserialize, Serialize};

use crate::{
    kms::client::KmsClient,
    native::{NativePassportServerRuntimeMountConfigV1, NativePassportServerRuntimeMountError},
};

pub const NATIVE_REGISTER_ROOT_TRUST_ANCHOR_SCHEMA_V1: &str =
    "svc-passport.native-register-root-trust-anchor.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRegisterRootTrustAnchorV1 {
    pub schema: String,
    pub network_id: NativePassportContextLabelV1,
    pub environment: NativePassportContextLabelV1,
    pub audience: NativePassportContextLabelV1,
    pub issuing_service_id: NativePassportContextLabelV1,
    pub service_key_id: ServiceKeyIdV1,
    pub service_public_key: Ed25519PublicKeyHex,
    pub challenge_ttl_ms: u64,
    pub trusted_initial_root_key_epoch: u64,
}

pub(crate) struct NativeRegisterRootTrustAnchorHttpState {
    anchor: NativeRegisterRootTrustAnchorV1,
}

impl NativeRegisterRootTrustAnchorHttpState {
    pub(crate) async fn new(
        kms: Arc<dyn KmsClient>,
        config: &NativePassportServerRuntimeMountConfigV1,
    ) -> Result<Self, NativePassportServerRuntimeMountError> {
        let signing_identity = kms.active_signing_identity().await.map_err(|error| {
            NativePassportServerRuntimeMountError::KmsUnavailable(error.to_string())
        })?;

        if signing_identity.kid.trim().is_empty() {
            return Err(NativePassportServerRuntimeMountError::InvalidServiceKeyIdentity);
        }

        let service_key_id = ServiceKeyIdV1::parse(signing_identity.kid)
            .map_err(|_| NativePassportServerRuntimeMountError::InvalidServiceKeyIdentity)?;

        let service_public_key =
            Ed25519PublicKeyHex::parse(lower_hex_32(&signing_identity.public_key))
                .map_err(|_| NativePassportServerRuntimeMountError::InvalidServiceKeyIdentity)?;

        Ok(Self {
            anchor: NativeRegisterRootTrustAnchorV1 {
                schema: NATIVE_REGISTER_ROOT_TRUST_ANCHOR_SCHEMA_V1.to_owned(),
                network_id: parse_context("network_id", &config.network_id)?,
                environment: parse_context("environment", &config.environment)?,
                audience: parse_context("audience", &config.audience)?,
                issuing_service_id: parse_context(
                    "issuing_service_id",
                    &config.issuing_service_id,
                )?,
                service_key_id,
                service_public_key,
                challenge_ttl_ms: config.challenge_ttl_ms,
                trusted_initial_root_key_epoch: config.trusted_initial_root_key_epoch,
            },
        })
    }
}

pub(crate) async fn read_register_root_trust_anchor(
    Extension(state): Extension<Arc<NativeRegisterRootTrustAnchorHttpState>>,
) -> Json<NativeRegisterRootTrustAnchorV1> {
    Json(state.anchor.clone())
}

fn parse_context(
    field: &'static str,
    value: &str,
) -> Result<NativePassportContextLabelV1, NativePassportServerRuntimeMountError> {
    NativePassportContextLabelV1::parse(value.to_owned()).map_err(|error| {
        NativePassportServerRuntimeMountError::InvalidTrustedContext {
            field,
            reason: error.to_string(),
        }
    })
}

fn lower_hex_32(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut output = String::with_capacity(64);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(*byte >> 4)]));
        output.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }

    output
}
