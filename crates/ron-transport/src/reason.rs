//! RO:WHAT — Canonical reject reasons (metrics label-safe).
//! RO:WHY  — Consistent observability & tests.

#[derive(Debug, Clone, Copy)]
pub enum RejectReason {
    OverCapacity,
    BadFrame,
    TooLarge,
    Timeout,
    Io,
    Tls,
}

impl RejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OverCapacity => "over_capacity",
            Self::BadFrame => "bad_frame",
            Self::TooLarge => "too_large",
            Self::Timeout => "timeout",
            Self::Io => "io",
            Self::Tls => "tls",
        }
    }
}
