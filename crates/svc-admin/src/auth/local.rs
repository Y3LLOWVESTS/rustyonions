//! RO:WHAT — Local username/password auth for svc-admin with server-side sessions + RBAC store.
//! RO:WHY  — Pillar 3 (Auth), local dev + single-node operator installs.
//! RO:SECURITY — Argon2id PHC hashes, constant-time verify path, cookie sessions (HttpOnly).
//! RO:INVARIANTS —
//!   - No blocking IO in request path (RBAC JSON IO only during init/bootstrap).
//!   - Session cookie is the only auth token in local mode (server-side sessions).
//!   - RBAC denies by default; permissions derived from roles.

#![forbid(unsafe_code)]

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{FromRef, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use parking_lot::Mutex;
use rand::{distr::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

/// Local auth configuration.
#[derive(Debug, Clone)]
pub struct LocalAuthCfg {
    pub rbac_path: PathBuf,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub cookie_path: String,
    pub session_ttl: Duration,
    pub session_idle: Duration,
    pub bootstrap_admin_username: String,
    pub bootstrap_admin_password_env: String,
}

/// Auth engine: RBAC store + sessions.
#[derive(Debug)]
pub struct LocalAuth {
    cfg: LocalAuthCfg,
    rbac: Arc<Mutex<RbacStore>>,
    sessions: Arc<Mutex<SessionStore>>,
}

impl LocalAuth {
    pub fn new(cfg: LocalAuthCfg) -> Result<Self, std::io::Error> {
        let rbac = load_or_init_rbac(&cfg)?;
        let sessions = SessionStore::new(cfg.session_ttl, cfg.session_idle);

        Ok(Self {
            cfg,
            rbac: Arc::new(Mutex::new(rbac)),
            sessions: Arc::new(Mutex::new(sessions)),
        })
    }

    pub fn cfg(&self) -> &LocalAuthCfg {
        &self.cfg
    }

    /// Build a router for local auth endpoints.
    ///
    /// NOTE: This generic form is useful if you have `FromRef` wiring.
    /// In svc-admin we additionally mount wrappers in router.rs so we can avoid
    /// orphan-rule issues while still keeping these handlers reusable.
    pub fn routes<S>() -> Router<S>
    where
        Arc<LocalAuth>: FromRef<S>,
        S: Clone + Send + Sync + 'static,
    {
        Router::new()
            .route("/api/auth/login", post(login))
            .route("/api/auth/logout", post(logout))
            .route("/api/auth/me", get(me))
    }

    /// Middleware-ish helper: used by outer router to enforce auth on /api/*.
    /// Returns (maybe_ctx, maybe_set_cookie_to_refresh_idle).
    pub fn authenticate_headers(
        &self,
        headers: &HeaderMap,
    ) -> Result<(Option<UserContext>, Option<String>), AuthFail> {
        let sid = read_cookie(headers, &self.cfg.cookie_name);
        let Some(sid) = sid else {
            return Ok((None, None));
        };

        let mut store = self.sessions.lock();
        let (ctx_opt, refresh_cookie_opt) = store.authenticate(&sid);

        if let Some(refresh_sid) = refresh_cookie_opt {
            let sc = build_set_cookie(&self.cfg, &refresh_sid, Some(self.cfg.session_ttl));
            return Ok((ctx_opt, Some(sc)));
        }

        Ok((ctx_opt, None))
    }
}

// -----------------------------------------------------------------------------
// DTOs
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub username: String,
    pub roles: Vec<String>,
    pub expires_at_unix_s: i64,
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub enum AuthFail {
    Unauthorized,
    Forbidden,
    Misconfigured(String),
    Io(std::io::Error),
    Other(String),
}

impl From<std::io::Error> for AuthFail {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl IntoResponse for AuthFail {
    fn into_response(self) -> axum::response::Response {
        match self {
            AuthFail::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
            AuthFail::Forbidden => StatusCode::FORBIDDEN.into_response(),
            AuthFail::Misconfigured(msg) => {
                tracing::error!(target: "svc_admin::auth", %msg, "auth misconfigured");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AuthFail::Io(e) => {
                tracing::error!(target: "svc_admin::auth", error = %e, "auth io error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AuthFail::Other(msg) => {
                tracing::error!(target: "svc_admin::auth", %msg, "auth error");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// -----------------------------------------------------------------------------
// RBAC store (JSON)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RbacStore {
    users: HashMap<String, RbacUser>,
    roles: HashMap<String, RbacRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RbacUser {
    username: String,
    password_phc: String,
    roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RbacRole {
    name: String,
    permissions: Vec<String>,
}

fn load_or_init_rbac(cfg: &LocalAuthCfg) -> Result<RbacStore, std::io::Error> {
    if cfg.rbac_path.exists() {
        let bytes = std::fs::read(&cfg.rbac_path)?;
        let parsed: RbacStore = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        return Ok(parsed);
    }

    // Create parent dirs.
    if let Some(parent) = cfg.rbac_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Create empty RBAC store.
    let mut store = RbacStore {
        users: HashMap::new(),
        roles: HashMap::new(),
    };

    // Seed admin role with broad permissions (MVP).
    store.roles.insert(
        "admin".to_string(),
        RbacRole {
            name: "admin".to_string(),
            permissions: vec!["*".to_string()],
        },
    );

    // Optional bootstrap admin from env.
    if let Ok(pw) = std::env::var(&cfg.bootstrap_admin_password_env) {
        if !pw.trim().is_empty() {
            let username = cfg.bootstrap_admin_username.trim().to_string();
            let phc = hash_password_phc(&pw)?;
            store.users.insert(
                username.clone(),
                RbacUser {
                    username,
                    password_phc: phc,
                    roles: vec!["admin".to_string()],
                },
            );
        }
    }

    save_rbac(cfg, &store)?;
    Ok(store)
}

fn save_rbac(cfg: &LocalAuthCfg, store: &RbacStore) -> Result<(), std::io::Error> {
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&cfg.rbac_path, bytes)?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Sessions
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UserContext {
    pub username: String,
    pub roles: Vec<String>,
    pub expires_at_unix_s: i64,
}

#[derive(Debug)]
struct SessionRecord {
    ctx: UserContext,
    created_at_unix_s: i64,
    last_seen_unix_s: i64,
}

#[derive(Debug)]
struct SessionStore {
    ttl: Duration,
    idle: Duration,
    map: HashMap<String, SessionRecord>,
}

impl SessionStore {
    fn new(ttl: Duration, idle: Duration) -> Self {
        Self {
            ttl,
            idle,
            map: HashMap::new(),
        }
    }

    fn create_session(&mut self, ctx: UserContext) -> String {
        let sid = random_sid();
        let now = now_unix_s();
        self.map.insert(
            sid.clone(),
            SessionRecord {
                ctx,
                created_at_unix_s: now,
                last_seen_unix_s: now,
            },
        );
        sid
    }

    fn delete_session(&mut self, sid: &str) {
        self.map.remove(sid);
    }

    /// Returns (ctx_opt, refresh_sid_opt).
    ///
    /// If the session is valid but idle-expiring, we refresh it by rotating SID.
    fn authenticate(&mut self, sid: &str) -> (Option<UserContext>, Option<String>) {
        self.gc();

        let now = now_unix_s();
        let Some(rec) = self.map.get_mut(sid) else {
            return (None, None);
        };

        // TTL absolute expiry check
        let ttl_s = self.ttl.as_secs() as i64;
        if rec.created_at_unix_s + ttl_s <= now {
            self.map.remove(sid);
            return (None, None);
        }

        // Idle expiry check
        let idle_s = self.idle.as_secs() as i64;
        if rec.last_seen_unix_s + idle_s <= now {
            self.map.remove(sid);
            return (None, None);
        }

        // Update last seen
        rec.last_seen_unix_s = now;

        // If we're beyond half of idle window, rotate SID to refresh cookie (MVP).
        let should_rotate = rec.last_seen_unix_s - rec.created_at_unix_s > (idle_s / 2);
        if should_rotate {
            let ctx = rec.ctx.clone();
            self.map.remove(sid);
            let new_sid = self.create_session(ctx.clone());
            return (Some(ctx), Some(new_sid));
        }

        (Some(rec.ctx.clone()), None)
    }

    fn gc(&mut self) {
        let now = now_unix_s();
        let ttl_s = self.ttl.as_secs() as i64;
        let idle_s = self.idle.as_secs() as i64;
        self.map
            .retain(|_, rec| rec.created_at_unix_s + ttl_s > now && rec.last_seen_unix_s + idle_s > now);
    }
}

// -----------------------------------------------------------------------------
// Password hashing
// -----------------------------------------------------------------------------

fn hash_password_phc(password: &str) -> Result<String, std::io::Error> {
    // IMPORTANT:
    // Use the rand_core version re-exported by `argon2::password_hash` to avoid
    // rand_core version skew with rand 0.9.
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(hash.to_string())
}

fn verify_password_phc(password: &str, phc: &str) -> bool {
    let parsed = PasswordHash::new(phc);
    if parsed.is_err() {
        // Still take a verify path to reduce timing diff
        let dummy = PasswordHash::new(
            "$argon2id$v=19$m=19456,t=2,p=1$YWJjZGVmZw$3I3v2yTq3dVQn8V3Xf7l4d9yJpW7x2bVf9s7tQK0bZE",
        );
        if let Ok(dummy) = dummy {
            let _ = Argon2::default().verify_password(password.as_bytes(), &dummy);
        }
        return false;
    }

    let parsed = parsed.unwrap();
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

// -----------------------------------------------------------------------------
// Cookie helpers
// -----------------------------------------------------------------------------

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in raw.split(';') {
        let p = part.trim();
        if let Some((k, v)) = p.split_once('=') {
            if k.trim() == name {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn build_set_cookie(cfg: &LocalAuthCfg, sid: &str, max_age: Option<Duration>) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("{}={}", cfg.cookie_name, sid));
    parts.push(format!("Path={}", cfg.cookie_path));
    parts.push("HttpOnly".to_string());
    parts.push("SameSite=Lax".to_string());

    if cfg.cookie_secure {
        parts.push("Secure".to_string());
    }

    if let Some(domain) = &cfg.cookie_domain {
        parts.push(format!("Domain={domain}"));
    }

    if let Some(ma) = max_age {
        parts.push(format!("Max-Age={}", ma.as_secs()));
    }

    parts.join("; ")
}

fn build_clear_cookie(cfg: &LocalAuthCfg) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("{}=deleted", cfg.cookie_name));
    parts.push(format!("Path={}", cfg.cookie_path));
    parts.push("HttpOnly".to_string());
    parts.push("SameSite=Lax".to_string());
    parts.push("Max-Age=0".to_string());

    if cfg.cookie_secure {
        parts.push("Secure".to_string());
    }

    if let Some(domain) = &cfg.cookie_domain {
        parts.push(format!("Domain={domain}"));
    }

    parts.join("; ")
}

// -----------------------------------------------------------------------------
// Public handlers (svc-admin router mounts wrappers that call these)
// -----------------------------------------------------------------------------

pub(crate) async fn login(
    State(auth): State<Arc<LocalAuth>>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, HeaderMap, Json<MeResponse>), AuthFail> {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return Err(AuthFail::Unauthorized);
    }

    // Always do a PHC parse+verify path to reduce user-enum timing differences.
    let (user_opt, roles_opt) = {
        let store = auth.rbac.lock();
        if let Some(u) = store.users.get(&username) {
            (Some(u.password_phc.clone()), Some(u.roles.clone()))
        } else {
            (None, None)
        }
    };

    let ok = if let Some(phc) = user_opt {
        verify_password_phc(&req.password, &phc)
    } else {
        // dummy path
        verify_password_phc(
            &req.password,
            "$argon2id$v=19$m=19456,t=2,p=1$YWJjZGVmZw$3I3v2yTq3dVQn8V3Xf7l4d9yJpW7x2bVf9s7tQK0bZE",
        )
    };

    if !ok {
        return Err(AuthFail::Unauthorized);
    }

    let roles = roles_opt.unwrap_or_default();
    let now = now_unix_s();
    let expires = now + auth.cfg.session_ttl.as_secs() as i64;

    let ctx = UserContext {
        username: username.clone(),
        roles: roles.clone(),
        expires_at_unix_s: expires,
    };

    let sid = {
        let mut store = auth.sessions.lock();
        store.create_session(ctx.clone())
    };

    let mut out_headers = HeaderMap::new();
    let set_cookie = build_set_cookie(&auth.cfg, &sid, Some(auth.cfg.session_ttl));
    out_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&set_cookie).map_err(|e| AuthFail::Other(e.to_string()))?,
    );

    let resp = MeResponse {
        username,
        roles,
        expires_at_unix_s: expires,
    };

    Ok((StatusCode::OK, out_headers, Json(resp)))
}

pub(crate) async fn logout(
    State(auth): State<Arc<LocalAuth>>,
    headers: HeaderMap,
) -> Result<(StatusCode, HeaderMap), AuthFail> {
    if let Some(sid) = read_cookie(&headers, &auth.cfg.cookie_name) {
        let mut store = auth.sessions.lock();
        store.delete_session(&sid);
    }

    let mut out_headers = HeaderMap::new();
    let sc = build_clear_cookie(&auth.cfg);
    out_headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&sc).map_err(|e| AuthFail::Other(e.to_string()))?,
    );

    Ok((StatusCode::OK, out_headers))
}

pub(crate) async fn me(
    State(auth): State<Arc<LocalAuth>>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<MeResponse>), AuthFail> {
    let (ctx_opt, set_cookie_opt) = auth.authenticate_headers(&headers)?;
    let Some(ctx) = ctx_opt else {
        return Err(AuthFail::Unauthorized);
    };

    let mut out_headers = HeaderMap::new();
    if let Some(sc) = set_cookie_opt {
        out_headers.insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&sc).map_err(|e| AuthFail::Other(e.to_string()))?,
        );
    }

    Ok((
        out_headers,
        Json(MeResponse {
            username: ctx.username,
            roles: ctx.roles,
            expires_at_unix_s: ctx.expires_at_unix_s,
        }),
    ))
}

// -----------------------------------------------------------------------------
// Misc helpers
// -----------------------------------------------------------------------------

fn random_sid() -> String {
    // rand 0.9: thread_rng() renamed to rng()
    let mut rng = rand::rng();
    let s: String = (0..48).map(|_| rng.sample(Alphanumeric) as char).collect();
    format!("s_{s}")
}

fn now_unix_s() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs() as i64
}
