//! RO:WHAT — Bounded alternate-provider fetch orchestration with full BLAKE3 verification.
//! RO:WHY — Phase 12 requires fetches to continue past failed or corrupt providers.
//! RO:INTERACTS — provider::Store, provider status hints, types::{B3Cid, CrabNodeId}.
//! RO:INVARIANTS — status-aware ordering; bounded attempts; full CID verification.
//! RO:SECURITY — accepts typed Crab node IDs only and never exposes transport addresses.
//! RO:TEST — tests/provider_alternate_fetch.rs.

use crate::{
    provider::{ProviderStatusHint, Store},
    types::{B3Cid, CrabNodeId},
};
use bytes::Bytes;
use std::{fmt, future::Future};

/// Successful, integrity-verified provider fetch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateFetchSuccess {
    /// Provider that returned the accepted bytes.
    pub provider: CrabNodeId,

    /// Full bytes whose BLAKE3 digest matched the requested CID.
    pub bytes: Bytes,

    /// Number of providers attempted before success.
    pub attempts: usize,
}

/// Truthful failure returned by bounded alternate-provider fetching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateFetchError {
    /// Callers must permit at least one attempt.
    InvalidMaxAttempts,

    /// No live, selectable provider candidates existed.
    NoCandidates,

    /// Every selected provider failed transport or integrity review.
    Exhausted { attempts: usize },
}

impl CandidateFetchError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidMaxAttempts => "invalid_max_attempts",
            Self::NoCandidates => "no_candidates",
            Self::Exhausted { .. } => "candidate_fetch_exhausted",
        }
    }
}

impl fmt::Display for CandidateFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted { attempts } => {
                write!(f, "{} after {attempts} attempts", self.as_str())
            }
            other => f.write_str(other.as_str()),
        }
    }
}

impl std::error::Error for CandidateFetchError {}

/// Fetch bytes from status-ordered providers until one returns bytes matching
/// the requested BLAKE3 CID.
///
/// The supplied callback is the transport boundary. This function does not
/// invent a route or claim that live transport exists.
///
/// Local status observations are updated as follows:
///
/// - verified success: `Responsive`
/// - transport/callback failure: `Degraded`
/// - returned bytes with the wrong digest: `Unavailable`
pub async fn fetch_from_candidates<F, Fut, E>(
    store: &Store,
    cid: &B3Cid,
    max_attempts: usize,
    mut fetch: F,
) -> Result<CandidateFetchSuccess, CandidateFetchError>
where
    F: FnMut(CrabNodeId, B3Cid) -> Fut,
    Fut: Future<Output = Result<Bytes, E>>,
{
    if max_attempts == 0 {
        return Err(CandidateFetchError::InvalidMaxAttempts);
    }

    let candidates = store.select_candidates(cid.as_str(), max_attempts);

    if candidates.is_empty() {
        return Err(CandidateFetchError::NoCandidates);
    }

    let mut attempts = 0usize;

    for provider in candidates {
        attempts += 1;

        match fetch(provider, cid.clone()).await {
            Ok(bytes) if bytes_match_cid(cid, &bytes) => {
                let _ = store.record_local_status(
                    cid.as_str(),
                    provider,
                    ProviderStatusHint::Responsive,
                );

                return Ok(CandidateFetchSuccess { provider, bytes, attempts });
            }
            Ok(_) => {
                // Returning corrupt or incorrectly addressed bytes is unsafe.
                // Exclude this provider until a later local observation clears it.
                let _ = store.record_local_status(
                    cid.as_str(),
                    provider,
                    ProviderStatusHint::Unavailable,
                );
            }
            Err(_) => {
                // A failed attempt is not proof of permanent unavailability.
                // Downrank the provider while retaining it as a last resort.
                let _ =
                    store.record_local_status(cid.as_str(), provider, ProviderStatusHint::Degraded);
            }
        }
    }

    Err(CandidateFetchError::Exhausted { attempts })
}

fn bytes_match_cid(cid: &B3Cid, bytes: &Bytes) -> bool {
    let expected = cid.as_str().strip_prefix("b3:").expect("B3Cid guarantees the canonical prefix");

    let actual = blake3::hash(bytes.as_ref()).to_hex();

    actual.as_str() == expected
}
