//! RO:WHAT  BLAKE3 keyed MAC chaining with domain separation (low-alloc path).
//! RO:WHY   Integrity over (tenant,kid,scope,caveats...) in strict order.
//! RO:INVARIANTS No SHA; constant-time compare; buffer reuse to cut allocs.

use crate::{
    cbor,
    types::{Capability, Caveat, MacKey, Scope},
};

const DOMAIN_SEP: &[u8] = b"RON-AUTHv1\0";

#[inline]
fn init_tag(key: &MacKey, tid: &str, kid: &str, scope: &Scope, buf: &mut Vec<u8>) -> [u8; 32] {
    // tag0 = BLAKE3(key, DOMAIN || CBOR{tid,kid,scope})
    buf.clear();
    buf.extend_from_slice(DOMAIN_SEP);
    cbor::cbor_fragment_into(&(tid, kid, scope), buf);
    *blake3::keyed_hash(&key.0, buf).as_bytes()
}

#[inline]
fn fold_caveats(
    key: &MacKey,
    mut tag: [u8; 32],
    caveats: &[Caveat],
    frag: &mut Vec<u8>,
    fold: &mut Vec<u8>,
) -> [u8; 32] {
    for c in caveats {
        // Serialize caveat into frag (reused)
        cbor::cbor_fragment_into(c, frag);

        // fold = tag || frag
        fold.clear();
        fold.extend_from_slice(&tag);
        fold.extend_from_slice(frag);

        tag = *blake3::keyed_hash(&key.0, fold).as_bytes();
    }
    tag
}

/// Compute MAC with minimal transient allocations by reusing small buffers.
#[inline]
pub fn compute_mac(key: &MacKey, cap: &Capability) -> [u8; 32] {
    let mut init_buf = Vec::with_capacity(128);
    let tag0 = init_tag(key, &cap.tid, &cap.kid, &cap.scope, &mut init_buf);

    let mut frag = Vec::with_capacity(128);
    let mut fold = Vec::with_capacity(160); // 32 + typical frag
    fold_caveats(key, tag0, &cap.caveats, &mut frag, &mut fold)
}

/// Constant-time MAC comparison (works for equal-length slices).
/// Runs in time proportional to length; no early-exit and no branching on secrets.
#[inline]
pub fn macs_equal(ct_a: &[u8], ct_b: &[u8]) -> bool {
    if ct_a.len() != ct_b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    // Iterate over all bytes; XOR accumulates any difference.
    for i in 0..ct_a.len() {
        // SAFETY: bounds-checked by loop condition
        diff |= ct_a[i] ^ ct_b[i];
    }
    // `diff == 0` after the loop implies equality; single comparison independent of contents.
    diff == 0
}
