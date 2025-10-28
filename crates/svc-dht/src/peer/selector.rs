//! RO:WHAT — α-parallel, β-hedged selection placeholder
//! RO:WHY — Tail control; Concerns: PERF/RES
pub struct Selector {
    pub alpha: usize,
    pub beta: usize,
}
impl Selector {
    pub fn new(alpha: usize, beta: usize) -> Self {
        Self { alpha, beta }
    }
}
