use std::sync::{Arc, Mutex};
use std::thread;

use ron_proto::{ContentId, NodeRewardRecipientStateV1};
use svc_registry::rewards::RewardBindingRegistry;

fn cid(byte: char) -> ContentId {
    let raw = format!("b3:{}", byte.to_string().repeat(64));
    ContentId::parse(&raw).expect("test cid should parse")
}

#[test]
fn reward_binding_registry_can_be_shared_for_read_status_checks() {
    let registry = Arc::new(Mutex::new(RewardBindingRegistry::new(cid('c'), cid('d'))));

    let reader = {
        let registry = Arc::clone(&registry);
        thread::spawn(move || {
            let status = registry
                .lock()
                .expect("registry lock should not be poisoned")
                .status("node_unbound", 1, 1);

            status
                .validate()
                .expect("unbound status should remain valid under shared access");

            status.state
        })
    };

    let state = reader.join().expect("reader thread should finish");
    assert_eq!(state, NodeRewardRecipientStateV1::Unbound);
}
