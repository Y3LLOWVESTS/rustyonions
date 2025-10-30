//! RO:WHAT — Machine+human readable reasons for decisions.

#[derive(Debug, Clone)]
pub struct Reason {
    pub code: &'static str,
    pub message: String,
}

impl Reason {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
