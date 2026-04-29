//! RO:WHAT — Lightweight concurrency smoke test for memory-store pointer updates.
//! RO:WHY — Replaces placeholder with a real no-panic concurrent read/write check.
//! RO:INVARIANTS — no locks are held across await; this test is synchronous and bounded.

use std::thread;

use svc_index::{store::Store, types::AssetManifestPointer};

const ASSET_CID: &str = "b3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const MANIFEST_CID_A: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MANIFEST_CID_B: &str = "b3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn pointer(manifest_cid: &str, updated_at_ms: u64) -> AssetManifestPointer {
    AssetManifestPointer {
        version: 1,
        asset_cid: ASSET_CID.to_owned(),
        asset_kind: "image".to_owned(),
        manifest_cid: manifest_cid.to_owned(),
        owner_passport_subject: None,
        owner_wallet_account: None,
        updated_at_ms,
    }
}

#[test]
fn memory_store_survives_concurrent_pointer_updates() {
    let store = Store::new(false).expect("memory store");

    let a = store.clone();
    let writer_a = thread::spawn(move || {
        for idx in 0..50 {
            a.put_asset_manifest_pointer(&pointer(MANIFEST_CID_A, 1_000 + idx))
                .expect("write pointer a");
        }
    });

    let b = store.clone();
    let writer_b = thread::spawn(move || {
        for idx in 0..50 {
            b.put_asset_manifest_pointer(&pointer(MANIFEST_CID_B, 2_000 + idx))
                .expect("write pointer b");
        }
    });

    writer_a.join().expect("writer a joins");
    writer_b.join().expect("writer b joins");

    let fetched = store
        .get_asset_manifest_pointer(ASSET_CID)
        .expect("final pointer exists");

    assert!(fetched.manifest_cid == MANIFEST_CID_A || fetched.manifest_cid == MANIFEST_CID_B);
    assert!(fetched.updated_at_ms >= 1_000);
}
