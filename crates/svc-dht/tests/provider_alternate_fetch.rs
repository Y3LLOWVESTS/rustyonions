//! RO:WHAT — Alternate-provider fetch and integrity-verification tests.
//! RO:WHY — Phase 12 requires failed or corrupt providers not to end retrieval.
//! RO:INTERACTS — pipeline::fetch, provider::Store, local provider status.
//! RO:INVARIANTS — attempts bounded; corrupt bytes rejected; alternatives tried.
//! RO:SECURITY — tests use canonical crab://node identities without raw addresses.
//! RO:TEST — cargo test -p svc-dht --test provider_alternate_fetch.

use bytes::Bytes;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use svc_dht::{
    pipeline::fetch::{fetch_from_candidates, CandidateFetchError},
    provider::{ProviderStatusHint, ProviderStatusUpdateOutcome, Store},
    types::{B3Cid, CrabNodeId},
};

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_C_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000c3";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MockFetchError {
    Unreachable,
}

fn node(uri: &str) -> CrabNodeId {
    CrabNodeId::from_uri(uri).expect("test node URI must be canonical")
}

fn cid_for(bytes: &[u8]) -> B3Cid {
    format!("b3:{}", blake3::hash(bytes).to_hex())
        .parse()
        .expect("computed digest must form a canonical B3 CID")
}

fn add_provider(store: &Store, cid: &B3Cid, uri: &str) {
    store
        .add(cid.to_string(), uri.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical provider record");
}

fn observed_attempts(attempts: &Arc<Mutex<Vec<CrabNodeId>>>) -> Vec<CrabNodeId> {
    attempts.lock().expect("attempt log lock").clone()
}

#[tokio::test]
async fn fetch_tries_alternates_until_verified_bytes_arrive() {
    let expected_bytes = Bytes::from_static(b"phase-12 verified alternate bytes");
    let cid = cid_for(expected_bytes.as_ref());
    let store = Store::new(Duration::from_secs(60));

    for uri in [NODE_A_URI, NODE_B_URI, NODE_C_URI] {
        add_provider(&store, &cid, uri);
    }

    // Candidate order becomes A (responsive), B (unknown), C (degraded).
    assert_eq!(
        store.record_local_status(cid.as_str(), node(NODE_A_URI), ProviderStatusHint::Responsive,),
        ProviderStatusUpdateOutcome::Updated
    );

    assert_eq!(
        store.record_local_status(cid.as_str(), node(NODE_C_URI), ProviderStatusHint::Degraded,),
        ProviderStatusUpdateOutcome::Updated
    );

    let attempts = Arc::new(Mutex::new(Vec::new()));
    let attempt_log = attempts.clone();
    let success_bytes = expected_bytes.clone();

    let result = fetch_from_candidates(&store, &cid, 3, move |provider, _requested_cid| {
        let attempt_log = attempt_log.clone();
        let success_bytes = success_bytes.clone();

        async move {
            attempt_log.lock().expect("attempt log lock").push(provider);

            if provider == node(NODE_A_URI) {
                return Err(MockFetchError::Unreachable);
            }

            if provider == node(NODE_B_URI) {
                return Ok(Bytes::from_static(b"wrong bytes from provider"));
            }

            Ok(success_bytes)
        }
    })
    .await
    .expect("third provider should return verified bytes");

    assert_eq!(result.provider, node(NODE_C_URI));
    assert_eq!(result.bytes, expected_bytes);
    assert_eq!(result.attempts, 3);

    assert_eq!(
        observed_attempts(&attempts),
        vec![node(NODE_A_URI), node(NODE_B_URI), node(NODE_C_URI),]
    );

    // C became responsive, A became degraded, and corrupt B became
    // unavailable and is excluded.
    assert_eq!(store.select_candidates(cid.as_str(), 3), vec![node(NODE_C_URI), node(NODE_A_URI)]);
}

#[tokio::test]
async fn fetch_never_exceeds_the_requested_attempt_bound() {
    let expected_bytes = Bytes::from_static(b"bounded alternate fetch");
    let cid = cid_for(expected_bytes.as_ref());
    let store = Store::new(Duration::from_secs(60));

    for uri in [NODE_A_URI, NODE_B_URI, NODE_C_URI] {
        add_provider(&store, &cid, uri);
    }

    let attempts = Arc::new(Mutex::new(Vec::new()));
    let attempt_log = attempts.clone();

    let error = fetch_from_candidates(&store, &cid, 2, move |provider, _requested_cid| {
        let attempt_log = attempt_log.clone();

        async move {
            attempt_log.lock().expect("attempt log lock").push(provider);

            Err::<Bytes, MockFetchError>(MockFetchError::Unreachable)
        }
    })
    .await
    .expect_err("two failed attempts must exhaust the bound");

    assert_eq!(error, CandidateFetchError::Exhausted { attempts: 2 });

    assert_eq!(observed_attempts(&attempts), vec![node(NODE_A_URI), node(NODE_B_URI)]);
}

#[tokio::test]
async fn fetch_reports_invalid_bound_and_absent_candidates_truthfully() {
    let expected_bytes = Bytes::from_static(b"no candidate fetch");
    let cid = cid_for(expected_bytes.as_ref());
    let store = Store::new(Duration::from_secs(60));

    let invalid_bound = fetch_from_candidates(&store, &cid, 0, |_provider, _requested_cid| async {
        Ok::<Bytes, MockFetchError>(Bytes::new())
    })
    .await
    .expect_err("zero attempts must be rejected");

    assert_eq!(invalid_bound, CandidateFetchError::InvalidMaxAttempts);

    let no_candidates = fetch_from_candidates(&store, &cid, 3, |_provider, _requested_cid| async {
        Ok::<Bytes, MockFetchError>(Bytes::new())
    })
    .await
    .expect_err("empty store must report no candidates");

    assert_eq!(no_candidates, CandidateFetchError::NoCandidates);
}
