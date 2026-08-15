//! RO:WHAT — Explicit private-beta bootstrap for one macronode QuickChain quorum signer.
//! RO:WHY — Join real macronode processes to the Phase 19 cryptographic quorum path.
//! RO:INVARIANTS — disabled by default; explicit identity + seed required; no random fallback.
//! RO:SECURITY — seed is never logged or returned; no quorum aggregation, wallet/ledger
//!               mutation, payout execution, receipt creation, or finality authority.

#![forbid(unsafe_code)]

use std::{env, fmt, sync::Arc};

use ron_kms::{backends::ed25519, Alg, KeyId, KmsError, Signer};

use crate::{
    services::quorum_participation::{
        QuorumParticipationRuntime, QuorumParticipationRuntimeError, ServiceNodeQuorumParticipant,
    },
    types::RuntimeStatus,
};

pub const PRIVATE_BETA_QUORUM_ENABLE_ENV: &str = "RON_QUICKCHAIN_PRIVATE_BETA_SIGNING_ENABLED";

pub const PRIVATE_BETA_QUORUM_CHAIN_ENV: &str = "RON_QUICKCHAIN_PRIVATE_BETA_CHAIN_ID";

pub const PRIVATE_BETA_QUORUM_NODE_ENV: &str = "RON_QUICKCHAIN_PRIVATE_BETA_SERVICE_NODE_ID";

pub const PRIVATE_BETA_QUORUM_KEY_REF_ENV: &str = "RON_QUICKCHAIN_PRIVATE_BETA_KEY_REF";

pub const PRIVATE_BETA_QUORUM_SEED_ENV: &str = "RON_QUICKCHAIN_PRIVATE_BETA_SEED_HEX";

const MAX_IDENTITY_LABEL_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateBetaQuorumIdentity {
    pub chain_id: String,
    pub service_node_id: String,
    pub logical_key_ref: String,
    pub public_key_hex: String,
}

struct PrivateBetaQuorumConfig {
    chain_id: String,
    service_node_id: String,
    logical_key_ref: String,
    seed: [u8; 32],
}

struct PrivateBetaSeedSigner {
    key_id: KeyId,
    seed: [u8; 32],
}

impl PrivateBetaSeedSigner {
    #[must_use]
    fn new(key_id: KeyId, seed: [u8; 32]) -> Self {
        Self { key_id, seed }
    }
}

impl Signer for PrivateBetaSeedSigner {
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

impl Drop for PrivateBetaSeedSigner {
    fn drop(&mut self) {
        self.seed.fill(0);
    }
}

pub fn maybe_register_from_env(
    runtime: &RuntimeStatus,
) -> Result<Option<PrivateBetaQuorumIdentity>, QuorumBootstrapError> {
    let enabled = parse_enable_flag(optional_env(PRIVATE_BETA_QUORUM_ENABLE_ENV)?.as_deref())?;

    if !enabled {
        return Ok(None);
    }

    let config = PrivateBetaQuorumConfig {
        chain_id: required_identity_env(PRIVATE_BETA_QUORUM_CHAIN_ENV)?,

        service_node_id: required_identity_env(PRIVATE_BETA_QUORUM_NODE_ENV)?,

        logical_key_ref: required_identity_env(PRIVATE_BETA_QUORUM_KEY_REF_ENV)?,

        seed: decode_seed_hex(&required_secret_env(PRIVATE_BETA_QUORUM_SEED_ENV)?)?,
    };

    register_config(runtime, config).map(Some)
}

fn register_config(
    runtime: &RuntimeStatus,
    config: PrivateBetaQuorumConfig,
) -> Result<PrivateBetaQuorumIdentity, QuorumBootstrapError> {
    let public_key = ed25519::public_key(&config.seed);

    let public_key_hex = encode_lower_hex(&public_key);

    let concrete_key = KeyId::new(
        config.service_node_id.clone(),
        config.logical_key_ref.clone(),
        Alg::Ed25519,
    );

    let participant = ServiceNodeQuorumParticipant::new(
        config.chain_id.clone(),
        config.service_node_id.clone(),
        config.logical_key_ref.clone(),
        concrete_key.clone(),
    );

    let signer: Arc<dyn Signer> = Arc::new(PrivateBetaSeedSigner::new(concrete_key, config.seed));

    runtime
        .register_quorum_participant(Arc::new(QuorumParticipationRuntime::new(
            participant,
            signer,
        )))
        .map_err(QuorumBootstrapError::Runtime)?;

    Ok(PrivateBetaQuorumIdentity {
        chain_id: config.chain_id,

        service_node_id: config.service_node_id,

        logical_key_ref: config.logical_key_ref,

        public_key_hex,
    })
}

fn parse_enable_flag(raw: Option<&str>) -> Result<bool, QuorumBootstrapError> {
    let Some(raw) = raw else {
        return Ok(false);
    };

    match raw {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),

        _ => Err(QuorumBootstrapError::InvalidField {
            field: PRIVATE_BETA_QUORUM_ENABLE_ENV,

            reason: "expected true/false, 1/0, yes/no, or on/off",
        }),
    }
}

fn optional_env(key: &'static str) -> Result<Option<String>, QuorumBootstrapError> {
    match env::var(key) {
        Ok(value) => Ok(Some(value)),

        Err(env::VarError::NotPresent) => Ok(None),

        Err(env::VarError::NotUnicode(_)) => Err(QuorumBootstrapError::InvalidField {
            field: key,
            reason: "must contain valid UTF-8",
        }),
    }
}

fn required_identity_env(key: &'static str) -> Result<String, QuorumBootstrapError> {
    let value = optional_env(key)?.ok_or(QuorumBootstrapError::MissingField { field: key })?;

    validate_identity_label(key, value)
}

fn required_secret_env(key: &'static str) -> Result<String, QuorumBootstrapError> {
    let value = optional_env(key)?.ok_or(QuorumBootstrapError::MissingField { field: key })?;

    if value.trim() != value {
        return Err(QuorumBootstrapError::InvalidField {
            field: key,
            reason: "must not contain surrounding whitespace",
        });
    }

    if value.is_empty() {
        return Err(QuorumBootstrapError::InvalidField {
            field: key,
            reason: "must not be empty",
        });
    }

    Ok(value)
}

fn validate_identity_label(
    field: &'static str,
    value: String,
) -> Result<String, QuorumBootstrapError> {
    if value.trim() != value {
        return Err(QuorumBootstrapError::InvalidField {
            field,
            reason: "must not contain surrounding whitespace",
        });
    }

    if value.is_empty() {
        return Err(QuorumBootstrapError::InvalidField {
            field,
            reason: "must not be empty",
        });
    }

    if value.len() > MAX_IDENTITY_LABEL_BYTES {
        return Err(QuorumBootstrapError::InvalidField {
            field,
            reason: "exceeds private-beta identity length limit",
        });
    }

    if value.chars().any(char::is_control) {
        return Err(QuorumBootstrapError::InvalidField {
            field,
            reason: "contains forbidden control characters",
        });
    }

    Ok(value)
}

fn decode_seed_hex(raw: &str) -> Result<[u8; 32], QuorumBootstrapError> {
    if raw.len() != 64 {
        return Err(QuorumBootstrapError::InvalidField {
            field: PRIVATE_BETA_QUORUM_SEED_ENV,

            reason: "must contain exactly 64 hexadecimal characters",
        });
    }

    let bytes = raw.as_bytes();
    let mut seed = [0_u8; 32];

    for index in 0..32 {
        let high = decode_hex_nibble(bytes[index * 2])?;

        let low = decode_hex_nibble(bytes[index * 2 + 1])?;

        seed[index] = (high << 4) | low;
    }

    Ok(seed)
}

fn decode_hex_nibble(byte: u8) -> Result<u8, QuorumBootstrapError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),

        _ => Err(QuorumBootstrapError::InvalidField {
            field: PRIVATE_BETA_QUORUM_SEED_ENV,

            reason: "contains a non-hexadecimal character",
        }),
    }
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[usize::from(byte >> 4)] as char);

        encoded.push(HEX[usize::from(byte & 0x0f)] as char);
    }

    encoded
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuorumBootstrapError {
    MissingField {
        field: &'static str,
    },

    InvalidField {
        field: &'static str,
        reason: &'static str,
    },

    Runtime(QuorumParticipationRuntimeError),
}

impl fmt::Display for QuorumBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField { field } => write!(
                formatter,
                "private-beta QuickChain quorum bootstrap requires {field}"
            ),

            Self::InvalidField { field, reason } => write!(
                formatter,
                "invalid private-beta QuickChain quorum field {field}: {reason}"
            ),

            Self::Runtime(error) => write!(
                formatter,
                "private-beta QuickChain quorum runtime registration failed: {error}"
            ),
        }
    }
}

impl std::error::Error for QuorumBootstrapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_is_disabled_when_enable_flag_is_absent() {
        assert_eq!(
            parse_enable_flag(None).expect("absent flag must parse"),
            false
        );
    }

    #[test]
    fn bootstrap_enable_flag_is_strict() {
        assert!(parse_enable_flag(Some("TRUE"),).is_err());

        assert_eq!(
            parse_enable_flag(Some("true"),).expect("canonical true flag"),
            true
        );
    }

    #[test]
    fn seed_hex_requires_exact_32_byte_ed25519_seed() {
        assert!(decode_seed_hex("00").is_err());

        assert!(decode_seed_hex(&"zz".repeat(32),).is_err());

        assert_eq!(
            decode_seed_hex(&"42".repeat(32),).expect("valid seed"),
            [0x42_u8; 32],
        );
    }

    #[test]
    fn seed_signer_refuses_wrong_concrete_key() {
        let expected_key = KeyId::new("service_node:alpha", "key:alpha", Alg::Ed25519);

        let signer = PrivateBetaSeedSigner::new(expected_key.clone(), [0x11_u8; 32]);

        let wrong_key = KeyId::new("service_node:alpha", "key:alpha", Alg::Ed25519);

        assert_ne!(expected_key, wrong_key);

        assert!(matches!(
            signer.sign(&wrong_key, b"message",),
            Err(KmsError::NoSuchKey)
        ));
    }

    #[test]
    fn valid_bootstrap_registers_deterministic_public_identity() {
        let runtime = RuntimeStatus::new();

        let config = PrivateBetaQuorumConfig {
            chain_id: "rustyonions-dev".to_owned(),

            service_node_id: "service_node:alpha".to_owned(),

            logical_key_ref: "key:phase19:alpha".to_owned(),

            seed: [0x42_u8; 32],
        };

        let expected_public_key = encode_lower_hex(&ed25519::public_key(&[0x42_u8; 32]));

        let identity = register_config(&runtime, config).expect("valid bootstrap must register");

        assert_eq!(identity.chain_id, "rustyonions-dev");

        assert_eq!(identity.service_node_id, "service_node:alpha");

        assert_eq!(identity.logical_key_ref, "key:phase19:alpha");

        assert_eq!(identity.public_key_hex, expected_public_key);

        assert!(runtime.quorum_participation_active());
    }

    #[test]
    fn bootstrap_refuses_second_runtime_identity() {
        let runtime = RuntimeStatus::new();

        let first = PrivateBetaQuorumConfig {
            chain_id: "rustyonions-dev".to_owned(),

            service_node_id: "service_node:alpha".to_owned(),

            logical_key_ref: "key:phase19:alpha".to_owned(),

            seed: [0x11_u8; 32],
        };

        register_config(&runtime, first).expect("first bootstrap must register");

        let second = PrivateBetaQuorumConfig {
            chain_id: "rustyonions-dev".to_owned(),

            service_node_id: "service_node:beta".to_owned(),

            logical_key_ref: "key:phase19:beta".to_owned(),

            seed: [0x22_u8; 32],
        };

        let error = register_config(&runtime, second).expect_err("second bootstrap must reject");

        assert!(matches!(
            error,
            QuorumBootstrapError::Runtime(QuorumParticipationRuntimeError::AlreadyConfigured)
        ));
    }

    #[test]
    fn seed_backed_signer_matches_public_key_derivation() {
        let seed = [0x33_u8; 32];

        let key = KeyId::new("service_node:alpha", "key:phase19:alpha", Alg::Ed25519);

        let signer = PrivateBetaSeedSigner::new(key.clone(), seed);

        let message = b"phase19-live-service-node";

        let signature = signer.sign(&key, message).expect("matching key must sign");

        let signature: [u8; 64] = signature
            .try_into()
            .expect("Ed25519 signature must be 64 bytes");

        let public_key = ed25519::public_key(&seed);

        assert!(ed25519::verify(&public_key, message, &signature,));
    }
}
