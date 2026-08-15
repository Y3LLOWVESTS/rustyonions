//! RO:WHAT — Explicit private-beta bootstrap for one QuickChain checkpoint validator signer.
//! RO:WHY — FINAL_BETA Phase 19 needs independent live macronode processes to sign the same reproduced checkpoint.
//! RO:INTERACTS — typed validator identity from ron-proto, ron-kms Ed25519 backend, RuntimeStatus checkpoint signing slot.
//! RO:INVARIANTS — disabled by default; exact typed validator identity + explicit 32-byte seed required; no random fallback.
//! RO:METRICS — none.
//! RO:CONFIG — three explicit private-beta environment variables.
//! RO:SECURITY — seed is never logged or returned; identity validation precedes registration; no committee aggregation or finality authority.
//! RO:TEST — focused tests in this module.

#![forbid(unsafe_code)]

use std::{env, sync::Arc};

use ron_kms::{backends::ed25519, Alg, KeyId, KmsError, Signer};
use ron_proto::quickchain::QuickChainValidatorIdentityV1;

use crate::{
    services::checkpoint_validator_signing::{
        CheckpointValidatorParticipant, CheckpointValidatorSigningError,
        CheckpointValidatorSigningRuntime, CheckpointValidatorSigningRuntimeError,
    },
    types::RuntimeStatus,
};

/// Explicit enable flag for private-beta checkpoint signing.
pub const PRIVATE_BETA_CHECKPOINT_ENABLE_ENV: &str =
    "RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_SIGNING_ENABLED";

/// Exact JSON encoding of one `QuickChainValidatorIdentityV1`.
pub const PRIVATE_BETA_CHECKPOINT_VALIDATOR_JSON_ENV: &str =
    "RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_VALIDATOR_JSON";

/// Exactly 32 Ed25519 seed bytes encoded as 64 hexadecimal characters.
pub const PRIVATE_BETA_CHECKPOINT_SEED_ENV: &str =
    "RON_QUICKCHAIN_PRIVATE_BETA_CHECKPOINT_SEED_HEX";

const MAX_VALIDATOR_JSON_BYTES: usize = 16 * 1024;

/// Public identity facts safe to report after successful private-beta
/// checkpoint-validator bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateBetaCheckpointValidatorIdentity {
    /// Reviewed chain binding.
    pub chain_id: String,

    /// Reviewed epoch binding.
    pub epoch_id: String,

    /// Reviewed validator identifier.
    pub validator_id: String,

    /// Reviewed logical signing-key identifier.
    pub logical_key_id: String,

    /// Deterministically derived Ed25519 public key in lowercase hex.
    pub public_key_hex: String,
}

struct PrivateBetaCheckpointValidatorConfig {
    identity: QuickChainValidatorIdentityV1,
    seed: [u8; 32],
}

impl Drop for PrivateBetaCheckpointValidatorConfig {
    fn drop(&mut self) {
        self.seed.fill(0);
    }
}

struct PrivateBetaCheckpointSeedSigner {
    key_id: KeyId,
    seed: [u8; 32],
}

impl PrivateBetaCheckpointSeedSigner {
    #[must_use]
    fn new(key_id: KeyId, seed: [u8; 32]) -> Self {
        Self { key_id, seed }
    }
}

impl Signer for PrivateBetaCheckpointSeedSigner {
    fn sign(&self, kid: &KeyId, message: &[u8]) -> Result<Vec<u8>, KmsError> {
        if kid != &self.key_id {
            return Err(KmsError::NoSuchKey);
        }

        if kid.alg != Alg::Ed25519 {
            return Err(KmsError::AlgUnavailable);
        }

        Ok(ed25519::sign(&self.seed, message).to_vec())
    }
}

impl Drop for PrivateBetaCheckpointSeedSigner {
    fn drop(&mut self) {
        self.seed.fill(0);
    }
}

/// Install one private-beta checkpoint validator when explicitly enabled.
///
/// Normal macronode startup remains unconfigured.
///
/// # Errors
///
/// Rejects malformed enable state, missing or malformed validator identity,
/// invalid seed material, invalid validator lifecycle/algorithm state, or
/// duplicate runtime registration.
pub fn maybe_register_from_env(
    runtime: &RuntimeStatus,
) -> Result<Option<PrivateBetaCheckpointValidatorIdentity>, CheckpointValidatorBootstrapError> {
    let enabled = parse_enable_flag(optional_env(PRIVATE_BETA_CHECKPOINT_ENABLE_ENV)?.as_deref())?;

    if !enabled {
        return Ok(None);
    }

    let identity_json = required_bounded_env(
        PRIVATE_BETA_CHECKPOINT_VALIDATOR_JSON_ENV,
        MAX_VALIDATOR_JSON_BYTES,
    )?;

    let identity: QuickChainValidatorIdentityV1 =
        serde_json::from_str(&identity_json).map_err(|error| {
            CheckpointValidatorBootstrapError::InvalidValidatorJson(error.to_string())
        })?;

    let seed = decode_seed_hex(&required_secret_env(PRIVATE_BETA_CHECKPOINT_SEED_ENV)?)?;

    register_config(
        runtime,
        PrivateBetaCheckpointValidatorConfig { identity, seed },
    )
    .map(Some)
}

fn register_config(
    runtime: &RuntimeStatus,
    config: PrivateBetaCheckpointValidatorConfig,
) -> Result<PrivateBetaCheckpointValidatorIdentity, CheckpointValidatorBootstrapError> {
    let public_key = ed25519::public_key(&config.seed);

    let public_key_hex = encode_lower_hex(&public_key);

    let concrete_key = KeyId::new(
        config.identity.validator_id.clone(),
        config.identity.key_id.clone(),
        Alg::Ed25519,
    );

    let participant = CheckpointValidatorParticipant::from_reviewed_identity(
        &config.identity,
        concrete_key.clone(),
    )?;

    let public_identity = PrivateBetaCheckpointValidatorIdentity {
        chain_id: config.identity.chain_id.clone(),

        epoch_id: config.identity.epoch_id.clone(),

        validator_id: config.identity.validator_id.clone(),

        logical_key_id: config.identity.key_id.clone(),

        public_key_hex,
    };

    let signer: Arc<dyn Signer> = Arc::new(PrivateBetaCheckpointSeedSigner::new(
        concrete_key,
        config.seed,
    ));

    runtime.register_checkpoint_validator_signer(Arc::new(
        CheckpointValidatorSigningRuntime::new(participant, signer),
    ))?;

    Ok(public_identity)
}

fn parse_enable_flag(raw: Option<&str>) -> Result<bool, CheckpointValidatorBootstrapError> {
    let Some(raw) = raw else {
        return Ok(false);
    };

    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),

        "0" | "false" | "no" | "off" => Ok(false),

        _ => Err(CheckpointValidatorBootstrapError::InvalidField {
            field: PRIVATE_BETA_CHECKPOINT_ENABLE_ENV,

            reason: "expected true/false, 1/0, yes/no, or on/off".to_owned(),
        }),
    }
}

fn optional_env(key: &'static str) -> Result<Option<String>, CheckpointValidatorBootstrapError> {
    match env::var(key) {
        Ok(value) => Ok(Some(value)),

        Err(env::VarError::NotPresent) => Ok(None),

        Err(env::VarError::NotUnicode(_)) => Err(CheckpointValidatorBootstrapError::InvalidField {
            field: key,
            reason: "must contain valid UTF-8".to_owned(),
        }),
    }
}

fn required_bounded_env(
    key: &'static str,
    maximum_bytes: usize,
) -> Result<String, CheckpointValidatorBootstrapError> {
    let value =
        optional_env(key)?.ok_or(CheckpointValidatorBootstrapError::MissingField { field: key })?;

    if value.trim() != value {
        return Err(CheckpointValidatorBootstrapError::InvalidField {
            field: key,
            reason: "must not contain surrounding whitespace".to_owned(),
        });
    }

    if value.is_empty() {
        return Err(CheckpointValidatorBootstrapError::InvalidField {
            field: key,
            reason: "must not be empty".to_owned(),
        });
    }

    if value.len() > maximum_bytes {
        return Err(CheckpointValidatorBootstrapError::InvalidField {
            field: key,
            reason: format!("exceeds {maximum_bytes} byte limit"),
        });
    }

    Ok(value)
}

fn required_secret_env(key: &'static str) -> Result<String, CheckpointValidatorBootstrapError> {
    required_bounded_env(key, 256)
}

fn decode_seed_hex(encoded: &str) -> Result<[u8; 32], CheckpointValidatorBootstrapError> {
    if encoded.len() != 64 {
        return Err(CheckpointValidatorBootstrapError::InvalidField {
            field: PRIVATE_BETA_CHECKPOINT_SEED_ENV,

            reason: "must encode exactly 32 bytes as 64 hexadecimal characters".to_owned(),
        });
    }

    let bytes = encoded.as_bytes();

    let mut seed = [0u8; 32];

    for index in 0..32 {
        let high = decode_hex_nibble(bytes[index * 2]).ok_or_else(|| {
            CheckpointValidatorBootstrapError::InvalidField {
                field: PRIVATE_BETA_CHECKPOINT_SEED_ENV,

                reason: "contains non-hexadecimal characters".to_owned(),
            }
        })?;

        let low = decode_hex_nibble(bytes[index * 2 + 1]).ok_or_else(|| {
            CheckpointValidatorBootstrapError::InvalidField {
                field: PRIVATE_BETA_CHECKPOINT_SEED_ENV,

                reason: "contains non-hexadecimal characters".to_owned(),
            }
        })?;

        seed[index] = (high << 4) | low;
    }

    Ok(seed)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),

        b'a'..=b'f' => Some(byte - b'a' + 10),

        b'A'..=b'F' => Some(byte - b'A' + 10),

        _ => None,
    }
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);

        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }

    encoded
}

/// Private-beta checkpoint-validator bootstrap failure.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointValidatorBootstrapError {
    /// Required private-beta environment value is absent.
    #[error("missing required checkpoint-validator bootstrap field {field}")]
    MissingField {
        /// Environment variable name.
        field: &'static str,
    },

    /// Private-beta environment value is malformed.
    #[error("invalid checkpoint-validator bootstrap field {field}: {reason}")]
    InvalidField {
        /// Environment variable name.
        field: &'static str,

        /// Validation failure.
        reason: String,
    },

    /// Typed validator JSON could not be decoded.
    #[error("invalid checkpoint-validator identity JSON: {0}")]
    InvalidValidatorJson(String),

    /// Typed validator identity could not become an active signer.
    #[error("checkpoint-validator identity rejected: {0}")]
    Participant(#[from] CheckpointValidatorSigningError),

    /// Runtime registration failed closed.
    #[error("checkpoint-validator runtime registration failed: {0}")]
    Runtime(#[from] CheckpointValidatorSigningRuntimeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    use ron_proto::{
        checkpoint_validator_signature_message_bytes,
        quantum::SignatureAlg,
        quickchain::{
            QuickChainValidatorLifecycleStatusV1, QUICKCHAIN_DTO_VERSION,
            QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA,
        },
        ContentId,
    };

    const CHAIN_ID: &str = "ron-devnet";

    const EPOCH_ID: &str = "epoch_phase19_live_checkpoint";

    fn cid(character: char) -> ContentId {
        format!("b3:{}", character.to_string().repeat(64),)
            .parse()
            .expect("fixture content id must parse")
    }

    fn identity(suffix: &str) -> QuickChainValidatorIdentityV1 {
        QuickChainValidatorIdentityV1 {
            schema: QUICKCHAIN_VALIDATOR_IDENTITY_SCHEMA.to_owned(),

            version: QUICKCHAIN_DTO_VERSION,

            chain_id: CHAIN_ID.to_owned(),

            epoch_id: EPOCH_ID.to_owned(),

            validator_id: format!("validator-{suffix}"),

            passport_subject: format!("@validator-{suffix}"),

            registry_entry_id: format!("registry:validator-{suffix}"),

            key_id: format!("key:validator-{suffix}:001"),

            capability_id: format!("cap:validator-{suffix}:verify:001"),

            signature_algorithm: SignatureAlg::Ed25519,

            lifecycle_status: QuickChainValidatorLifecycleStatusV1::Active,

            not_before_ms: 1_800_000_000_000,

            expires_at_ms: 1_800_086_400_000,
        }
    }

    fn config(suffix: &str, seed_byte: u8) -> PrivateBetaCheckpointValidatorConfig {
        PrivateBetaCheckpointValidatorConfig {
            identity: identity(suffix),

            seed: [seed_byte; 32],
        }
    }

    fn decode_signature(encoded: &str) -> [u8; 64] {
        assert_eq!(encoded.len(), 128,);

        let mut signature = [0u8; 64];

        for index in 0..64 {
            let high = decode_hex_nibble(encoded.as_bytes()[index * 2])
                .expect("fixture signature high nibble");

            let low = decode_hex_nibble(encoded.as_bytes()[index * 2 + 1])
                .expect("fixture signature low nibble");

            signature[index] = (high << 4) | low;
        }

        signature
    }

    #[test]
    fn checkpoint_bootstrap_is_disabled_by_default() {
        assert_eq!(
            parse_enable_flag(None).expect("missing enable flag must be valid",),
            false,
        );
    }

    #[test]
    fn explicit_checkpoint_config_registers_real_seed_backed_signer() {
        let runtime = RuntimeStatus::new();

        let seed = [0x11u8; 32];

        let expected_public_key = ed25519::public_key(&seed);

        let public_identity = register_config(
            &runtime,
            PrivateBetaCheckpointValidatorConfig {
                identity: identity("alpha"),

                seed,
            },
        )
        .expect("explicit checkpoint validator config must register");

        assert_eq!(public_identity.chain_id, CHAIN_ID,);

        assert_eq!(public_identity.epoch_id, EPOCH_ID,);

        assert_eq!(public_identity.validator_id, "validator-alpha",);

        assert!(runtime.checkpoint_validator_signing_active(),);

        let signature = runtime
            .sign_checkpoint_validator(19, &cid('a'))
            .expect("registered checkpoint validator must sign");

        let message = checkpoint_validator_signature_message_bytes(&signature.signing_payload())
            .expect("checkpoint signature message must encode");

        let raw_signature = decode_signature(&signature.signature_wire);

        assert!(
            ed25519::verify(&expected_public_key, &message, &raw_signature,),
            "private-beta runtime signature must verify against seed-derived public key",
        );
    }

    #[test]
    fn duplicate_private_beta_checkpoint_bootstrap_rejects() {
        let runtime = RuntimeStatus::new();

        register_config(&runtime, config("alpha", 0x11))
            .expect("first checkpoint validator must register");

        let error = register_config(&runtime, config("beta", 0x22))
            .expect_err("second checkpoint validator must not silently replace first");

        assert!(matches!(
            error,
            CheckpointValidatorBootstrapError::Runtime(
                CheckpointValidatorSigningRuntimeError::AlreadyConfigured
            )
        ),);
    }

    #[test]
    fn malformed_checkpoint_seed_rejects_before_registration() {
        let error = decode_seed_hex("abcd").expect_err("short checkpoint seed must reject");

        assert!(matches!(
            error,
            CheckpointValidatorBootstrapError::InvalidField {
                field: PRIVATE_BETA_CHECKPOINT_SEED_ENV,
                ..
            }
        ),);
    }
}
