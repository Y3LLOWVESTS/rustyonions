//! RO:WHAT — Phase 23 stale-provider and provider-privacy chaos drill.
//! RO:WHY — Proves expired DHT records cannot create lookup success and
//! transport/IP-shaped provider advertisements fail closed.
//! RO:INTERACTS — provider::Store, pipeline::lookup, privacy reviewer,
//! typed B3 content IDs, and canonical crab://node identities.
//! RO:INVARIANTS — expired providers are excluded; a fresh alternate may
//! satisfy a degraded read; raw IP/transport identities never enter the store.
//! RO:SECURITY — no raw route publication, fake lookup success, reward,
//! wallet, ledger, receipt, payout, or finality authority.
//! RO:TEST — cargo test -p svc-dht --test
//! phase23_provider_discovery_chaos.

use std::{sync::Arc, time::Duration};

use serde_json::json;
use svc_dht::{
    pipeline::lookup::{LookupCtx, LookupRequest},
    privacy::review_public_json,
    provider::{ProviderStoreError, Store},
};

const CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const STALE_NODE_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

const FRESH_NODE_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000b2";

fn lookup_request() -> LookupRequest {
    LookupRequest {
        cid: CID.to_owned(),
        alpha: 3,
        beta: 1,
        hop_budget: 6,
        deadline: Duration::from_millis(300),
        hedge_stagger: Duration::from_millis(15),
        min_leg_budget: Duration::from_millis(50),
    }
}

#[tokio::test]
async fn stale_and_ip_leaking_provider_records_fail_closed() {
    // A provider record that is already expired must not appear as live or
    // create a successful lookup result.
    let stale_only = Arc::new(Store::new(Duration::from_secs(60)));

    stale_only
        .add(CID.to_owned(), STALE_NODE_URI.to_owned(), Some(Duration::ZERO))
        .expect("canonical stale provider fixture must insert");

    assert!(
        stale_only.get_live(CID).is_empty(),
        "expired provider record must not appear in live inventory",
    );

    let stale_lookup = LookupCtx::new(stale_only, 16);

    let error = stale_lookup
        .run(lookup_request())
        .await
        .expect_err("stale-only lookup must not fabricate success");

    assert!(
        error.to_string().contains("lookup failed or timed out"),
        "unexpected stale lookup error: {error}",
    );

    // Reads may degrade to a different live provider, but the expired
    // provider must still remain excluded.
    let with_alternate = Arc::new(Store::new(Duration::from_secs(60)));

    with_alternate
        .add(CID.to_owned(), STALE_NODE_URI.to_owned(), Some(Duration::ZERO))
        .expect("canonical stale provider fixture must insert");

    with_alternate
        .add(CID.to_owned(), FRESH_NODE_URI.to_owned(), Some(Duration::from_secs(60)))
        .expect("canonical fresh provider fixture must insert");

    let alternate_lookup = LookupCtx::new(with_alternate.clone(), 16);

    let result = alternate_lookup
        .run(lookup_request())
        .await
        .expect("fresh alternate must satisfy degraded lookup");

    assert_eq!(result.providers, vec![FRESH_NODE_URI.to_owned()]);
    assert!(!result.providers.iter().any(|uri| uri == STALE_NODE_URI));

    let safe_public_projection = json!({
        "cid": CID,
        "providers": result.providers,
        "lookupStatus": "degraded_to_fresh_alternate",
    });

    assert!(
        review_public_json("phase23.provider-discovery.safe", &safe_public_projection,).is_empty(),
        "canonical logical provider identities must remain display-safe",
    );

    // A provider cannot advertise a raw TCP/IP route as its identity.
    let rejected_store = Store::new(Duration::from_secs(60));

    let rejected = rejected_store
        .add(CID.to_owned(), "tcp://192.168.1.10:7000".to_owned(), Some(Duration::from_secs(60)))
        .expect_err("raw provider route must fail before insertion");

    assert!(matches!(rejected, ProviderStoreError::InvalidNode(_)));

    assert!(
        rejected_store.get_live(CID).is_empty(),
        "rejected provider identity must not mutate the store",
    );

    let leaking_projection = json!({
        "cid": CID,
        "providers": ["tcp://192.168.1.10:7000"],
        "source_ip": "192.168.1.10",
    });

    let findings = review_public_json("phase23.provider-discovery.leaking", &leaking_projection);

    assert!(!findings.is_empty(), "public privacy reviewer must flag raw route and IP leakage",);

    assert!(
        findings.iter().any(|finding| {
            finding.reason.contains("forbidden")
                || finding.reason.contains("IPv4")
                || finding.reason.contains("route")
        }),
        "privacy findings must identify the route/IP violation: {findings:?}",
    );

    println!(
        "Phase 23C passed: stale DHT records produced no fake lookup success, \
         reads degraded only to a fresh canonical alternate, raw IP/transport \
         provider identities were rejected before insertion, and public \
         privacy review detected the leak."
    );
}
