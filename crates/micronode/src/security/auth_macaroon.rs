// crates/micronode/src/security/auth_macaroon.rs
//! RO:WHAT — Minimal macaroon/capability extraction helpers for HTTP requests.
//! RO:WHY  — Keep header parsing logic out of handlers and treat macaroons as
//!           opaque blobs that downstream auth services can verify.
//! RO:INTERACTS — `http` handlers can call `extract_raw_macaroon(&Request<Body>)`.
//! RO:INVARIANTS — Never panic on malformed headers and never log token contents.
//! RO:CONFIG — Uses an `Authorization` header with the `Macro <token>` scheme.
//! RO:SECURITY — This module only parses macaroons; verification happens elsewhere.
//! RO:TEST — Covered by unit tests here and by higher-level integration tests.

use http::{header, Request};

/// Scheme prefix for macaroon-style capabilities carried in `Authorization`.
///
/// Example: `Authorization: Macro aGVsbG8uLi4`.
pub const MACROON_SCHEME: &str = "macro";

/// Opaque wrapper around a raw macaroon/capability token.
#[derive(Debug, Clone)]
pub struct RawMacaroon {
    token: String,
}

impl RawMacaroon {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }

    pub fn into_inner(self) -> String {
        self.token
    }
}

/// Attempt to extract a `RawMacaroon` from an HTTP request.
///
/// We look for an `Authorization` header of the form `Macro <token>`
/// and match the scheme case-insensitively.
pub fn extract_raw_macaroon<B>(req: &Request<B>) -> Option<RawMacaroon> {
    let hdr = req.headers().get(header::AUTHORIZATION)?;
    let value = hdr.to_str().ok()?.trim();

    let mut parts = value.splitn(2, char::is_whitespace);
    let scheme = parts.next()?.trim();
    let token = parts.next().unwrap_or("").trim();

    if scheme.eq_ignore_ascii_case(MACROON_SCHEME) && !token.is_empty() {
        Some(RawMacaroon::new(token.to_owned()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{header, Request};

    #[test]
    fn extract_valid_macroon_authorization() {
        let req = Request::builder()
            .method("GET")
            .uri("/v1/kv/demo/k")
            .header(header::AUTHORIZATION, "Macro abc123")
            .body(())
            .unwrap();

        let mac = extract_raw_macaroon(&req).expect("expected macaroon");
        assert_eq!(mac.as_str(), "abc123");
    }

    #[test]
    fn extract_is_case_insensitive_on_scheme() {
        let req = Request::builder()
            .method("GET")
            .uri("/v1/kv/demo/k")
            .header(header::AUTHORIZATION, "mAcRo xyz")
            .body(())
            .unwrap();

        let mac = extract_raw_macaroon(&req).expect("expected macaroon");
        assert_eq!(mac.as_str(), "xyz");
    }

    #[test]
    fn extract_rejects_unrelated_schemes_or_missing_token() {
        let req = Request::builder()
            .method("GET")
            .uri("/v1/kv/demo/k")
            .header(header::AUTHORIZATION, "Bearer something")
            .body(())
            .unwrap();
        assert!(extract_raw_macaroon(&req).is_none());

        let req = Request::builder()
            .method("GET")
            .uri("/v1/kv/demo/k")
            .header(header::AUTHORIZATION, "Macro")
            .body(())
            .unwrap();
        assert!(extract_raw_macaroon(&req).is_none());

        let req = Request::builder().method("GET").uri("/v1/kv/demo/k").body(()).unwrap();
        assert!(extract_raw_macaroon(&req).is_none());
    }

    #[test]
    fn raw_macaroon_wrapper_roundtrips() {
        let mac = RawMacaroon::new("token-123".to_string());
        assert_eq!(mac.as_str(), "token-123");
        let owned = mac.clone().into_inner();
        assert_eq!(owned, "token-123");
    }
}
