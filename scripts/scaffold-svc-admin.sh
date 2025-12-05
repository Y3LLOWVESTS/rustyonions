#!/usr/bin/env bash
set -euo pipefail

# Scaffolds the svc-admin crate under crates/svc-admin.
# - Creates directories.
# - Populates minimal starter files for Rust service + React/TS SPA.
# - Skips any files that already exist.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
crate_dir="${repo_root}/crates/svc-admin"

echo "Scaffolding svc-admin into: ${crate_dir}"

mkdir -p "${crate_dir}"

create_dir() {
  mkdir -p "$1"
}

# IMPORTANT: single-quoted heredoc (<<'EOF') so Bash does NOT expand ${...} etc.
create_file() {
  local path="$1"
  shift || true
  if [ -e "${path}" ]; then
    echo "SKIP (exists): ${path}"
    return 0
  fi
  mkdir -p "$(dirname "${path}")"
  cat > "${path}" <<'EOF'
'"$*"'
EOF
  # Remove the extra quoting wrapper we just inserted
  # (we wrote "$*" as literal content; now replace the file with the args)
  # If no content was passed, file will be empty.
  if [ "$#" -gt 0 ]; then
    # Rewrite file with the actual content in "$*"
    cat > "${path}" <<'EOF'
'"$*"'
EOF
  else
    # If no content, truncate to empty
    : > "${path}"
  fi
  echo "CREATE: ${path}"
}

# The above generic create_file is slightly awkward because we want <<'EOF'
# but also want to pass multi-line content. To avoid weirdness, we will NOT
# use the "$*" trick below. Instead, we will call cat <<'EOF' directly in place.
# So redefine create_file simpler, now that we explained the issue.

create_file() {
  local path="$1"
  shift || true
  if [ -e "${path}" ]; then
    echo "SKIP (exists): ${path}"
    return 0
  fi
  mkdir -p "$(dirname "${path}")"
  # Content will be provided by the caller via a subsequent heredoc.
  # This function only creates the file and echoes the path.
  : > "${path}"
  echo "CREATE: ${path}"
}

########################################
# Top-level files
########################################

create_file "${crate_dir}/Cargo.toml"
cat > "${crate_dir}/Cargo.toml" <<'EOF'
[package]
name = "svc-admin"
version = "0.1.0"
edition = "2021"
description = "Admin-plane HTTP + SPA service for RON-CORE nodes"
license = "MIT OR Apache-2.0"
publish = false

[package.metadata]
# See docs/IDB.md and docs/GOVERNANCE.MD for invariants and governance.

[lib]
name = "svc_admin"
path = "src/lib.rs"

[[bin]]
name = "svc-admin"
path = "src/bin/svc-admin.rs"

[features]
default = ["axum", "tokio", "serde"]
tls = ["tokio-rustls"]        # TLS for serving HTTPS directly
otel = []                     # Optional OpenTelemetry exporters
passport = []                 # Optional passport/JWT validation mode

[dependencies]
anyhow = "1"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
axum = { version = "0.7", features = ["tokio", "http1", "http2", "json"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls-native-roots"] }
prometheus = "0.14"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
tokio-rustls = { version = "0.26", optional = true }
url = "2"

[dev-dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "rustls-tls-native-roots"] }
EOF

create_file "${crate_dir}/README.md"
cat > "${crate_dir}/README.md" <<'EOF'
# svc-admin

> **Role:** Admin-plane GUI + HTTP/JSON proxy for RON-CORE nodes  
> **Status:** draft (scaffolded)

See docs/IDB.md, docs/API.MD, and docs/OBSERVABILITY.MD for the canonical design.

This file was scaffolded; replace with the full README once the crate is wired.
EOF

create_file "${crate_dir}/CHANGELOG.md"
cat > "${crate_dir}/CHANGELOG.md" <<'EOF'
# Changelog — svc-admin

All notable changes to this crate will be documented in this file.

## [0.1.0] - UNRELEASED
- Initial scaffold of svc-admin (crate structure, UI skeleton, docs placeholders).
EOF

create_file "${crate_dir}/build.rs"
cat > "${crate_dir}/build.rs" <<'EOF'
#![allow(clippy::all)]

fn main() {
    // In the future, this build script can:
    // - Run the UI build (ui/ -> ui/dist)
    // - Embed static assets into the binary (e.g., via include_dir or rust-embed)
    // For now, it is a no-op scaffold.
}
EOF

create_file "${crate_dir}/.gitignore"
cat > "${crate_dir}/.gitignore" <<'EOF'
# Rust
/target/
/**/target/

# Local configs
*.log
*.tmp
*.bak

# UI
ui/node_modules/
ui/dist/
ui/.cache/

# Generated static assets
static/
EOF

########################################
# src/ core modules
########################################

create_dir "${crate_dir}/src"
create_dir "${crate_dir}/src/bin"
create_dir "${crate_dir}/src/auth"
create_dir "${crate_dir}/src/nodes"
create_dir "${crate_dir}/src/metrics"
create_dir "${crate_dir}/src/dto"

create_file "${crate_dir}/src/lib.rs"
cat > "${crate_dir}/src/lib.rs" <<'EOF'
pub mod cli;
pub mod config;
pub mod dto;
pub mod error;
pub mod metrics;
pub mod nodes;
pub mod observability;
pub mod router;
pub mod server;
pub mod state;
pub mod interop;

pub use crate::config::Config;
pub use crate::error::Error;
EOF

create_file "${crate_dir}/src/cli.rs"
cat > "${crate_dir}/src/cli.rs" <<'EOF'
use crate::config::Config;
use anyhow::Result;

/// Parse CLI arguments and environment variables to produce a Config.
pub fn parse_args() -> Result<Config> {
    // TODO: Use clap or similar for real CLI parsing.
    // For now, delegate directly to Config::load().
    Config::load()
}
EOF

create_file "${crate_dir}/src/error.rs"
cat > "${crate_dir}/src/error.rs" <<'EOF'
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("upstream node error: {0}")]
    Upstream(String),

    #[error("other: {0}")]
    Other(String),
}
EOF

create_file "${crate_dir}/src/config.rs"
cat > "${crate_dir}/src/config.rs" <<'EOF'
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerCfg,
    pub auth: AuthCfg,
    pub ui: UiCfg,
    pub nodes: NodesCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCfg {
    pub bind_addr: String,
    pub metrics_addr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCfg {
    pub mode: String, // "none" | "ingress" | "passport"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiCfg {
    pub default_theme: String,
    pub default_language: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodesCfg {
    // TODO: Fill with node registry schema
}

impl Config {
    pub fn load() -> Result<Self> {
        // TODO: merge env vars, CLI args, and config file per docs/CONFIG.MD
        Err(Error::Config("Config::load() not implemented".into()))
    }
}
EOF

create_file "${crate_dir}/src/server.rs"
cat > "${crate_dir}/src/server.rs" <<'EOF'
use crate::config::Config;
use crate::observability;
use crate::router::build_router;
use crate::state::AppState;
use crate::error::Result;
use axum::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

pub async fn run(config: Config) -> Result<()> {
    observability::init_tracing();

    let state = Arc::new(AppState::new(config.clone()));
    let app = build_router(state.clone());

    let addr: SocketAddr = config
        .server
        .bind_addr
        .parse()
        .expect("invalid bind_addr in config");

    tracing::info!(%addr, "svc-admin listening");

    Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| crate::error::Error::Other(e.to_string()))
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
EOF

create_file "${crate_dir}/src/router.rs"
cat > "${crate_dir}/src/router.rs" <<'EOF'
use crate::dto;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/ui-config", get(ui_config))
        .route("/api/me", get(me))
        .route("/api/nodes", get(nodes))
        .route("/api/nodes/:id/status", get(node_status))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ready": true }))
}

async fn ui_config(State(state): State<Arc<AppState>>) -> Json<dto::ui::UiConfigDto> {
    Json(dto::ui::UiConfigDto::from_cfg(&state.config))
}

async fn me() -> Json<dto::me::MeResponse> {
    Json(dto::me::MeResponse::dev_default())
}

async fn nodes(State(_state): State<Arc<AppState>>) -> Json<Vec<dto::node::NodeSummary>> {
    Json(vec![]) // TODO: list nodes from registry
}

async fn node_status(
    State(_state): State<Arc<AppState>>,
) -> Json<dto::node::AdminStatusView> {
    Json(dto::node::AdminStatusView::placeholder())
}
EOF

create_file "${crate_dir}/src/state.rs"
cat > "${crate_dir}/src/state.rs" <<'EOF'
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    // TODO: add node registry, metrics buffers, auth caches, etc.
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
EOF

create_file "${crate_dir}/src/observability.rs"
cat > "${crate_dir}/src/observability.rs" <<'EOF'
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,svc_admin=info,axum=warn,tower_http=warn"));

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .finish()
        .init();
}
EOF

create_file "${crate_dir}/src/interop.rs"
cat > "${crate_dir}/src/interop.rs" <<'EOF'
// Interop helper module for tests and SDK harnesses.
// See docs/INTEROP.MD for the invariants and supported flows.

pub struct InteropHarness;

impl InteropHarness {
    pub fn new() -> Self {
        Self
    }
}
EOF

########################################
# src/auth
########################################

create_file "${crate_dir}/src/auth/mod.rs"
cat > "${crate_dir}/src/auth/mod.rs" <<'EOF'
pub mod none;
pub mod ingress;
pub mod passport;

// TODO: define a common AuthMode trait and identity type for /api/me.
EOF

create_file "${crate_dir}/src/auth/none.rs"
cat > "${crate_dir}/src/auth/none.rs" <<'EOF'
// auth.mode = "none" — dev mode only.

pub fn dev_identity() -> String {
    "dev-operator".to_string()
}
EOF

create_file "${crate_dir}/src/auth/ingress.rs"
cat > "${crate_dir}/src/auth/ingress.rs" <<'EOF'
// auth.mode = "ingress" — trust headers set by ingress/proxy.
// TODO: extract subject and roles from headers.
EOF

create_file "${crate_dir}/src/auth/passport.rs"
cat > "${crate_dir}/src/auth/passport.rs" <<'EOF'
// auth.mode = "passport" — validate JWT/passport tokens.
// TODO: implement JWKS fetch, token validation, and role extraction.
EOF

########################################
# src/nodes
########################################

create_file "${crate_dir}/src/nodes/mod.rs"
cat > "${crate_dir}/src/nodes/mod.rs" <<'EOF'
pub mod registry;
pub mod client;
pub mod status;
EOF

create_file "${crate_dir}/src/nodes/registry.rs"
cat > "${crate_dir}/src/nodes/registry.rs" <<'EOF'
use crate::config::NodesCfg;

// TODO: implement an in-memory registry for node configurations.
pub struct NodeRegistry {
    pub cfg: NodesCfg,
}
EOF

create_file "${crate_dir}/src/nodes/client.rs"
cat > "${crate_dir}/src/nodes/client.rs" <<'EOF'
use crate::error::Result;

// TODO: wrap reqwest::Client and talk to node admin endpoints.
pub struct NodeClient;

impl NodeClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn ping_node(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
EOF

create_file "${crate_dir}/src/nodes/status.rs"
cat > "${crate_dir}/src/nodes/status.rs" <<'EOF'
use crate::dto::node::AdminStatusView;

// TODO: normalize node health/ready/version/status into AdminStatusView.
pub fn build_status_placeholder() -> AdminStatusView {
    AdminStatusView::placeholder()
}
EOF

########################################
# src/metrics
########################################

create_file "${crate_dir}/src/metrics/mod.rs"
cat > "${crate_dir}/src/metrics/mod.rs" <<'EOF'
pub mod sampler;
pub mod facet;
pub mod prometheus_bridge;
EOF

create_file "${crate_dir}/src/metrics/sampler.rs"
cat > "${crate_dir}/src/metrics/sampler.rs" <<'EOF'
// Background tasks that scrape node /metrics and status.
// TODO: implement sampler loops and rolling windows.
EOF

create_file "${crate_dir}/src/metrics/facet.rs"
cat > "${crate_dir}/src/metrics/facet.rs" <<'EOF'
// Facet-aware metrics logic.
// TODO: group metrics by facet label/prefix and compute summaries.
EOF

create_file "${crate_dir}/src/metrics/prometheus_bridge.rs"
cat > "${crate_dir}/src/metrics/prometheus_bridge.rs" <<'EOF'
// Bridge between scraped metrics and exported Prometheus metrics.
// TODO: register and update svc-admin metrics (svc_admin_node_fanout_*, etc.).
EOF

########################################
# src/dto
########################################

create_file "${crate_dir}/src/dto/mod.rs"
cat > "${crate_dir}/src/dto/mod.rs" <<'EOF'
pub mod ui;
pub mod me;
pub mod node;
pub mod metrics;
EOF

create_file "${crate_dir}/src/dto/ui.rs"
cat > "${crate_dir}/src/dto/ui.rs" <<'EOF'
use serde::{Deserialize, Serialize};
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfigDto {
    pub default_theme: String,
    pub available_themes: Vec<String>,
    pub default_language: String,
    pub available_languages: Vec<String>,
    pub read_only: bool,
}

impl UiConfigDto {
    pub fn from_cfg(cfg: &Config) -> Self {
        Self {
            default_theme: cfg.ui.default_theme.clone(),
            available_themes: vec!["light".into(), "dark".into()],
            default_language: cfg.ui.default_language.clone(),
            available_languages: vec!["en-US".into(), "es-ES".into()],
            read_only: cfg.ui.read_only,
        }
    }
}
EOF

create_file "${crate_dir}/src/dto/me.rs"
cat > "${crate_dir}/src/dto/me.rs" <<'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub subject: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub auth_mode: String,
    pub login_url: Option<String>,
}

impl MeResponse {
    pub fn dev_default() -> Self {
        Self {
            subject: "dev-operator".into(),
            display_name: "Dev Operator".into(),
            roles: vec!["admin".into()],
            auth_mode: "none".into(),
            login_url: None,
        }
    }
}
EOF

create_file "${crate_dir}/src/dto/node.rs"
cat > "${crate_dir}/src/dto/node.rs" <<'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub display_name: String,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaneStatus {
    pub name: String,
    pub health: String,
    pub ready: bool,
    pub restart_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminStatusView {
    pub id: String,
    pub display_name: String,
    pub profile: Option<String>,
    pub version: Option<String>,
    pub planes: Vec<PlaneStatus>,
}

impl AdminStatusView {
    pub fn placeholder() -> Self {
        Self {
            id: "example-node".into(),
            display_name: "Example Node".into(),
            profile: Some("macronode".into()),
            version: Some("0.0.0".into()),
            planes: vec![],
        }
    }
}
EOF

create_file "${crate_dir}/src/dto/metrics.rs"
cat > "${crate_dir}/src/dto/metrics.rs" <<'EOF'
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetMetricsSummary {
    pub facet: String,
    pub rps: f64,
    pub error_rate: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}
EOF

########################################
# src/bin
########################################

create_file "${crate_dir}/src/bin/svc-admin.rs"
cat > "${crate_dir}/src/bin/svc-admin.rs" <<'EOF'
use svc_admin::{cli, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = cli::parse_args()?;
    server::run(cfg).await?;
    Ok(())
}
EOF

########################################
# docs (only create if missing)
########################################

docs_dir="${crate_dir}/docs"
create_dir "${docs_dir}"

for f in API.MD CONCURRENCY.MD CONFIG.MD GOVERNANCE.MD IDB.md INTEROP.MD OBSERVABILITY.MD PERFORMANCE.MD RUNBOOK.MD SECURITY.MD TESTS.MD; do
  if [ ! -e "${docs_dir}/${f}" ]; then
    create_file "${docs_dir}/${f}"
    cat > "${docs_dir}/${f}" <<'EOF'
Scaffold placeholder. See ALL_DOCS.md for the canonical version.
EOF
  else
    echo "SKIP (exists): ${docs_dir}/${f}"
  fi
done

########################################
# ui (React + TS)
########################################

ui_dir="${crate_dir}/ui"
create_dir "${ui_dir}"

create_file "${ui_dir}/package.json"
cat > "${ui_dir}/package.json" <<'EOF'
{
  "name": "svc-admin-ui",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "lint": "eslint src --ext .ts,.tsx"
  },
  "dependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0",
    "react-router-dom": "^6.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.0.0",
    "@types/react-dom": "^18.0.0",
    "@typescript-eslint/eslint-plugin": "^8.0.0",
    "@typescript-eslint/parser": "^8.0.0",
    "eslint": "^9.0.0",
    "eslint-plugin-react-hooks": "^5.0.0",
    "typescript": "^5.0.0",
    "vite": "^6.0.0",
    "@vitejs/plugin-react": "^4.0.0"
  }
}
EOF

create_file "${ui_dir}/tsconfig.json"
cat > "${ui_dir}/tsconfig.json" <<'EOF'
{
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ESNext"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "baseUrl": "./",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"]
}
EOF

create_file "${ui_dir}/vite.config.ts"
cat > "${ui_dir}/vite.config.ts" <<'EOF'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:5300',
      '/healthz': 'http://127.0.0.1:5310',
      '/readyz': 'http://127.0.0.1:5310',
      '/metrics': 'http://127.0.0.1:5310'
    }
  }
})
EOF

create_file "${ui_dir}/.eslintrc.cjs"
cat > "${ui_dir}/.eslintrc.cjs" <<'EOF'
module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 2020, sourceType: 'module' },
  env: { browser: true, es2021: true },
  plugins: ['@typescript-eslint', 'react-hooks'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended'
  ],
  rules: {
    'react-hooks/rules-of-hooks': 'error',
    'react-hooks/exhaustive-deps': 'warn'
  }
}
EOF

create_file "${ui_dir}/.prettierrc"
cat > "${ui_dir}/.prettierrc" <<'EOF'
{
  "singleQuote": true,
  "semi": false,
  "trailingComma": "es5"
}
EOF

create_file "${ui_dir}/index.html"
cat > "${ui_dir}/index.html" <<'EOF'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>RON-CORE Admin</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
EOF

########################################
# ui/public
########################################

create_dir "${ui_dir}/public"
create_dir "${ui_dir}/public/locales"

create_file "${ui_dir}/public/favicon.svg"
cat > "${ui_dir}/public/favicon.svg" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect width="64" height="64" rx="12" fill="#0b7285"/>
  <text x="50%" y="55%" text-anchor="middle" font-size="28" fill="#ffffff" font-family="system-ui, -apple-system, BlinkMacSystemFont">RON</text>
</svg>
EOF

create_file "${ui_dir}/public/robots.txt"
cat > "${ui_dir}/public/robots.txt" <<'EOF'
User-agent: *
Disallow:
EOF

create_file "${ui_dir}/public/locales/en-US.json"
cat > "${ui_dir}/public/locales/en-US.json" <<'EOF'
{
  "app.title": "RON-CORE Admin",
  "nav.nodes": "Nodes",
  "nav.settings": "Settings",
  "nav.login": "Login"
}
EOF

create_file "${ui_dir}/public/locales/es-ES.json"
cat > "${ui_dir}/public/locales/es-ES.json" <<'EOF'
{
  "app.title": "RON-CORE Admin (ES)",
  "nav.nodes": "Nodos",
  "nav.settings": "Ajustes",
  "nav.login": "Iniciar sesión"
}
EOF

########################################
# ui/src core
########################################

ui_src="${ui_dir}/src"
create_dir "${ui_src}"

create_file "${ui_src}/main.tsx"
cat > "${ui_src}/main.tsx" <<'EOF'
import React from 'react'
import ReactDOM from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import { ThemeProvider } from './theme/ThemeProvider'
import { I18nProvider } from './i18n'

const rootElement = document.getElementById('root') as HTMLElement

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <I18nProvider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </I18nProvider>
    </ThemeProvider>
  </React.StrictMode>
)
EOF

create_file "${ui_src}/App.tsx"
cat > "${ui_src}/App.tsx" <<'EOF'
import React from 'react'
import { Routes, Route } from 'react-router-dom'
import { Shell } from './components/layout/Shell'
import { NodeListPage } from './routes/NodeListPage'
import { NodeDetailPage } from './routes/NodeDetailPage'
import { SettingsPage } from './routes/SettingsPage'
import { LoginPage } from './routes/LoginPage'
import { NotFoundPage } from './routes/NotFoundPage'

export default function App() {
  return (
    <Shell>
      <Routes>
        <Route path="/" element={<NodeListPage />} />
        <Route path="/nodes/:id" element={<NodeDetailPage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Routes>
    </Shell>
  )
}
EOF

########################################
# ui/src/routes
########################################

routes_dir="${ui_src}/routes"
create_dir "${routes_dir}"

create_file "${routes_dir}/index.tsx"
cat > "${routes_dir}/index.tsx" <<'EOF'
// Route definitions are centralized in App.tsx for now.
// This file exists as a future extension point if we move to route objects.
EOF

create_file "${routes_dir}/NodeListPage.tsx"
cat > "${routes_dir}/NodeListPage.tsx" <<'EOF'
import React from 'react'

export function NodeListPage() {
  return (
    <div>
      <h1>Nodes</h1>
      <p>Node list will appear here.</p>
    </div>
  )
}
EOF

create_file "${routes_dir}/NodeDetailPage.tsx"
cat > "${routes_dir}/NodeDetailPage.tsx" <<'EOF'
import React from 'react'

export function NodeDetailPage() {
  return (
    <div>
      <h1>Node Detail</h1>
      <p>Node status and facet metrics will appear here.</p>
    </div>
  )
}
EOF

create_file "${routes_dir}/SettingsPage.tsx"
cat > "${routes_dir}/SettingsPage.tsx" <<'EOF'
import React from 'react'

export function SettingsPage() {
  return (
    <div>
      <h1>Settings</h1>
      <p>Theme, language, and UI preferences will appear here.</p>
    </div>
  )
}
EOF

create_file "${routes_dir}/LoginPage.tsx"
cat > "${routes_dir}/LoginPage.tsx" <<'EOF'
import React from 'react'

export function LoginPage() {
  return (
    <div>
      <h1>Login</h1>
      <p>Passport / SSO login flow will appear here.</p>
    </div>
  )
}
EOF

create_file "${routes_dir}/NotFoundPage.tsx"
cat > "${routes_dir}/NotFoundPage.tsx" <<'EOF'
import React from 'react'

export function NotFoundPage() {
  return (
    <div>
      <h1>404</h1>
      <p>Page not found.</p>
    </div>
  )
}
EOF

########################################
# ui/src/components
########################################

components_dir="${ui_src}/components"
create_dir "${components_dir}"
create_dir "${components_dir}/layout"
create_dir "${components_dir}/nodes"
create_dir "${components_dir}/metrics"
create_dir "${components_dir}/shared"

create_file "${components_dir}/layout/Shell.tsx"
cat > "${components_dir}/layout/Shell.tsx" <<'EOF'
import React from 'react'
import { Sidebar } from './Sidebar'
import { TopBar } from './TopBar'

type Props = {
  children: React.ReactNode
}

export function Shell({ children }: Props) {
  return (
    <div className="svc-admin-shell">
      <Sidebar />
      <div className="svc-admin-main">
        <TopBar />
        <main>{children}</main>
      </div>
    </div>
  )
}
EOF

create_file "${components_dir}/layout/Sidebar.tsx"
cat > "${components_dir}/layout/Sidebar.tsx" <<'EOF'
import React from 'react'
import { Link } from 'react-router-dom'

export function Sidebar() {
  return (
    <aside className="svc-admin-sidebar">
      <h2>RON-CORE</h2>
      <nav>
        <ul>
          <li><Link to="/">Nodes</Link></li>
          <li><Link to="/settings">Settings</Link></li>
        </ul>
      </nav>
    </aside>
  )
}
EOF

create_file "${components_dir}/layout/TopBar.tsx"
cat > "${components_dir}/layout/TopBar.tsx" <<'EOF'
import React from 'react'
import { ThemeToggle } from './ThemeToggle'
import { LanguageSwitcher } from './LanguageSwitcher'

export function TopBar() {
  return (
    <header className="svc-admin-topbar">
      <div className="svc-admin-topbar-left">
        <span>RON-CORE Admin</span>
      </div>
      <div className="svc-admin-topbar-right">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>
    </header>
  )
}
EOF

create_file "${components_dir}/layout/ThemeToggle.tsx"
cat > "${components_dir}/layout/ThemeToggle.tsx" <<'EOF'
import React from 'react'

export function ThemeToggle() {
  return (
    <button type="button">
      Theme
    </button>
  )
}
EOF

create_file "${components_dir}/layout/LanguageSwitcher.tsx"
cat > "${components_dir}/layout/LanguageSwitcher.tsx" <<'EOF'
import React from 'react'

export function LanguageSwitcher() {
  return (
    <select defaultValue="en-US">
      <option value="en-US">EN</option>
      <option value="es-ES">ES</option>
    </select>
  )
}
EOF

create_file "${components_dir}/nodes/NodeCard.tsx"
cat > "${components_dir}/nodes/NodeCard.tsx" <<'EOF'
import React from 'react'

export function NodeCard() {
  return (
    <div className="svc-admin-node-card">
      <h3>Example Node</h3>
      <p>Status: healthy</p>
    </div>
  )
}
EOF

create_file "${components_dir}/nodes/NodeStatusBadge.tsx"
cat > "${components_dir}/nodes/NodeStatusBadge.tsx" <<'EOF'
import React from 'react'

type Props = {
  status: 'healthy' | 'degraded' | 'down'
}

export function NodeStatusBadge({ status }: Props) {
  return (
    <span className={`svc-admin-node-status svc-admin-node-status-${status}`}>
      {status}
    </span>
  )
}
EOF

create_file "${components_dir}/nodes/PlaneStatusTable.tsx"
cat > "${components_dir}/nodes/PlaneStatusTable.tsx" <<'EOF'
import React from 'react'

export function PlaneStatusTable() {
  return (
    <table>
      <thead>
        <tr>
          <th>Plane</th>
          <th>Health</th>
          <th>Ready</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>gateway</td>
          <td>healthy</td>
          <td>true</td>
        </tr>
      </tbody>
    </table>
  )
}
EOF

create_file "${components_dir}/metrics/MetricChart.tsx"
cat > "${components_dir}/metrics/MetricChart.tsx" <<'EOF'
import React from 'react'

export function MetricChart() {
  return (
    <div>
      <p>Metric chart placeholder</p>
    </div>
  )
}
EOF

create_file "${components_dir}/metrics/FacetMetricsPanel.tsx"
cat > "${components_dir}/metrics/FacetMetricsPanel.tsx" <<'EOF'
import React from 'react'

export function FacetMetricsPanel() {
  return (
    <section>
      <h2>Facet Metrics</h2>
      <p>Facet-aware metrics will appear here.</p>
    </section>
  )
}
EOF

create_file "${components_dir}/shared/LoadingSpinner.tsx"
cat > "${components_dir}/shared/LoadingSpinner.tsx" <<'EOF'
import React from 'react'

export function LoadingSpinner() {
  return <div>Loading...</div>
}
EOF

create_file "${components_dir}/shared/ErrorBanner.tsx"
cat > "${components_dir}/shared/ErrorBanner.tsx" <<'EOF'
import React from 'react'

type Props = { message: string }

export function ErrorBanner({ message }: Props) {
  return (
    <div className="svc-admin-error-banner">
      {message}
    </div>
  )
}
EOF

create_file "${components_dir}/shared/EmptyState.tsx"
cat > "${components_dir}/shared/EmptyState.tsx" <<'EOF'
import React from 'react'

type Props = { message: string }

export function EmptyState({ message }: Props) {
  return (
    <div className="svc-admin-empty-state">
      {message}
    </div>
  )
}
EOF

########################################
# ui/src/api
########################################

api_dir="${ui_src}/api"
create_dir "${api_dir}"

create_file "${api_dir}/adminClient.ts"
cat > "${api_dir}/adminClient.ts" <<'EOF'
import type { UiConfigDto, MeResponse, NodeSummary, AdminStatusView } from '../types/admin-api'

const base = ''

async function getJson<T>(path: string): Promise<T> {
  const rsp = await fetch(base + path)
  if (!rsp.ok) {
    throw new Error(`Request failed: ${rsp.status}`)
  }
  return rsp.json() as Promise<T>
}

export const adminClient = {
  getUiConfig: () => getJson<UiConfigDto>('/api/ui-config'),
  getMe: () => getJson<MeResponse>('/api/me'),
  getNodes: () => getJson<NodeSummary[]>('/api/nodes'),
  getNodeStatus: (id: string) => getJson<AdminStatusView>(`/api/nodes/${id}/status`)
}
EOF

create_file "${api_dir}/ronCorePlaygroundClient.ts"
cat > "${api_dir}/ronCorePlaygroundClient.ts" <<'EOF'
// Placeholder for integration with ron-app-sdk-ts
// This will power a future "App Plane Playground" inside svc-admin.
EOF

########################################
# ui/src/i18n
########################################

i18n_dir="${ui_src}/i18n"
create_dir "${i18n_dir}"

create_file "${i18n_dir}/index.ts"
cat > "${i18n_dir}/index.ts" <<'EOF'
import React, { createContext, useContext, useState, useEffect } from 'react'

type I18nContextValue = {
  locale: string
  t: (key: string) => string
  setLocale: (locale: string) => void
}

const I18nContext = createContext<I18nContextValue | undefined>(undefined)

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [locale, setLocale] = useState('en-US')
  const [messages, setMessages] = useState<Record<string, string>>({})

  useEffect(() => {
    fetch(`/locales/${locale}.json`)
      .then((rsp) => rsp.json())
      .then((data) => setMessages(data))
      .catch(() => setMessages({}))
  }, [locale])

  const t = (key: string) => messages[key] ?? key

  return (
    <I18nContext.Provider value={{ locale, t, setLocale }}>
      {children}
    </I18nContext.Provider>
  )
}

export function useI18n() {
  const ctx = useContext(I18nContext)
  if (!ctx) throw new Error('useI18n must be used within I18nProvider')
  return ctx
}
EOF

create_file "${i18n_dir}/useI18n.ts"
cat > "${i18n_dir}/useI18n.ts" <<'EOF'
export { useI18n } from './index'
EOF

########################################
# ui/src/theme
########################################

theme_dir="${ui_src}/theme"
create_dir "${theme_dir}"

create_file "${theme_dir}/ThemeProvider.tsx"
cat > "${theme_dir}/ThemeProvider.tsx" <<'EOF'
import React, { createContext, useContext, useState, useEffect } from 'react'
import { themes } from './themes'

type Theme = keyof typeof themes

type ThemeContextValue = {
  theme: Theme
  setTheme: (theme: Theme) => void
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState<Theme>('light')

  useEffect(() => {
    document.documentElement.dataset.theme = theme
  }, [theme])

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider')
  return ctx
}
EOF

create_file "${theme_dir}/tokens.ts"
cat > "${theme_dir}/tokens.ts" <<'EOF'
export const tokens = {
  radius: {
    sm: 4,
    md: 8,
    lg: 12
  }
}
EOF

create_file "${theme_dir}/themes.ts"
cat > "${theme_dir}/themes.ts" <<'EOF'
export const themes = {
  light: {
    background: '#f8fafc',
    foreground: '#0f172a'
  },
  dark: {
    background: '#020617',
    foreground: '#e2e8f0'
  }
}
EOF

########################################
# ui/src/templates
########################################

templates_dir="${ui_src}/templates"
create_dir "${templates_dir}"

create_file "${templates_dir}/index.ts"
cat > "${templates_dir}/index.ts" <<'EOF'
// Template registry for custom dashboards.
// TODO: expose a stable API for devs to register their own templates.
EOF

create_file "${templates_dir}/NodeOverviewTemplate.tsx"
cat > "${templates_dir}/NodeOverviewTemplate.tsx" <<'EOF'
import React from 'react'

export function NodeOverviewTemplate() {
  return (
    <section>
      <h2>Node Overview</h2>
      <p>Default overview template.</p>
    </section>
  )
}
EOF

create_file "${templates_dir}/FacetMetricsTemplate.tsx"
cat > "${templates_dir}/FacetMetricsTemplate.tsx" <<'EOF'
import React from 'react'

export function FacetMetricsTemplate() {
  return (
    <section>
      <h2>Facet Metrics</h2>
      <p>Default facet metrics template.</p>
    </section>
  )
}
EOF

create_file "${templates_dir}/CustomTemplateRegistry.tsx"
cat > "${templates_dir}/CustomTemplateRegistry.tsx" <<'EOF'
import React from 'react'

// Placeholder for a future dynamic template registry.
export function CustomTemplateRegistry() {
  return (
    <section>
      <h2>Custom Templates</h2>
      <p>Custom template support will be added here.</p>
    </section>
  )
}
EOF

########################################
# ui/src/types
########################################

types_dir="${ui_src}/types"
create_dir "${types_dir}"

create_file "${types_dir}/admin-api.ts"
cat > "${types_dir}/admin-api.ts" <<'EOF'
export type UiConfigDto = {
  default_theme: string
  available_themes: string[]
  default_language: string
  available_languages: string[]
  read_only: boolean
}

export type MeResponse = {
  subject: string
  display_name: string
  roles: string[]
  auth_mode: string
  login_url: string | null
}

export type NodeSummary = {
  id: string
  display_name: string
  profile?: string | null
}

export type PlaneStatus = {
  name: string
  health: string
  ready: boolean
  restart_count: number
}

export type AdminStatusView = {
  id: string
  display_name: string
  profile?: string | null
  version?: string | null
  planes: PlaneStatus[]
}
EOF

########################################
# static
########################################

static_dir="${crate_dir}/static"
create_dir "${static_dir}"

create_file "${static_dir}/README.md"
cat > "${static_dir}/README.md" <<'EOF'
# static/ — svc-admin

Built UI assets (from ui/dist) will eventually be placed here or embedded via build.rs.

This directory is scaffolded; contents may be overwritten by the UI build pipeline.
EOF

########################################
# scripts
########################################

scripts_dir="${crate_dir}/scripts"
create_dir "${scripts_dir}"

create_file "${scripts_dir}/build-ui.sh"
cat > "${scripts_dir}/build-ui.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Build the svc-admin UI and copy artifacts into static/ (or for embedding).

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

cd "${crate_dir}/ui"
npm install
npm run build

mkdir -p "${crate_dir}/static"
cp -R dist/* "${crate_dir}/static/"
EOF
chmod +x "${scripts_dir}/build-ui.sh"

create_file "${scripts_dir}/dev-ui.sh"
cat > "${scripts_dir}/dev-ui.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Start the UI dev server.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

cd "${crate_dir}/ui"
npm install
npm run dev
EOF
chmod +x "${scripts_dir}/dev-ui.sh"

create_file "${scripts_dir}/lint-ui.sh"
cat > "${scripts_dir}/lint-ui.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

cd "${crate_dir}/ui"
npm install
npm run lint
EOF
chmod +x "${scripts_dir}/lint-ui.sh"

create_file "${scripts_dir}/sync-ui-assets.sh"
cat > "${scripts_dir}/sync-ui-assets.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Convenience wrapper: build the UI and sync assets.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/.." && pwd)"

"${crate_dir}/scripts/build-ui.sh"
EOF
chmod +x "${scripts_dir}/sync-ui-assets.sh"

########################################
# tests
########################################

tests_dir="${crate_dir}/tests"
create_dir "${tests_dir}"

create_file "${tests_dir}/http_smoke.rs"
cat > "${tests_dir}/http_smoke.rs" <<'EOF'
use svc_admin::server;
use svc_admin::config::{Config, ServerCfg, AuthCfg, UiCfg, NodesCfg};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn healthz_smoke() {
    let cfg = Config {
        server: ServerCfg {
            bind_addr: "127.0.0.1:5300".into(),
            metrics_addr: "127.0.0.1:5310".into(),
        },
        auth: AuthCfg { mode: "none".into() },
        ui: UiCfg {
            default_theme: "light".into(),
            default_language: "en-US".into(),
            read_only: true,
        },
        nodes: NodesCfg {},
    };

    tokio::spawn(async move {
        let _ = server::run(cfg).await;
    });

    sleep(Duration::from_millis(200)).await;

    let body = reqwest::get("http://127.0.0.1:5310/healthz")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert_eq!(body, "ok");
}
EOF

create_file "${tests_dir}/fake_node.rs"
cat > "${tests_dir}/fake_node.rs" <<'EOF'
// TODO: implement fake node admin endpoints for integration tests.
// This will validate normalization of AdminStatusView and facet metrics.
EOF

echo "Done. svc-admin scaffolded. Next steps: wire Config::load(), real handlers, and UI <> API types."
