use std::time::Duration;
use svc_dht::peer::id::NodeId;
use svc_dht::provider::Store;
use svc_dht::types::B3Cid;

const NODE_A_URI: &str =
    "crab://node/00000000000000000000000000000000000000000000000000000000000000a1";

#[test]
fn nodeid_distance_xor() {
    let a = NodeId::from_pubkey(b"A");
    let b = NodeId::from_pubkey(b"B");
    let d_ab = a.distance(&b);
    let d_ba = b.distance(&a);
    assert_eq!(d_ab, d_ba, "XOR is symmetric");
    assert_eq!(a.distance(&a), [0u8; 32], "distance to self is zero");
}

#[test]
fn provider_store_ttl_expiry() {
    let ttl = Duration::from_millis(20);
    let st = Store::new(ttl);
    let cid: B3Cid =
        "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".parse().unwrap();

    st.add(cid.to_string(), NODE_A_URI.to_string(), Some(ttl))
        .expect("valid crab node provider identity");

    let got1 = st.get_live(cid.as_str());
    assert_eq!(got1, vec![NODE_A_URI.to_string()]);

    std::thread::sleep(ttl + Duration::from_millis(5));
    st.purge_expired();

    let got2 = st.get_live(cid.as_str());
    assert!(got2.is_empty(), "expired provider should be pruned");
}
