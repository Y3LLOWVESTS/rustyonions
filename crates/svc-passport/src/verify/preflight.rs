//! RO:WHAT — Cheap checks (iat/nbf/exp/aud size) before crypto.
//! RO:INVARIANTS — bounded skew; msg bytes parsed once.

use crate::{dto::verify::Envelope, error::Error, token::encode::decode_envelope, Config};
use serde_json::Value;
use time::OffsetDateTime;

pub fn time_window(cfg: &Config, env: &Envelope) -> Result<(), Error> {
    let (msg, _sig) = decode_envelope(env).map_err(|_| Error::Malformed)?;
    let v: Value = serde_json::from_slice(&msg).map_err(|_| Error::Malformed)?;
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let skew = cfg.passport.clock_skew_s;

    let iat = v
        .get("iat")
        .and_then(|x| x.as_i64())
        .ok_or(Error::Malformed)?;
    let exp = v
        .get("exp")
        .and_then(|x| x.as_i64())
        .ok_or(Error::Malformed)?;
    let nbf = v.get("nbf").and_then(|x| x.as_i64()).unwrap_or(iat);

    if now + skew < nbf {
        return Err(Error::NotBefore);
    }
    if now - skew > exp {
        return Err(Error::Expired);
    }
    Ok(())
}
