//! Application state and router assembly.

use crate::api;
use crate::config::ControlConfig;
use crate::db::Db;
use crate::provisioning::{ProvisioningService, SecretStore};
use crate::services::auth::AuthService;
use crate::services::crypto::CryptoService;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Json, Router};
use serde_json::json;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Database handle.
    pub db: Db,
    /// Auth service.
    pub auth: AuthService,
    /// Crypto service for secrets at rest.
    pub crypto: CryptoService,
    /// Loaded config.
    pub config: Arc<ControlConfig>,
    /// Login rate limiter state.
    pub login_limiter: Arc<LoginRateLimiter>,
    /// Provisioning job service.
    pub provisioning: ProvisioningService,
    /// RAM-only SSH secrets for provisioning jobs.
    pub secret_store: SecretStore,
}

/// Simple in-process login rate limiter (per client key).
#[derive(Default)]
pub struct LoginRateLimiter {
    inner: Mutex<HashMap<String, Vec<Instant>>>,
}

impl LoginRateLimiter {
    /// Returns true if the request is allowed.
    pub fn check(&self, key: &str, limit: usize, window: Duration) -> bool {
        let mut guard = self.inner.lock().expect("login limiter lock");
        let now = Instant::now();
        let entries = guard.entry(key.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < window);
        if entries.len() >= limit {
            return false;
        }
        entries.push(now);
        true
    }
}

/// Builds the HTTP application with middleware and optional static SPA.
pub fn build_app(state: AppState) -> Router {
    let static_dir = state.config.static_dir.clone();
    let api = api::router(state.clone()).route(
        "/api/{*rest}",
        any(|| async { (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))) }),
    );

    let mut app = Router::new().merge(api).layer(from_fn_with_state(
        state.clone(),
        login_rate_limit_with_state,
    ));

    if let Some(dir) = static_dir.filter(|p| p.is_dir()) {
        let fallback = {
            let spa = dir.join("200.html");
            if spa.exists() {
                spa
            } else {
                dir.join("index.html")
            }
        };
        // Use `fallback` (not `not_found_service`) so SPA routes keep HTTP 200.
        let serve = ServeDir::new(dir.clone())
            .append_index_html_on_directories(true)
            .fallback(ServeFile::new(fallback));
        app = app
            .fallback_service(serve)
            .layer(from_fn(static_cache_headers));
    } else {
        app = app
            .fallback(|| async { (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))) });
    }

    app.layer(TraceLayer::new_for_http()).with_state(state)
}

async fn static_cache_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    if path.starts_with("/_app/immutable/") {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    } else if path.ends_with(".html")
        || path == "/"
        || !path.contains('.')
        || path.ends_with("/200.html")
    {
        // SPA HTML / route fallbacks
        let ctype = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if ctype.starts_with("text/html") || !path.contains('.') {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        }
    }
    response
}

async fn login_rate_limit_with_state(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if request.uri().path() == "/api/auth/login" && request.method() == axum::http::Method::POST {
        let key = client_ip_key(&state, &request);
        if !state.login_limiter.check(&key, 20, Duration::from_secs(60)) {
            return (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
        }
    }
    next.run(request).await
}

fn client_ip_key(state: &AppState, request: &Request<Body>) -> String {
    if state.config.trust_forwarded_headers {
        if let Some(xff) = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
        {
            return xff
                .split(',')
                .next()
                .unwrap_or("unknown")
                .trim()
                .to_string();
        }
    }
    "local".into()
}

/// Constructs [`AppState`] from config and an open database.
pub fn build_state(config: ControlConfig, db: Db) -> anyhow::Result<AppState> {
    let master_key = config.require_master_key()?;
    let crypto = CryptoService::new(master_key);
    let auth = AuthService::new(db.clone(), config.session_hours, config.cookie_secure);
    let provisioning = ProvisioningService::new(db.clone());
    Ok(AppState {
        db,
        auth,
        crypto,
        config: Arc::new(config),
        login_limiter: Arc::new(LoginRateLimiter::default()),
        provisioning,
        secret_store: SecretStore::default(),
    })
}

/// Parses the HTTP bind address, falling back to localhost.
pub fn bind_addr(bind: &str) -> std::net::SocketAddr {
    bind.parse()
        .unwrap_or_else(|_| std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080))
}

/// Used by tests that need a temporary static dir path.
pub fn static_dir_hint(path: PathBuf) -> PathBuf {
    path
}
