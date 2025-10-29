//! Amnesia toggle (stub used by scripts & future policy flow).

#[allow(dead_code)]
pub struct Amnesia(pub bool);

#[allow(dead_code)]
impl Amnesia {
    pub fn is_on(&self) -> bool {
        self.0
    }
}
