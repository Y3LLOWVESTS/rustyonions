use time::OffsetDateTime;

#[must_use]
pub fn now_utc_ms() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
}
