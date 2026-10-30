//! Application state and router assembly.

use crate::api;
use crate::config::ControlConfig;
use crate::db::Db;
use crate::services::auth::AuthService;
use crate::services::crypto::CryptoService;
use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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

/// Builds the HTTP application with middleware.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .merge(api::router(state.clone()))
        .layer(from_fn_with_state(
            state.clone(),
            login_rate_limit_with_state,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn login_rate_limit_with_state(
    axum::extract::State(state): axum::extract::State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if request.uri().path() == "/api/auth/login" && request.method() == axum::http::Method::POST {
        let key = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("local")
            .to_string();
        if !state
            .login_limiter
            .check(&key, 20, Duration::from_secs(60))
        {
            return (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
        }
    }
    next.run(request).await
}

/// Constructs [`AppState`] from config and an open database.
pub fn build_state(config: ControlConfig, db: Db) -> anyhow::Result<AppState> {
    let master_key = config.require_master_key()?;
    let crypto = CryptoService::new(master_key);
    let auth = AuthService::new(db.clone(), config.session_hours, config.cookie_secure);
    Ok(AppState {
        db,
        auth,
        crypto,
        config: Arc::new(config),
        login_limiter: Arc::new(LoginRateLimiter::default()),
    })
}

/// Parses the HTTP bind address, falling back to localhost.
pub fn bind_addr(bind: &str) -> std::net::SocketAddr {
    bind.parse()
        .unwrap_or_else(|_| std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080))
}
