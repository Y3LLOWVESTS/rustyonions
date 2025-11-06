//! RO:WHAT  Batch verification with a fast all-or-nothing path using dalek's batch API,
//!          and a per-item fallback to produce precise booleans when a batch contains failures.
//! RO:WHY   Batch verify amortizes expensive scalar multiplications; this is our “God tier” lever.
//! RO:INV   - All items must share `Alg::Ed25519` for fast path.
//!          - If the batch fails as a whole, we fall back to per-item verify to return Vec<bool>.

use crate::{
    error::KmsError,
    traits::pubkey::PubkeyProvider,
    traits::Verifier,
    types::{Alg, KeyId},
};

/// One item to verify in a batch.
pub struct VerifyItem<'m, 's> {
    pub kid: &'m KeyId,
    pub msg: &'m [u8],
    pub sig: &'s [u8],
}

/// Verify a batch of (kid, msg, sig). Returns per-item booleans.
///
/// Fast path (all Ed25519, all pass): single dalek batch verify → Vec<true>.
/// If the batch fails, we fall back to per-item verify for correctness.
/// For mixed algorithms, we chunk by alg (currently only Ed25519 supported).
pub fn verify_batch<K>(kms: &K, items: &[VerifyItem<'_, '_>]) -> Result<Vec<bool>, KmsError>
where
    K: Verifier + PubkeyProvider,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }

    // Verify all items are Ed25519 (current supported fast path).
    let all_ed25519 = items.iter().all(|it| it.kid.alg == Alg::Ed25519);
    if !all_ed25519 {
        // Fallback: per-item verify for any non-Ed25519.
        return items
            .iter()
            .map(|it| kms.verify(it.kid, it.msg, it.sig))
            .collect();
    }

    // Collect publics, messages, signatures for dalek batch.
    let mut msgs: Vec<&[u8]> = Vec::with_capacity(items.len());
    let mut sigs: Vec<ed25519_dalek::Signature> = Vec::with_capacity(items.len());
    let mut pubs: Vec<ed25519_dalek::VerifyingKey> = Vec::with_capacity(items.len());

    for it in items {
        let pk_bytes: [u8; 32] = kms.verifying_key_bytes(it.kid)?;
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|_| KmsError::VerifyFailed)?;
        let sig =
            ed25519_dalek::Signature::from_slice(it.sig).map_err(|_| KmsError::VerifyFailed)?;

        pubs.push(vk);
        sigs.push(sig);
        msgs.push(it.msg);
    }

    // Fast all-or-nothing batch verify (requires ed25519-dalek "batch" feature).
    let batch_ok = ed25519_dalek::verify_batch(&msgs, &sigs, &pubs).is_ok();
    if batch_ok {
        return Ok(vec![true; items.len()]);
    }

    // Fallback: precise booleans per item.
    items
        .iter()
        .map(|it| kms.verify(it.kid, it.msg, it.sig))
        .collect()
}
