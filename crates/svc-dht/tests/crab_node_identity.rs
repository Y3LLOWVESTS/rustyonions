use std::time::Duration;

use svc_dht::provider::{ProviderStoreError, Store};
use svc_dht::types::{validate_node_uri, CrabNodeId, CrabNodeIdError};

const NODE_A_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000a1";
const NODE_B_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000b2";
const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

#[test]
fn crab_node_uri_round_trips() {
    let node = CrabNodeId::from_uri(NODE_A_URI).expect("valid crab node URI");

    assert_eq!(node.to_node_hex(), NODE_A_HEX);
    assert_eq!(node.to_uri(), NODE_A_URI);
    assert_eq!(node.to_string(), NODE_A_URI);
    assert_eq!(node, NODE_A_URI.parse::<CrabNodeId>().unwrap());
}

#[test]
fn only_crab_node_uris_are_valid_provider_identities() {
    assert!(validate_node_uri(NODE_A_URI));

    for bad in [
        "",
        "not a uri",
        "local://nodeA",
        "relay://nodeA",
        "onion://nodeA",
        "service://nodeA",
        "tcp://192.168.1.7:7000",
        "http://10.0.0.2:8080",
        "crab://service/00000000000000000000000000000000000000000000000000000000000000a1",
        "crab://node/00000000000000000000000000000000000000000000000000000000000000A1",
        "crab://node/abc",
        " crab://node/00000000000000000000000000000000000000000000000000000000000000a1",
    ] {
        assert!(!validate_node_uri(bad), "unexpectedly accepted {bad}");
    }
}

#[test]
fn crab_node_error_reasons_are_stable() {
    assert_eq!(CrabNodeId::from_uri("").unwrap_err(), CrabNodeIdError::Empty);
    assert_eq!(CrabNodeId::from_uri("crab://node/abc").unwrap_err(), CrabNodeIdError::BadLength);
    assert_eq!(
        CrabNodeId::from_uri("local://nodeA").unwrap_err(),
        CrabNodeIdError::MissingCrabNodePrefix
    );
    assert_eq!(
        CrabNodeId::from_uri(
            "crab://node/00000000000000000000000000000000000000000000000000000000000000ZZ"
        )
        .unwrap_err(),
        CrabNodeIdError::BadHex
    );
}

#[test]
fn provider_store_rejects_transport_specific_provider_routes() {
    let store = Store::new(Duration::from_secs(60));
    let cid = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    for bad in [
        "local://nodeA",
        "relay://nodeA",
        "onion://nodeA",
        "service://nodeA",
        "tcp://192.168.1.7:7000",
        "http://10.0.0.2:8080",
    ] {
        let err = store
            .add(cid.to_string(), bad.to_string(), Some(Duration::from_secs(60)))
            .expect_err("store must reject non-crab provider route");
        assert_eq!(err, ProviderStoreError::InvalidNode(CrabNodeIdError::MissingCrabNodePrefix));
    }

    let node_b = format!("crab://node/{NODE_B_HEX}");
    store
        .add(cid.to_string(), node_b.clone(), Some(Duration::from_secs(60)))
        .expect("valid crab node provider identity accepted");

    assert_eq!(store.get_live(cid), vec![node_b]);
}
