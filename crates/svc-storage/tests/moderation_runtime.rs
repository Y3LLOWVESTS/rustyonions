//! Focused bounded moderation-policy loader tests.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use ron_policy::{B3Id, ModerationReasonCode};
use svc_storage::moderation_runtime::{
    load_moderation_policy_file, ModerationPolicyLoadError, MAX_MODERATION_POLICY_BYTES,
};

const BLOCKED_CID: &str = "b3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct TempPolicyFile {
    path: PathBuf,
}

impl TempPolicyFile {
    fn write(label: &str, bytes: &[u8]) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time must be after epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "svc-storage-moderation-{label}-{}-{nonce}.json",
            std::process::id(),
        ));

        fs::write(&path, bytes).expect("temporary moderation policy should write");

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPolicyFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[test]
fn bounded_loader_accepts_policy_and_reports_counts() {
    let json = format!(
        r#"{{
            "local_block": ["{BLOCKED_CID}"]
        }}"#
    );

    let file = TempPolicyFile::write("valid", json.as_bytes());

    let loaded =
        load_moderation_policy_file(file.path()).expect("valid moderation policy should load");

    assert_eq!(loaded.counts.global_deny, 0);
    assert_eq!(loaded.counts.local_block, 1);
    assert_eq!(loaded.counts.local_allow, 0);
    assert_eq!(loaded.counts.owner_tombstone, 0);
    assert_eq!(loaded.counts.quarantine, 0);
    assert_eq!(loaded.counts.total(), 1);

    let object: B3Id = BLOCKED_CID.parse().expect("test b3 identifier must parse");

    let decision = loaded.policy.evaluate(&object);

    assert!(!decision.permits_serve());
    assert_eq!(decision.reason, ModerationReasonCode::LocalBlock);
}

#[test]
fn bounded_loader_rejects_unknown_fields() {
    let file = TempPolicyFile::write("unknown-field", br#"{"unexpected": true}"#);

    assert!(matches!(
        load_moderation_policy_file(file.path()),
        Err(ModerationPolicyLoadError::Json(_))
    ));
}

#[test]
fn bounded_loader_rejects_empty_file() {
    let file = TempPolicyFile::write("empty", b" \n\t ");

    assert!(matches!(
        load_moderation_policy_file(file.path()),
        Err(ModerationPolicyLoadError::Empty)
    ));
}

#[test]
fn bounded_loader_rejects_oversized_file() {
    let bytes = vec![b' '; MAX_MODERATION_POLICY_BYTES + 1];

    let file = TempPolicyFile::write("oversized", &bytes);

    assert!(matches!(
        load_moderation_policy_file(file.path()),
        Err(ModerationPolicyLoadError::TooLarge { .. })
    ));
}
