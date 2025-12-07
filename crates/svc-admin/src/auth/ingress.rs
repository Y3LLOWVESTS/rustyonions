// crates/svc-admin/src/auth/ingress.rs
//
// WHAT: "ingress" auth backend.
// WHY:  Trust an upstream reverse proxy / ingress controller to perform auth
//       and inject identity via headers.
//
// Default header contract (v1):
//   - X-User   → operator subject (e.g. email or username)
//   - X-Groups → comma-separated roles (e.g. "admin,ops")

use axum::http::HeaderMap;

use super::{AuthError, Identity};

const HDR_USER: &str = "x-user";
const HDR_GROUPS: &str = "x-groups";

/// Resolve an `Identity` from ingress-provided headers.
///
/// Behavior:
///   - Missing X-User   ⇒ "anonymous" subject.
///   - Missing X-Groups ⇒ empty roles.
///   - Malformed UTF-8  ⇒ AuthError::Invalid.
///
/// We don't treat missing headers as a hard failure yet; `/api/me` is
/// informational and the console is still read-only.
pub fn identity_from_headers(headers: &HeaderMap) -> Result<Identity, AuthError> {
    // Subject / display name
    let subject = match headers.get(HDR_USER) {
        Some(v) => v
            .to_str()
            .map_err(|_| AuthError::Invalid("X-User header is not valid UTF-8"))?
            .trim()
            .to_string(),
        None => "anonymous".to_string(),
    };

    let display_name = subject.clone();

    // Roles from X-Groups
    let roles = match headers.get(HDR_GROUPS) {
        Some(v) => {
            let raw = v
                .to_str()
                .map_err(|_| AuthError::Invalid("X-Groups header is not valid UTF-8"))?;
            raw.split(',')
                .map(|part| part.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        }
        None => Vec::new(),
    };

    Ok(Identity {
        subject,
        display_name,
        roles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn identity_from_headers_with_user_and_groups_parses_correctly() {
        let mut headers = HeaderMap::new();
        headers.insert("x-user", "stevan@example.com".parse().unwrap());
        headers.insert("x-groups", "admin, ops ,  ,dev".parse().unwrap());

        let id = identity_from_headers(&headers).expect("identity");
        assert_eq!(id.subject, "stevan@example.com");
        assert_eq!(id.display_name, "stevan@example.com");

        // Whitespace trimmed, empty entries filtered.
        assert_eq!(
            id.roles,
            vec!["admin".to_string(), "ops".to_string(), "dev".to_string()]
        );
    }

    #[test]
    fn identity_from_headers_missing_headers_falls_back_safely() {
        let headers = HeaderMap::new();

        let id = identity_from_headers(&headers).expect("identity");
        assert_eq!(id.subject, "anonymous");
        assert_eq!(id.display_name, "anonymous");
        assert!(id.roles.is_empty(), "roles should be empty with no X-Groups");
    }
}
