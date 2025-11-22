//! RO:WHAT — App-plane contract (Hello World).
//!
//! Minimal, boring types that define how a RON app exposes routes to a
//! host (micronode / macronode + gateway / omnigate). This module is
//! deliberately small and synchronous so it maps cleanly into other
//! SDKs (TS, Go, Python) and into different HTTP stacks.
//!
//! RO:WHY  —
//!   - Give apps a single, unified interface (`RonApp`) to implement.
//!   - Make it trivial for hosts to mount apps and dispatch requests.
//!   - Keep the contract DTO-only; no servers or runtime here.
//!
//! RO:INVARIANTS —
//!   - Handlers are **synchronous** (Option A) and total over `AppRequest`.
//!   - Types are `Clone + Debug` where reasonable for DX.
//!   - No direct dependency on Axum/reqwest/http; hosts perform mapping.
//!
//! This is App Plane **Alpha**. We expect to:
//!   - Add richer auth (JWT/macaroon/passport) later,
//!   - Add per-app metrics + tracing hooks,
//!   - Potentially hoist stable DTOs into `ron-proto` once the shape
//!     is locked across SDKs.

use std::collections::BTreeMap;

/// Minimal HTTP-ish method set for app routes.
///
/// We keep this small and SDK-local so it can be mirrored easily in
/// other languages and mapped into concrete HTTP stacks by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// Minimal representation of caller identity.
///
/// During App Plane Alpha this is intentionally underspecified:
/// hosts may stuff a raw bearer token, macaroon, or opaque session ID
/// into `token`. As we lock the auth story, this will grow a more
/// structured shape (e.g. parsed claims, capability scopes).
#[derive(Debug, Clone, Default)]
pub struct AppAuth {
    /// Opaque token conveyed by the host (if any).
    pub token: Option<String>,
}

/// Simple header map representation.
///
/// We use a sorted map for deterministic iteration and ease of
/// cross-language mapping. Hosts can normalize case as needed.
#[derive(Debug, Clone, Default)]
pub struct AppHeaders {
    /// Case-insensitive header names are expected; hosts should
    /// normalize to lowercase when populating this map.
    pub inner: BTreeMap<String, String>,
}

impl AppHeaders {
    /// Create empty headers.
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    /// Insert or replace a header.
    pub fn insert<K, V>(&mut self, name: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.inner.insert(name.into(), value.into());
    }

    /// Get a header value by (case-normalized) name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// True if there are no headers.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of headers.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Iterate over (name, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Minimal request object passed into app handlers.
///
/// This is what gateway/omnigate will construct from an inbound
/// HTTP/OAP request before dispatching into `RonApp` handlers.
#[derive(Debug, Clone)]
pub struct AppRequest {
    /// Path after the `/app` prefix, e.g. `/hello` or `/kv/get`.
    pub path: String,
    /// Logical method (GET/POST/etc.).
    pub method: AppMethod,
    /// Raw request body.
    pub body: Vec<u8>,
    /// Caller identity, if any.
    pub auth: AppAuth,
    /// Request headers as a simple map.
    pub headers: AppHeaders,
}

/// Minimal response type returned by handlers.
///
/// Hosts are responsible for mapping `status` + headers + body back
/// into their HTTP/OAP representation.
#[derive(Debug, Clone)]
pub struct AppResponse {
    /// HTTP-like status code (e.g. 200, 404, 500).
    pub status: u16,
    /// Raw response body.
    pub body: Vec<u8>,
    /// Response headers.
    pub headers: AppHeaders,
}

impl AppResponse {
    /// Convenience constructor for a text response (UTF-8).
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into().into_bytes(),
            headers: AppHeaders::new(),
        }
    }

    /// Convenience constructor for a successful (200) text response.
    pub fn ok(body: impl Into<String>) -> Self {
        Self::text(200, body)
    }
}

/// Canonical error type for app handlers.
///
/// This is intentionally small and “stringly typed” for now so SDKs in
/// other languages can match on `code` and propagate `message`.
#[derive(Debug, Clone)]
pub struct AppError {
    /// Stable, machine-readable error code, e.g. `"not_found"`,
    /// `"bad_request"`, `"internal"`.
    pub code: &'static str,
    /// Human-readable message suitable for logs or client display.
    pub message: String,
}

impl AppError {
    /// Create a new error with a static code and message.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// A generic "internal" error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    /// A generic "not_found" error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    /// A generic "bad_request" error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("bad_request", message)
    }
}

/// Synchronous handler function type (Option A).
///
/// Handlers run in the host's thread of execution; the host is free to
/// execute them in a worker pool, `spawn_blocking`, etc. but from the
/// app's perspective this is a plain function.
///
/// We deliberately avoid `async` here for:
///   - simpler cross-language mirroring,
///   - easier testing,
///   - no direct dependency on any async runtime in the contract.
pub type AppHandler = fn(AppRequest) -> Result<AppResponse, AppError>;

/// A single route exposed by an app.
///
/// `path` is expected to be a leading-slash path relative to the app
/// mount point (`/app`), e.g. `/hello`, `/echo`, `/kv/get`.
#[derive(Clone)]
pub struct AppRoute {
    pub method: AppMethod,
    pub path: &'static str,
    pub handler: AppHandler,
}

/// Helper to construct a route in a concise way.
///
/// ```rust
/// use ron_app_sdk::app::{route, AppMethod, AppRequest, AppResponse, AppError};
///
/// fn hello(req: AppRequest) -> Result<AppResponse, AppError> {
///     let _ = req; // not used in this trivial example
///     Ok(AppResponse::ok("hello from RON"))
/// }
///
/// let r = route(AppMethod::Get, "/hello", hello);
/// assert_eq!(r.path, "/hello");
/// ```
pub fn route(method: AppMethod, path: &'static str, handler: AppHandler) -> AppRoute {
    AppRoute {
        method,
        path,
        handler,
    }
}

/// Trait implemented by RON apps.
///
/// Hosts (micronode/macronode + gateway/omnigate) will take a `RonApp`
/// implementation, call `routes()`, and then wire those `AppRoute`s
/// into their HTTP/OAP router.
pub trait RonApp: Send + Sync + 'static {
    /// Return the full set of routes exposed by this app.
    fn routes(&self) -> Vec<AppRoute>;
}

/// Minimal helper to “mount” an app's routes into an existing route set.
///
/// This does **not** know about any specific HTTP stack; it simply
/// concatenates route vectors. Hosts can then map the combined route
/// list into Axum, Hyper, or any other router.
///
/// Typical usage in a host crate:
///
/// ```ignore
/// use ron_app_sdk::app::{mount_app, RonApp, AppRoute};
///
/// fn build_routes(apps: &[Box<dyn RonApp>]) -> Vec<AppRoute> {
///     let mut routes = Vec::new();
///     for app in apps {
///         routes = mount_app(routes, &**app);
///     }
///     routes
/// }
/// ```
pub fn mount_app(mut routes: Vec<AppRoute>, app: &impl RonApp) -> Vec<AppRoute> {
    routes.extend(app.routes().into_iter());
    routes
}
