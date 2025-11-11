//! Byte utilities (stub).

/// Clamp `n` to `max`.
pub fn clamp_len(n: usize, max: usize) -> usize {
    if n > max { max } else { n }
}
