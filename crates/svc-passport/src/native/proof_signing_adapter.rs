//! RO:WHAT — Native Passport injected proof-signing adapter seam.
//! RO:WHY — P3 Identity & Keys. Allows a future platform-local signer to produce a signed payload only after Phase 9A preflight accepted the exact challenge/proof/request binding.
//! RO:INTERACTS — Phase 9A proof-signing boundary decisions and future platform/native signer implementations.
//! RO:INVARIANTS — the adapter validates decision, signer label, authority, algorithm, transcript hash, timing, and unsafe flags before calling an injected signer. It does not load keys, unlock vaults, derive keys, verify proofs, mutate replay/storage, add routes, perform runtime I/O, or touch wallet/ledger state.
//! RO:TEST — tests/native_passport_phase9b_native_proof_signing_adapter.rs.

use super::{
    B3DigestHex, ChallengeIdV1, DeviceIdV1, NativePassportProofAuthority, NativePassportProofKind,
    NativeProofSignatureAlgorithm, NativeProofSigningContractBoundaryDecisionV1,
    NativeProofSigningOperationKind, PassportIdV1, PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    PHASE8B_PROOF_CONTRACT_DOMAIN, PHASE8B_PROOF_CONTRACT_VERSION,
    PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM, PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN,
    PHASE8C_PROOF_REQUEST_CONTRACT_VERSION,
};

pub const NATIVE_PASSPORT_PHASE9B_LABEL: &str =
    "NATIVE_PASSPORT_PHASE9B_NATIVE_PROOF_SIGNING_ADAPTER";

pub const PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN: &str = "native-passport/proof-signing-adapter/v1";

pub const PHASE9B_PROOF_SIGNING_ADAPTER_VERSION: u16 = 1;

pub const PHASE9B_FORBIDDEN_SIGNING_ADAPTER_AUTHORITY_FLAGS: &[&str] = &[
    "secret_key_access",
    "key_loading",
    "key_derivation_runtime",
    "vault_unlock",
    "vault_runtime",
    "platform_sealer_unseal",
    "proof_signature_verification",
    "request_proof_verification",
    "service_challenge_verification",
    "replay_store_mutation",
    "challenge_consumption",
    "capability_consumption",
    "capability_issuance",
    "runtime_io",
    "route_added",
    "storage_mutation",
    "wallet_spend",
    "ledger_mutation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSignedPayloadHex(String);

impl NativeProofSignedPayloadHex {
    pub fn parse(
        label: &'static str,
        value: impl Into<String>,
    ) -> Result<Self, NativeProofSigningAdapterReviewError> {
        let value = value.into();

        if value.len() != 128 || !value.as_bytes().iter().all(u8::is_ascii_hexdigit) {
            return Err(NativeProofSigningAdapterReviewError::InvalidSignedPayload(
                label,
            ));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningAdapterDraftV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub signer_authority: NativePassportProofAuthority,
    pub local_signer_unlocked: bool,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub requested_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signing_requested_at_ms: u64,
    pub allows_injected_local_signer: bool,
    pub requests_secret_key_access: bool,
    pub requests_key_loading: bool,
    pub requests_key_derivation_runtime: bool,
    pub requests_vault_unlock: bool,
    pub includes_vault_runtime: bool,
    pub requests_platform_sealer_unseal: bool,
    pub requests_proof_signature_verification: bool,
    pub requests_request_proof_verification: bool,
    pub requests_service_challenge_verification: bool,
    pub requests_replay_store_mutation: bool,
    pub requests_challenge_consumption: bool,
    pub requests_capability_consumption: bool,
    pub requests_capability_issuance: bool,
    pub requests_runtime_io: bool,
    pub adds_routes: bool,
    pub requests_storage_mutation: bool,
    pub requests_wallet_or_ledger_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningAdapterRequestV1 {
    pub operation_kind: NativeProofSigningOperationKind,
    pub signer_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub requested_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signing_requested_at_ms: u64,
    pub contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningAdapterEnvelopeV1 {
    pub contract_domain: &'static str,
    pub contract_version: u16,
    pub operation_kind: NativeProofSigningOperationKind,
    pub signer_authority: NativePassportProofAuthority,
    pub challenge_id: ChallengeIdV1,
    pub passport_id: Option<PassportIdV1>,
    pub device_id: Option<DeviceIdV1>,
    pub proof_contract_domain: &'static str,
    pub proof_contract_version: u16,
    pub request_contract_domain: Option<&'static str>,
    pub request_contract_version: Option<u16>,
    pub proof_kind: NativePassportProofKind,
    pub transcript_hash: B3DigestHex,
    pub signer_public_key_label: &'static str,
    pub requested_signature_algorithm: NativeProofSignatureAlgorithm,
    pub signing_requested_at_ms: u64,
    pub signed_payload_hex: NativeProofSignedPayloadHex,
    pub signed_payload_present: bool,
    pub secret_material_exposed: bool,
    pub runtime_io_performed: bool,
    pub wallet_or_ledger_mutated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeProofSigningAdapterReviewError {
    ContractDomainMismatch,
    ContractVersionMismatch,
    BoundaryDecisionInvalid,
    BoundaryDecisionAlreadyContainsSignedPayload,
    InjectedSignerNotAllowed,
    OperationMismatch,
    SignerAuthorityMismatch,
    ChallengeIdMismatch,
    PassportBindingMismatch,
    DeviceBindingMismatch,
    ProofContractDomainMismatch,
    ProofContractVersionMismatch,
    RequestProofContractDomainMismatch,
    RequestProofContractVersionMismatch,
    ProofKindMismatch,
    TranscriptHashMismatch,
    MissingSignerPublicKeyLabel,
    SignerPublicKeyLabelMismatch,
    LocalSignerLocked,
    SignatureAlgorithmMismatch,
    SigningTimeMismatch,
    UnsafeAdapterAuthorityFlag,
    SignerRejected,
    InvalidSignedPayload(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProofSigningAdapterPosture {
    pub phase_label: &'static str,
    pub proof_signing_adapter_added: bool,
    pub root_proof_signing_adapter_added: bool,
    pub device_proof_signing_adapter_added: bool,
    pub request_proof_signing_adapter_added: bool,
    pub secret_key_access_added: bool,
    pub key_loading_added: bool,
    pub key_derivation_runtime_added: bool,
    pub vault_unlock_added: bool,
    pub vault_runtime_added: bool,
    pub platform_sealer_unseal_added: bool,
    pub proof_signature_verification_added: bool,
    pub request_proof_verification_added: bool,
    pub replay_store_mutation_added: bool,
    pub challenge_consumption_added: bool,
    pub capability_consumption_added: bool,
    pub capability_issuance_added: bool,
    pub runtime_io_added: bool,
    pub routes_added: bool,
    pub storage_mutation_added: bool,
    pub wallet_or_ledger_mutation_added: bool,
    pub runtime_authority_changed: bool,
    pub native_secret_implementation_added: bool,
    pub forbidden_authority_flags: &'static [&'static str],
}

pub trait NativeLocalProofSigningAdapter {
    fn produce_signed_payload(
        &self,
        request: &NativeProofSigningAdapterRequestV1,
    ) -> Result<NativeProofSignedPayloadHex, NativeProofSigningAdapterReviewError>;
}

pub fn native_proof_signing_adapter_posture() -> NativeProofSigningAdapterPosture {
    NativeProofSigningAdapterPosture {
        phase_label: NATIVE_PASSPORT_PHASE9B_LABEL,
        proof_signing_adapter_added: true,
        root_proof_signing_adapter_added: true,
        device_proof_signing_adapter_added: true,
        request_proof_signing_adapter_added: true,
        secret_key_access_added: false,
        key_loading_added: false,
        key_derivation_runtime_added: false,
        vault_unlock_added: false,
        vault_runtime_added: false,
        platform_sealer_unseal_added: false,
        proof_signature_verification_added: false,
        request_proof_verification_added: false,
        replay_store_mutation_added: false,
        challenge_consumption_added: false,
        capability_consumption_added: false,
        capability_issuance_added: false,
        runtime_io_added: false,
        routes_added: false,
        storage_mutation_added: false,
        wallet_or_ledger_mutation_added: false,
        runtime_authority_changed: false,
        native_secret_implementation_added: false,
        forbidden_authority_flags: PHASE9B_FORBIDDEN_SIGNING_ADAPTER_AUTHORITY_FLAGS,
    }
}

pub fn execute_native_proof_signing_adapter<S: NativeLocalProofSigningAdapter>(
    decision: &NativeProofSigningContractBoundaryDecisionV1,
    draft: NativeProofSigningAdapterDraftV1,
    signer: &S,
) -> Result<NativeProofSigningAdapterEnvelopeV1, NativeProofSigningAdapterReviewError> {
    validate_decision(decision)?;
    validate_adapter_draft(decision, &draft)?;

    let request = NativeProofSigningAdapterRequestV1 {
        operation_kind: draft.operation_kind,
        signer_authority: draft.signer_authority,
        challenge_id: draft.challenge_id.clone(),
        passport_id: draft.passport_id.clone(),
        device_id: draft.device_id.clone(),
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash.clone(),
        signer_public_key_label: draft.signer_public_key_label,
        requested_signature_algorithm: draft.requested_signature_algorithm,
        signing_requested_at_ms: draft.signing_requested_at_ms,
        contract_only: true,
    };

    let signed_payload_hex = signer.produce_signed_payload(&request)?;
    let signed_payload_hex =
        NativeProofSignedPayloadHex::parse("signed_payload_hex", signed_payload_hex.as_str())?;

    Ok(NativeProofSigningAdapterEnvelopeV1 {
        contract_domain: PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN,
        contract_version: PHASE9B_PROOF_SIGNING_ADAPTER_VERSION,
        operation_kind: draft.operation_kind,
        signer_authority: draft.signer_authority,
        challenge_id: draft.challenge_id,
        passport_id: draft.passport_id,
        device_id: draft.device_id,
        proof_contract_domain: PHASE8B_PROOF_CONTRACT_DOMAIN,
        proof_contract_version: PHASE8B_PROOF_CONTRACT_VERSION,
        request_contract_domain: draft.request_contract_domain,
        request_contract_version: draft.request_contract_version,
        proof_kind: draft.proof_kind,
        transcript_hash: draft.transcript_hash,
        signer_public_key_label: draft.signer_public_key_label,
        requested_signature_algorithm: draft.requested_signature_algorithm,
        signing_requested_at_ms: draft.signing_requested_at_ms,
        signed_payload_hex,
        signed_payload_present: true,
        secret_material_exposed: false,
        runtime_io_performed: false,
        wallet_or_ledger_mutated: false,
    })
}

fn validate_decision(
    decision: &NativeProofSigningContractBoundaryDecisionV1,
) -> Result<(), NativeProofSigningAdapterReviewError> {
    if !decision.contract_only || decision.signing_runtime_enabled {
        return Err(NativeProofSigningAdapterReviewError::BoundaryDecisionInvalid);
    }

    if decision.signed_payload_present {
        return Err(
            NativeProofSigningAdapterReviewError::BoundaryDecisionAlreadyContainsSignedPayload,
        );
    }

    Ok(())
}

fn validate_adapter_draft(
    decision: &NativeProofSigningContractBoundaryDecisionV1,
    draft: &NativeProofSigningAdapterDraftV1,
) -> Result<(), NativeProofSigningAdapterReviewError> {
    if draft.contract_domain != PHASE9B_PROOF_SIGNING_ADAPTER_DOMAIN {
        return Err(NativeProofSigningAdapterReviewError::ContractDomainMismatch);
    }

    if draft.contract_version != PHASE9B_PROOF_SIGNING_ADAPTER_VERSION {
        return Err(NativeProofSigningAdapterReviewError::ContractVersionMismatch);
    }

    if !draft.allows_injected_local_signer {
        return Err(NativeProofSigningAdapterReviewError::InjectedSignerNotAllowed);
    }

    if draft.operation_kind != decision.operation_kind {
        return Err(NativeProofSigningAdapterReviewError::OperationMismatch);
    }

    if draft.signer_authority != decision.accepted_authority {
        return Err(NativeProofSigningAdapterReviewError::SignerAuthorityMismatch);
    }

    if draft.challenge_id != decision.challenge_id {
        return Err(NativeProofSigningAdapterReviewError::ChallengeIdMismatch);
    }

    if draft.passport_id != decision.passport_id {
        return Err(NativeProofSigningAdapterReviewError::PassportBindingMismatch);
    }

    if draft.device_id != decision.device_id {
        return Err(NativeProofSigningAdapterReviewError::DeviceBindingMismatch);
    }

    if draft.proof_contract_domain != decision.proof_contract_domain
        || draft.proof_contract_domain != PHASE8B_PROOF_CONTRACT_DOMAIN
    {
        return Err(NativeProofSigningAdapterReviewError::ProofContractDomainMismatch);
    }

    if draft.proof_contract_version != decision.proof_contract_version
        || draft.proof_contract_version != PHASE8B_PROOF_CONTRACT_VERSION
    {
        return Err(NativeProofSigningAdapterReviewError::ProofContractVersionMismatch);
    }

    if draft.request_contract_domain != decision.request_contract_domain {
        return Err(NativeProofSigningAdapterReviewError::RequestProofContractDomainMismatch);
    }

    if draft.request_contract_version != decision.request_contract_version {
        return Err(NativeProofSigningAdapterReviewError::RequestProofContractVersionMismatch);
    }

    if let Some(domain) = draft.request_contract_domain {
        if domain != PHASE8C_PROOF_REQUEST_CONTRACT_DOMAIN {
            return Err(NativeProofSigningAdapterReviewError::RequestProofContractDomainMismatch);
        }
    }

    if let Some(version) = draft.request_contract_version {
        if version != PHASE8C_PROOF_REQUEST_CONTRACT_VERSION {
            return Err(NativeProofSigningAdapterReviewError::RequestProofContractVersionMismatch);
        }
    }

    if draft.proof_kind != decision.proof_kind {
        return Err(NativeProofSigningAdapterReviewError::ProofKindMismatch);
    }

    if draft.transcript_hash != decision.transcript_hash {
        return Err(NativeProofSigningAdapterReviewError::TranscriptHashMismatch);
    }

    if draft.signer_public_key_label.is_empty() {
        return Err(NativeProofSigningAdapterReviewError::MissingSignerPublicKeyLabel);
    }

    if draft.signer_public_key_label != decision.signer_public_key_label {
        return Err(NativeProofSigningAdapterReviewError::SignerPublicKeyLabelMismatch);
    }

    if !draft.local_signer_unlocked {
        return Err(NativeProofSigningAdapterReviewError::LocalSignerLocked);
    }

    if draft.requested_signature_algorithm != decision.requested_signature_algorithm {
        return Err(NativeProofSigningAdapterReviewError::SignatureAlgorithmMismatch);
    }

    validate_signature_algorithm(draft.signer_authority, draft.requested_signature_algorithm)?;

    if draft.signing_requested_at_ms != decision.signing_requested_at_ms {
        return Err(NativeProofSigningAdapterReviewError::SigningTimeMismatch);
    }

    validate_no_unsafe_flags(draft)
}

fn validate_signature_algorithm(
    authority: NativePassportProofAuthority,
    algorithm: NativeProofSignatureAlgorithm,
) -> Result<(), NativeProofSigningAdapterReviewError> {
    let expected = match authority {
        NativePassportProofAuthority::RootAuthority => PHASE8B_ROOT_PROOF_SIGNATURE_ALGORITHM,
        NativePassportProofAuthority::DeviceAuthority => PHASE8B_DEVICE_PROOF_SIGNATURE_ALGORITHM,
    };

    if algorithm != expected {
        return Err(NativeProofSigningAdapterReviewError::SignatureAlgorithmMismatch);
    }

    Ok(())
}

fn validate_no_unsafe_flags(
    draft: &NativeProofSigningAdapterDraftV1,
) -> Result<(), NativeProofSigningAdapterReviewError> {
    if draft.requests_secret_key_access
        || draft.requests_key_loading
        || draft.requests_key_derivation_runtime
        || draft.requests_vault_unlock
        || draft.includes_vault_runtime
        || draft.requests_platform_sealer_unseal
        || draft.requests_proof_signature_verification
        || draft.requests_request_proof_verification
        || draft.requests_service_challenge_verification
        || draft.requests_replay_store_mutation
        || draft.requests_challenge_consumption
        || draft.requests_capability_consumption
        || draft.requests_capability_issuance
        || draft.requests_runtime_io
        || draft.adds_routes
        || draft.requests_storage_mutation
        || draft.requests_wallet_or_ledger_mutation
    {
        return Err(NativeProofSigningAdapterReviewError::UnsafeAdapterAuthorityFlag);
    }

    Ok(())
}
