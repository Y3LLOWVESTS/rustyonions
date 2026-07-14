//! RO:WHAT — Local reward-recipient operator endpoints for crabnode and CrabLink.
//! RO:WHY — Phase 21 adds verified signed operator intent while preserving legacy runtime-local CLI behavior.
//! RO:INTERACTS — AppState::operator, crabnode rewards CLI, CrabLink Operator Mode, future svc-registry/passport flow.
//! RO:INVARIANTS — runtime-local request/display state only; no registry finality, wallet mutation, ledger mutation, or confirmed ROC.
//! RO:SECURITY — rejects malformed @ addresses; sensitive POSTs are guarded; signed intents are timestamped and replay-rejected.
//! RO:TEST — focused unit tests below plus macronode rewards_http integration tests.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::types::{AppState, RewardRecipientSnapshot};

const SIGNED_INTENT_VERSION: u8 = 1;
const SIGNER_KIND: &str = "admin_bearer_blake3_keyed_v1";
const MAX_INTENT_AGE_MS: u64 = 5 * 60 * 1_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindRewardRecipientRequest {
    reward_recipient_display_address: String,

    #[serde(default)]
    note: Option<String>,

    #[serde(default)]
    signed_intent: Option<SignedRewardBindingIntentRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SignedRewardBindingIntentRequest {
    version: u8,
    signer_kind: String,
    created_at_ms: u64,
    nonce: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateRewardRecipientRequest {
    new_reward_recipient_display_address: String,

    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientStatusResponse {
    version: u8,
    service_node_id: &'static str,
    state: &'static str,
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    updated_at_unix_s: Option<u64>,
    signed_intent_supported: bool,
    registry_finality: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
    confirmed_roc: Option<u64>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientMutationResponse {
    status: &'static str,
    state: &'static str,
    reward_recipient_display_address: Option<String>,
    pending_rotation_display_address: Option<String>,
    signed_intent_verified: bool,
    intent_signer_kind: Option<&'static str>,
    registry_finality: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
    confirmed_roc: Option<u64>,
    note: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RewardRecipientErrorResponse {
    status: &'static str,
    error: String,
    signed_intent_verified: bool,
    registry_finality: bool,
    wallet_mutation: bool,
    ledger_mutation: bool,
}

#[derive(Debug)]
struct VerifiedSignedIntent {
    nonce: String,
}

pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let snapshot = state.operator.reward_recipient_snapshot();

    Json(RewardRecipientStatusResponse {
        version: 1,
        service_node_id: "local_service_node",
        state: snapshot.state,
        reward_recipient_display_address: snapshot.reward_recipient_display_address,
        pending_rotation_display_address: snapshot.pending_rotation_display_address,
        updated_at_unix_s: snapshot.updated_at_unix_s,
        signed_intent_supported: true,
        registry_finality: false,
        wallet_mutation: false,
        ledger_mutation: false,
        confirmed_roc: None,
        note: "Runtime-local operator request state only; registry, wallet, ledger, and confirmed ROC remain separate.",
    })
}

pub async fn bind(
    State(state): State<AppState>,
    Json(req): Json<BindRewardRecipientRequest>,
) -> impl IntoResponse {
    let address = req.reward_recipient_display_address.trim();

    if let Err(error) = validate_crablink_address(address) {
        return error_response(StatusCode::BAD_REQUEST, "reward recipient rejected", error);
    }

    let verified_intent = match req.signed_intent.as_ref() {
        Some(intent) => {
            let verified = match verify_signed_intent_from_env(address, intent) {
                Ok(verified) => verified,
                Err(error) => {
                    return error_response(
                        StatusCode::UNAUTHORIZED,
                        "reward recipient rejected",
                        error,
                    );
                }
            };

            if let Err(error) = state
                .operator
                .register_reward_binding_intent_nonce(&verified.nonce)
            {
                return error_response(
                    StatusCode::CONFLICT,
                    "reward recipient rejected",
                    error.to_string(),
                );
            }

            Some(verified)
        }
        None => None,
    };

    let snapshot = state.operator.bind_reward_recipient(address.to_string());

    let _ = req.note;

    (
        StatusCode::ACCEPTED,
        Json(response_from_snapshot(
            "binding request recorded",
            snapshot,
            "Request recorded locally for operator flow; no registry finality, wallet/ledger mutation, or confirmed ROC.",
            verified_intent.is_some(),
        )),
    )
        .into_response()
}

pub async fn rotate(
    State(state): State<AppState>,
    Json(req): Json<RotateRewardRecipientRequest>,
) -> impl IntoResponse {
    let address = req.new_reward_recipient_display_address.trim();

    if let Err(error) = validate_crablink_address(address) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "reward recipient rotation rejected",
            error,
        );
    }

    let current = state.operator.reward_recipient_snapshot();

    if current.reward_recipient_display_address.is_none() {
        return error_response(
            StatusCode::CONFLICT,
            "reward recipient rotation rejected",
            "cannot rotate before a reward recipient is bound".to_string(),
        );
    }

    let snapshot = state
        .operator
        .request_reward_recipient_rotation(address.to_string());

    let _ = req.note;

    (
        StatusCode::ACCEPTED,
        Json(response_from_snapshot(
            "rotation request recorded",
            snapshot,
            "Rotation request recorded locally; signed future-epoch registry acceptance remains future work.",
            false,
        )),
    )
        .into_response()
}

fn response_from_snapshot(
    status: &'static str,
    snapshot: RewardRecipientSnapshot,
    note: &'static str,
    signed_intent_verified: bool,
) -> RewardRecipientMutationResponse {
    RewardRecipientMutationResponse {
        status,
        state: snapshot.state,
        reward_recipient_display_address: snapshot.reward_recipient_display_address,
        pending_rotation_display_address: snapshot.pending_rotation_display_address,
        signed_intent_verified,
        intent_signer_kind: signed_intent_verified.then_some(SIGNER_KIND),
        registry_finality: false,
        wallet_mutation: false,
        ledger_mutation: false,
        confirmed_roc: None,
        note,
    }
}

fn error_response(status: StatusCode, label: &'static str, error: String) -> Response {
    (
        status,
        Json(RewardRecipientErrorResponse {
            status: label,
            error,
            signed_intent_verified: false,
            registry_finality: false,
            wallet_mutation: false,
            ledger_mutation: false,
        }),
    )
        .into_response()
}

fn verify_signed_intent_from_env(
    address: &str,
    intent: &SignedRewardBindingIntentRequest,
) -> Result<VerifiedSignedIntent, String> {
    let admin_token = env::var("RON_ADMIN_TOKEN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "signed reward binding requires RON_ADMIN_TOKEN".to_string())?;

    verify_signed_intent(address, intent, admin_token.trim(), now_ms())
}

fn verify_signed_intent(
    address: &str,
    intent: &SignedRewardBindingIntentRequest,
    admin_token: &str,
    observed_at_ms: u64,
) -> Result<VerifiedSignedIntent, String> {
    if intent.version != SIGNED_INTENT_VERSION {
        return Err("unsupported signed reward-binding intent version".to_string());
    }

    if intent.signer_kind != SIGNER_KIND {
        return Err("unsupported signed reward-binding signer kind".to_string());
    }

    if observed_at_ms.abs_diff(intent.created_at_ms) > MAX_INTENT_AGE_MS {
        return Err(
            "signed reward-binding intent timestamp is outside the allowed window".to_string(),
        );
    }

    if !(16..=128).contains(&intent.nonce.len())
        || !intent
            .nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("signed reward-binding intent nonce is malformed".to_string());
    }

    if intent.signature.len() != 64
        || !intent
            .signature
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err("signed reward-binding intent signature is malformed".to_string());
    }

    let expected = sign_intent(admin_token, address, intent.created_at_ms, &intent.nonce);

    if !constant_time_eq(expected.as_bytes(), intent.signature.as_bytes()) {
        return Err("signed reward-binding intent signature is invalid".to_string());
    }

    Ok(VerifiedSignedIntent {
        nonce: intent.nonce.clone(),
    })
}

fn sign_intent(admin_token: &str, address: &str, created_at_ms: u64, nonce: &str) -> String {
    let key = blake3::hash(admin_token.as_bytes());
    let payload = canonical_intent(address, created_at_ms, nonce);

    blake3::keyed_hash(key.as_bytes(), payload.as_bytes())
        .to_hex()
        .to_string()
}

fn canonical_intent(address: &str, created_at_ms: u64, nonce: &str) -> String {
    format!(
        "crablink.reward_binding.intent.v1\n\
         {address}\n\
         {created_at_ms}\n\
         {nonce}"
    )
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut difference = 0_u8;

    for (left_byte, right_byte) in left.iter().zip(right) {
        difference |= *left_byte ^ *right_byte;
    }

    difference == 0
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn validate_crablink_address(address: &str) -> Result<(), String> {
    let Some(username) = address.strip_prefix('@') else {
        return Err("reward recipient must be a CrabLink/RON @ address like @operator".to_string());
    };

    let bytes = username.as_bytes();

    if !(3..=32).contains(&bytes.len()) {
        return Err("reward recipient username must be 3..=32 characters after @".to_string());
    }

    if !bytes[0].is_ascii_alphanumeric() {
        return Err(
            "reward recipient username must start with an ASCII letter or digit".to_string(),
        );
    }

    if matches!(bytes[bytes.len() - 1], b'.' | b'-' | b'_') {
        return Err("reward recipient username must not end with '.', '-', or '_'".to_string());
    }

    let mut previous_dot = false;

    for byte in bytes {
        let valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(*byte, b'_' | b'-' | b'.');

        if !valid {
            return Err(
                "reward recipient username must use lowercase ASCII letters, digits, '.', '-', or '_'"
                    .to_string(),
            );
        }

        if previous_dot && *byte == b'.' {
            return Err("reward recipient username must not contain consecutive dots".to_string());
        }

        previous_dot = *byte == b'.';
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent_for(
        token: &str,
        address: &str,
        created_at_ms: u64,
        nonce: &str,
    ) -> SignedRewardBindingIntentRequest {
        SignedRewardBindingIntentRequest {
            version: SIGNED_INTENT_VERSION,
            signer_kind: SIGNER_KIND.to_string(),
            created_at_ms,
            nonce: nonce.to_string(),
            signature: sign_intent(token, address, created_at_ms, nonce),
        }
    }

    #[test]
    fn valid_admin_authenticated_signed_intent_verifies() {
        let nonce = "a".repeat(64);
        let intent = intent_for("secret", "@operator", 10_000, &nonce);

        let verified = verify_signed_intent("@operator", &intent, "secret", 10_100)
            .expect("valid signed intent");

        assert_eq!(verified.nonce, nonce);
    }

    #[test]
    fn signed_intent_is_bound_to_address_and_admin_token() {
        let intent = intent_for("secret", "@operator", 10_000, &"b".repeat(64));

        assert!(verify_signed_intent("@other", &intent, "secret", 10_100,).is_err());

        assert!(verify_signed_intent("@operator", &intent, "other-secret", 10_100,).is_err());
    }

    #[test]
    fn stale_or_malformed_signed_intent_rejects() {
        let mut intent = intent_for("secret", "@operator", 10_000, &"c".repeat(64));

        assert!(verify_signed_intent(
            "@operator",
            &intent,
            "secret",
            10_000 + MAX_INTENT_AGE_MS + 1,
        )
        .is_err());

        intent.signature = "AA".repeat(32);

        assert!(verify_signed_intent("@operator", &intent, "secret", 10_100,).is_err());
    }

    #[test]
    fn response_never_claims_registry_wallet_or_ledger_truth() {
        let snapshot = RewardRecipientSnapshot {
            state: "bound",
            reward_recipient_display_address: Some("@operator".to_string()),
            pending_rotation_display_address: None,
            updated_at_unix_s: Some(1),
        };

        let response = response_from_snapshot(
            "binding request recorded",
            snapshot,
            "runtime-local only",
            true,
        );

        assert!(response.signed_intent_verified);
        assert!(!response.registry_finality);
        assert!(!response.wallet_mutation);
        assert!(!response.ledger_mutation);
        assert_eq!(response.confirmed_roc, None);
    }
}
