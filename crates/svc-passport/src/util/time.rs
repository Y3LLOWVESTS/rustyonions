//! RO:WHAT — Time helpers (UTC seconds).
pub fn now_s() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
