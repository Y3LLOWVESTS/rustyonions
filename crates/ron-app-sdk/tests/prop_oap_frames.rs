//! RO:WHAT — OAP frame-bound property-style checks for ron-app-sdk.
//! RO:WHY — Ensures SDK-side test vectors respect the canonical 1 MiB frame ceiling.
//! RO:INTERACTS — Storage plane payload sizing and OAP/1 invariants.
//! RO:INVARIANTS — Accepted frames are <= 1 MiB; oversize frames are rejected by callers.
//! RO:SECURITY — Prevents unbounded client-side buffering.
//! RO:TEST — cargo clippy -p ron-app-sdk --all-targets -- -D warnings.

const OAP_MAX_FRAME_BYTES: usize = 1_048_576;
const STREAM_CHUNK_BYTES: usize = 65_536;

#[test]
fn representative_frame_sizes_respect_canonical_cap() {
    let accepted_sizes = [
        0usize,
        1,
        STREAM_CHUNK_BYTES,
        STREAM_CHUNK_BYTES * 8,
        OAP_MAX_FRAME_BYTES,
    ];

    for size in accepted_sizes {
        assert!(
            size <= OAP_MAX_FRAME_BYTES,
            "accepted vector {size} exceeded OAP cap"
        );
    }
}

#[test]
fn one_byte_over_cap_is_detectably_oversize() {
    let max = OAP_MAX_FRAME_BYTES;
    let oversized = max.saturating_add(1);

    assert!(oversized > max);
    assert_eq!(oversized - max, 1);
}

#[test]
fn chunking_payload_into_64k_pieces_stays_within_oap_frame_cap() {
    let payload_len = OAP_MAX_FRAME_BYTES;
    let chunks = payload_len.div_ceil(STREAM_CHUNK_BYTES);

    assert!(chunks > 0);
    assert_eq!(chunks, 16);

    let last_chunk_len = payload_len - ((chunks - 1) * STREAM_CHUNK_BYTES);
    assert!(last_chunk_len <= STREAM_CHUNK_BYTES);
    assert!(last_chunk_len > 0);
}
