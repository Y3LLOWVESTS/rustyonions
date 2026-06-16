//! RO:WHAT — Inventory gate for every checked-in QuickChain vector file.
//! RO:WHY — ECON/GOV: vector files must be classified before any root-producing code is allowed.
//! RO:INTERACTS — tests/vectors/quickchain, QuickChainTestVectorV1, BLAKE3 locked-hash expectations.
//! RO:INVARIANTS — every vector is classified; locked bytes carry no hash; locked hashes are genuine; no roots are produced.
//! RO:METRICS — none.
//! RO:CONFIG — none.
//! RO:SECURITY — expected_b3 is test-vector evidence only and grants no wallet, receipt, settlement, or authority.
//! RO:TEST — this file plus tests/tools/verify_quickchain_vector_inventory.py.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use ron_proto::{
    QuickChainTestVectorV1, QuickChainVectorStatusV1, QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1, QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1,
    QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1, QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
    QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
    QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1, QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1, QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
    QUICKCHAIN_TEST_VECTOR_SCHEMA,
};
use serde_json::Value;

const EXPECTED_VECTOR_FILES: usize = 36;
const EXPECTED_LOCKED_BYTES: usize = 31;
const EXPECTED_LOCKED_HASH: usize = 5;
const EXPECTED_SKETCH: usize = 0;
const EXPECTED_TEST_VECTOR_DTO_FILES: usize = 31;

const REVIEWED_SPECIAL_VECTOR_SCHEMAS: &[&str] = &[
    "quickchain.hold-compaction-vector.v1",
    "quickchain.concurrent-hold-replay-vector.v1",
    "quickchain.receipt-sort-key-vector-set.v1",
    "quickchain.replay-scenario-vector-set.v1",
    "quickchain.sort-key-vector-set.v1",
];

#[test]
fn every_checked_in_quickchain_vector_is_classified_and_audited() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/quickchain");
    let mut files = Vec::new();
    collect_json_files(&root, &mut files);
    files.sort();

    assert_eq!(
        files.len(),
        EXPECTED_VECTOR_FILES,
        "QuickChain vector corpus changed; update this inventory gate after review"
    );

    let allowed_domains = allowed_domains();
    let mut vector_ids = BTreeSet::<String>::new();
    let mut status_counts = BTreeMap::<String, usize>::new();
    let mut schema_counts = BTreeMap::<String, usize>::new();
    let mut canonical_payload_bytes = 0_usize;
    let mut locked_hash_preimage_bytes = 0_usize;

    for path in files {
        let rel = relative_key(&root, &path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {rel}: {error}"));
        let value: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("{rel} must be valid JSON: {error}"));

        let schema = required_string(&value, "schema", &rel);
        let version = required_u64(&value, "version", &rel);
        let status = required_string(&value, "status", &rel);

        assert_eq!(version, 1, "{rel}: vector version must remain 1");
        assert!(
            schema.starts_with("quickchain.") && schema.ends_with(".v1"),
            "{rel}: schema must be a versioned quickchain v1 schema"
        );

        *status_counts.entry(status.to_string()).or_default() += 1;
        *schema_counts.entry(schema.to_string()).or_default() += 1;

        assert_filename_matches_status(&rel, status);
        assert_notes_are_present(&value, &rel);

        if schema == QUICKCHAIN_TEST_VECTOR_SCHEMA {
            let vector: QuickChainTestVectorV1 = serde_json::from_value(value.clone())
                .unwrap_or_else(|error| panic!("{rel}: typed vector parse failed: {error}"));
            vector
                .validate()
                .unwrap_or_else(|error| panic!("{rel}: typed vector validation failed: {error}"));

            assert!(
                vector_ids.insert(vector.vector_id.clone()),
                "{rel}: duplicate vector_id {}",
                vector.vector_id
            );
            assert!(
                allowed_domains.contains(vector.domain_separator.as_str()),
                "{rel}: unreviewed domain separator {}",
                vector.domain_separator
            );

            canonical_payload_bytes += audit_typed_vector_payload_and_hash(&rel, &vector);
        } else {
            assert!(
                REVIEWED_SPECIAL_VECTOR_SCHEMAS.contains(&schema),
                "{rel}: special vector schema is not in the reviewed allowlist: {schema}"
            );
            audit_special_vector_payloads(&rel, &value, status, &mut canonical_payload_bytes);
        }

        if status == "locked_hash" {
            locked_hash_preimage_bytes += required_string(&value, "preimage_hex", &rel).len() / 2;
        }
    }

    assert_eq!(
        status_counts.get("sketch").copied().unwrap_or_default(),
        EXPECTED_SKETCH
    );
    assert_eq!(
        status_counts
            .get("locked_bytes")
            .copied()
            .unwrap_or_default(),
        EXPECTED_LOCKED_BYTES
    );
    assert_eq!(
        status_counts
            .get("locked_hash")
            .copied()
            .unwrap_or_default(),
        EXPECTED_LOCKED_HASH
    );
    assert_eq!(
        schema_counts
            .get(QUICKCHAIN_TEST_VECTOR_SCHEMA)
            .copied()
            .unwrap_or_default(),
        EXPECTED_TEST_VECTOR_DTO_FILES
    );
    assert_eq!(
        vector_ids.len(),
        EXPECTED_TEST_VECTOR_DTO_FILES,
        "every typed QuickChain test vector must own one unique vector_id"
    );

    assert!(
        canonical_payload_bytes > 10_000,
        "vector corpus should carry substantial locked canonical bytes"
    );
    assert!(
        locked_hash_preimage_bytes > 1_000,
        "locked_hash vectors should carry reviewed preimage bytes"
    );
}

fn audit_typed_vector_payload_and_hash(rel: &str, vector: &QuickChainTestVectorV1) -> usize {
    match vector.status {
        QuickChainVectorStatusV1::Sketch => {
            assert!(
                vector.canonical_payload_utf8.is_none()
                    && vector.canonical_payload_hex.is_none()
                    && vector.preimage_hex.is_none()
                    && vector.expected_b3.is_none(),
                "{rel}: sketch vectors must not contain bytes, preimages, or expected hashes"
            );
            0
        }
        QuickChainVectorStatusV1::LockedBytes => {
            let payload = vector
                .canonical_payload_utf8
                .as_ref()
                .unwrap_or_else(|| panic!("{rel}: locked_bytes missing canonical_payload_utf8"));
            assert!(
                vector.preimage_hex.is_none() && vector.expected_b3.is_none(),
                "{rel}: locked_bytes vectors must not carry preimage_hex or expected_b3"
            );
            assert_minified_json_payload(rel, payload);
            payload.len()
        }
        QuickChainVectorStatusV1::LockedHash => {
            let payload = vector
                .canonical_payload_utf8
                .as_ref()
                .unwrap_or_else(|| panic!("{rel}: locked_hash missing canonical_payload_utf8"));
            let preimage_hex = vector
                .preimage_hex
                .as_ref()
                .unwrap_or_else(|| panic!("{rel}: locked_hash missing preimage_hex"));
            let expected_b3 = vector
                .expected_b3
                .as_ref()
                .unwrap_or_else(|| panic!("{rel}: locked_hash missing expected_b3"));

            assert_minified_json_payload(rel, payload);
            assert_not_placeholder_b3(rel, expected_b3.as_str());

            let preimage_bytes = hex_to_bytes(preimage_hex)
                .unwrap_or_else(|error| panic!("{rel}: invalid preimage_hex: {error}"));
            let expected_preimage =
                framed_preimage_bytes(&vector.domain_separator, payload.as_bytes());
            assert_eq!(
                preimage_bytes, expected_preimage,
                "{rel}: preimage must equal domain_separator_bytes || 0x00 || canonical_payload_bytes"
            );

            let actual_b3 = format!("b3:{}", blake3::hash(&preimage_bytes).to_hex());
            assert_eq!(
                actual_b3,
                expected_b3.as_str(),
                "{rel}: expected_b3 must be genuine BLAKE3-256 over the reviewed preimage"
            );

            payload.len()
        }
        _ => panic!("{rel}: unreviewed future vector status"),
    }
}

fn audit_special_vector_payloads(
    rel: &str,
    value: &Value,
    status: &str,
    canonical_payload_bytes: &mut usize,
) {
    assert_eq!(
        status, "locked_bytes",
        "{rel}: special vector sets must remain locked_bytes until separately reviewed"
    );

    let payload = optional_string(value, "canonical_payload_utf8", rel);
    let payload_hex = optional_string(value, "canonical_payload_hex", rel);
    let preimage_hex = optional_string(value, "preimage_hex", rel);
    let expected_b3 = optional_string(value, "expected_b3", rel);

    assert!(
        preimage_hex.is_none() && expected_b3.is_none(),
        "{rel}: special locked_bytes vector sets must not claim preimage_hex or expected_b3"
    );

    match (payload, payload_hex) {
        (Some(payload), Some(payload_hex)) => {
            assert_minified_json_payload(rel, payload);
            assert_eq!(
                lower_hex(payload.as_bytes()),
                payload_hex,
                "{rel}: canonical_payload_hex must match canonical_payload_utf8 bytes"
            );
            *canonical_payload_bytes += payload.len();
        }
        (None, None) => {
            assert!(
                rel == "receipt_order/receipt_sort_keys_locked_bytes_v1.json"
                    || rel == "replay/replay_idempotency_locked_bytes_v1.json"
                    || rel == "sort_keys/sort_keys_locked_bytes_v1.json",
                "{rel}: only reviewed vector-set files may omit canonical_payload_utf8"
            );
        }
        _ => {
            panic!("{rel}: canonical_payload_utf8 and canonical_payload_hex must appear as a pair")
        }
    }
}

fn assert_minified_json_payload(rel: &str, payload: &str) {
    assert!(
        !payload.is_empty(),
        "{rel}: canonical payload must be nonempty"
    );
    assert!(
        payload.starts_with('{') || payload.starts_with('['),
        "{rel}: canonical payload should be JSON object or array bytes"
    );
    assert!(
        !payload.contains('\n') && !payload.contains('\r') && !payload.contains('\t'),
        "{rel}: canonical payload must be minified single-line JSON"
    );
    assert!(
        !payload.contains(": ") && !payload.contains(", "),
        "{rel}: canonical payload must not contain pretty JSON spacing"
    );
}

fn assert_filename_matches_status(rel: &str, status: &str) {
    let expected_suffix = match status {
        "sketch" => "_sketch_v1.json",
        "locked_bytes" => "_locked_bytes_v1.json",
        "locked_hash" => "_locked_hash_v1.json",
        other => panic!("{rel}: unreviewed vector status {other}"),
    };

    assert!(
        rel.ends_with(expected_suffix),
        "{rel}: filename must end with {expected_suffix} for status={status}"
    );
}

fn assert_notes_are_present(value: &Value, rel: &str) {
    let notes = value
        .get("notes")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{rel}: notes must be an array"));

    assert!(!notes.is_empty(), "{rel}: notes must not be empty");

    for note in notes {
        let note = note
            .as_str()
            .unwrap_or_else(|| panic!("{rel}: every note must be a string"));
        assert!(
            !note.is_empty(),
            "{rel}: notes must not contain empty strings"
        );
    }
}

fn assert_not_placeholder_b3(rel: &str, cid: &str) {
    assert!(
        cid.starts_with("b3:") && cid.len() == 67,
        "{rel}: expected_b3 must be b3:<64 lowercase hex>"
    );

    let hex = &cid[3..];
    assert!(
        hex.bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "{rel}: expected_b3 must use lowercase hex"
    );

    for repeated in [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ] {
        assert_ne!(
            hex,
            repeated.to_string().repeat(64),
            "{rel}: expected_b3 must not be a repeated-nibble placeholder"
        );
    }
}

fn allowed_domains() -> BTreeSet<&'static str> {
    [
        QUICKCHAIN_RECEIPT_HASH_DOMAIN_V1,
        QUICKCHAIN_OPERATION_INTENT_HASH_DOMAIN_V1,
        QUICKCHAIN_ACCOUNT_LEAF_HASH_DOMAIN_V1,
        QUICKCHAIN_HOLD_LEAF_HASH_DOMAIN_V1,
        QUICKCHAIN_HOLD_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_RECEIPT_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_STATE_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_ACCOUNTING_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_REWARD_ROOT_HASH_DOMAIN_V1,
        QUICKCHAIN_CHECKPOINT_HASH_DOMAIN_V1,
        QUICKCHAIN_CHAIN_PARAMS_HASH_DOMAIN_V1,
    ]
    .into_iter()
    .collect()
}

fn collect_json_files(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read directory entry: {error}"))
                .path()
        })
        .collect();

    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

fn relative_key(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn required_string<'a>(value: &'a Value, field: &'static str, rel: &str) -> &'a str {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{rel}: {field} must be a string"))
}

fn optional_string<'a>(value: &'a Value, field: &'static str, rel: &str) -> Option<&'a str> {
    match value.get(field) {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.as_str()),
        Some(_) => panic!("{rel}: {field} must be a string or null when present"),
    }
}

fn required_u64(value: &Value, field: &'static str, rel: &str) -> u64 {
    value
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("{rel}: {field} must be an unsigned integer"))
}

fn framed_preimage_bytes(domain_separator: &str, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(domain_separator.len() + 1 + payload.len());
    bytes.extend_from_slice(domain_separator.as_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(payload);
    bytes
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("hex length must be even".to_string());
    }

    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();

    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        out.push((high << 4) | low);
    }

    Ok(out)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Err("uppercase hex is not canonical".to_string()),
        other => Err(format!("invalid hex byte 0x{other:02x}")),
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
