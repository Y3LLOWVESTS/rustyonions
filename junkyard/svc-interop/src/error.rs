// Central error taxonomy (placeholder)
#[derive(Debug)]
pub enum WireError {
    BadOrigin,
    Unauth,
    BodyLimit,
    DecompLimit,
    RateLimit,
    Backpressure,
    DownstreamUnavailable,
    PolicyBlocked,
    Malformed,
    ClockSkew,
}
