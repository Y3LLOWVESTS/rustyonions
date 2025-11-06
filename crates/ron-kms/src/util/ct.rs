// RO:WHAT Constant-time helpers (tiny, just equality for now).
#[must_use]
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}
